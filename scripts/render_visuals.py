#!/usr/bin/env python3
"""Render the README visuals + brand marks in the paper visual language.

Inputs:
  fixtures/golden/{name}.wav for the four canonical voices.

Outputs (overwrites):
  docs/screenshots/waveforms-all.{svg}
  docs/screenshots/spectrogram-biblical.{svg}
  docs/marks/wordmark.{svg}
  docs/marks/signature.{svg}
  docs/marks/monogram.{svg}
  docs/marks/og-card.{svg}

Visual language (mirrors apps/web/style.css light palette):
  - warm paper canvas        #f7f1e3 → #efe7d2 (subtle vertical wash)
  - ink                      #1a1612 (primary), #3a3128 (muted)
  - oxblood accent           #8c2f1e (data rail), #c2533a (glow), #7a2812 (deep)
  - fingerprint paper grain  two-octave fractal noise, ink overlay
  - soft bloom on data layers
  - personality labels in Charter / Iowan italic

Run from the repo root:
  python3 scripts/render_visuals.py
  bash scripts/render_all_visuals.sh    # rasterises every SVG via headless Chrome
"""

from __future__ import annotations

import math
import wave
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parent.parent
FIX = REPO / "fixtures" / "golden"
OUT_SCR = REPO / "docs" / "screenshots"
OUT_MARKS = REPO / "docs" / "marks"

PERSONALITIES = ["polite-cough", "default", "biblical", "silent-but-deadly"]

# ---------- palette (must match apps/web/style.css light mode) ------------------

PAPER = "#f7f1e3"
PAPER_2 = "#efe7d2"
INK = "#1a1612"
INK_2 = "#3a3128"
INK_MUTED = "#6b5e4f"  # for axis numerals
OXBLOOD = "#8c2f1e"  # primary data rail
GLOW = "#c2533a"  # warm wash / inner halo
DEEP = "#7a2812"  # low-intensity heatmap cells
EMBER = "#d97a4a"  # high-intensity heatmap cells

# Family stacks. Charter is macOS default; Linux falls back to the chain.
DISPLAY = (
    "Charter, 'Iowan Old Style', 'Source Serif Pro', "
    "'Apple Garamond', Cambria, Georgia, serif"
)
MONO = "'Berkeley Mono', 'JetBrains Mono', 'SF Mono', Menlo, monospace"

# ---------- shared SVG fragments ------------------------------------------------

# Paper canvas + fingerprint grain (dark ink overlay) + soft bloom.
# The fingerprint is a two-octave fractal-noise filter whose result is composited
# with low-alpha ink colour, producing the dotted, slightly biological paper
# texture the user asked for. Bloom is softer than the dark-canvas version so it
# reads as "warm halo" not "glow against black".
DEFS = """\
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"  stop-color="#f7f1e3"/>
      <stop offset="100%" stop-color="#efe7d2"/>
    </linearGradient>
    <linearGradient id="band" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"   stop-color="#8c2f1e" stop-opacity="0.0"/>
      <stop offset="50%"  stop-color="#8c2f1e" stop-opacity="0.04"/>
      <stop offset="100%" stop-color="#8c2f1e" stop-opacity="0.0"/>
    </linearGradient>
    <filter id="bloom" x="-20%" y="-50%" width="140%" height="200%">
      <feGaussianBlur stdDeviation="2.6"/>
    </filter>
    <filter id="bloom-strong" x="-20%" y="-50%" width="140%" height="200%">
      <feGaussianBlur stdDeviation="5"/>
    </filter>
    <filter id="fingerprint" x="0%" y="0%" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="1.2" numOctaves="2" seed="7" result="fine"/>
      <feColorMatrix in="fine" values="0 0 0 0 0.10
                                       0 0 0 0 0.085
                                       0 0 0 0 0.07
                                       0 0 0 0.18 0" result="fineLit"/>
      <feTurbulence type="fractalNoise" baseFrequency="0.35" numOctaves="1" seed="13" result="coarse"/>
      <feColorMatrix in="coarse" values="0 0 0 0 0.10
                                         0 0 0 0 0.085
                                         0 0 0 0 0.07
                                         0 0 0 0.08 0" result="coarseLit"/>
      <feMerge><feMergeNode in="coarseLit"/><feMergeNode in="fineLit"/></feMerge>
    </filter>
    <radialGradient id="vignette" cx="50%" cy="50%" r="75%">
      <stop offset="60%" stop-color="#1a1612" stop-opacity="0"/>
      <stop offset="100%" stop-color="#1a1612" stop-opacity="0.10"/>
    </radialGradient>
  </defs>"""


# ---------- helpers -------------------------------------------------------------


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
    """Stacked four-lane waveform comparison on warm paper canvas."""
    samples_all: dict[str, tuple[int, np.ndarray]] = {
        name: read_wav(FIX / f"{name}.wav") for name in PERSONALITIES
    }
    max_dur_s = max(arr.shape[0] / s for s, arr in samples_all.values())
    axis_end_s = math.ceil(max_dur_s * 2) / 2

    W, H = 1600, 600
    margin_l, margin_r = 80, 60
    margin_t, margin_b = 98, 94
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
    svg.append(
        f'<rect width="{W}" height="{H}" filter="url(#fingerprint)" opacity="0.55"/>'
    )
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#band)"/>')

    svg.append(
        f'<text x="{margin_l}" y="64" font-family="{DISPLAY}" font-style="italic" '
        f'font-size="36" fill="{INK}" letter-spacing="0.3">'
        f'canonical voices</text>'
    )

    # data layers
    for i, name in enumerate(PERSONALITIES):
        s_sr, samples = samples_all[name]
        n = samples.shape[0]
        dur_s = n / s_sr
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
            f'stroke="{OXBLOOD}" stroke-opacity="0.12" stroke-width="1"/>'
        )
        # glow wash + bright-core two-pass envelope. On paper canvas we lead
        # with the deep oxblood fill (gives the body weight), then a softer
        # glow stroke, then the dark ink stroke as the sharp signal edge.
        svg.append(
            f'<path d="{d}" fill="{OXBLOOD}" fill-opacity="0.32" '
            f'filter="url(#bloom-strong)"/>'
        )
        svg.append(
            f'<path d="{d}" fill="none" stroke="{GLOW}" stroke-width="1.1" '
            f'filter="url(#bloom)" opacity="0.85"/>'
        )
        svg.append(
            f'<path d="{d}" fill="none" stroke="{INK}" stroke-width="0.6" '
            f'opacity="0.92"/>'
        )

        # label — display serif italic, wide tracking, ink
        label_y = lane_y_center - lane_h * 0.32
        svg.append(
            f'<text x="{margin_l}" y="{label_y:.2f}" font-family="{DISPLAY}" '
            f'font-style="italic" font-size="26" fill="{INK}" '
            f'letter-spacing="0.6">{name}</text>'
        )
        # mono duration label, right-aligned, wide tracking
        ms = int(round(dur_s * 1000))
        svg.append(
            f'<text x="{W - margin_r}" y="{label_y:.2f}" font-family="{MONO}" '
            f'font-size="11" fill="{INK_MUTED}" letter-spacing="1.5" '
            f'text-anchor="end" text-transform="uppercase">{ms} MS</text>'
        )

    # shared time axis ticks
    ticks = 8
    for k in range(ticks + 1):
        t = (k / ticks) * axis_end_s
        x = margin_l + plot_w * (t / axis_end_s)
        svg.append(
            f'<line x1="{x:.2f}" y1="{H - margin_b}" x2="{x:.2f}" '
            f'y2="{H - margin_b + 6}" stroke="{OXBLOOD}" stroke-opacity="0.45" '
            f'stroke-width="1"/>'
        )
        svg.append(
            f'<text x="{x:.2f}" y="{H - margin_b + 24}" font-family="{MONO}" '
            f'font-size="11" fill="{INK_MUTED}" letter-spacing="1.6" '
            f'text-anchor="middle">{t:.2f}</text>'
        )
    # vignette (very light on paper — just corner darkening to ground the figure)
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#vignette)"/>')
    svg.append("</svg>")

    out = OUT_SCR / "waveforms-all.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


# ---------- spectrogram-biblical ------------------------------------------------


def render_spectrogram_biblical() -> None:
    """STFT-based spectrogram of biblical.wav, paper canvas + warm heatmap."""
    sr, samples = read_wav(FIX / "biblical.wav")
    dur_s = samples.shape[0] / sr

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
    spec = np.abs(np.fft.rfft(frames, axis=1))

    eps = 1e-8
    db = 20.0 * np.log10(spec + eps)
    db_max = float(db.max())
    db -= db_max
    db_floor = -55.0
    db = np.clip(db, db_floor, 0.0)
    intensity = (db - db_floor) / (-db_floor)
    intensity = np.power(intensity, 1.35)

    f_lo, f_hi = 50.0, 2500.0
    freqs = np.fft.rfftfreq(nfft, 1.0 / sr)
    f_mask = (freqs >= f_lo) & (freqs <= f_hi)
    intensity = intensity[:, f_mask]
    band_freqs = freqs[f_mask]
    y_bins = 110
    log_lo, log_hi = math.log(f_lo), math.log(f_hi)
    log_centres = np.linspace(log_lo, log_hi, y_bins)
    src_logs = np.log(band_freqs + 1e-9)
    src_idx = np.searchsorted(src_logs, log_centres)
    src_idx = np.clip(src_idx, 0, intensity.shape[1] - 1)
    intensity = intensity[:, src_idx]

    x_cols = 320
    t_idx = np.linspace(0, intensity.shape[0] - 1, x_cols).astype(int)
    intensity = intensity[t_idx, :]

    W, H = 1600, 520
    margin_l, margin_r, margin_t, margin_b = 80, 70, 92, 88
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
    svg.append(
        f'<rect width="{W}" height="{H}" filter="url(#fingerprint)" opacity="0.55"/>'
    )

    svg.append(
        f'<text x="{margin_l}" y="62" font-family="{DISPLAY}" font-style="italic" '
        f'font-size="36" fill="{INK}" letter-spacing="0.35">biblical</text>'
    )
    svg.append(
        f'<text x="{margin_l + 180}" y="60" font-family="{MONO}" font-size="11" '
        f'fill="{INK_2}" letter-spacing="2.1" text-transform="uppercase">'
        f'seed 3 · pressure 0.8 · 48 kHz mono</text>'
    )

    # cells: low intensity = warm wash, high intensity = deep saturated oxblood
    # (inverse contrast vs the dark-canvas version: paper bg needs darker peaks)
    svg.append('<g id="heatmap">')
    for x in range(x_cols):
        col = intensity[x]
        for y in range(y_bins):
            alpha = float(col[y])
            if alpha < 0.06:
                continue
            if alpha > 0.78:
                fill = DEEP  # darkest, most saturated peak
            elif alpha > 0.55:
                fill = OXBLOOD
            elif alpha > 0.3:
                fill = GLOW
            else:
                fill = EMBER
            rx = margin_l + x * cell_w
            ry = H - margin_b - (y + 1) * cell_h
            svg.append(
                f'<rect x="{rx:.2f}" y="{ry:.2f}" width="{cell_w + 0.5:.2f}" '
                f'height="{cell_h + 0.5:.2f}" fill="{fill}" '
                f'fill-opacity="{alpha * 0.88:.2f}"/>'
            )
    svg.append("</g>")
    # Bloomed copy for warmth
    svg.append('<g filter="url(#bloom)" opacity="0.26">')
    for x in range(0, x_cols, 2):
        col = intensity[x]
        for y in range(0, y_bins, 2):
            alpha = float(col[y])
            if alpha < 0.48:
                continue
            rx = margin_l + x * cell_w
            ry = H - margin_b - (y + 2) * cell_h
            svg.append(
                f'<rect x="{rx:.2f}" y="{ry:.2f}" width="{cell_w * 2.5:.2f}" '
                f'height="{cell_h * 2.5:.2f}" fill="{GLOW}" '
                f'fill-opacity="{alpha * 0.34:.2f}"/>'
            )
    svg.append("</g>")

    # HPF / LPF rails
    def freq_to_y(hz: float) -> float:
        f = math.log(hz)
        return H - margin_b - ((f - log_lo) / (log_hi - log_lo)) * plot_h

    for hz, label, dash in [(60.0, "HPF 60 Hz", "5,5"), (2000.0, "LPF 2 kHz", "5,5")]:
        y = freq_to_y(hz)
        if y < margin_t or y > H - margin_b:
            continue
        svg.append(
            f'<line x1="{margin_l}" y1="{y:.2f}" x2="{W - margin_r}" '
            f'y2="{y:.2f}" stroke="{OXBLOOD}" stroke-opacity="0.58" stroke-width="1" '
            f'stroke-dasharray="{dash}"/>'
        )
        svg.append(
            f'<text x="{W - margin_r}" y="{y - 6:.2f}" font-family="{MONO}" '
            f'font-size="11" fill="{OXBLOOD}" fill-opacity="0.78" letter-spacing="1.8" '
            f'text-anchor="end" text-transform="uppercase">— {label}</text>'
        )

    # Y axis labels (log)
    for hz in [60, 100, 250, 500, 1000, 2000]:
        y = freq_to_y(hz)
        if y < margin_t or y > H - margin_b:
            continue
        svg.append(
            f'<text x="{margin_l - 12}" y="{y + 4:.2f}" font-family="{MONO}" '
            f'font-size="10" fill="{INK_MUTED}" letter-spacing="1.2" '
            f'text-anchor="end">{hz} Hz</text>'
        )

    # X axis ticks
    x_ticks = 8
    for k in range(x_ticks + 1):
        t = (k / x_ticks) * dur_s
        x = margin_l + plot_w * (k / x_ticks)
        svg.append(
            f'<line x1="{x:.2f}" y1="{H - margin_b}" x2="{x:.2f}" '
            f'y2="{H - margin_b + 6}" stroke="{OXBLOOD}" stroke-opacity="0.45" '
            f'stroke-width="1"/>'
        )
        svg.append(
            f'<text x="{x:.2f}" y="{H - margin_b + 24}" font-family="{MONO}" '
            f'font-size="11" fill="{INK_MUTED}" letter-spacing="1.6" '
            f'text-anchor="middle">{t:.2f}</text>'
        )
    svg.append(
        f'<text x="{W/2:.2f}" y="{H - 20}" font-family="{MONO}" font-size="11" '
        f'fill="{INK_MUTED}" letter-spacing="3.2" text-anchor="middle" '
        f'text-transform="uppercase">seconds</text>'
    )

    svg.append(f'<rect width="{W}" height="{H}" fill="url(#vignette)"/>')
    svg.append("</svg>")

    out = OUT_SCR / "spectrogram-biblical.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


# ---------- brand marks: standalone, paper-canvas, same DEFS family -------------


def _paper_canvas_rect(w: int, h: int, rx: int = 0) -> list[str]:
    """Background + fingerprint pass for any standalone mark."""
    parts: list[str] = []
    if rx:
        parts.append(
            f'<rect width="{w}" height="{h}" rx="{rx}" fill="url(#bg)"/>'
        )
        parts.append(
            f'<rect width="{w}" height="{h}" rx="{rx}" '
            f'filter="url(#fingerprint)" opacity="0.6"/>'
        )
    else:
        parts.append(f'<rect width="{w}" height="{h}" fill="url(#bg)"/>')
        parts.append(
            f'<rect width="{w}" height="{h}" filter="url(#fingerprint)" opacity="0.6"/>'
        )
    return parts


def _grain_radial_gradient(id_: str) -> str:
    """Warm grain radial — dark core, oxblood mid, fading to transparent.
    On paper canvas we want the core to be visible, so we use a saturated
    oxblood centre rather than a near-white core."""
    return (
        f'<radialGradient id="{id_}" cx="50%" cy="50%" r="60%">'
        f'<stop offset="0%"   stop-color="{DEEP}"    stop-opacity="0.92"/>'
        f'<stop offset="30%"  stop-color="{OXBLOOD}" stop-opacity="0.78"/>'
        f'<stop offset="65%"  stop-color="{GLOW}"    stop-opacity="0.42"/>'
        f'<stop offset="100%" stop-color="{GLOW}"    stop-opacity="0"/>'
        f'</radialGradient>'
    )


def render_wordmark() -> None:
    """Primary brand mark: italic Charter wordmark on paper, oxblood rule."""
    W, H = 800, 240
    svg: list[str] = []
    svg.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
        f'role="img" aria-label="flatus wordmark">'
    )
    svg.append("<defs>")
    svg.append(
        f'<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">'
        f'<stop offset="0%" stop-color="{PAPER}"/>'
        f'<stop offset="100%" stop-color="{PAPER_2}"/>'
        f'</linearGradient>'
    )
    svg.append(
        '<linearGradient id="rule" x1="0" x2="1" y1="0" y2="0">'
        f'<stop offset="0%"   stop-color="{OXBLOOD}" stop-opacity="0"/>'
        f'<stop offset="15%"  stop-color="{OXBLOOD}" stop-opacity="0.88"/>'
        f'<stop offset="85%"  stop-color="{OXBLOOD}" stop-opacity="0.88"/>'
        f'<stop offset="100%" stop-color="{OXBLOOD}" stop-opacity="0"/>'
        "</linearGradient>"
    )
    svg.append(
        '<filter id="fingerprint" x="0%" y="0%" width="100%" height="100%">'
        '<feTurbulence type="fractalNoise" baseFrequency="1.2" numOctaves="2" '
        'seed="7" result="fine"/>'
        '<feColorMatrix in="fine" values="0 0 0 0 0.10  0 0 0 0 0.085  '
        '0 0 0 0 0.07  0 0 0 0.18 0" result="fineLit"/>'
        '<feTurbulence type="fractalNoise" baseFrequency="0.35" numOctaves="1" '
        'seed="13" result="coarse"/>'
        '<feColorMatrix in="coarse" values="0 0 0 0 0.10  0 0 0 0 0.085  '
        '0 0 0 0 0.07  0 0 0 0.08 0" result="coarseLit"/>'
        '<feMerge><feMergeNode in="coarseLit"/><feMergeNode in="fineLit"/></feMerge>'
        "</filter>"
    )
    svg.append("</defs>")
    svg.extend(_paper_canvas_rect(W, H))
    svg.append(
        f'<text x="60" y="170" font-family="{DISPLAY}" font-size="200" '
        f'font-style="italic" font-weight="400" fill="{INK}" letter-spacing="-2">'
        "flatus</text>"
    )
    svg.append(
        f'<line x1="60" y1="200" x2="700" y2="200" stroke="url(#rule)" '
        'stroke-width="2"/>'
    )
    svg.append(
        f'<text x="60" y="228" font-family="{MONO}" font-size="14" '
        f'fill="{INK_MUTED}" letter-spacing="2.5">A SMALL APPARATUS FOR MOVING AIR</text>'
    )
    svg.append("</svg>")
    out = OUT_MARKS / "wordmark.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


def render_signature() -> None:
    """Six-grain time-strip lifted from the banner, on paper."""
    W, H = 800, 200
    svg: list[str] = []
    svg.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
        f'role="img" aria-label="flatus signature — six warm grains across a time strip">'
    )
    svg.append("<defs>")
    svg.append(
        f'<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">'
        f'<stop offset="0%" stop-color="{PAPER}"/>'
        f'<stop offset="100%" stop-color="{PAPER_2}"/>'
        f'</linearGradient>'
    )
    svg.append(_grain_radial_gradient("g"))
    svg.append(
        '<filter id="bloom" x="-60%" y="-100%" width="220%" height="300%">'
        '<feGaussianBlur stdDeviation="3"  result="b1"/>'
        '<feGaussianBlur stdDeviation="10" in="SourceGraphic" result="b2"/>'
        '<feGaussianBlur stdDeviation="20" in="SourceGraphic" result="b3"/>'
        '<feMerge>'
        '<feMergeNode in="b3"/><feMergeNode in="b2"/>'
        '<feMergeNode in="b1"/><feMergeNode in="SourceGraphic"/>'
        '</feMerge>'
        "</filter>"
    )
    svg.append(
        '<filter id="fingerprint" x="0%" y="0%" width="100%" height="100%">'
        '<feTurbulence type="fractalNoise" baseFrequency="1.2" numOctaves="2" '
        'seed="7" result="fine"/>'
        '<feColorMatrix in="fine" values="0 0 0 0 0.10  0 0 0 0 0.085  '
        '0 0 0 0 0.07  0 0 0 0.18 0" result="fineLit"/>'
        '<feTurbulence type="fractalNoise" baseFrequency="0.35" numOctaves="1" '
        'seed="13" result="coarse"/>'
        '<feColorMatrix in="coarse" values="0 0 0 0 0.10  0 0 0 0 0.085  '
        '0 0 0 0 0.07  0 0 0 0.08 0" result="coarseLit"/>'
        '<feMerge><feMergeNode in="coarseLit"/><feMergeNode in="fineLit"/></feMerge>'
        "</filter>"
    )
    svg.append("</defs>")
    svg.extend(_paper_canvas_rect(W, H))
    svg.append('<g filter="url(#bloom)">')
    svg.append('<ellipse cx="120" cy="100" rx="24" ry="32" fill="url(#g)" opacity="0.82"/>')
    svg.append('<ellipse cx="230" cy="100" rx="34" ry="42" fill="url(#g)" opacity="0.96"/>')
    svg.append('<ellipse cx="380" cy="100" rx="48" ry="56" fill="url(#g)" opacity="1.0"/>')
    svg.append('<ellipse cx="530" cy="100" rx="40" ry="50" fill="url(#g)" opacity="0.97"/>')
    svg.append('<ellipse cx="650" cy="100" rx="28" ry="38" fill="url(#g)" opacity="0.78"/>')
    svg.append('<ellipse cx="740" cy="100" rx="18" ry="26" fill="url(#g)" opacity="0.54"/>')
    svg.append("</g>")
    svg.append("</svg>")
    out = OUT_MARKS / "signature.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


def render_monogram() -> None:
    """Square 'f' + three grains. Already on paper; restyle to new grain palette."""
    W, H = 240, 240
    svg: list[str] = []
    svg.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
        f'role="img" aria-label="flatus monogram — italic f with three grains">'
    )
    svg.append("<defs>")
    svg.append(
        f'<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">'
        f'<stop offset="0%" stop-color="{PAPER}"/>'
        f'<stop offset="100%" stop-color="{PAPER_2}"/>'
        f'</linearGradient>'
    )
    svg.append(_grain_radial_gradient("gm"))
    svg.append(
        '<filter id="bloomM" x="-60%" y="-80%" width="220%" height="260%">'
        '<feGaussianBlur stdDeviation="2"  result="m1"/>'
        '<feGaussianBlur stdDeviation="6"  in="SourceGraphic" result="m2"/>'
        '<feGaussianBlur stdDeviation="14" in="SourceGraphic" result="m3"/>'
        '<feMerge>'
        '<feMergeNode in="m3"/><feMergeNode in="m2"/>'
        '<feMergeNode in="m1"/><feMergeNode in="SourceGraphic"/>'
        '</feMerge>'
        "</filter>"
    )
    svg.append(
        '<filter id="fingerprint" x="0%" y="0%" width="100%" height="100%">'
        '<feTurbulence type="fractalNoise" baseFrequency="1.4" numOctaves="2" '
        'seed="7" result="fine"/>'
        '<feColorMatrix in="fine" values="0 0 0 0 0.10  0 0 0 0 0.085  '
        '0 0 0 0 0.07  0 0 0 0.20 0" result="fineLit"/>'
        '<feMerge><feMergeNode in="fineLit"/></feMerge>'
        "</filter>"
    )
    svg.append("</defs>")
    svg.extend(_paper_canvas_rect(W, H, rx=44))
    svg.append(
        f'<text x="60" y="172" font-family="{DISPLAY}" font-size="190" '
        f'font-style="italic" font-weight="400" fill="{INK}" letter-spacing="-2">f</text>'
    )
    svg.append('<g filter="url(#bloomM)">')
    svg.append('<ellipse cx="130" cy="200" rx="9"  ry="13" fill="url(#gm)" opacity="0.88"/>')
    svg.append('<ellipse cx="156" cy="200" rx="13" ry="17" fill="url(#gm)" opacity="0.98"/>')
    svg.append('<ellipse cx="186" cy="200" rx="9"  ry="13" fill="url(#gm)" opacity="0.74"/>')
    svg.append("</g>")
    svg.append("</svg>")
    out = OUT_MARKS / "monogram.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


def render_og_card() -> None:
    """1200×630 social preview. Paper canvas, wordmark + signature + URL."""
    W, H = 1200, 630
    svg: list[str] = []
    svg.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
        f'role="img" aria-label="flatus — a small apparatus for moving air. '
        f'Open Graph preview card.">'
    )
    svg.append("<defs>")
    svg.append(
        f'<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">'
        f'<stop offset="0%" stop-color="{PAPER}"/>'
        f'<stop offset="100%" stop-color="{PAPER_2}"/>'
        f'</linearGradient>'
    )
    svg.append(_grain_radial_gradient("ogG"))
    svg.append(
        '<filter id="ogBloom" x="-60%" y="-80%" width="220%" height="260%">'
        '<feGaussianBlur stdDeviation="4"  result="ob1"/>'
        '<feGaussianBlur stdDeviation="14" in="SourceGraphic" result="ob2"/>'
        '<feGaussianBlur stdDeviation="28" in="SourceGraphic" result="ob3"/>'
        '<feMerge>'
        '<feMergeNode in="ob3"/><feMergeNode in="ob2"/>'
        '<feMergeNode in="ob1"/><feMergeNode in="SourceGraphic"/>'
        '</feMerge>'
        "</filter>"
    )
    svg.append(
        '<filter id="fingerprint" x="0%" y="0%" width="100%" height="100%">'
        '<feTurbulence type="fractalNoise" baseFrequency="1.0" numOctaves="2" '
        'seed="7" result="fine"/>'
        '<feColorMatrix in="fine" values="0 0 0 0 0.10  0 0 0 0 0.085  '
        '0 0 0 0 0.07  0 0 0 0.16 0" result="fineLit"/>'
        '<feTurbulence type="fractalNoise" baseFrequency="0.30" numOctaves="1" '
        'seed="13" result="coarse"/>'
        '<feColorMatrix in="coarse" values="0 0 0 0 0.10  0 0 0 0 0.085  '
        '0 0 0 0 0.07  0 0 0 0.07 0" result="coarseLit"/>'
        '<feMerge><feMergeNode in="coarseLit"/><feMergeNode in="fineLit"/></feMerge>'
        "</filter>"
    )
    svg.append(
        '<radialGradient id="ogVig" cx="50%" cy="55%" r="75%">'
        f'<stop offset="55%" stop-color="{INK}" stop-opacity="0"/>'
        f'<stop offset="100%" stop-color="{INK}" stop-opacity="0.16"/>'
        '</radialGradient>'
    )
    svg.append("</defs>")
    svg.extend(_paper_canvas_rect(W, H))

    # wordmark
    svg.append(
        f'<text x="74" y="340" font-family="{DISPLAY}" font-size="220" '
        f'font-style="italic" font-weight="400" fill="{INK}" letter-spacing="-3">'
        f'flatus</text>'
    )
    # tagline
    svg.append(
        f'<text x="78" y="388" font-family="{DISPLAY}" font-size="28" '
        f'font-style="italic" fill="{INK_2}">a small apparatus for moving air.</text>'
    )
    # signature: 6 grains, oxblood, bloomed
    svg.append('<g filter="url(#ogBloom)">')
    svg.append('<ellipse cx="150"  cy="500" rx="32" ry="44" fill="url(#ogG)" opacity="0.82"/>')
    svg.append('<ellipse cx="310"  cy="500" rx="46" ry="58" fill="url(#ogG)" opacity="0.96"/>')
    svg.append('<ellipse cx="500"  cy="500" rx="64" ry="76" fill="url(#ogG)" opacity="1.0"/>')
    svg.append('<ellipse cx="700"  cy="500" rx="54" ry="68" fill="url(#ogG)" opacity="0.97"/>')
    svg.append('<ellipse cx="870"  cy="500" rx="38" ry="50" fill="url(#ogG)" opacity="0.78"/>')
    svg.append('<ellipse cx="1010" cy="500" rx="22" ry="32" fill="url(#ogG)" opacity="0.54"/>')
    svg.append("</g>")
    # footer URL
    svg.append(
        f'<text x="600" y="600" font-family="{MONO}" font-size="18" '
        f'fill="{INK_MUTED}" fill-opacity="0.82" letter-spacing="2" text-anchor="middle">flatus.vercel.app</text>'
    )
    svg.append(f'<rect width="{W}" height="{H}" fill="url(#ogVig)"/>')
    svg.append("</svg>")
    out = OUT_MARKS / "og-card.svg"
    out.write_text("\n".join(svg))
    print(f"wrote {out}")


if __name__ == "__main__":
    OUT_SCR.mkdir(parents=True, exist_ok=True)
    OUT_MARKS.mkdir(parents=True, exist_ok=True)
    render_waveforms_all()
    render_spectrogram_biblical()
    render_wordmark()
    render_signature()
    render_monogram()
    render_og_card()
