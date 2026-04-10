"""Line fit with scatter data points."""
import numpy as np
import inkblot

rng = np.random.default_rng(7)
x_data = np.sort(rng.uniform(0, 10, 40))
y_data = 2.5 * x_data + 3.0 + rng.normal(0, 3, len(x_data))

# Fit
coeffs = np.polyfit(x_data, y_data, 1)
x_fit = np.linspace(0, 10, 100)
y_fit = np.polyval(coeffs, x_fit)

fig = inkblot.Figure(width=800, height=600)
fig.scatter(x_data, y_data, color="coral", size=4.0)
fig.line(x_fit, y_fit, color="steelblue", linewidth=2.0)
fig.title(f"Linear Fit: y = {coeffs[0]:.2f}x + {coeffs[1]:.2f}")
fig.xlabel("x")
fig.ylabel("y")
fig.grid(True)
fig.save("examples/line_and_scatter.png")
print("saved examples/line_and_scatter.png")
