"""2D Gaussian heatmap."""
import numpy as np
import inkblot

y, x = np.mgrid[-3:3:0.02, -3:3:0.02]
z = np.exp(-(x**2 + y**2)) + 0.5 * np.exp(-((x - 1.5)**2 + (y + 1)**2) / 0.3)

inkblot.imshow(z, title="2D Gaussian", xlabel="x", ylabel="y",
               save="examples/imshow_gaussian.png")
print("saved examples/imshow_gaussian.png")
