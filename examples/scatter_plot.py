"""Scatter plot with random data."""
import numpy as np
import inkblot

rng = np.random.default_rng(42)
x = rng.normal(0, 1, 200)
y = 0.8 * x + rng.normal(0, 0.4, 200)

inkblot.scatter(x, y, color="royalblue", size=3.0,
                title="Correlated Scatter", xlabel="x", ylabel="y",
                grid=True, save="examples/scatter_plot.png")
print("saved examples/scatter_plot.png")
