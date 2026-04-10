"""Benchmark: inkblot vs matplotlib for astronomical image rendering.

Loads a real all-sky camera FITS image from allclear and renders it as
a frameless grayscale PNG with both libraries. Measures wall-clock time.
"""
import time
import sys
import numpy as np
from pathlib import Path

FITS_PATH = "/stars/src/allclear/benchmark/data/haleakala/2026-03-14-0311_8-CapObj_0478.fits"

# --- Load FITS ---
print("Loading FITS image...")
from astropy.io import fits
from astropy.visualization import ZScaleInterval

hdul = fits.open(FITS_PATH)
raw = np.asarray(hdul[0].data, dtype=np.float64)
hdul.close()
print(f"  Image: {raw.shape[1]}x{raw.shape[0]} ({raw.shape[0]*raw.shape[1]:,} pixels)")

# --- ZScale normalization (shared) ---
print("Applying ZScale normalization...")
t0 = time.perf_counter()
interval = ZScaleInterval(contrast=0.2)
vmin, vmax = interval.get_limits(raw[np.isfinite(raw)])
scaled = np.clip((raw - vmin) / (vmax - vmin), 0, 1)
t_zscale = time.perf_counter() - t0
print(f"  ZScale: {t_zscale:.3f}s")

rows, cols = scaled.shape

# ============================================================
# MATPLOTLIB
# ============================================================
print("\n--- matplotlib ---")

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

times_mpl = []
for trial in range(3):
    t0 = time.perf_counter()

    dpi = 75 if max(rows, cols) > 4000 else 150
    fig = plt.figure(figsize=(cols / dpi, rows / dpi), dpi=dpi, frameon=False)
    fig.subplots_adjust(left=0, right=1, bottom=0, top=1, wspace=0, hspace=0)
    ax = fig.add_subplot(111)
    ax.set_frame_on(False)
    ax.imshow(scaled, cmap="gray", origin="lower")
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_xlim(0, cols - 1)
    ax.set_ylim(0, rows - 1)
    plt.savefig("examples/bench_matplotlib.png", dpi=dpi, bbox_inches="tight", pad_inches=0)
    plt.close(fig)

    dt = time.perf_counter() - t0
    times_mpl.append(dt)
    print(f"  Trial {trial+1}: {dt:.3f}s")

mpl_best = min(times_mpl)
mpl_size = Path("examples/bench_matplotlib.png").stat().st_size

# ============================================================
# INKBLOT
# ============================================================
print("\n--- inkblot ---")

import inkblot

times_ink = []
for trial in range(3):
    t0 = time.perf_counter()

    fig = inkblot.Figure.from_image(scaled, cmap="gray")
    fig.save("examples/bench_inkblot.png")

    dt = time.perf_counter() - t0
    times_ink.append(dt)
    print(f"  Trial {trial+1}: {dt:.3f}s")

ink_best = min(times_ink)
ink_size = Path("examples/bench_inkblot.png").stat().st_size

# ============================================================
# Results
# ============================================================
print("\n" + "=" * 50)
print(f"Image: {cols}x{rows} = {cols*rows:,} pixels")
print(f"ZScale normalization: {t_zscale:.3f}s (shared)")
print(f"")
print(f"matplotlib: {mpl_best:.3f}s  (PNG: {mpl_size/1024:.0f} KB)")
print(f"inkblot:    {ink_best:.3f}s  (PNG: {ink_size/1024:.0f} KB)")
print(f"")
speedup = mpl_best / ink_best
if speedup > 1:
    print(f"inkblot is {speedup:.1f}x faster")
else:
    print(f"matplotlib is {1/speedup:.1f}x faster")
