// flatus — settings popover frontend.
//
// Vanilla JS, no build step. The webview is UI only — synthesis happens in Rust.
// We use the `withGlobalTauri: true` config flag to expose `window.__TAURI__`
// so we can call commands without bundling `@tauri-apps/api`.
//
// If you want TS, `main.ts` next to this file has the same logic with types;
// compile it via `pnpm tsc` and point index.html at the output.

const invoke = window.__TAURI__.core.invoke;

const $ = (id) => document.getElementById(id);

const personalitySel = $("personality");
const volumeSlider = $("volume");
const volumeReadout = $("volume-readout");
const outputSel = $("output");
const quietStart = $("quiet-start");
const quietEnd = $("quiet-end");
const fartNowBtn = $("fart-now");

let current = {
  personality: "default",
  volume: 1,
  output: "headphones",
  quiet_start: null,
  quiet_end: null,
};

async function mount() {
  const personalities = await invoke("list_personalities");
  // Drop the static fallback options (used when the page is rendered without
  // a Tauri runtime — e.g. screenshots or web previews) and replace with the
  // live list from Rust.
  personalitySel.innerHTML = "";
  for (const name of personalities) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = name;
    personalitySel.appendChild(opt);
  }

  const loaded = await invoke("get_settings");
  current = loaded;
  personalitySel.value = current.personality;
  volumeSlider.value = String(current.volume);
  outputSel.value = current.output;
  quietStart.value = current.quiet_start?.toString() ?? "";
  quietEnd.value = current.quiet_end?.toString() ?? "";
  updateVolumeReadout();

  personalitySel.addEventListener("change", () => {
    current.personality = personalitySel.value;
    push();
  });
  volumeSlider.addEventListener("input", () => {
    current.volume = parseFloat(volumeSlider.value);
    updateVolumeReadout();
    push();
  });
  outputSel.addEventListener("change", () => {
    current.output = outputSel.value;
    push();
  });
  quietStart.addEventListener("input", () => {
    current.quiet_start = quietStart.value ? parseInt(quietStart.value, 10) : null;
    push();
  });
  quietEnd.addEventListener("input", () => {
    current.quiet_end = quietEnd.value ? parseInt(quietEnd.value, 10) : null;
    push();
  });
  fartNowBtn.addEventListener("click", async () => {
    fartNowBtn.disabled = true;
    try {
      await invoke("fart_now");
    } finally {
      setTimeout(() => (fartNowBtn.disabled = false), 1000);
    }
  });
}

function push() {
  invoke("set_settings", { newSettings: current }).catch((e) => console.error(e));
}

function updateVolumeReadout() {
  volumeReadout.textContent = `${Math.round(current.volume * 100)}%`;
}

mount().catch((e) => console.error(e));
