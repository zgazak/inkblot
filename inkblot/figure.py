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

        # Frameless image mode:
        fig = Figure.from_image(data, scale="zscale", cmap="gray")
        fig.circle(100, 200, radius=10, color="red")
        fig.save("image.png")
    """

    def __init__(self, width=800, height=600, *, frameless=False, cmap=None):
        self._width = width
        self._height = height
        self._frameless = frameless
        self._cmap = cmap
        self._traces = []
        self._overlays = []
        self._title = None
        self._xlabel = None
        self._ylabel = None
        self._grid = False
        self._transform = None

    @classmethod
    def from_image(cls, data, *, scale=None, cmap="gray", **kwargs):
        """Create a frameless figure from a 2D array.

        The output image matches the input array dimensions pixel-for-pixel.

        Args:
            data: 2D numpy array
            scale: normalization name ("zscale", "asinh", "percent", "linear") or None
            cmap: colormap name (default "gray")
        """
        arr = np.asarray(data, dtype=np.float64)
        if arr.ndim != 2:
            raise ValueError("from_image requires a 2D array")
        rows, cols = arr.shape
        if scale is not None:
            from inkblot.scale import apply_scale
            arr = apply_scale(arr, scale)
        fig = cls(width=cols, height=rows, frameless=True, cmap=cmap, **kwargs)
        vmin = 0.0 if scale else None
        vmax = 1.0 if scale else None
        fig.imshow(arr, vmin=vmin, vmax=vmax, cmap=cmap)
        return fig

    # ── Trace methods ──────────────────────────────────────────────────

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

    def imshow(self, data, *, vmin=None, vmax=None, cmap=None, alpha=1.0,
               origin="upper", extent=None):
        """Add a 2D array as a heatmap image.

        Args:
            alpha: opacity (0.0-1.0). Values < 1.0 blend over existing content.
            origin: "upper" (default) or "lower" (FITS convention, row 0 at bottom).
            extent: [x_min, x_max, y_min, y_max] in pixel coords. When set,
                the data array is stretched to fill this region with bilinear
                interpolation (like matplotlib's extent + interpolation="bilinear").
        """
        arr = np.asarray(data, dtype=np.float64)
        if arr.ndim != 2:
            raise ValueError("imshow requires a 2D array")
        rows, cols = arr.shape
        flat = arr.ravel().tolist()
        if vmin is None:
            vmin = float(np.nanmin(arr[np.isfinite(arr)])) if np.any(np.isfinite(arr)) else 0.0
        if vmax is None:
            vmax = float(np.nanmax(arr[np.isfinite(arr)])) if np.any(np.isfinite(arr)) else 1.0
        trace = {
            "kind": "imshow",
            "data": flat,
            "rows": rows,
            "cols": cols,
            "vmin": float(vmin),
            "vmax": float(vmax),
            "cmap": cmap or self._cmap or "viridis",
            "alpha": float(alpha),
            "origin_lower": origin == "lower",
        }
        if extent is not None:
            trace["extent"] = [float(v) for v in extent]
        self._traces.append(trace)
        return self

    # ── Labels ─────────────────────────────────────────────────────────

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

    # ── Coordinate system ──────────────────────────────────────────────

    def set_transform(self, transform):
        """Set the active coordinate transform for overlay methods.

        After calling this, circle/crosshair/label coordinates are interpreted
        as world coordinates and resolved to pixels via the transform.
        Pass None to reset to pixel coordinates.
        """
        self._transform = transform
        return self

    def coord_grid(self, *, color="white", alpha=180, n_lines=3,
                   x_values=None, y_values=None, labels=True,
                   label_format=None):
        """Draw iso-coordinate lines through the active transform.

        This traces lines where one coordinate is held constant while the
        other varies — the core primitive that matplotlib lacks.

        Args:
            color: line color
            alpha: line alpha (0-255)
            n_lines: number of grid lines per axis
            x_values: explicit world-x grid values (None for auto)
            y_values: explicit world-y grid values (None for auto)
            labels: whether to add labels
            label_format: callable(axis, value) -> str
        """
        if self._transform is None:
            raise ValueError("coord_grid requires a transform (call set_transform first)")
        from inkblot.transform import generate_coord_grid
        overlays = generate_coord_grid(
            self._transform, self._width, self._height,
            color=color, alpha=alpha, n_lines=n_lines,
            x_values=x_values, y_values=y_values,
            labels=labels, label_format=label_format,
        )
        self._overlays.extend(overlays)
        return self

    # ── Overlay methods (pixel or world coords via transform) ──────────

    def _resolve_coords(self, x, y):
        if self._transform is not None:
            px, py = self._transform.world_to_pixel(x, y)
            return (float(px), float(py))
        return (float(x), float(y))

    def circle(self, x, y, *, radius=5.0, color="red", filled=False, linewidth=1.5):
        """Add a circle overlay."""
        px, py = self._resolve_coords(x, y)
        self._overlays.append({
            "kind": "circle",
            "x": px, "y": py,
            "radius": float(radius),
            "color": resolve_color(color),
            "filled": bool(filled),
            "linewidth": float(linewidth),
        })
        return self

    def crosshair(self, x, y, *, size=15.0, gap=3.0, color="lime", linewidth=1.5):
        """Add a crosshair marker overlay."""
        px, py = self._resolve_coords(x, y)
        self._overlays.append({
            "kind": "crosshair",
            "x": px, "y": py,
            "size": float(size), "gap": float(gap),
            "color": resolve_color(color),
            "linewidth": float(linewidth),
        })
        return self

    def polyline(self, xs, ys, *, color="white", linewidth=1.0, dashed=False):
        """Add a polyline overlay."""
        pxs, pys = [], []
        for wx, wy in zip(xs, ys):
            px, py = self._resolve_coords(wx, wy)
            pxs.append(px)
            pys.append(py)
        self._overlays.append({
            "kind": "polyline",
            "xs": pxs, "ys": pys,
            "color": resolve_color(color),
            "linewidth": float(linewidth),
            "dashed": bool(dashed),
        })
        return self

    def label(self, x, y, text, *, font_size=12.0, color="white",
              bg_color=None, rotation=0.0, stroke=False):
        """Add a text label overlay with optional background box and stroke outline."""
        px, py = self._resolve_coords(x, y)
        self._overlays.append({
            "kind": "label",
            "x": px, "y": py,
            "text": str(text),
            "font_size": float(font_size),
            "color": resolve_color(color),
            "bg_color": resolve_color(bg_color) if bg_color else None,
            "rotation": float(rotation),
            "stroke": bool(stroke),
        })
        return self

    def rect(self, x, y, w, h, *, color="green", filled=False, linewidth=1.5):
        """Add a rectangle overlay. (x, y) is top-left corner."""
        px, py = self._resolve_coords(x, y)
        self._overlays.append({
            "kind": "rect",
            "x": px, "y": py,
            "w": float(w), "h": float(h),
            "color": resolve_color(color),
            "filled": bool(filled),
            "linewidth": float(linewidth),
        })
        return self

    def polygon(self, xs, ys, *, color="white", linewidth=1.5, filled=False):
        """Add a closed polygon overlay (e.g. diamond marker)."""
        pxs, pys = [], []
        for wx, wy in zip(xs, ys):
            px, py = self._resolve_coords(wx, wy)
            pxs.append(px)
            pys.append(py)
        self._overlays.append({
            "kind": "polygon",
            "xs": pxs, "ys": pys,
            "color": resolve_color(color),
            "linewidth": float(linewidth),
            "filled": bool(filled),
        })
        return self

    def diamond(self, x, y, *, size=8, color="white", linewidth=1.5):
        """Add a diamond marker overlay."""
        px, py = self._resolve_coords(x, y)
        r = float(size)
        self._overlays.append({
            "kind": "polygon",
            "xs": [px, px + r, px, px - r],
            "ys": [py - r, py, py + r, py],
            "color": resolve_color(color),
            "linewidth": float(linewidth),
            "filled": False,
        })
        return self

    # ── Build + render ─────────────────────────────────────────────────

    def _build_spec(self):
        return {
            "width": self._width,
            "height": self._height,
            "frameless": self._frameless,
            "traces": self._traces,
            "overlays": self._overlays,
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
        with open(path, "wb") as f:
            f.write(self.render())
        return self

    def _repr_png_(self):
        """Jupyter notebook display integration."""
        return self.render()
