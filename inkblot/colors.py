"""Named color resolution and RGBA handling."""

NAMED_COLORS = {
    "black": [0, 0, 0, 255],
    "white": [255, 255, 255, 255],
    "red": [255, 0, 0, 255],
    "green": [0, 128, 0, 255],
    "blue": [0, 0, 255, 255],
    "cyan": [0, 255, 255, 255],
    "magenta": [255, 0, 255, 255],
    "yellow": [255, 255, 0, 255],
    "orange": [255, 165, 0, 255],
    "gray": [128, 128, 128, 255],
    "grey": [128, 128, 128, 255],
    "steelblue": [70, 130, 180, 255],
    "coral": [255, 127, 80, 255],
    "tomato": [255, 99, 71, 255],
    "gold": [255, 215, 0, 255],
    "forestgreen": [34, 139, 34, 255],
    "royalblue": [65, 105, 225, 255],
    "slategray": [112, 128, 144, 255],
    "darkviolet": [148, 0, 211, 255],
    "firebrick": [178, 34, 34, 255],
    "teal": [0, 128, 128, 255],
}

# Default color cycle for multiple traces
COLOR_CYCLE = [
    [70, 130, 180, 255],   # steelblue
    [255, 127, 80, 255],   # coral
    [34, 139, 34, 255],    # forestgreen
    [148, 0, 211, 255],    # darkviolet
    [255, 165, 0, 255],    # orange
    [255, 99, 71, 255],    # tomato
    [0, 128, 128, 255],    # teal
    [65, 105, 225, 255],   # royalblue
]


def resolve_color(color):
    """Convert a color specification to [r, g, b, a] list."""
    if isinstance(color, (list, tuple)):
        rgba = list(color)
        if len(rgba) == 3:
            rgba.append(255)
        return [int(c) for c in rgba[:4]]
    if isinstance(color, str):
        if color.startswith("#"):
            return _hex_to_rgba(color)
        lower = color.lower()
        if lower in NAMED_COLORS:
            return list(NAMED_COLORS[lower])
    return [0, 0, 0, 255]


def _hex_to_rgba(hex_str):
    """Parse #RGB, #RGBA, #RRGGBB, or #RRGGBBAA."""
    h = hex_str.lstrip("#")
    if len(h) == 3:
        r, g, b = (int(c * 2, 16) for c in h)
        return [r, g, b, 255]
    if len(h) == 4:
        r, g, b, a = (int(c * 2, 16) for c in h)
        return [r, g, b, a]
    if len(h) == 6:
        return [int(h[i : i + 2], 16) for i in (0, 2, 4)] + [255]
    if len(h) == 8:
        return [int(h[i : i + 2], 16) for i in (0, 2, 4, 6)]
    return [0, 0, 0, 255]
