#!/usr/bin/env python3
"""Render the README visuals in the banner's visual language.

Inputs: the four canonical golden WAVs at fixtures/golden/{name}.wav.
Outputs (overwrites):
  docs/screenshots/waveforms-all.svg + .png
  docs/screenshots/spectrogram-biblical.svg + .png

Visual language (mirrors apps/web/banner.svg):
  - vertical dark gradient backdrop  #15161b → #1c1d24 → #0f1014
  - warm grain palette: #ce552a (rail), #ef7e57 (glow), #fff3d6 (core)
  - feTurbulence paper grain overlay at 4% alpha
  - feGaussianBlur bloom on the data layers
  - personality labels in Charter / Iowan / system serif italic with wide tracking
  - small caps mono for axis labels + the [ p → q ] / spec frame

Run from the repo root:
  python3 scripts/render_visuals.py
  resvg --width 1600 docs/screenshots/waveforms-all.svg docs/screenshots/waveforms-all.png
  resvg --width 1600 docs/screenshots/spectrogram-biblical.svg docs/screenshots/spectrogram-biblical.png
"""

from __future__ import annotations

import math
import wave
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parent.parent
FIX = REPO / "fixtures" / "golden"
OUT = REPO / "docs" / "screenshots"

PERSONALITIES = ["polite-cough", "default", "biblical", "silent-but-deadly"]

# ---------- shared SVG fragments ------------------------------------------------

DEFS = """\
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"  stop-color="#15161b"/>
      <stop offset="55%" stop-color="#1c1d24"/>
      <stop offset="100%" stop-color="#0f1014"/>
    </linearGradient>
    <linearGradient id="band" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"  stop-color="#ce552a" stop-opacity="0.0"/>
      <stop offset="50%" stop-color="#ce552a" stop-opacity="0.08"/>
      <stop offset="100%" stop-color="#ce552a" stop-opacity="0.0"/>
    </linearGradient>
    <filter id="bloom" x="-10%" y="-50%" width="120%" height="200%">
      <feGaussianBlur stdDeviation="3.5"/>
    </filter>
    <filter id="bloom-strong" x="-10%" y="-50%" width="120%" height="200%">
      <feGaussianBlur stdDeviation="6"/>
    </filter>
    <filter id="grain" x="0%" y="0%" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="3"/>
      <feColorMatrix values="0 0 0 0 1
                              0 0 0 0 1
                              0 0 0 0 1
                              0 0 0 0.04 0"/>
    </filter>
    <radialGradient id="vignette" cx="50%" cy="50%" r="65%">
      <stop offset="0%" stop-color="#000" stop-opacity="0"/>
      <stop offset="100%" stop-color="#000" stop-opacity="0.40"/>
    </radialGradient>
  </defs>"""

# Charter / Iowan / system serif. Setting the family directly lets resvg pick
# whichever is on the host (Charter is the macOS default; Linux falls back to
# Iowan / Source Serif / Cambria / Georgia).
DISPLAY = (
    "Charter, 'Iowan Old Style', 'Source Serif Pro', "
    "'Apple Garamond', Cambria, Georgia, serif"
)
# SF Mono / Berkeley / JetBrains Mono, in priority order.
MONO = "'Berkeley Mono', 'JetBrains Mono', 'SF Mono', Menlo, monospace"


def read_wav(path: Path) -> tuple[int, np.ndarray]:
    """Read a mono int16 PCM WAV and return (sample_rate, float samples in [-1, 1])."""
    with wave.open(str(path), "rb") as w:
        assert w.getnchannels() == 1, f"{path} is not mono"
        assert w.getsampwidth() == 2, f"{path} is not 16-bit"
        sr = w.getframerate()
        n = w.getnframes()
        raw = w.readframes(n)
    arr = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
    return sr, arr


def peaks(samples: np.ndarray, cols: int) -> np.ndarray:
    """Down-sample by min/max peak per column. Shape: (cols, 2)."""
    if len(samples) == 0:
        return np.zeros((cols, 2), dtype=np.float32)
    per = max(1, len(samples) // cols)
    out = np.zeros((cols, 2), dtype=np.float32)
    for x in range(cols):
        seg = samples[x * per : (x + 1) * per]
        if seg.size:
            out[x, 0] = seg.min()
            out[x, 1] = seg.max()
    return out


def peaks_to_envelope_path(
    pk: np.ndarray, x0: float, x1: float, y_center: float, y_amp: float
) -> str:
    """Build an SVG path that draws the waveform as a filled envelope (top edge
    then bottom edge back). Suitable for both stroke and fill."""
    cols = pk.shape[0]
    span = x1 - x0
    parts: list[str] = []
    for x in range(cols):
        sx = x0 + (x / max(cols - 1, 1)) * span
        sy = y_center - pk[x, 1] * y_amp
        parts.append(("M" if x == 0 else "L") + f"{sx:.2f} {sy:.2f}")
    for x in range(cols - 1, -1, -1):
        sx = x0 + (x / max(cols - 1, 1)) * span
        sy = y_center - pk[x, 0] * y_amp
        parts.append("L" + f"{sx:.2f} {sy:.2f}")
    parts.append("Z")
    return " ".join(parts)


# ---------- waveforms-all -------------------------------------------------------


def render_waveforms_all() -> None:
    """Stacked four-lane waveform comparison. Each lane carries its own envelope
    on the same shared time axis."""
    sr, _ = read_wav(FIX / "default.wav")
    samples_all: dict[str, tuple[int, np.ndarray]] = {
        name: read_wav(FIX / f"{name}.wav") for name in PERSONALITIES
    }
    max_dur_s = max(arr.shape[0] / s for s, arr in samples_all.values())
    # Round up to a friendly tick (0.5s granularity).
    axis_end_s = math.ceil(max_dur_s * 2) / 2

    W, H = 1600, 600
    margin_l, margin_r = 80, 60
    margin_t, margin_b = 90, 80
    lane_h = (H - margin_t - margin_b) / len(PERSONALITIES)
    plot_w = W - margin_l - margin_r

    svg: list[str] = []
    svg.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
        f'role="img" aria-label="Waveform comparison of all four personalities, '
        f'shared time axis up to {axis_end_s:.2f} seconds.">'
    )
    svg.append(DEFS)
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#bg)"/>')
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#band)"/>')

    # frame header
    svg.append(
        f'<text x="{margin_l}" y="44" font-family="{MONO}" font-size="14" '
        f'fill="#ef7e57" letter-spacing="3" text-transform="uppercase">[ p → q ]</text>'
    )
    svg.append(
        f'<text x="{W - margin_r}" y="44" font-family="{MONO}" font-size="14" '
        f'fill="#8d836d" letter-spacing="3" text-anchor="end">spec 01 · flatus 0.1.0</text>'
    )
    svg.append(
        f'<text x="{margin_l}" y="70" font-family="{DISPLAY}" font-style="italic" '
        f'font-size="22" fill="#f0e7d3" letter-spacing="0.5">'
        f'four canonical voices, shared time axis</text>'
    )

    # data layers
    for i, name in enumerate(PERSONALITIES):
        s_sr, samples = samples_all[name]
        n = samples.shape[0]
        dur_s = n / s_sr
        # Each lane's right-most x is at (dur / axis_end_s) of the plot width.
        lane_y_center = margin_t + lane_h * (i + 0.5)
        x0 = margin_l
        x1 = margin_l + plot_w * (dur_s / axis_end_s)
        pk = peaks(samples, max(80, int((x1 - x0) / 2)))
        amp = lane_h * 0.42
        d = peaks_to_envelope_path(pk, x0, x1, lane_y_center, amp)

        # baseline rule
        svg.append(
            f'<line x1="{margin_l}" y1="{lane_y_center:.2f}" '
            f'x2="{W - margin_r}" y2="{lane_y_center:.2f}" '
            f'stroke="rgba(239, 126, 87, 0.10)" stroke-width="1"/>'
        )
        # glow + bright-core two-pass envelope, like the apps/web waveform
        svg.append(
            f'<path d="{d}" fill="#ef7e57" fill-opacity="0.55" '
            f'filter="url(#bloom-strong)"/>'
        )
        svg.append(
            f'<path d="{d}" fill="none" stroke="#ef7e57" stroke-width="0.9" '
            f'filter="url(#bloom)" opacity="0.9"/>'
        )
        svg.append(
            f'<path d="{d}" fill="none" stroke="#fff3d6" stroke-width="0.55" '
            f'opacity="0.95"/>'
        )

        # label — display serif italic, wide tracking
        label_y = lane_y_center - lane_h * 0.32
        svg.append(
            f'<text x="{margin_l}" y="{label_y:.2f}" font-family="{DISPLAY}" '
            f'font-style="italic" font-size="26" fill="#f0e7d3" '
            f'letter-spacing="0.6">{name}</text>'
        )
        # mono duration label, right-aligned, wide tracking
        ms = int(round(dur_s * 1000))
        svg.append(
            f'<text x="{W - margin_r}" y="{label_y:.2f}" font-family="{MONO}" '
            f'font-size="12" fill="#8d836d" letter-spacing="2" '
            f'text-anchor="end" text-transform="uppercase">{ms} MS</text>'
        )

    # shared time axis ticks
    ticks = 8
    for k in range(ticks + 1):
        t = (k / ticks) * axis_end_s
        x = margin_l + plot_w * (t / axis_end_s)
        svg.append(
            f'<line x1="{x:.2f}" y1="{H - margin_b}" x2="{x:.2f}" '
            f'y2="{H - margin_b + 6}" stroke="rgba(239, 126, 87, 0.30)" '
            f'stroke-width="1"/>'
        )
        svg.append(
            f'<text x="{x:.2f}" y="{H - margin_b + 24}" font-family="{MONO}" '
            f'font-size="12" fill="#8d836d" letter-spacing="2" '
            f'text-anchor="middle">{t:.2f}</text>'
        )
    svg.append(
        f'<text x="{W/2:.2f}" y="{H - 22}" font-family="{MONO}" '
        f'font-size="12" fill="#8d836d" letter-spacing="4" '
        f'text-anchor="middle" text-transform="uppercase">seconds · shared scale</text>'
    )

    # paper grain on top, low alpha
    svg.append(f'<rect width="{W}" height="{H}" fill="#000" filter="url(#grain)"/>')
    # vignette
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#vignette)"/>')
    svg.append("</svg>")

    out = OUT / "waveforms-all.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


# ---------- spectrogram-biblical ------------------------------------------------


def render_spectrogram_biblical() -> None:
    """STFT-based spectrogram of biblical.wav with a warm grain heatmap and a
    bloomed copy on top for the banner-style glow."""
    sr, samples = read_wav(FIX / "biblical.wav")
    dur_s = samples.shape[0] / sr

    # STFT params: 1024-pt window, 50% hop. Hann window.
    nfft = 1024
    hop = 256
    win = 0.5 - 0.5 * np.cos(2 * np.pi * np.arange(nfft) / nfft)
    n_frames = max(1, 1 + (len(samples) - nfft) // hop)
    frames = np.lib.stride_tricks.as_strided(
        samples,
        shape=(n_frames, nfft),
        strides=(samples.strides[0] * hop, samples.strides[0]),
        writeable=False,
    ).copy()
    frames *= win
    spec = np.abs(np.fft.rfft(frames, axis=1))  # (frames, nfft/2 + 1)

    # Convert to dB, clip floor.
    eps = 1e-8
    db = 20.0 * np.log10(spec + eps)
    db_max = float(db.max())
    db -= db_max  # peak normalize → 0 dB top
    db_floor = -55.0
    db = np.clip(db, db_floor, 0.0)
    # 0 = brightest, db_floor = transparent. Map to opacity [0, 1].
    intensity = (db - db_floor) / (-db_floor)

    # We only care about 50 Hz–2.5 kHz for visual band — that's where flatus
    # lives. Log-frequency Y.
    f_lo, f_hi = 50.0, 2500.0
    freqs = np.fft.rfftfreq(nfft, 1.0 / sr)
    f_mask = (freqs >= f_lo) & (freqs <= f_hi)
    intensity = intensity[:, f_mask]
    band_freqs = freqs[f_mask]
    # Resample y axis logarithmically to 110 bins.
    y_bins = 110
    log_lo, log_hi = math.log(f_lo), math.log(f_hi)
    log_centres = np.linspace(log_lo, log_hi, y_bins)
    src_logs = np.log(band_freqs + 1e-9)
    # Nearest neighbour on log-freq axis.
    src_idx = np.searchsorted(src_logs, log_centres)
    src_idx = np.clip(src_idx, 0, intensity.shape[1] - 1)
    intensity = intensity[:, src_idx]

    # Resample x axis to 320 columns for cell count.
    x_cols = 320
    t_idx = np.linspace(0, intensity.shape[0] - 1, x_cols).astype(int)
    intensity = intensity[t_idx, :]

    W, H = 1600, 520
    margin_l, margin_r, margin_t, margin_b = 80, 70, 95, 80
    plot_w = W - margin_l - margin_r
    plot_h = H - margin_t - margin_b
    cell_w = plot_w / x_cols
    cell_h = plot_h / y_bins

    svg: list[str] = []
    svg.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
        f'role="img" aria-label="Spectrogram of biblical.wav across {dur_s:.2f} seconds, '
        f'energy concentrated between 60 Hz and 2 kHz.">'
    )
    svg.append(DEFS)
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#bg)"/>')

    # frame header
    svg.append(
        f'<text x="{margin_l}" y="44" font-family="{MONO}" font-size="14" '
        f'fill="#ef7e57" letter-spacing="3" text-transform="uppercase">[ p → q ]</text>'
    )
    svg.append(
        f'<text x="{W - margin_r}" y="44" font-family="{MONO}" font-size="14" '
        f'fill="#8d836d" letter-spacing="3" text-anchor="end">spec 01 · flatus 0.1.0</text>'
    )
    svg.append(
        f'<text x="{margin_l}" y="74" font-family="{DISPLAY}" font-style="italic" '
        f'font-size="30" fill="#f0e7d3" letter-spacing="0.7">biblical.wav</text>'
    )
    svg.append(
        f'<text x="{margin_l + 220}" y="74" font-family="{MONO}" font-size="13" '
        f'fill="#8d836d" letter-spacing="3" text-transform="uppercase">'
        f'seed 3 · pressure 0.8 · 48 kHz mono · spectrogram</text>'
    )

    # cells — emit as a single <g> with stroke-free rects to keep file small
    svg.append('<g id="heatmap">')
    for x in range(x_cols):
        # Stack from the bottom (low freq) up.
        col = intensity[x]
        for y in range(y_bins):
            alpha = float(col[y])
            if alpha < 0.04:
                continue
            # Map intensity to a warm gradient: low = #7a2812, high = #fff3d6
            # Choose 3 anchors for nice falloff.
            if alpha > 0.78:
                fill = "#fff3d6"
            elif alpha > 0.55:
                fill = "#f5a26b"
            elif alpha > 0.3:
                fill = "#ce552a"
            else:
                fill = "#7a2812"
            rx = margin_l + x * cell_w
            ry = H - margin_b - (y + 1) * cell_h
            svg.append(
                f'<rect x="{rx:.2f}" y="{ry:.2f}" width="{cell_w + 0.5:.2f}" '
                f'height="{cell_h + 0.5:.2f}" fill="{fill}" '
                f'fill-opacity="{alpha:.2f}"/>'
            )
    svg.append("</g>")
    # Bloomed copy of the heatmap for warmth — reference the same group via use,
    # but resvg has limited <use> support across filters; cheaper to render a
    # softened second pass directly.
    svg.append('<g filter="url(#bloom)" opacity="0.55">')
    for x in range(0, x_cols, 2):
        col = intensity[x]
        for y in range(0, y_bins, 2):
            alpha = float(col[y])
            if alpha < 0.35:
                continue
            rx = margin_l + x * cell_w
            ry = H - margin_b - (y + 2) * cell_h
            svg.append(
                f'<rect x="{rx:.2f}" y="{ry:.2f}" width="{cell_w * 2.5:.2f}" '
                f'height="{cell_h * 2.5:.2f}" fill="#ef7e57" '
                f'fill-opacity="{alpha * 0.55:.2f}"/>'
            )
    svg.append("</g>")

    # HPF / LPF rails — log-mapped y for 60 Hz and 2000 Hz
    def freq_to_y(hz: float) -> float:
        f = math.log(hz)
        return H - margin_b - ((f - log_lo) / (log_hi - log_lo)) * plot_h

    for hz, label, dash in [(60.0, "HPF 60 Hz", "5,5"), (2000.0, "LPF 2 kHz", "5,5")]:
        y = freq_to_y(hz)
        # Lines may fall outside the plotted band; clamp visibly.
        if y < margin_t or y > H - margin_b:
            continue
        svg.append(
            f'<line x1="{margin_l}" y1="{y:.2f}" x2="{W - margin_r}" '
            f'y2="{y:.2f}" stroke="#ef7e57" stroke-opacity="0.7" stroke-width="1" '
            f'stroke-dasharray="{dash}"/>'
        )
        svg.append(
            f'<text x="{W - margin_r}" y="{y - 6:.2f}" font-family="{MONO}" '
            f'font-size="12" fill="#ef7e57" fill-opacity="0.85" letter-spacing="2" '
            f'text-anchor="end" text-transform="uppercase">— {label}</text>'
        )

    # Y axis labels (log)
    for hz in [60, 100, 250, 500, 1000, 2000]:
        y = freq_to_y(hz)
        if y < margin_t or y > H - margin_b:
            continue
        svg.append(
            f'<text x="{margin_l - 12}" y="{y + 4:.2f}" font-family="{MONO}" '
            f'font-size="11" fill="#8d836d" letter-spacing="1.5" '
            f'text-anchor="end">{hz} Hz</text>'
        )

    # X axis ticks
    x_ticks = 8
    for k in range(x_ticks + 1):
        t = (k / x_ticks) * dur_s
        x = margin_l + plot_w * (k / x_ticks)
        svg.append(
            f'<line x1="{x:.2f}" y1="{H - margin_b}" x2="{x:.2f}" '
            f'y2="{H - margin_b + 6}" stroke="rgba(239, 126, 87, 0.30)" '
            f'stroke-width="1"/>'
        )
        svg.append(
            f'<text x="{x:.2f}" y="{H - margin_b + 24}" font-family="{MONO}" '
            f'font-size="12" fill="#8d836d" letter-spacing="2" '
            f'text-anchor="middle">{t:.2f}</text>'
        )
    svg.append(
        f'<text x="{W/2:.2f}" y="{H - 22}" font-family="{MONO}" font-size="12" '
        f'fill="#8d836d" letter-spacing="4" text-anchor="middle" '
        f'text-transform="uppercase">seconds</text>'
    )

    # paper grain on top, low alpha
    svg.append(f'<rect width="{W}" height="{H}" fill="#000" filter="url(#grain)"/>')
    # vignette
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#vignette)"/>')
    svg.append("</svg>")

    out = OUT / "spectrogram-biblical.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    render_waveforms_all()
    render_spectrogram_biblical()
