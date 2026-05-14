// flatus desktop — single-window frontend + waveform preview.

const tauriInvoke = window.__TAURI__?.core?.invoke;
if (tauriInvoke) document.documentElement.classList.add("is-tauri");

const SAMPLE_RATE = 48000;
const GITHUB_URL = "https://github.com/p-to-q/flatus";

const MOCK_PROFILES = [
  { name: "polite-cough", headline: "short, dry, plausibly deniable.", reference_seed: 7 },
  { name: "default", headline: "the canon. wet enough, not so wet.", reference_seed: 17 },
  { name: "biblical", headline: "slow, low, devastating.", reference_seed: 31 },
  { name: "silent-but-deadly", headline: "exactly what it says.", reference_seed: 9 },
];

const mockSnapshot = {
  settings: {
    version: 2,
    personality: "default",
    play_mode: "single",
    volume: 1,
    output: "speakers",
    quiet_start: null,
    quiet_end: null,
    onboarding_completed: true,
    auto_play_enabled: false,
    manual_seed: 17,
  },
  audio_baseline: "fixtures-v0.4 + web-specimen-reference",
  version: "0.2.2",
  profiles: MOCK_PROFILES,
};

let state = structuredClone(mockSnapshot);
let previewRequestSerial = 0;

function invoke(command, payload) {
  if (!tauriInvoke) return mockInvoke(command, payload);
  return tauriInvoke(command, payload);
}

/** Minimal mono PCM WAV (silence) for browser-only preview when Rust is unavailable. */
function buildSilentWavBytes(sampleRate, durationSec) {
  const n = Math.floor(sampleRate * durationSec);
  const dataSize = n * 2;
  const buf = new ArrayBuffer(44 + dataSize);
  const u8 = new Uint8Array(buf);
  const dv = new DataView(buf);
  u8[0] = 0x52; u8[1] = 0x49; u8[2] = 0x46; u8[3] = 0x46;
  dv.setUint32(4, 36 + dataSize, true);
  u8[8] = 0x57; u8[9] = 0x41; u8[10] = 0x56; u8[11] = 0x45;
  u8[12] = 0x66; u8[13] = 0x6d; u8[14] = 0x74; u8[15] = 0x20;
  dv.setUint32(16, 16, true);
  dv.setUint16(20, 1, true);
  dv.setUint16(22, 1, true);
  dv.setUint32(24, sampleRate, true);
  dv.setUint32(28, sampleRate * 2, true);
  dv.setUint16(32, 2, true);
  dv.setUint16(34, 16, true);
  u8[36] = 0x64; u8[37] = 0x61; u8[38] = 0x74; u8[39] = 0x61;
  dv.setUint32(40, dataSize, true);
  return u8;
}

async function mockInvoke(command, payload) {
  switch (command) {
    case "get_app_snapshot":
      return structuredClone(state);
    case "set_settings":
      state.settings = { ...payload.newSettings };
      return structuredClone(state.settings);
    case "fart_now":
      console.info("[preview] fart_now", state.settings.personality);
      state.settings.manual_seed = Math.floor(Math.random() * 1e9);
      return state.settings.personality;
    case "render_preview_wav":
      return buildSilentWavBytes(SAMPLE_RATE, 1.2);
    case "complete_onboarding":
      if (state.settings.onboarding_completed) return null;
      state.settings.onboarding_completed = true;
      return null;
    case "list_personality_profiles":
      return structuredClone(state.profiles);
    case "show_main_window_command":
    case "open_github":
    case "quit_app":
    case "reset_onboarding":
    case "main_window_hide":
    case "main_window_minimize":
    case "main_window_toggle_maximize":
      return null;
    case "export_audio_debug_bundle":
      return {
        audio_baseline: state.audio_baseline,
        version: state.version,
        chosen_personality: state.settings.personality,
        manual_seed: state.settings.manual_seed,
        pressure: 0.6,
        source_sample_rate_hz: SAMPLE_RATE,
        rendered_frames: Math.floor(SAMPLE_RATE * 1.2),
        device: {
          device_name: "browser preview",
          sample_format: "mock",
          channels: 2,
          device_sample_rate_hz: SAMPLE_RATE,
        },
        rendered_wav_path: "/tmp/flatus-preview.wav",
        report_path: "/tmp/flatus-preview.json",
      };
    default:
      return null;
  }
}

const $ = (sel) => document.querySelector(sel);
const $all = (sel) => Array.from(document.querySelectorAll(sel));

/** `fart_now` advances seed on a background thread; re-fetch once it has landed. */
async function refreshSnapshotAfterFart() {
  await new Promise((r) => setTimeout(r, 180));
  try {
    state = await invoke("get_app_snapshot");
    renderState();
  } catch (err) {
    console.warn("post-fart snapshot refresh failed", err);
  }
}

function quietSummary(s) {
  if (s.quiet_start == null || s.quiet_end == null) return "always live";
  const fmt = (h) => `${String(h).padStart(2, "0")}:00`;
  return `silent ${fmt(s.quiet_start)} → ${fmt(s.quiet_end)}`;
}

function referenceSeedFor(name) {
  return state.profiles.find((profile) => profile.name === name)?.reference_seed ?? 17;
}

function populateQuietSelects() {
  for (const select of $all(".quiet-select")) {
    if (select.options.length > 1) continue;
    for (let h = 0; h < 24; h += 1) {
      const opt = document.createElement("option");
      opt.value = String(h);
      opt.textContent = `${String(h).padStart(2, "0")}:00`;
      select.appendChild(opt);
    }
  }
}

function decodeWavSamplesSync(bytes) {
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
  const headerLen = 44;
  const n = Math.max(0, Math.floor((dv.byteLength - headerLen) / 2));
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    out[i] = dv.getInt16(headerLen + i * 2, true) / 32768;
  }
  return out;
}

function scopePalette() {
  const css = getComputedStyle(document.documentElement);
  const get = (n, fb) => (css.getPropertyValue(n).trim() || fb);
  return {
    bgTop: "#15161b",
    bgMid: "#1c1d24",
    bgBot: "#0f1014",
    grid: "rgba(239, 126, 87, 0.10)",
    band: "rgba(239, 126, 87, 0.08)",
    cap: "rgba(239, 126, 87, 0.40)",
    glow: get("--warm", "#ef7e57"),
    core: "#fff3d6",
    muted: get("--muted", "#8d836d"),
  };
}

function paintScopeBackground(ctx, cssW, cssH) {
  const c = scopePalette();
  const grad = ctx.createLinearGradient(0, 0, 0, cssH);
  grad.addColorStop(0, c.bgTop);
  grad.addColorStop(0.55, c.bgMid);
  grad.addColorStop(1, c.bgBot);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, cssW, cssH);

  const band = ctx.createLinearGradient(0, 0, 0, cssH);
  band.addColorStop(0, "rgba(239, 126, 87, 0)");
  band.addColorStop(0.5, c.band);
  band.addColorStop(1, "rgba(239, 126, 87, 0)");
  ctx.fillStyle = band;
  ctx.fillRect(0, cssH * 0.2, cssW, cssH * 0.6);

  ctx.strokeStyle = c.grid;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, cssH / 2);
  ctx.lineTo(cssW, cssH / 2);
  ctx.stroke();

  return c;
}

function computePeaks(samples, cssW) {
  const samplesPerPixel = Math.max(1, Math.floor(samples.length / cssW));
  const peaks = new Array(cssW);
  for (let x = 0; x < cssW; x++) {
    let min = 1;
    let max = -1;
    const start = x * samplesPerPixel;
    const end = Math.min(start + samplesPerPixel, samples.length);
    for (let i = start; i < end; i++) {
      const v = samples[i];
      if (v < min) min = v;
      if (v > max) max = v;
    }
    peaks[x] = { min, max };
  }
  return peaks;
}

function strokeWaveformPath(ctx, peaks, cssW, cssH, envelopeScale = 1) {
  const cy = cssH / 2;
  const s = Math.max(0.25, Math.min(envelopeScale, 4));
  ctx.beginPath();
  for (let x = 0; x < peaks.length; x++) {
    let { min, max } = peaks[x];
    max = Math.max(-1, Math.min(1, max * s));
    min = Math.max(-1, Math.min(1, min * s));
    const y1 = cy - max * cy;
    const y2 = cy - min * cy;
    if (x === 0) ctx.moveTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, y2);
  }
  ctx.stroke();
}

function drawPreviewFromSamples(samples) {
  const canvas = $("[data-preview-canvas]");
  if (!canvas) return;
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || 560;
  const cssH = canvas.clientHeight || 200;
  canvas.width = Math.floor(cssW * dpr);
  canvas.height = Math.floor(cssH * dpr);
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);

  const c = paintScopeBackground(ctx, cssW, cssH);

  const headphones = state.settings.output === "headphones";
  const capLinear = headphones ? 10 ** (-18 / 20) : 10 ** (-6 / 20);
  ctx.setLineDash([4, 4]);
  ctx.strokeStyle = c.cap;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, (1 - capLinear) * (cssH / 2));
  ctx.lineTo(cssW, (1 - capLinear) * (cssH / 2));
  ctx.moveTo(0, (1 + capLinear) * (cssH / 2));
  ctx.lineTo(cssW, (1 + capLinear) * (cssH / 2));
  ctx.stroke();
  ctx.setLineDash([]);

  const peaks = computePeaks(samples, cssW);
  let env = 0;
  for (const p of peaks) {
    env = Math.max(env, Math.abs(p.min), Math.abs(p.max));
  }
  /** Fill ~90% of half-height when the buffer is quiet (closer to the web demo). */
  const envelopeScale = env > 1e-5 ? Math.min(0.9 / env, 3.2) : 1;

  ctx.lineJoin = "round";
  ctx.shadowColor = c.glow;
  ctx.shadowBlur = 16;
  ctx.strokeStyle = c.glow;
  ctx.lineWidth = 1.8;
  strokeWaveformPath(ctx, peaks, cssW, cssH, envelopeScale);

  ctx.shadowBlur = 0;
  ctx.strokeStyle = c.core;
  ctx.lineWidth = 1;
  strokeWaveformPath(ctx, peaks, cssW, cssH, envelopeScale);
}

let previewTimer = null;
let lastPreviewSettingsKey = null;

function settingsPreviewKey(settings) {
  return [
    settings.output,
    settings.personality,
    settings.volume,
    settings.manual_seed,
    settings.play_mode ?? "single",
  ].join("\0");
}
function schedulePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(runPreview, 140);
}

/** Seed value to send to `render_preview_wav`: explicit override, else the
 *  seed field if it has a parseable value (while typing), else persisted settings. */
function previewSeedValue(overrideSeed) {
  if (Number.isFinite(overrideSeed)) {
    return Math.max(0, Math.floor(overrideSeed));
  }
  const inp = $('input[data-setting="manual_seed"]');
  if (inp) {
    const raw = String(inp.value ?? "").trim();
    if (raw !== "") {
      const v = Math.floor(Number(raw));
      if (Number.isFinite(v) && v >= 0) return v;
    }
  }
  return Math.max(0, Math.floor(Number(state.settings.manual_seed) || 0));
}

/** Real-time waveform preview — always forwards an explicit `seed` so Rust
 *  matches the UI (including in-flight edits before persist). */
async function runPreview(overrideSeed) {
  const requestId = ++previewRequestSerial;
  try {
    const seed = previewSeedValue(overrideSeed);
    const bytes = await invoke("render_preview_wav", { seed });
    if (requestId !== previewRequestSerial) return;
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    const samples = decodeWavSamplesSync(u8);
    drawPreviewFromSamples(samples);
  } catch (e) {
    if (requestId !== previewRequestSerial) return;
    console.warn("preview failed", e);
  }
}

function renderModes() {
  const list = $("[data-mode-list]");
  if (!list) return;
  list.innerHTML = "";
  for (const profile of state.profiles) {
    const item = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "mode-item";
    btn.dataset.voice = profile.name;
    if (profile.name === state.settings.personality) btn.classList.add("is-active");
    btn.innerHTML = `
      <div class="mode-text">
        <div class="mode-name">${profile.name}</div>
        <div class="mode-desc">${profile.headline ?? ""}</div>
      </div>
      <span class="mode-flag">ref ${profile.reference_seed}</span>
    `;
    item.appendChild(btn);
    list.appendChild(item);
  }
}

function renderState() {
  const onboarding = $("[data-onboarding]");
  if (onboarding) {
    const done = Boolean(state.settings?.onboarding_completed);
    const wasHidden = onboarding.hidden;
    onboarding.hidden = done;
    onboarding.classList.toggle("intro--dismissed", done);
    if (done) onboarding.setAttribute("hidden", "");
    else onboarding.removeAttribute("hidden");
    if (wasHidden !== onboarding.hidden) {
      const content = $(".content");
      if (content) content.scrollTop = 0;
    }
  }

  for (const slider of $all('input[data-setting="volume"]')) {
    slider.value = String(state.settings.volume);
  }
  for (const r of $all("[data-volume-readout]")) {
    r.textContent = `${Math.round(state.settings.volume * 100)}%`;
  }

  const ms = Math.max(0, Math.floor(Number(state.settings.manual_seed) || 0));
  const msStr = String(ms);
  for (const inp of $all('input[data-setting="manual_seed"]')) {
    if (document.activeElement !== inp && inp.value !== msStr) inp.value = msStr;
  }

  for (const btn of $all("[data-output]")) {
    btn.classList.toggle("is-active", btn.dataset.output === state.settings.output);
  }

  for (const sel of $all('[data-setting="quiet_start"]')) {
    sel.value = state.settings.quiet_start == null ? "" : String(state.settings.quiet_start);
  }
  for (const sel of $all('[data-setting="quiet_end"]')) {
    sel.value = state.settings.quiet_end == null ? "" : String(state.settings.quiet_end);
  }
  for (const r of $all("[data-quiet-summary]")) {
    r.textContent = quietSummary(state.settings);
  }

  for (const btn of $all("[data-auto-play]")) {
    const enabled = Boolean(state.settings.auto_play_enabled);
    const wantOn = btn.dataset.autoPlay === "true";
    btn.classList.toggle("is-active", wantOn === enabled);
  }

  for (const r of $all("[data-version]")) {
    r.textContent = `v${state.version}`;
  }

  const shuffle = state.settings.play_mode === "shuffle";
  for (const item of $all(".mode-item")) {
    const isCurrent = !shuffle && item.dataset.voice === state.settings.personality;
    item.classList.toggle("is-active", isCurrent);
  }
  for (const btn of $all("[data-play-mode]")) {
    btn.classList.toggle("is-active", btn.dataset.playMode === (state.settings.play_mode ?? "single"));
  }
  document.body.classList.toggle("is-shuffle", shuffle);

  const previewKey = settingsPreviewKey(state.settings);
  if (previewKey !== lastPreviewSettingsKey) {
    lastPreviewSettingsKey = previewKey;
    schedulePreview();
  }
}

async function persist() {
  try {
    const saved = await invoke("set_settings", { newSettings: state.settings });
    if (saved) state.settings = saved;
  } catch (err) {
    console.error("persist failed", err);
    try {
      state = await invoke("get_app_snapshot");
    } catch (snapshotErr) {
      console.error("snapshot fallback failed", snapshotErr);
    }
  } finally {
    renderState();
  }
}

async function openGithub() {
  if (tauriInvoke) {
    try {
      await invoke("open_github");
      return;
    } catch (err) {
      console.error("open_github failed", err);
    }
  }
  window.open(GITHUB_URL, "_blank", "noopener,noreferrer");
}

function bindKeyboardShortcuts() {
  window.addEventListener("keydown", async (e) => {
    const githubLink = e.target?.closest?.('[data-action="open-github"]');
    if (githubLink && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      await openGithub();
      return;
    }

    if (!tauriInvoke) return;
    // When the main webview has focus (no global menu in Accessory mode).
    // Cmd+Q / Cmd+W / Cmd+M — not registered globally to avoid hijacking other apps.
    const cmd = e.metaKey || e.ctrlKey;
    if (!cmd) return;
    const k = e.key;
    if (k === "q" || k === "Q") {
      e.preventDefault();
      try {
        await invoke("quit_app");
      } catch (err) {
        console.error(err);
      }
      return;
    }
    if (k === "w" || k === "W") {
      e.preventDefault();
      try {
        await invoke("main_window_hide");
      } catch (err) {
        console.error(err);
      }
      return;
    }
    if (k === "m" || k === "M") {
      e.preventDefault();
      try {
        await invoke("main_window_minimize");
      } catch (err) {
        console.error(err);
      }
    }
  });
}

function setSupportCopy(text) {
  const el = $("[data-support-copy]");
  if (el) el.textContent = text;
}

function bind() {
  for (const slider of $all('input[data-setting="volume"]')) {
    slider.addEventListener("input", () => {
      state.settings.volume = Number(slider.value);
      renderState();
    });
    slider.addEventListener("change", async () => {
      state.settings.volume = Number(slider.value);
      await persist();
    });
  }

  for (const inp of $all('input[data-setting="manual_seed"]')) {
    // Live waveform: re-render the preview from Rust as the user types,
    // without persisting on every keystroke.
    inp.addEventListener("input", () => {
      const v = Math.max(0, Math.floor(Number(inp.value) || 0));
      void runPreview(v);
    });
    // Persist + re-render once the user commits the value (blur / Enter).
    inp.addEventListener("change", async () => {
      const v = Math.max(0, Math.floor(Number(inp.value) || 0));
      state.settings.manual_seed = v;
      await persist();
    });
  }

  for (const sel of $all('[data-setting="quiet_start"]')) {
    sel.addEventListener("change", async () => {
      state.settings.quiet_start = sel.value === "" ? null : Math.min(23, Math.max(0, Number(sel.value)));
      renderState();
      await persist();
    });
  }
  for (const sel of $all('[data-setting="quiet_end"]')) {
    sel.addEventListener("change", async () => {
      state.settings.quiet_end = sel.value === "" ? null : Math.min(23, Math.max(0, Number(sel.value)));
      renderState();
      await persist();
    });
  }

  document.addEventListener("click", async (event) => {
    const outBtn = event.target.closest("[data-output]");
    if (outBtn) {
      state.settings.output = outBtn.dataset.output;
      renderState();
      await persist();
      return;
    }

    const playModeBtn = event.target.closest("[data-play-mode]");
    if (playModeBtn) {
      state.settings.play_mode = playModeBtn.dataset.playMode;
      renderState();
      await persist();
      return;
    }

    const autoPlayBtn = event.target.closest("[data-auto-play]");
    if (autoPlayBtn) {
      state.settings.auto_play_enabled = autoPlayBtn.dataset.autoPlay === "true";
      renderState();
      await persist();
      return;
    }

    const modeBtn = event.target.closest(".mode-item");
    if (modeBtn) {
      const nextVoice = modeBtn.dataset.voice;
      const voiceChanged = nextVoice !== state.settings.personality;
      if (voiceChanged) {
        state.settings.manual_seed = referenceSeedFor(nextVoice);
      }
      state.settings.personality = nextVoice;
      state.settings.play_mode = "single";
      renderState();
      await persist();
      return;
    }

    const actionEl = event.target.closest("[data-action]");
    if (!actionEl) return;
    const action = actionEl.dataset.action;
    if (action === "window-hide") {
      if (tauriInvoke) {
        try {
          await invoke("main_window_hide");
        } catch (err) {
          console.error(err);
        }
      }
      return;
    }
    if (action === "window-minimize") {
      if (tauriInvoke) {
        try {
          await invoke("main_window_minimize");
        } catch (err) {
          console.error(err);
        }
      }
      return;
    }
    if (action === "window-zoom") {
      if (tauriInvoke) {
        try {
          await invoke("main_window_toggle_maximize");
        } catch (err) {
          console.error(err);
        }
      }
      return;
    }
    if (action === "replay-preview") {
      const newSeed = Math.floor(Math.random() * 1e9);
      state.settings.manual_seed = newSeed;
      const seedInp = $('input[data-setting="manual_seed"]');
      if (seedInp) seedInp.value = String(newSeed);
      void runPreview(newSeed);
      await persist();
      return;
    }
    if (action === "fart-now") {
      actionEl.disabled = true;
      try {
        await invoke("fart_now");
        state = await invoke("get_app_snapshot");
        renderState();
        void refreshSnapshotAfterFart();
      } catch (err) {
        console.error(err);
      } finally {
        renderState();
        setTimeout(() => {
          actionEl.disabled = false;
        }, 500);
      }
      return;
    }
    if (action === "open-github") {
      await openGithub();
      return;
    }
    if (action === "complete-onboarding") {
      if (state.settings.onboarding_completed) return;
      actionEl.disabled = true;
      try {
        await invoke("complete_onboarding");
        state = await invoke("get_app_snapshot");
      } catch (err) {
        console.error("complete_onboarding failed", err);
        try {
          const saved = await invoke("set_settings", {
            newSettings: { ...state.settings, onboarding_completed: true },
          });
          if (saved) state.settings = saved;
        } catch (err2) {
          console.error("set_settings fallback failed", err2);
          state.settings = { ...state.settings, onboarding_completed: true };
        }
      } finally {
        renderState();
        actionEl.disabled = false;
      }
      return;
    }
    if (action === "reset-onboarding") {
      actionEl.disabled = true;
      try {
        await invoke("reset_onboarding");
        state = await invoke("get_app_snapshot");
      } catch (err) {
        console.error(err);
      } finally {
        actionEl.disabled = false;
        renderState();
      }
      return;
    }
    if (action === "export-audio-debug") {
      actionEl.disabled = true;
      try {
        const bundle = await invoke("export_audio_debug_bundle");
        if (bundle?.report_path) {
          setSupportCopy(
            `Audio debug written for ${bundle.chosen_personality} (seed ${bundle.manual_seed}) to ${bundle.report_path}. The paired WAV sits next to it and captures the exact desktop manual render path.`
          );
        }
      } catch (err) {
        console.error(err);
        setSupportCopy(
          "Could not export the audio debug bundle. Please try again after the app has fully loaded."
        );
      } finally {
        actionEl.disabled = false;
      }
      return;
    }
  });
}

async function mount() {
  try {
    state = await invoke("get_app_snapshot");
  } catch (err) {
    console.warn("snapshot fetch failed, using mock", err);
  }
  populateQuietSelects();
  renderModes();
  bind();
  bindKeyboardShortcuts();
  renderState();
  window.addEventListener("resize", () => schedulePreview());
}

mount().catch((err) => console.error(err));
