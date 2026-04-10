"""Image normalization/scaling functions. All return [0, 1] float64 arrays."""

import numpy as np


def linear(data, vmin=None, vmax=None):
    """Linear clip and normalize to [0, 1]."""
    arr = np.asarray(data, dtype=np.float64)
    finite = np.isfinite(arr)
    if vmin is None:
        vmin = float(np.nanmin(arr)) if np.any(finite) else 0.0
    if vmax is None:
        vmax = float(np.nanmax(arr)) if np.any(finite) else 1.0
    rng = max(vmax - vmin, 1e-12)
    out = np.clip((arr - vmin) / rng, 0, 1)
    out[~finite] = 0.0
    return out


def percent(data, lo=1, hi=99):
    """Percentile stretch to [0, 1]."""
    arr = np.asarray(data, dtype=np.float64)
    finite = arr[np.isfinite(arr)]
    if len(finite) == 0:
        return np.zeros_like(arr)
    vmin, vmax = np.percentile(finite, [lo, hi])
    return linear(arr, float(vmin), float(vmax))


def asinh(data, a=0.1):
    """Arcsinh stretch. Percentile-normalizes first, then applies asinh(x/a)/asinh(1/a)."""
    arr = percent(data, lo=0.5, hi=99.5)
    return np.arcsinh(arr / a) / np.arcsinh(1.0 / a)


def zscale(data, contrast=0.2):
    """ZScale interval (uses astropy if available, falls back to percentile)."""
    arr = np.asarray(data, dtype=np.float64)
    finite = arr[np.isfinite(arr)]
    if len(finite) == 0:
        return np.zeros_like(arr)
    try:
        from astropy.visualization import ZScaleInterval
        interval = ZScaleInterval(contrast=contrast)
        vmin, vmax = interval.get_limits(finite)
    except ImportError:
        vmin, vmax = np.percentile(finite, [1, 99])
    return linear(arr, float(vmin), float(vmax))


def apply_scale(data, name, **kwargs):
    """Dispatch scaling by name string."""
    funcs = {"linear": linear, "percent": percent, "asinh": asinh, "zscale": zscale}
    if name not in funcs:
        raise ValueError(f"Unknown scale: {name!r}. Choose from {list(funcs)}")
    return funcs[name](data, **kwargs)
