"""Simulated emission spectrum."""
import numpy as np
import inkblot

wavelength = np.linspace(3800, 7200, 1000)

# Continuum + emission lines
flux = 0.5 + 0.1 * np.exp(-0.5 * ((wavelength - 5500) / 800) ** 2)
lines = [(4861, 0.6, 15), (5007, 0.9, 12), (6563, 1.2, 20)]
for center, amp, sigma in lines:
    flux += amp * np.exp(-0.5 * ((wavelength - center) / sigma) ** 2)

flux += np.random.default_rng(99).normal(0, 0.02, len(wavelength))

fig = inkblot.Figure(width=1000, height=400)
fig.line(wavelength, flux, color="darkviolet", linewidth=1.0)
fig.title("Emission Spectrum")
fig.xlabel("Wavelength (A)")
fig.ylabel("Flux")
fig.grid(True)
fig.save("examples/spectrum.png")
print("saved examples/spectrum.png")
