"""Frameless image with overlays and coordinate grid.

Demonstrates:
- Figure.from_image() with grayscale colormap
- Overlay primitives: circles, crosshairs, labels, polylines
- Coordinate transform with iso-line grid
"""
import numpy as np
import inkblot
from inkblot.transform import CallableTransform

# --- Synthetic "astronomical" image ---
rng = np.random.default_rng(42)
rows, cols = 400, 600

# Background gradient + noise + a few "stars" (Gaussians)
y, x = np.mgrid[0:rows, 0:cols]
sky = 100 + 20 * np.sin(2 * np.pi * x / cols) + 10 * np.cos(2 * np.pi * y / rows)
sky += rng.normal(0, 8, sky.shape)

# Add point sources
stars = [(150, 300, 800), (80, 100, 500), (350, 450, 600),
         (200, 500, 400), (50, 250, 350), (300, 150, 700)]
for sy, sx, flux in stars:
    r2 = (x - sx)**2 + (y - sy)**2
    sky += flux * np.exp(-r2 / 8)

# --- Frameless image with percent scaling ---
fig = inkblot.Figure.from_image(sky, scale="percent", cmap="gray")

# --- Overlay markers at pixel coords ---
for sy, sx, flux in stars:
    fig.crosshair(sx, rows - 1 - sy, size=18, gap=4, color="lime", linewidth=1.5)
    fig.circle(sx, rows - 1 - sy, radius=25, color="red", linewidth=1.0)

# Labels
fig.label(10, 20, "inkblot: image + overlays", font_size=14,
          color="white", bg_color="black")
fig.label(10, rows - 20, f"{cols}x{rows} synthetic field",
          font_size=10, color="yellow")

# --- Coordinate grid via an affine transform ---
# Simulate a simple WCS: pixel (0,0) = RA 180.5, Dec -30.2
# with plate scale 0.001 deg/pixel and a 15-degree rotation
theta = np.radians(15)
scale_val = 0.001  # deg/pixel
ra0, dec0 = 180.5, -30.2

cos_t, sin_t = np.cos(theta), np.sin(theta)

def pixel_to_world(px, py):
    dx = px - cols / 2
    dy = py - rows / 2
    ra = ra0 + scale_val * (cos_t * dx - sin_t * dy)
    dec = dec0 + scale_val * (sin_t * dx + cos_t * dy)
    return (ra, dec)

def world_to_pixel(ra, dec):
    dra = (ra - ra0) / scale_val
    ddec = (dec - dec0) / scale_val
    px = cos_t * dra + sin_t * ddec + cols / 2
    py = -sin_t * dra + cos_t * ddec + rows / 2
    return (px, py)

transform = CallableTransform(world_to_pixel, pixel_to_world)
fig.set_transform(transform)
fig.coord_grid(color="cyan", alpha=140, n_lines=4,
               label_format=lambda axis, val: f"{val:.3f}°")

fig.save("examples/image_overlays.png")
print("saved examples/image_overlays.png")
