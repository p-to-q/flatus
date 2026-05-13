// flatus — Tauri v2 menubar shell.
//
// Architecture commitments (mirroring PLAN.md §5):
//   - `ActivationPolicy::Accessory` → no dock icon, tray-only.
//   - Left-click on tray  = fire a fart now (the OpenWhip move).
//   - Right-click on tray = open/toggle the settings popover.
//   - Synthesis runs in this Rust process via `fart-synth` + `cpal`.
//     The webview is UI only; it calls `invoke("fart_now", …)` and Rust does the work.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod idle;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use fart_synth::{
    personalities::{lookup_personality, sample_params, PERSONALITIES},
    pressure::{ActivitySignal, Pressure, TickResult},
    prng::Mulberry32,
    render, safety, RenderConfig,
};

// -------------------- Settings --------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    personality: String,
    /// 0–1 volume on top of the engineering cap.
    volume: f32,
    /// "speakers" or "headphones".
    output: String,
    /// Hour 0–23 for quiet-hours start. `None` = disabled.
    quiet_start: Option<u8>,
    quiet_end: Option<u8>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            personality: "default".to_string(),
            volume: 1.0,
            output: "headphones".to_string(), // safer default — see PLAN.md §6
            quiet_start: None,
            quiet_end: None,
        }
    }
}

#[derive(Clone)]
struct AppState {
    settings: Arc<Mutex<Settings>>,
    pressure: Arc<Mutex<Pressure>>,
}

impl AppState {
    fn new() -> Self {
        let personality = &PERSONALITIES[1]; // default
        Self {
            settings: Arc::new(Mutex::new(Settings::default())),
            pressure: Arc::new(Mutex::new(Pressure::new(personality, seed_now()))),
        }
    }
}

// -------------------- Invoke handlers --------------------

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
fn set_settings(new_settings: Settings, state: tauri::State<AppState>) {
    *state.settings.lock() = new_settings;
}

#[tauri::command]
fn list_personalities() -> Vec<String> {
    PERSONALITIES.iter().map(|p| p.name.to_string()).collect()
}

/// Fire a fart immediately, regardless of pressure. Returns the personality used.
#[tauri::command]
fn fart_now(state: tauri::State<AppState>) -> Result<String, String> {
    let (personality_name, dbfs) = {
        let s = state.settings.lock();
        let dbfs = if s.output == "headphones" {
            safety::HEADPHONE_DBFS
        } else {
            safety::MAX_OUTPUT_DBFS
        };
        (s.personality.clone(), dbfs)
    };

    let personality = lookup_personality(&personality_name)
        .ok_or_else(|| format!("unknown personality: {}", personality_name))?;

    {
        let mut p = state.pressure.lock();
        let _ = p.force_fire(personality);
    }

    let mut rng = Mulberry32::new(seed_now());
    let params = sample_params(personality, &mut rng, 0.7);
    let cfg = RenderConfig {
        sample_rate_hz: safety::SAMPLE_RATE_HZ,
        output_gain_dbfs: dbfs,
    };
    let samples = render(&params, &cfg);

    thread::spawn(move || {
        let _ = play_blocking(samples, cfg.sample_rate_hz);
    });

    Ok(personality_name)
}

// -------------------- Audio playback (cpal, same shape as the CLI) --------------------

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
                        *o = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    }
                },
                err_fn,
                None,
            )?
        }
        other => return Err(anyhow::anyhow!("unsupported sample format: {:?}", other)),
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
    let ratio = from_hz as f64 / to_hz as f64;
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
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xc0ff_eeee_c0ff_eeee)
}

// -------------------- Pressure background loop --------------------

/// "Active" = HID input within this many seconds. Long enough that brief
/// reading pauses don't toggle the bonus; short enough that walking away
/// cools the pressure rate quickly.
const ACTIVE_THRESHOLD_SECS: u64 = 60;

fn spawn_pressure_loop(app: AppHandle) {
    let state = app.state::<AppState>().inner().clone();
    thread::spawn(move || {
        let mut last = std::time::Instant::now();
        loop {
            thread::sleep(Duration::from_secs(1));
            let now = std::time::Instant::now();
            let dt = now.duration_since(last).as_secs_f32();
            last = now;

            let (personality_name, dbfs) = {
                let s = state.settings.lock();
                let dbfs = if s.output == "headphones" {
                    safety::HEADPHONE_DBFS
                } else {
                    safety::MAX_OUTPUT_DBFS
                };
                (s.personality.clone(), dbfs)
            };
            let Some(personality) = lookup_personality(&personality_name) else {
                continue;
            };

            // Real macOS activity detection via `IOHIDSystem`'s `HIDIdleTime`
            // (see `idle.rs`). Platforms without an implementation report
            // `None`, which falls back to the baseline rate (inactive).
            let activity = ActivitySignal {
                user_is_active: idle::user_idle_secs().is_some_and(|s| s < ACTIVE_THRESHOLD_SECS),
            };

            let result = state.pressure.lock().tick(dt, activity, personality);
            if let TickResult::Fart { pressure } = result {
                let mut rng = Mulberry32::new(seed_now());
                let params = sample_params(personality, &mut rng, pressure);
                let cfg = RenderConfig {
                    sample_rate_hz: safety::SAMPLE_RATE_HZ,
                    output_gain_dbfs: dbfs,
                };
                let samples = render(&params, &cfg);
                let _ = play_blocking(samples, cfg.sample_rate_hz);
            }
        }
    });
}

// -------------------- Tray + setup --------------------

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            list_personalities,
            fart_now,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Settings window starts hidden; we toggle it from the tray.
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.hide();
            }

            // Tray menu (shown on right-click and via the keyboard shortcut).
            let item_fart = MenuItem::with_id(app, "fart_now", "Fart now", true, None::<&str>)?;
            let item_settings =
                MenuItem::with_id(app, "open_settings", "Settings…", true, None::<&str>)?;
            let item_quit = MenuItem::with_id(app, "quit", "Quit flatus", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&item_fart, &item_settings, &item_quit])?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tauri::include_image!("icons/icon.png"))
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "fart_now" => {
                        let state: tauri::State<AppState> = app.state();
                        let _ = fart_now(state);
                    }
                    "open_settings" => toggle_settings(app),
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = event
                    {
                        if button_state == MouseButtonState::Up {
                            match button {
                                MouseButton::Left => {
                                    let app = tray.app_handle().clone();
                                    let state: tauri::State<AppState> = app.state();
                                    let _ = fart_now(state);
                                }
                                MouseButton::Right => toggle_settings(tray.app_handle()),
                                _ => {}
                            }
                        }
                    }
                })
                .build(app)?;

            spawn_pressure_loop(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn toggle_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let visible = w.is_visible().unwrap_or(false);
        if visible {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}
