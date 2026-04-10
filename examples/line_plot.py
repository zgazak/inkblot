"""Basic line plot."""
import numpy as np
import inkblot

x = np.linspace(0, 2 * np.pi, 200)
y = np.sin(x)

inkblot.plot(x, y, title="Sine Wave", xlabel="x", ylabel="sin(x)", grid=True,
             save="examples/line_plot.png")
print("saved examples/line_plot.png")
