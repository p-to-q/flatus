// flatus — landing page wiring.
//
// Boots the wasm-bindgen module produced from `crates/fart-synth`, then drives
// the live synth panel, the specimen grid, and the download CTA. No frameworks,
// no build step — this is a hand-written ES module loaded directly by index.html.

import init, {
  renderWav,
  listPersonalities,
  version as wasmVersion,
} from "./wasm/fart_synth.js";

// --- Static content ------------------------------------------------------------

// Short, human descriptions for the four canonical voices. Keep the silly
// register; the technical voice lives in docs/, not on the marketing page.
const PERSONALITY_DESCRIPTIONS = {
  "polite-cough": "short, dry, plausibly deniable.",
  "default": "the canon. wet enough, not so wet.",
  "biblical": "slow, low, devastating.",
  "silent-but-deadly": "exactly what it says.",
};

// Per-personality default seed for the specimen previews. Picked once so the
// preview audio you hear here matches the WAV you would render with the same
// inputs in the CLI — i.e. these are reproducible, not random.
const SPECIMEN_DEFAULT_SEED = {
  "polite-cough": 7,
  "default": 17,
  "biblical": 31,
  "silent-but-deadly": 9,
};
const DEFAULT_PRESSURE = 0.6;

// --- Page lookups --------------------------------------------------------------

const $ = (sel) => document.querySelector(sel);
const personalityGroup = $("#personality-group");
const specimenGrid = $("#specimen-grid");
const pressureSlider = $("#pressure");
const pressureReadout = $("#pressure-readout");
const seedInput = $("#seed");
const seedRandomBtn = $("#seed-random");
const capRadios = document.querySelectorAll('input[name="cap"]');
const renderBtn = $("#render-btn");
const renderStatus = $("#render-status");
const renderLabel = renderBtn.querySelector(".render-label");
const downloadWav = $("#download-wav");
const waveformCanvas = $("#waveform");
const synthVersionEl = $("#synth-version");
const ctaBtn = $("#download-cta");
const ctaMeta = $("#cta-meta");
const ctaHint = $("#cta-hint");
const installSection = $("#install");

// --- State ---------------------------------------------------------------------

let personalities = []; // string[]
let activePersonality = "default";
let audioCtx = null;
let lastPlaybackSource = null;
let lastWavBlob = null;
let lastWavUrl = null;
let wasmReady = false;

// --- Helpers -------------------------------------------------------------------

function ensureAudioCtx() {
  if (!audioCtx) {
    audioCtx = new (window.AudioContext || window.webkitAudioContext)({
      sampleRate: 48000,
    });
  }
  if (audioCtx.state === "suspended") {
    audioCtx.resume();
  }
  return audioCtx;
}

function setStatus(text) {
  renderStatus.textContent = text;
}

function setRenderEnabled(enabled) {
  renderBtn.disabled = !enabled;
}

function fmtBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}

function bytesToBlobUrl(bytes) {
  if (lastWavUrl) URL.revokeObjectURL(lastWavUrl);
  lastWavBlob = new Blob([bytes], { type: "audio/wav" });
  lastWavUrl = URL.createObjectURL(lastWavBlob);
  return lastWavUrl;
}

function currentInputs() {
  const cap = Array.from(capRadios).find((r) => r.checked)?.value ?? "speakers";
  return {
    personality: activePersonality,
    seed: Math.max(0, Math.floor(Number(seedInput.value) || 0)),
    pressure: Number(pressureSlider.value),
    headphones: cap === "headphones",
  };
}

// --- Waveform rendering --------------------------------------------------------

function drawWaveform(audioBuffer) {
  const canvas = waveformCanvas;
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth;
  const cssH = canvas.clientHeight;
  canvas.width = Math.floor(cssW * dpr);
  canvas.height = Math.floor(cssH * dpr);
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);

  const styles = getComputedStyle(document.documentElement);
  const ink = styles.getPropertyValue("--ink-2").trim() || "#3a3128";
  const accent = styles.getPropertyValue("--accent").trim() || "#8c2f1e";
  const rule = styles.getPropertyValue("--rule").trim() || "#d9cfb7";
  const paper2 = styles.getPropertyValue("--paper-2").trim() || "#efe7d2";

  ctx.fillStyle = paper2;
  ctx.fillRect(0, 0, cssW, cssH);

  // centre line + ±1 reference lines
  ctx.strokeStyle = rule;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, cssH / 2);
  ctx.lineTo(cssW, cssH / 2);
  ctx.stroke();

  // Compute peaks per pixel.
  const data = audioBuffer.getChannelData(0);
  const samplesPerPixel = Math.max(1, Math.floor(data.length / cssW));
  ctx.strokeStyle = ink;
  ctx.lineWidth = 1.5;
  ctx.lineJoin = "round";
  ctx.beginPath();
  for (let x = 0; x < cssW; x++) {
    let min = 1;
    let max = -1;
    const start = x * samplesPerPixel;
    const end = Math.min(start + samplesPerPixel, data.length);
    for (let i = start; i < end; i++) {
      const v = data[i];
      if (v < min) min = v;
      if (v > max) max = v;
    }
    const y1 = (1 - max) * (cssH / 2);
    const y2 = (1 - min) * (cssH / 2);
    if (x === 0) ctx.moveTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, y2);
  }
  ctx.stroke();

  // Cap reference at the configured dBFS (drawn as faint dashed lines).
  const capLinear = currentInputs().headphones ? Math.pow(10, -18 / 20) : Math.pow(10, -6 / 20);
  ctx.setLineDash([4, 4]);
  ctx.strokeStyle = accent;
  ctx.globalAlpha = 0.35;
  ctx.beginPath();
  ctx.moveTo(0, (1 - capLinear) * (cssH / 2));
  ctx.lineTo(cssW, (1 - capLinear) * (cssH / 2));
  ctx.moveTo(0, (1 + capLinear) * (cssH / 2));
  ctx.lineTo(cssW, (1 + capLinear) * (cssH / 2));
  ctx.stroke();
  ctx.setLineDash([]);
  ctx.globalAlpha = 1;
}

function clearWaveformBlank() {
  const canvas = waveformCanvas;
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth;
  const cssH = canvas.clientHeight;
  canvas.width = Math.floor(cssW * dpr);
  canvas.height = Math.floor(cssH * dpr);
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);
  const styles = getComputedStyle(document.documentElement);
  ctx.fillStyle = styles.getPropertyValue("--paper-2").trim() || "#efe7d2";
  ctx.fillRect(0, 0, cssW, cssH);
  ctx.fillStyle = styles.getPropertyValue("--muted").trim() || "#8d836d";
  ctx.font = `12px "Berkeley Mono", ui-monospace, monospace`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("press render to synthesize", cssW / 2, cssH / 2);
}

// Decode the 16-bit mono PCM samples out of a WAV byte array without going
// through Web Audio. Lets us draw the waveform on first paint, before the
// browser's autoplay policy lets us instantiate an AudioContext.
function decodeWavSamplesSync(bytes) {
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  // The header is 44 bytes for the canonical RIFF/fmt /data layout produced
  // by `write_wav_into`. We don't bother scanning for the chunk; it's fixed.
  const headerLen = 44;
  const n = Math.max(0, Math.floor((dv.byteLength - headerLen) / 2));
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    out[i] = dv.getInt16(headerLen + i * 2, true) / 32768;
  }
  return out;
}

function fakeAudioBufferFromSamples(samples, sampleRate) {
  return {
    duration: samples.length / sampleRate,
    sampleRate,
    numberOfChannels: 1,
    length: samples.length,
    getChannelData: () => samples,
  };
}

// --- Render & play -------------------------------------------------------------

async function renderAndPlay({ autoPlay = true } = {}) {
  if (!wasmReady) {
    setStatus("wasm not loaded yet…");
    return;
  }
  const { personality, seed, pressure, headphones } = currentInputs();
  setRenderEnabled(false);
  setStatus(`rendering ${personality}…`);
  const t0 = performance.now();
  let bytes;
  try {
    bytes = renderWav(personality, seed, pressure, headphones);
  } catch (err) {
    console.error(err);
    setStatus(`render failed: ${err?.message || err}`);
    setRenderEnabled(true);
    return;
  }
  const elapsed = performance.now() - t0;

  if (!bytes || bytes.length === 0) {
    setStatus(`unknown personality: ${personality}`);
    setRenderEnabled(true);
    return;
  }

  const ctx = ensureAudioCtx();
  // decodeAudioData wants a fresh ArrayBuffer; copy into one.
  const ab = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
  let buffer;
  try {
    buffer = await ctx.decodeAudioData(ab);
  } catch (err) {
    console.error(err);
    setStatus("playback decode failed (browser audio)");
    setRenderEnabled(true);
    return;
  }

  drawWaveform(buffer);

  if (autoPlay) {
    if (lastPlaybackSource) {
      try { lastPlaybackSource.stop(); } catch { /* already stopped */ }
    }
    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.connect(ctx.destination);
    src.start();
    lastPlaybackSource = src;
  }

  const url = bytesToBlobUrl(bytes);
  downloadWav.hidden = false;
  downloadWav.href = url;
  downloadWav.download = `flatus-${personality}-${seed}.wav`;

  setStatus(
    `${personality} · seed ${seed} · ${(buffer.duration).toFixed(2)}s · ${fmtBytes(bytes.length)} · synth ${elapsed.toFixed(0)} ms`,
  );
  setRenderEnabled(true);
}

// --- Personality buttons + specimen grid ---------------------------------------

function setActivePersonality(name, { autoRender = false } = {}) {
  if (!personalities.includes(name)) return;
  activePersonality = name;
  for (const btn of personalityGroup.querySelectorAll(".personality")) {
    btn.setAttribute("aria-pressed", btn.dataset.personality === name ? "true" : "false");
  }
  if (autoRender) renderAndPlay();
}

function buildPersonalityButtons() {
  personalityGroup.innerHTML = "";
  for (const name of personalities) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "personality";
    btn.dataset.personality = name;
    btn.setAttribute("role", "radio");
    btn.setAttribute("aria-pressed", name === activePersonality ? "true" : "false");
    btn.textContent = name;
    btn.addEventListener("click", () => setActivePersonality(name));
    personalityGroup.appendChild(btn);
  }
}

function buildSpecimenGrid() {
  specimenGrid.innerHTML = "";
  for (const name of personalities) {
    const desc = PERSONALITY_DESCRIPTIONS[name] || "—";
    const li = document.createElement("li");
    li.className = "specimen";
    li.tabIndex = 0;
    li.setAttribute("role", "button");
    li.setAttribute("aria-label", `Load ${name} into the instrument and play`);
    li.innerHTML = `
      <div class="name">${name}</div>
      <div class="desc">${desc}</div>
      <div class="play">▸ play</div>
    `;
    const trigger = () => {
      setActivePersonality(name);
      seedInput.value = SPECIMEN_DEFAULT_SEED[name] ?? 7;
      pressureSlider.value = DEFAULT_PRESSURE;
      pressureReadout.value = DEFAULT_PRESSURE.toFixed(2);
      renderAndPlay();
      // Scroll to the instrument so the user sees the rendered waveform.
      document.getElementById("instrument").scrollIntoView({ behavior: "smooth", block: "start" });
    };
    li.addEventListener("click", trigger);
    li.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        trigger();
      }
    });
    specimenGrid.appendChild(li);
  }
}

// --- Download CTA --------------------------------------------------------------

const RELEASE_BASE = "https://github.com/p-to-q/flatus/releases/latest";

function detectArch() {
  const ua = navigator.userAgent || "";
  const platform = navigator.platform || "";
  const isMac = /Mac/i.test(platform) || /Mac OS X/i.test(ua);
  if (!isMac) {
    // Best-effort: detect Linux / Windows for the message; download still goes
    // to the releases page so the user can pick the right asset.
    if (/Win/i.test(platform)) return { os: "windows", arch: "unknown" };
    if (/Linux/i.test(platform)) return { os: "linux", arch: "unknown" };
    return { os: "unknown", arch: "unknown" };
  }
  // navigator.userAgent on Apple Silicon Safari can still report Intel for
  // compatibility, so we rely on WebGL renderer as a hint.
  let arch = "arm64";
  try {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
    if (gl) {
      const dbg = gl.getExtension("WEBGL_debug_renderer_info");
      const renderer = dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : "";
      if (typeof renderer === "string" && /Intel/i.test(renderer)) arch = "x86_64";
    }
  } catch {
    // ignore; default arm64 (v0.1 only ships aarch64 anyway)
  }
  return { os: "macos", arch };
}

function applyArchToCta(detected) {
  if (detected.os === "macos") {
    const archLabel = detected.arch === "x86_64" ? "Intel" : "Apple Silicon";
    ctaMeta.textContent = `for macOS · ${archLabel} · v0.1.0 · unsigned`;
    ctaHint.textContent = `Detected ${archLabel}. v0.1.0 is unsigned — Gatekeeper needs one right-click → Open the first time.`;
    ctaBtn.dataset.state = "ready";
  } else if (detected.os === "linux") {
    ctaMeta.textContent = `Linux · build from source`;
    ctaHint.textContent = "Linux build is CLI-only for v0.1. Scroll to the Command line section.";
    ctaBtn.href = "#cli";
    ctaBtn.dataset.state = "redirect-cli";
  } else if (detected.os === "windows") {
    ctaMeta.textContent = `Windows · build from source`;
    ctaHint.textContent = "Windows isn't packaged yet for v0.1 — the CLI builds with cargo. See below.";
    ctaBtn.href = "#cli";
    ctaBtn.dataset.state = "redirect-cli";
  } else {
    ctaHint.textContent = "Couldn't detect your platform — head to the releases page.";
  }
}

function wireDownloadCta() {
  const detected = detectArch();
  applyArchToCta(detected);
  ctaBtn.href = RELEASE_BASE;
  ctaBtn.addEventListener("click", (e) => {
    if (ctaBtn.dataset.state === "ready") {
      // Reveal install steps; let the link navigate to the release page in a new tab.
      installSection.hidden = false;
      installSection.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  });
}

// --- Boot ----------------------------------------------------------------------

async function boot() {
  clearWaveformBlank();
  setStatus("loading wasm…");
  setRenderEnabled(false);

  try {
    await init();
    wasmReady = true;
  } catch (err) {
    console.error(err);
    setStatus(`failed to load synth: ${err?.message || err}`);
    return;
  }

  const personalitiesCsv = listPersonalities();
  personalities = personalitiesCsv.split(",").filter(Boolean);
  if (!personalities.includes(activePersonality)) {
    activePersonality = personalities[0] || "default";
  }
  buildPersonalityButtons();
  buildSpecimenGrid();

  synthVersionEl.textContent = `Rust → wasm32 · ${wasmVersion()}`;
  setRenderEnabled(true);

  // First paint: render the default personality synchronously and draw the
  // waveform on canvas, without instantiating an AudioContext. Browsers block
  // AudioContext until a user gesture; the WAV bytes have no such restriction.
  // This way the page shows a real synthesised waveform instead of a "press
  // render" placeholder before the user has clicked anything.
  try {
    const { personality, seed, pressure, headphones } = currentInputs();
    const t0 = performance.now();
    const bytes = renderWav(personality, seed, pressure, headphones);
    const elapsed = performance.now() - t0;
    if (bytes && bytes.length > 0) {
      const samples = decodeWavSamplesSync(bytes);
      drawWaveform(fakeAudioBufferFromSamples(samples, 48000));
      const url = bytesToBlobUrl(bytes);
      downloadWav.hidden = false;
      downloadWav.href = url;
      downloadWav.download = `flatus-${personality}-${seed}.wav`;
      setStatus(
        `previewed ${personality} · seed ${seed} · ${(samples.length / 48000).toFixed(2)}s · ${fmtBytes(bytes.length)} · synth ${elapsed.toFixed(0)} ms · press to play`,
      );
    } else {
      setStatus("ready · press render");
    }
  } catch (err) {
    console.error(err);
    setStatus("ready · press render");
  }

  // wire control interactions
  pressureSlider.addEventListener("input", () => {
    pressureReadout.value = Number(pressureSlider.value).toFixed(2);
  });
  pressureReadout.value = Number(pressureSlider.value).toFixed(2);
  seedRandomBtn.addEventListener("click", () => {
    seedInput.value = Math.floor(Math.random() * 1e8);
  });
  renderBtn.addEventListener("click", () => renderAndPlay());
  // Re-draw cap reference lines on cap change without rerendering audio.
  for (const r of capRadios) {
    r.addEventListener("change", () => {
      // Cheap refresh: if we have a buffer cached on the source, redraw from it;
      // otherwise just keep waiting for the next render.
      if (lastPlaybackSource?.buffer) drawWaveform(lastPlaybackSource.buffer);
    });
  }

  wireDownloadCta();
}

boot();
