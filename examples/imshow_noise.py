"""Perlin-like structured noise field."""
import numpy as np
import inkblot

rng = np.random.default_rng(42)

# Build a structured noise field via summed low-freq components
shape = (200, 300)
field = np.zeros(shape)
for scale in [8, 16, 32, 64]:
    small = rng.normal(size=(shape[0] // scale + 1, shape[1] // scale + 1))
    # Bilinear upsample
    from numpy import interp
    rows = np.linspace(0, small.shape[0] - 1, shape[0])
    cols = np.linspace(0, small.shape[1] - 1, shape[1])
    # Row interpolation
    r_idx = rows.astype(int).clip(0, small.shape[0] - 2)
    r_frac = rows - r_idx
    col_idx = cols.astype(int).clip(0, small.shape[1] - 2)
    c_frac = cols - col_idx
    top = small[r_idx][:, col_idx] * (1 - c_frac) + small[r_idx][:, col_idx + 1] * c_frac
    bot = small[r_idx + 1][:, col_idx] * (1 - c_frac) + small[r_idx + 1][:, col_idx + 1] * c_frac
    upsampled = top * (1 - r_frac[:, None]) + bot * r_frac[:, None]
    field += upsampled / scale

inkblot.imshow(field, title="Structured Noise", save="examples/imshow_noise.png",
               width=900, height=500)
print("saved examples/imshow_noise.png")
