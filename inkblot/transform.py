"""Coordinate transforms and iso-line grid generation."""

import numpy as np

from inkblot.colors import resolve_color


class PixelTransform:
    """Identity transform — coordinates are already in pixels."""

    def world_to_pixel(self, wx, wy):
        return (float(wx), float(wy))

    def pixel_to_world(self, px, py):
        return (float(px), float(py))


class WCSTransform:
    """Wraps an astropy WCS object."""

    def __init__(self, wcs):
        self._wcs = wcs

    def world_to_pixel(self, wx, wy):
        px, py = self._wcs.world_to_pixel_values(float(wx), float(wy))
        return (float(px), float(py))

    def pixel_to_world(self, px, py):
        wx, wy = self._wcs.pixel_to_world_values(float(px), float(py))
        return (float(wx), float(wy))


class CallableTransform:
    """Wrap any pair of functions as a transform.

    Args:
        w2p: callable(wx, wy) -> (px, py)
        p2w: callable(px, py) -> (wx, wy)
    """

    def __init__(self, w2p, p2w):
        self._w2p = w2p
        self._p2w = p2w

    def world_to_pixel(self, wx, wy):
        return self._w2p(wx, wy)

    def pixel_to_world(self, px, py):
        return self._p2w(px, py)


def generate_coord_grid(transform, width, height, *, color="white", alpha=180,
                        n_lines=3, n_samples=200, x_values=None, y_values=None,
                        labels=True, label_format=None):
    """Generate iso-coordinate line overlays for a coordinate transform.

    Traces lines where one world coordinate is held constant while the other
    varies — the fundamental operation that matplotlib has no primitive for.

    Args:
        transform: object with world_to_pixel() and pixel_to_world()
        width, height: image dimensions in pixels
        color: line color (name, hex, or RGBA)
        alpha: line alpha (0-255)
        n_lines: number of grid lines per axis (auto-positioned)
        n_samples: density of line sampling
        x_values: explicit world-x grid values (overrides auto)
        y_values: explicit world-y grid values (overrides auto)
        labels: whether to add labels at line edges
        label_format: callable(axis, value) -> str for custom formatting

    Returns:
        list of overlay dicts (polylines + labels)
    """
    overlays = []
    rgba = resolve_color(color)
    rgba[3] = alpha

    # 1. Sample pixel grid → world coordinates to discover ranges
    px_grid = np.linspace(0, width - 1, 50)
    py_grid = np.linspace(0, height - 1, 50)
    world_x_all, world_y_all = [], []

    for px in px_grid:
        for py in py_grid:
            try:
                wx, wy = transform.pixel_to_world(px, py)
                if np.isfinite(wx) and np.isfinite(wy):
                    world_x_all.append(wx)
                    world_y_all.append(wy)
            except Exception:
                continue

    if len(world_x_all) < 4:
        return overlays

    world_x_all = np.array(world_x_all)
    world_y_all = np.array(world_y_all)

    # 2. Choose grid values
    if x_values is None:
        x_lo, x_hi = np.percentile(world_x_all, [10, 90])
        x_values = np.linspace(x_lo, x_hi, n_lines + 2)[1:-1]
    if y_values is None:
        y_lo, y_hi = np.percentile(world_y_all, [10, 90])
        y_values = np.linspace(y_lo, y_hi, n_lines + 2)[1:-1]

    # Extend sampling ranges beyond image bounds
    wy_min, wy_max = world_y_all.min(), world_y_all.max()
    wy_pad = (wy_max - wy_min) * 0.2
    y_range = np.linspace(wy_min - wy_pad, wy_max + wy_pad, n_samples)

    wx_min, wx_max = world_x_all.min(), world_x_all.max()
    wx_pad = (wx_max - wx_min) * 0.2
    x_range = np.linspace(wx_min - wx_pad, wx_max + wx_pad, n_samples)

    max_jump = max(width, height) * 0.25

    # 3. Trace constant-x lines (vertical-ish)
    for xv in x_values:
        segments = _trace_iso_line(
            transform, xv, y_range, width, height, max_jump, axis="x"
        )
        for pxs, pys in segments:
            overlays.append({
                "kind": "polyline",
                "xs": pxs, "ys": pys,
                "color": list(rgba), "linewidth": 0.8,
            })
        if labels and segments:
            last_seg = segments[-1]
            if len(last_seg[0]) >= 2:
                lx, ly = last_seg[0][-1], last_seg[1][-1]
                angle = _line_angle_at(last_seg[0], last_seg[1], -1)
                text = _format_value("x", xv, label_format)
                overlays.append({
                    "kind": "label",
                    "x": float(lx), "y": float(ly) - 5,
                    "text": text, "font_size": 10.0,
                    "color": [255, 255, 255, 220],
                    "bg_color": [0, 0, 0, 160],
                    "rotation": float(angle),
                })

    # 4. Trace constant-y lines (horizontal-ish)
    for yv in y_values:
        segments = _trace_iso_line(
            transform, yv, x_range, width, height, max_jump, axis="y"
        )
        for pxs, pys in segments:
            overlays.append({
                "kind": "polyline",
                "xs": pxs, "ys": pys,
                "color": list(rgba), "linewidth": 0.8,
            })
        if labels and segments:
            first_seg = segments[0]
            if len(first_seg[0]) >= 2:
                lx, ly = first_seg[0][0], first_seg[1][0]
                angle = _line_angle_at(first_seg[0], first_seg[1], 0)
                text = _format_value("y", yv, label_format)
                overlays.append({
                    "kind": "label",
                    "x": float(lx) + 5, "y": float(ly),
                    "text": text, "font_size": 10.0,
                    "color": [255, 255, 255, 220],
                    "bg_color": [0, 0, 0, 160],
                    "rotation": float(angle),
                })

    return overlays


def _trace_iso_line(transform, const_val, sweep_range, width, height, max_jump, axis):
    """Trace a line of constant value through the transform.

    Returns list of (xs, ys) segments (broken at discontinuities/out-of-bounds).
    """
    segments = []
    cur_xs, cur_ys = [], []

    for sv in sweep_range:
        try:
            if axis == "x":
                px, py = transform.world_to_pixel(const_val, sv)
            else:
                px, py = transform.world_to_pixel(sv, const_val)
        except Exception:
            _flush_segment(segments, cur_xs, cur_ys)
            cur_xs, cur_ys = [], []
            continue

        if not (np.isfinite(px) and np.isfinite(py)):
            _flush_segment(segments, cur_xs, cur_ys)
            cur_xs, cur_ys = [], []
            continue

        # Check for discontinuity (wraparound, projection boundary)
        if cur_xs and (abs(px - cur_xs[-1]) > max_jump or abs(py - cur_ys[-1]) > max_jump):
            _flush_segment(segments, cur_xs, cur_ys)
            cur_xs, cur_ys = [], []

        # Only keep points near the image (with some margin)
        margin = max(width, height) * 0.1
        if -margin <= px <= width + margin and -margin <= py <= height + margin:
            cur_xs.append(float(px))
            cur_ys.append(float(py))

    _flush_segment(segments, cur_xs, cur_ys)
    return segments


def _flush_segment(segments, xs, ys):
    if len(xs) >= 2:
        segments.append((list(xs), list(ys)))


def _line_angle_at(xs, ys, idx):
    """Compute line angle in degrees at a point, clamped to [-90, 90]."""
    n = len(xs)
    if n < 2:
        return 0.0
    # Use neighbors for tangent
    if idx < 0:
        idx = n + idx
    i0 = max(0, idx - 1)
    i1 = min(n - 1, idx + 1)
    dx = xs[i1] - xs[i0]
    dy = ys[i1] - ys[i0]
    angle = np.degrees(np.arctan2(dy, dx))
    # Keep text upright
    if angle > 90:
        angle -= 180
    elif angle < -90:
        angle += 180
    return angle


def _format_value(axis, value, label_format):
    if label_format is not None:
        return label_format(axis, value)
    if abs(value) < 1e-10:
        return "0"
    if abs(value) >= 1000 or abs(value) < 0.01:
        return f"{value:.3g}"
    return f"{value:.2f}"
