"""Figure builder — accumulates plot commands, renders via Rust."""

import numpy as np

from inkblot import _core
from inkblot.colors import resolve_color


class Figure:
    """A composable plot builder.

    Usage::

        fig = Figure(width=800, height=600)
        fig.line(x, y, color="steelblue")
        fig.title("My Plot")
        fig.save("plot.png")
    """

    def __init__(self, width=800, height=600):
        self._width = width
        self._height = height
        self._traces = []
        self._title = None
        self._xlabel = None
        self._ylabel = None
        self._grid = False

    def line(self, x, y, *, color="steelblue", linewidth=1.5, label=None):
        """Add a line trace."""
        self._traces.append({
            "kind": "line",
            "x": np.asarray(x, dtype=np.float64),
            "y": np.asarray(y, dtype=np.float64),
            "color": resolve_color(color),
            "linewidth": float(linewidth),
            "label": label,
        })
        return self

    def scatter(self, x, y, *, color="coral", size=3.0, label=None):
        """Add a scatter trace."""
        self._traces.append({
            "kind": "scatter",
            "x": np.asarray(x, dtype=np.float64),
            "y": np.asarray(y, dtype=np.float64),
            "color": resolve_color(color),
            "size": float(size),
            "label": label,
        })
        return self

    def imshow(self, data, *, vmin=None, vmax=None):
        """Add a 2D array as a heatmap image."""
        arr = np.asarray(data, dtype=np.float64)
        if arr.ndim != 2:
            raise ValueError("imshow requires a 2D array")
        rows, cols = arr.shape
        flat = arr.ravel().tolist()
        if vmin is None:
            vmin = float(np.nanmin(arr))
        if vmax is None:
            vmax = float(np.nanmax(arr))
        self._traces.append({
            "kind": "imshow",
            "data": flat,
            "rows": rows,
            "cols": cols,
            "vmin": float(vmin),
            "vmax": float(vmax),
        })
        return self

    def title(self, text):
        self._title = str(text)
        return self

    def xlabel(self, text):
        self._xlabel = str(text)
        return self

    def ylabel(self, text):
        self._ylabel = str(text)
        return self

    def grid(self, on=True):
        self._grid = bool(on)
        return self

    def _build_spec(self):
        """Serialize the plot description for the Rust renderer."""
        return {
            "width": self._width,
            "height": self._height,
            "traces": self._traces,
            "title": self._title,
            "xlabel": self._xlabel,
            "ylabel": self._ylabel,
            "grid": self._grid,
        }

    def render(self):
        """Render to PNG bytes via the Rust core."""
        return _core.render_plot(self._build_spec())

    def save(self, path):
        """Render and write PNG to disk."""
        png_bytes = self.render()
        with open(path, "wb") as f:
            f.write(png_bytes)
        return self

    def _repr_png_(self):
        """Jupyter notebook display integration."""
        return self.render()
