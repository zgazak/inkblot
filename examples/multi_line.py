"""Multiple traces on one figure."""
import numpy as np
import inkblot

x = np.linspace(0, 4 * np.pi, 300)

fig = inkblot.Figure(width=1000, height=500)
fig.line(x, np.sin(x), color="steelblue", linewidth=2.0)
fig.line(x, np.cos(x), color="coral", linewidth=2.0)
fig.line(x, np.sin(x) * np.cos(x), color="forestgreen", linewidth=1.5)
fig.title("Trigonometric Functions")
fig.xlabel("x")
fig.ylabel("f(x)")
fig.grid(True)
fig.save("examples/multi_line.png")
print("saved examples/multi_line.png")
