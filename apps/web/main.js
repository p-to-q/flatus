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
const liveDot = $("#live-dot");
const liveLabel = $("#live-label");
const bannerTrigger = $("#banner-trigger");
const overlayWave = $("#overlay-wave");
const overlayWaveCore = $("#overlay-wave-core");

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

// Shared "scope" palette — pulled once per draw so the page can switch from
// light to dark mode without reloading. Matches the banner's gradient + warm
// grain colours so the waveform sits visually inside the same instrument.
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
  // vertical gradient like the banner
  const grad = ctx.createLinearGradient(0, 0, 0, cssH);
  grad.addColorStop(0, c.bgTop);
  grad.addColorStop(0.55, c.bgMid);
  grad.addColorStop(1, c.bgBot);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, cssW, cssH);

  // bandpass corridor wash — warm orange centre band
  const band = ctx.createLinearGradient(0, 0, 0, cssH);
  band.addColorStop(0, "rgba(239, 126, 87, 0)");
  band.addColorStop(0.5, c.band);
  band.addColorStop(1, "rgba(239, 126, 87, 0)");
  ctx.fillStyle = band;
  ctx.fillRect(0, cssH * 0.2, cssW, cssH * 0.6);

  // centre rule
  ctx.strokeStyle = c.grid;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, cssH / 2);
  ctx.lineTo(cssW, cssH / 2);
  ctx.stroke();

  return c;
}

function strokeWaveformPath(ctx, peaks, cssW, cssH) {
  const cy = cssH / 2;
  ctx.beginPath();
  for (let x = 0; x < peaks.length; x++) {
    const { min, max } = peaks[x];
    const y1 = cy - max * cy;
    const y2 = cy - min * cy;
    if (x === 0) ctx.moveTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, y2);
  }
  ctx.stroke();
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

function drawWaveform(audioBuffer) {
  const canvas = waveformCanvas;
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth;
  const cssH = canvas.clientHeight;
  canvas.width = Math.floor(cssW * dpr);
  canvas.height = Math.floor(cssH * dpr);
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);

  const c = paintScopeBackground(ctx, cssW, cssH);

  // dBFS cap reference lines — drawn first so the waveform glow can overlap.
  const capLinear = currentInputs().headphones ? Math.pow(10, -18 / 20) : Math.pow(10, -6 / 20);
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

  const peaks = computePeaks(audioBuffer.getChannelData(0), cssW);

  // Two-pass glow: a wide, blurred, warm pass for the bloom; then a thin
  // bright pass for the readable line on top. Mirrors the banner's grain
  // bloom + bright core, but as a waveform.
  ctx.lineJoin = "round";
  ctx.shadowColor = c.glow;
  ctx.shadowBlur = 16;
  ctx.strokeStyle = c.glow;
  ctx.lineWidth = 1.6;
  strokeWaveformPath(ctx, peaks, cssW, cssH);

  ctx.shadowBlur = 0;
  ctx.strokeStyle = c.core;
  ctx.lineWidth = 0.9;
  strokeWaveformPath(ctx, peaks, cssW, cssH);
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
  const c = paintScopeBackground(ctx, cssW, cssH);
  ctx.fillStyle = c.muted;
  ctx.font = `12px "Berkeley Mono", ui-monospace, monospace`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("loading wasm…", cssW / 2, cssH / 2);
}

// Build an SVG path string from the waveform's per-pixel peaks. Used to draw
// the live overlay on top of the static PNG banner. Coordinates are in the
// SVG's viewBox space (1200×320), so the path scales with the banner.
function peaksToSvgPath(peaks, viewW, viewH) {
  const cy = viewH / 2;
  const step = viewW / peaks.length;
  let d = "";
  // First pass: top envelope (max values), left to right.
  for (let x = 0; x < peaks.length; x++) {
    const y = cy - peaks[x].max * cy * 0.85;
    d += (x === 0 ? "M" : "L") + (x * step).toFixed(2) + " " + y.toFixed(2) + " ";
  }
  // Second pass: bottom envelope (min values), right to left.
  for (let x = peaks.length - 1; x >= 0; x--) {
    const y = cy - peaks[x].min * cy * 0.85;
    d += "L" + (x * step).toFixed(2) + " " + y.toFixed(2) + " ";
  }
  d += "Z";
  return d;
}

function paintBannerOverlay(samples) {
  if (!overlayWave || !overlayWaveCore) return;
  // 200 columns is plenty for a 1200-wide viewBox at typical render sizes.
  const peaks = computePeaks(samples, 200);
  const d = peaksToSvgPath(peaks, 1200, 320);
  overlayWave.setAttribute("d", d);
  overlayWaveCore.setAttribute("d", d);
  // Fade in, then fade out after the audio's natural length plus a small tail.
  overlayWave.animate(
    [
      { opacity: 0 },
      { opacity: 0.55 },
      { opacity: 0 },
    ],
    { duration: 1400, easing: "cubic-bezier(0.2, 0.6, 0.2, 1)" },
  );
  overlayWaveCore.animate(
    [
      { opacity: 0 },
      { opacity: 0.9 },
      { opacity: 0 },
    ],
    { duration: 1400, easing: "cubic-bezier(0.2, 0.6, 0.2, 1)" },
  );
}

// Smaller waveform variant for specimen cards — same scope palette, simpler
// composition (single warm pass, no cap lines), drawn into the card's
// dedicated canvas.
function drawSpecimenWaveform(canvas, samples) {
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth;
  const cssH = canvas.clientHeight;
  canvas.width = Math.floor(cssW * dpr);
  canvas.height = Math.floor(cssH * dpr);
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);

  const c = paintScopeBackground(ctx, cssW, cssH);

  const peaks = computePeaks(samples, cssW);
  ctx.lineJoin = "round";
  ctx.shadowColor = c.glow;
  ctx.shadowBlur = 10;
  ctx.strokeStyle = c.glow;
  ctx.lineWidth = 1.2;
  strokeWaveformPath(ctx, peaks, cssW, cssH);
  ctx.shadowBlur = 0;
  ctx.strokeStyle = c.core;
  ctx.lineWidth = 0.7;
  strokeWaveformPath(ctx, peaks, cssW, cssH);
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
  // Mirror the waveform onto the banner overlay so the hero comes alive when
  // a render runs — a soft warm pulse that fades back into the spectrogram.
  paintBannerOverlay(buffer.getChannelData(0));

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
    const seed = SPECIMEN_DEFAULT_SEED[name] ?? 7;
    const li = document.createElement("li");
    li.className = "specimen";
    li.tabIndex = 0;
    li.setAttribute("role", "button");
    li.setAttribute("aria-label", `Load ${name} into the instrument and play`);
    li.innerHTML = `
      <div class="specimen-head">
        <div class="name">${name}</div>
        <div class="play">▸ play</div>
      </div>
      <canvas class="specimen-wave" width="560" height="120" aria-hidden="true"></canvas>
      <div class="desc">${desc}</div>
      <div class="specimen-meta"><span>seed ${seed}</span><span>pressure 0.60</span></div>
    `;
    const trigger = () => {
      setActivePersonality(name);
      seedInput.value = seed;
      pressureSlider.value = DEFAULT_PRESSURE;
      pressureReadout.value = DEFAULT_PRESSURE.toFixed(2);
      renderAndPlay();
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

    // Render this personality once and draw the thumbnail into the canvas.
    try {
      const bytes = renderWav(name, seed, DEFAULT_PRESSURE, false);
      if (bytes && bytes.length > 0) {
        const samples = decodeWavSamplesSync(bytes);
        // requestAnimationFrame so the canvas has a layout size before we read it.
        requestAnimationFrame(() => {
          drawSpecimenWaveform(li.querySelector(".specimen-wave"), samples);
        });
      }
    } catch (err) {
      console.warn(`specimen render failed for ${name}`, err);
    }
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

// Fade sections in as they enter the viewport. Skips work if the user has
// asked for reduced motion at the OS level — the CSS reveal rule is also
// guarded behind prefers-reduced-motion, so the elements just stay visible.
function wireScrollReveals() {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    for (const el of document.querySelectorAll(".reveal")) el.classList.add("revealed");
    return;
  }
  if (!("IntersectionObserver" in window)) {
    for (const el of document.querySelectorAll(".reveal")) el.classList.add("revealed");
    return;
  }
  const io = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          entry.target.classList.add("revealed");
          io.unobserve(entry.target);
        }
      }
    },
    { rootMargin: "0px 0px -10% 0px", threshold: 0.05 },
  );
  for (const el of document.querySelectorAll(".reveal")) io.observe(el);
}

// Copy-to-clipboard for the CLI codeblock. Strips the leading `$ ` prompt and
// the inline comments so what ends up in the clipboard is just the commands.
function wireCliCopy() {
  const btn = document.getElementById("copy-cli");
  const code = document.getElementById("cli-code");
  if (!btn || !code) return;
  btn.addEventListener("click", async () => {
    const lines = code.innerText
      .split("\n")
      .map((l) => l.replace(/^\$\s?/, "").replace(/\s+#.*$/, ""))
      .filter((l) => l.trim().length > 0);
    const text = lines.join("\n");
    try {
      await navigator.clipboard.writeText(text);
      btn.dataset.state = "copied";
      btn.textContent = "copied";
      setTimeout(() => {
        btn.removeAttribute("data-state");
        btn.textContent = "copy";
      }, 1400);
    } catch {
      btn.textContent = "press ⌘C";
    }
  });
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
  wireCliCopy();
  wireScrollReveals();

  // Banner is a giant button — click anywhere on the spectrogram and we
  // randomise the inputs and play. Nice surprise interaction; the page
  // suddenly produces a real fart when the visitor pokes the picture.
  if (bannerTrigger) {
    bannerTrigger.addEventListener("click", () => {
      if (!wasmReady) return;
      const pick = personalities[Math.floor(Math.random() * personalities.length)] || "default";
      const randomSeed = Math.floor(Math.random() * 1e8);
      setActivePersonality(pick);
      seedInput.value = randomSeed;
      pressureSlider.value = DEFAULT_PRESSURE;
      pressureReadout.value = DEFAULT_PRESSURE.toFixed(2);
      renderAndPlay();
    });
  }

  // Live-dot: green warm pulse while wasm is loading, steady when ready.
  if (liveDot && liveLabel) {
    liveDot.classList.add("ready");
    liveLabel.textContent = "live";
  }
}

boot();
