"""Simulated astronomical light curve."""
import numpy as np
import inkblot

rng = np.random.default_rng(12)
time = np.linspace(0, 50, 500)

# Simulated variable star: sinusoidal + noise + transit dip
flux = 1.0 + 0.02 * np.sin(2 * np.pi * time / 5.3)
flux += rng.normal(0, 0.005, len(time))

# Add a transit dip
transit_mask = (time > 22) & (time < 24)
flux[transit_mask] -= 0.03

fig = inkblot.Figure(width=1000, height=400)
fig.scatter(time, flux, color="steelblue", size=1.5)
fig.title("Simulated Light Curve")
fig.xlabel("Time (days)")
fig.ylabel("Relative Flux")
fig.grid(True)
fig.save("examples/light_curve.png")
print("saved examples/light_curve.png")
