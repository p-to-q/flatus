// flatus — Tauri v2 desktop shell.
//
// Architecture:
//   - `ActivationPolicy::Accessory` → no dock icon, tray-first.
//   - Tray menu opens actions; the main webview window is the full UI.
//   - Synthesis runs in Rust via `fart-synth` + `cpal`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod idle;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    window::Color,
    AppHandle, Manager,
};
use time::OffsetDateTime;

use fart_synth::wav::write_wav_to_vec;
use fart_synth::{
    personalities::{lookup_personality, sample_params, PERSONALITIES},
    pressure::{ActivitySignal, Pressure, TickResult},
    prng::Mulberry32,
    render, safety, RenderConfig, VERSION,
};

const SETTINGS_VERSION: u32 = 2;
const SETTINGS_FILE: &str = "settings.json";
const AUDIO_BASELINE: &str = "fixtures-v0.4 + web-specimen-reference";
const DEFAULT_PREVIEW_PRESSURE: f32 = 0.6;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    version: u32,
    personality: String,
    /// "single" (always play the selected personality) or "shuffle" (pick one
    /// of the four personalities at random per fire). Manual preview/play and
    /// automatic playback both honor this mode. Defaults to "single".
    #[serde(default = "default_play_mode")]
    play_mode: String,
    /// 0–1 volume on top of the engineering cap.
    volume: f32,
    /// "speakers" or "headphones".
    output: String,
    /// Hour 0–23 for quiet-hours start. `None` = disabled.
    quiet_start: Option<u8>,
    quiet_end: Option<u8>,
    onboarding_completed: bool,
    /// When false, automatic ticks still run but no sound plays on `TickResult::Fart`.
    #[serde(default = "default_auto_play_enabled")]
    auto_play_enabled: bool,
    /// PRNG seed for the next manual `fart_now` / preview render.
    /// Rolled after each manual fire.
    #[serde(default = "default_manual_seed")]
    manual_seed: u64,
}

fn default_manual_seed() -> u64 {
    17
}

fn default_auto_play_enabled() -> bool {
    false
}

fn default_play_mode() -> String {
    "single".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            personality: "default".to_string(),
            play_mode: default_play_mode(),
            volume: 1.0,
            output: "speakers".to_string(),
            quiet_start: None,
            quiet_end: None,
            onboarding_completed: false,
            auto_play_enabled: default_auto_play_enabled(),
            manual_seed: default_manual_seed(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PersonalityProfile {
    name: String,
    headline: &'static str,
    reference_seed: u64,
    rhythm: &'static str,
    detail_lines: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct AppSnapshot {
    settings: Settings,
    audio_baseline: &'static str,
    version: &'static str,
    profiles: Vec<PersonalityProfile>,
}

#[derive(Clone)]
struct AppState {
    settings: Arc<Mutex<Settings>>,
    pressure: Arc<Mutex<Pressure>>,
    settings_path: Arc<PathBuf>,
}

impl AppState {
    fn new(settings: Settings, settings_path: PathBuf) -> Self {
        let personality = lookup_personality(&settings.personality).unwrap_or(&PERSONALITIES[1]);
        Self {
            settings: Arc::new(Mutex::new(settings)),
            pressure: Arc::new(Mutex::new(Pressure::new(personality, seed_now()))),
            settings_path: Arc::new(settings_path),
        }
    }

    fn save_settings(&self, settings: &Settings) -> Result<()> {
        save_settings(&self.settings_path, settings)
    }
}

#[derive(Clone, Copy)]
struct RenderPlan {
    sample_rate_hz: u32,
    output_gain_dbfs: f32,
    volume: f32,
}

fn sanitize_settings(mut settings: Settings) -> Settings {
    let previous_version = settings.version;
    settings.version = SETTINGS_VERSION;
    if previous_version < 2 {
        // Older builds shipped with background auto-play enabled by default.
        // That made manual reference checks sound like two unrelated voices
        // when the pressure loop fired during UI testing.
        settings.auto_play_enabled = false;
    }
    if lookup_personality(&settings.personality).is_none() {
        settings.personality = Settings::default().personality;
    }
    settings.play_mode = match settings.play_mode.as_str() {
        "shuffle" => "shuffle".to_string(),
        _ => "single".to_string(),
    };
    settings.volume = settings.volume.clamp(0.0, 1.0);
    settings.output = match settings.output.as_str() {
        "headphones" => "headphones".to_string(),
        _ => "speakers".to_string(),
    };
    settings.quiet_start = settings.quiet_start.map(|h| h.min(23));
    settings.quiet_end = settings.quiet_end.map(|h| h.min(23));
    // Keep within JS `Number` safe integer range for the webview UI.
    settings.manual_seed %= 1_000_000_000;
    settings
}

fn settings_path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("resolving app config directory")?;
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir.join(SETTINGS_FILE))
}

fn load_settings(path: &PathBuf) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
        .map(sanitize_settings)
        .unwrap_or_default()
}

fn save_settings(path: &PathBuf, settings: &Settings) -> Result<()> {
    let pretty = serde_json::to_string_pretty(settings)?;
    fs::write(path, pretty + "\n").with_context(|| format!("writing {}", path.display()))
}

fn personality_profiles() -> Vec<PersonalityProfile> {
    vec![
        PersonalityProfile {
            name: "polite-cough".to_string(),
            headline: "short, dry, plausibly deniable.",
            reference_seed: reference_seed_for_personality("polite-cough"),
            rhythm: "usually sparse, often under half a second, leaves before anyone makes eye contact.",
            detail_lines: vec![
                "A brief little throat-clearer with very little aftertaste.",
                "Best when the room needs a rumor, not an announcement.",
                "Small enough to pass as furniture if your luck holds.",
            ],
        },
        PersonalityProfile {
            name: "default".to_string(),
            headline: "the canon. wet enough, not so wet.",
            reference_seed: reference_seed_for_personality("default"),
            rhythm: "the best baseline for everyday cadence and the reference voice for signoff.",
            detail_lines: vec![
                "Balanced body, sensible pacing, enough texture to feel lived in.",
                "The one that should make the product make sense to a first-time listener.",
                "If a new build sounds wrong here, we stop and investigate.",
            ],
        },
        PersonalityProfile {
            name: "biblical".to_string(),
            headline: "slow, low, devastating.",
            reference_seed: reference_seed_for_personality("biblical"),
            rhythm: "rare, heavy, and comfortable taking its time once it commits.",
            detail_lines: vec![
                "The long-form disaster voice. Give it room.",
                "Large cavity, lower center, longer tail, less apology.",
                "This one defines the upper bound of acceptable absurdity.",
            ],
        },
        PersonalityProfile {
            name: "silent-but-deadly".to_string(),
            headline: "exactly what it says.",
            reference_seed: reference_seed_for_personality("silent-but-deadly"),
            rhythm: "more active, more crackle, and a little less interested in social harmony.",
            detail_lines: vec![
                "Fast onset, dirtier texture, and the most variable social outcome.",
                "Useful for testing how much motion the UI can tolerate before it feels busy.",
                "A good reminder that the display layer can be playful while the synth stays deterministic.",
            ],
        },
    ]
}

fn reference_seed_for_personality(personality_name: &str) -> u64 {
    match personality_name {
        "polite-cough" => 7,
        "biblical" => 31,
        "silent-but-deadly" => 9,
        _ => 17,
    }
}

/// Names of canonical personalities, in the order shown in the UI. Used to
/// pick a random voice in shuffle mode.
const CANONICAL_PERSONALITIES: &[&str] =
    &["polite-cough", "default", "biblical", "silent-but-deadly"];

/// Pick the personality for one fire.
///
/// Both manual (`fart_now`) and automatic (`spawn_pressure_loop`) fires go
/// through here so `single` / `shuffle` stays consistent across surfaces:
///
/// - `single` — always the personality the user picked.
/// - `shuffle` — uniform-random choice across the four canonical personalities.
fn select_fire_voice(settings: &Settings, rng_seed: u64) -> String {
    if settings.play_mode == "shuffle" {
        let mut rng = Mulberry32::new(rng_seed);
        let idx = (rng.next_u32() as usize) % CANONICAL_PERSONALITIES.len();
        CANONICAL_PERSONALITIES[idx].to_string()
    } else {
        settings.personality.clone()
    }
}

fn build_render_plan(settings: &Settings, _pressure: f32) -> RenderPlan {
    let headphones = settings.output == "headphones";
    RenderPlan {
        sample_rate_hz: safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: if headphones {
            safety::HEADPHONE_DBFS
        } else {
            safety::MAX_OUTPUT_DBFS
        },
        volume: settings.volume.clamp(0.0, 1.0),
    }
}

fn render_samples_for_settings(
    settings: &Settings,
    pressure: f32,
    seed: u64,
) -> Result<(Vec<f32>, RenderPlan)> {
    let personality = lookup_personality(&settings.personality)
        .ok_or_else(|| anyhow::anyhow!("unknown personality: {}", settings.personality))?;
    let mut rng = Mulberry32::new(seed);
    let params = sample_params(personality, &mut rng, pressure.clamp(0.0, 1.0));
    let plan = build_render_plan(settings, pressure);
    let cfg = RenderConfig {
        sample_rate_hz: plan.sample_rate_hz,
        output_gain_dbfs: plan.output_gain_dbfs,
    };
    let mut samples = render(&params, &cfg);
    if plan.volume <= 0.0 {
        samples.fill(0.0);
    } else if plan.volume < 1.0 {
        for sample in &mut samples {
            *sample *= plan.volume;
        }
    }
    Ok((samples, plan))
}

/// One reference event for desktop manual playback.
///
/// This intentionally mirrors the website specimen cards, not the website's
/// multi-event instrument preview. The user-facing desktop control should play
/// one clean line, so it cannot read as a second "instrument" layered on top.
fn render_manual_event(
    base_settings: &Settings,
    personality_name: &str,
    seed: u64,
) -> Result<(Vec<f32>, RenderPlan)> {
    let mut settings = base_settings.clone();
    settings.personality = personality_name.to_string();
    render_samples_for_settings(&settings, DEFAULT_PREVIEW_PRESSURE, seed)
}

fn quiet_hours_active(settings: &Settings, now_hour: u8) -> bool {
    match (settings.quiet_start, settings.quiet_end) {
        (Some(start), Some(end)) if start == end => true,
        (Some(start), Some(end)) if start < end => (start..end).contains(&now_hour),
        (Some(start), Some(end)) => now_hour >= start || now_hour < end,
        _ => false,
    }
}

fn current_local_hour() -> Option<u8> {
    OffsetDateTime::now_local().ok().map(OffsetDateTime::hour)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn main_window_hide(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?
        .hide()
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn main_window_minimize(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?
        .minimize()
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn main_window_toggle_maximize(app: AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?;
    if w.is_maximized().map_err(|e| e.to_string())? {
        w.unmaximize().map_err(|e| e.to_string())
    } else {
        w.maximize().map_err(|e| e.to_string())
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_app_snapshot(state: tauri::State<AppState>) -> AppSnapshot {
    AppSnapshot {
        settings: state.settings.lock().clone(),
        audio_baseline: AUDIO_BASELINE,
        version: VERSION,
        profiles: personality_profiles(),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn set_settings(new_settings: Settings, state: tauri::State<AppState>) -> Result<Settings, String> {
    let current = state.settings.lock().clone();
    let mut sanitized = sanitize_settings(new_settings);
    // A late `persist` from the webview can race after `complete_onboarding` and
    // send `onboarding_completed: false` with an older settings snapshot. First
    // launch is only cleared through `complete_onboarding`; it is re-opened only
    // via `reset_onboarding`, which does not use this path.
    if current.onboarding_completed && !sanitized.onboarding_completed {
        sanitized.onboarding_completed = true;
    }
    state
        .save_settings(&sanitized)
        .map_err(|err| format!("could not save settings: {err}"))?;
    *state.settings.lock() = sanitized.clone();
    Ok(sanitized)
}

#[tauri::command]
fn list_personality_profiles() -> Vec<PersonalityProfile> {
    personality_profiles()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn fart_now(state: tauri::State<AppState>) -> Result<String, String> {
    let app = state.inner().clone();
    let settings = app.settings.lock().clone();
    let chosen_name = select_fire_voice(&settings, settings.manual_seed);

    {
        let personality = lookup_personality(&chosen_name)
            .ok_or_else(|| format!("unknown personality: {chosen_name}"))?;
        let mut pressure = app.pressure.lock();
        let _ = pressure.force_fire(personality);
    }

    // Render + playback + seed bump can take tens of ms; keep the IPC path
    // (tray menu + webview button) responsive by doing that work off-thread.
    let settings_for_play = settings.clone();
    let chosen_for_play = chosen_name.clone();
    let app_for_thread = app.clone();
    thread::spawn(move || {
        match render_manual_event(
            &settings_for_play,
            &chosen_for_play,
            settings_for_play.manual_seed,
        ) {
            Ok((samples, plan)) => {
                let sample_rate_hz = plan.sample_rate_hz;
                let _ = play_output_exclusive(samples, sample_rate_hz);
            }
            Err(e) => eprintln!("fart_now render failed: {e}"),
        }
        let next_seed = seed_now() % 1_000_000_000;
        let mut g = app_for_thread.settings.lock();
        g.manual_seed = next_seed;
        let to_save = g.clone();
        drop(g);
        if let Err(e) = app_for_thread.save_settings(&to_save) {
            eprintln!("fart_now could not save next seed: {e}");
        }
    });

    Ok(chosen_name)
}

/// Real-time preview render. `seed` overrides the persisted `manual_seed` so
/// the frontend can show the waveform for the seed currently in the input
/// field without first writing it to disk. When `seed` is `None`, the
/// persisted `manual_seed` is used (matches what `fart_now` would play).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn render_preview_wav(seed: Option<u64>, state: tauri::State<AppState>) -> Result<Vec<u8>, String> {
    let settings = state.settings.lock().clone();
    let base_seed = seed.unwrap_or(settings.manual_seed);
    let name = select_fire_voice(&settings, base_seed);
    let (merged, plan) =
        render_manual_event(&settings, &name, base_seed).map_err(|e| e.to_string())?;
    Ok(write_wav_to_vec(&merged, plan.sample_rate_hz))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn show_main_window_command(app: AppHandle) {
    show_main_window(&app);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn open_github() -> Result<(), String> {
    Command::new("open")
        .arg("https://github.com/p-to-q/flatus")
        .status()
        .map_err(|err| format!("failed to launch browser: {err}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("browser exited with status {status}"))
            }
        })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn complete_onboarding(state: tauri::State<AppState>) -> Result<(), String> {
    let mut settings = state.settings.lock().clone();
    if settings.onboarding_completed {
        return Ok(());
    }
    settings.onboarding_completed = true;
    settings = sanitize_settings(settings);
    state
        .save_settings(&settings)
        .map_err(|err| format!("could not save onboarding state: {err}"))?;
    *state.settings.lock() = settings;
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn reset_onboarding(state: tauri::State<AppState>, app: AppHandle) -> Result<(), String> {
    let mut settings = state.settings.lock().clone();
    settings.onboarding_completed = false;
    settings = sanitize_settings(settings);
    state
        .save_settings(&settings)
        .map_err(|err| format!("could not reset onboarding: {err}"))?;
    *state.settings.lock() = settings;
    show_main_window(&app);
    Ok(())
}

/// One global lock for every `cpal` playback: manual `fart_now` runs in a
/// spawned thread while auto-play runs in the pressure thread — without this,
/// two streams overlap and the mix no longer matches the single-buffer website.
static AUDIO_OUTPUT_LOCK: Mutex<()> = Mutex::new(());

fn play_output_exclusive(samples: Vec<f32>, sample_rate_hz: u32) -> Result<()> {
    let _hold = AUDIO_OUTPUT_LOCK.lock();
    play_blocking(samples, sample_rate_hz)
}

fn play_blocking(samples: Vec<f32>, sample_rate_hz: u32) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("no default output device"))?;
    let supported = device.default_output_config()?;
    let device_channels = supported.channels() as usize;
    let device_sample_rate = supported.sample_rate().0;

    let mono: Vec<f32> = if device_sample_rate == sample_rate_hz {
        samples
    } else {
        resample_linear(&samples, sample_rate_hz, device_sample_rate)
    };
    let total = mono.len();
    let mut cursor = 0usize;

    let config = StreamConfig {
        channels: device_channels as u16,
        sample_rate: cpal::SampleRate(device_sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };
    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_output_stream(
            &config,
            move |out: &mut [f32], _| feed(&mono, &mut cursor, device_channels, out),
            err_fn,
            None,
        )?,
        SampleFormat::I16 => {
            let mono = mono.clone();
            device.build_output_stream(
                &config,
                move |out: &mut [i16], _| {
                    let mut tmp = vec![0.0_f32; out.len()];
                    feed(&mono, &mut cursor, device_channels, &mut tmp);
                    for (o, s) in out.iter_mut().zip(tmp.iter()) {
                        *o = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::U16 => {
            let mono = mono.clone();
            device.build_output_stream(
                &config,
                move |out: &mut [u16], _| {
                    let mut tmp = vec![0.0_f32; out.len()];
                    feed(&mono, &mut cursor, device_channels, &mut tmp);
                    for (o, s) in out.iter_mut().zip(tmp.iter()) {
                        let v = (s.clamp(-1.0, 1.0) * 0.5 + 0.5) * f32::from(u16::MAX);
                        *o = v as u16;
                    }
                },
                err_fn,
                None,
            )?
        }
        other => return Err(anyhow::anyhow!("unsupported sample format: {other:?}")),
    };
    stream.play()?;
    thread::sleep(Duration::from_secs_f32(
        total as f32 / device_sample_rate as f32 + 0.25,
    ));
    drop(stream);
    Ok(())
}

fn feed(mono: &[f32], cursor: &mut usize, channels: usize, out: &mut [f32]) {
    for frame in out.chunks_mut(channels) {
        let s = if *cursor < mono.len() {
            let v = mono[*cursor];
            *cursor += 1;
            v
        } else {
            0.0
        };
        for ch in frame.iter_mut() {
            *ch = s;
        }
    }
}

fn resample_linear(samples: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz {
        return samples.to_vec();
    }
    let ratio = f64::from(from_hz) / f64::from(to_hz);
    let out_len = ((samples.len() as f64) / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let lo = src.floor() as usize;
        let hi = (lo + 1).min(samples.len().saturating_sub(1));
        let frac = (src - lo as f64) as f32;
        let a = samples.get(lo).copied().unwrap_or(0.0);
        let b = samples.get(hi).copied().unwrap_or(0.0);
        out.push(a + (b - a) * frac);
    }
    out
}

fn seed_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0xc0ff_eeee_c0ff_eeee, |d| d.as_nanos() as u64)
}

#[allow(clippy::needless_pass_by_value)]
fn spawn_pressure_loop(app: AppHandle) {
    let state = app.state::<AppState>().inner().clone();
    thread::spawn(move || {
        let mut last = std::time::Instant::now();
        loop {
            thread::sleep(Duration::from_secs(1));
            let now = std::time::Instant::now();
            let dt_raw = now.duration_since(last).as_secs_f32();
            last = now;

            let settings = state.settings.lock().clone();
            let Some(personality) = lookup_personality(&settings.personality) else {
                continue;
            };

            let dt = dt_raw;

            let activity = ActivitySignal {
                user_is_active: idle::user_idle_secs().is_some_and(|s| s < 60),
            };

            let result = state.pressure.lock().tick(dt, activity, personality);
            // The pressure model decides *when* to fire; the chosen
            // personality plus render settings decide *what* it sounds like.
            if let TickResult::Fart { pressure } = result {
                if !settings.auto_play_enabled {
                    continue;
                }
                if current_local_hour().is_some_and(|hour| quiet_hours_active(&settings, hour)) {
                    continue;
                }
                let fire_seed = seed_now();
                let chosen_name = select_fire_voice(&settings, fire_seed);
                let render_settings = Settings {
                    personality: chosen_name,
                    ..settings.clone()
                };
                if let Ok((samples, plan)) =
                    render_samples_for_settings(&render_settings, pressure, fire_seed)
                {
                    let _ = play_output_exclusive(samples, plan.sample_rate_hz);
                }
            }
        }
    });
}

fn build_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>> {
    let item_show = MenuItem::with_id(app, "open_main", "Show window", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let item_fart = MenuItem::with_id(app, "fart_now", "Fart now", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let item_quit = MenuItem::with_id(app, "quit", "Quit flatus", true, Some("Cmd+Q"))?;
    Ok(Menu::with_items(
        app,
        &[&item_show, &sep1, &item_fart, &sep2, &item_quit],
    )?)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings_path = settings_path(app.handle())?;
            let settings = load_settings(&settings_path);
            save_settings(&settings_path, &settings)?;
            app.manage(AppState::new(settings.clone(), settings_path));

            let menu = build_tray_menu(app.handle())?;
            let handle = app.handle().clone();
            // Left-click drops the native menu (the "F · ⋯" feel the brand
            // wants). The menu's "Show window" item is the only path to the
            // big surface; there is no longer a webview popover.
            let _tray = TrayIconBuilder::with_id("main")
                .icon(tauri::include_image!("icons/tray-template@2x.png"))
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "fart_now" => {
                        let state: tauri::State<AppState> = app.state();
                        let _ = fart_now(state);
                    }
                    // Let the native menu finish closing before showing/focusing
                    // the window — otherwise `show`/`set_focus` can be flaky.
                    "open_main" => {
                        let h = app.clone();
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(50));
                            show_main_window(&h);
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            if let Some(main) = handle.get_webview_window("main") {
                let _ = main.set_shadow(true);
                let _ = main.set_background_color(Some(Color(0, 0, 0, 0)));
            }

            if !settings.onboarding_completed {
                show_main_window(&handle);
            } else if let Some(main) = handle.get_webview_window("main") {
                let _ = main.hide();
            }

            spawn_pressure_loop(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_app_snapshot,
            set_settings,
            list_personality_profiles,
            fart_now,
            render_preview_wav,
            main_window_hide,
            main_window_minimize,
            main_window_toggle_maximize,
            show_main_window_command,
            open_github,
            quit_app,
            complete_onboarding,
            reset_onboarding,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_supports_cross_midnight_ranges() {
        let settings = Settings {
            quiet_start: Some(22),
            quiet_end: Some(7),
            ..Settings::default()
        };
        assert!(quiet_hours_active(&settings, 23));
        assert!(quiet_hours_active(&settings, 3));
        assert!(!quiet_hours_active(&settings, 12));
    }

    #[test]
    fn render_plan_honors_output_and_volume() {
        let settings = Settings {
            output: "speakers".to_string(),
            volume: 0.35,
            ..Settings::default()
        };
        let plan = build_render_plan(&settings, 0.8);
        assert!((plan.output_gain_dbfs - safety::MAX_OUTPUT_DBFS).abs() < f32::EPSILON);
        assert!((plan.volume - 0.35).abs() < 1e-6);
    }

    #[test]
    fn sanitize_settings_restores_supported_personality() {
        let settings = Settings {
            personality: "mystery".to_string(),
            volume: 9.0,
            output: "sideways".to_string(),
            quiet_start: Some(99),
            quiet_end: Some(24),
            ..Settings::default()
        };
        let sanitized = sanitize_settings(settings);
        assert_eq!(sanitized.personality, "default");
        // Unknown output strings now fall back to "speakers" (the documented
        // default), not "headphones" — that earlier behaviour silently
        // halved the volume on legacy / corrupted settings files.
        assert_eq!(sanitized.output, "speakers");
        assert!((sanitized.volume - 1.0).abs() < f32::EPSILON);
        assert_eq!(sanitized.quiet_start, Some(23));
        assert_eq!(sanitized.quiet_end, Some(23));
    }

    #[test]
    fn sanitize_settings_normalises_play_mode() {
        let s = Settings {
            play_mode: "garbage".to_string(),
            ..Settings::default()
        };
        assert_eq!(sanitize_settings(s).play_mode, "single");

        let s = Settings {
            play_mode: "shuffle".to_string(),
            ..Settings::default()
        };
        assert_eq!(sanitize_settings(s).play_mode, "shuffle");
    }

    #[test]
    fn sanitize_settings_migrates_legacy_auto_play_off() {
        let s = Settings {
            version: 1,
            auto_play_enabled: true,
            ..Settings::default()
        };
        let sanitized = sanitize_settings(s);
        assert_eq!(sanitized.version, SETTINGS_VERSION);
        assert!(!sanitized.auto_play_enabled);
    }

    #[test]
    fn sanitize_settings_preserves_current_auto_play_choice() {
        let s = Settings {
            version: SETTINGS_VERSION,
            auto_play_enabled: true,
            ..Settings::default()
        };
        let sanitized = sanitize_settings(s);
        assert!(sanitized.auto_play_enabled);
    }

    #[test]
    fn select_fire_voice_single_uses_selected_personality() {
        let s = Settings {
            personality: "biblical".to_string(),
            play_mode: "single".to_string(),
            ..Settings::default()
        };
        for seed in [1, 7, 42, 0xdead_beef] {
            let name = select_fire_voice(&s, seed);
            assert_eq!(name, "biblical");
        }
    }

    #[test]
    fn select_fire_voice_shuffle_picks_only_canonical_personalities() {
        let s = Settings {
            play_mode: "shuffle".to_string(),
            ..Settings::default()
        };
        let mut hits = std::collections::HashSet::new();
        for seed in 0..1024_u64 {
            let name = select_fire_voice(&s, seed.wrapping_mul(0x9E37_79B9));
            assert!(
                CANONICAL_PERSONALITIES.contains(&name.as_str()),
                "shuffle picked unknown personality `{name}`",
            );
            hits.insert(name);
        }
        // Across 1024 seeds we should reach all four personalities.
        assert_eq!(hits.len(), CANONICAL_PERSONALITIES.len());
    }

    #[test]
    fn reference_seed_matches_web_reference_values() {
        assert_eq!(reference_seed_for_personality("polite-cough"), 7);
        assert_eq!(reference_seed_for_personality("default"), 17);
        assert_eq!(reference_seed_for_personality("biblical"), 31);
        assert_eq!(reference_seed_for_personality("silent-but-deadly"), 9);
    }
}
