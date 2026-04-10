"""inkblot - Fast, simple plotting for scientists. No matplotlib required."""

from inkblot.figure import Figure
from inkblot.colors import resolve_color, COLOR_CYCLE
from inkblot import scale
from inkblot.transform import PixelTransform, WCSTransform, CallableTransform

__version__ = "0.0.1"


def plot(x, y, *, color="steelblue", linewidth=1.5, title=None,
         xlabel=None, ylabel=None, grid=False, save=None,
         width=800, height=600):
    """Quick line plot. Returns a Figure for further customization."""
    fig = Figure(width=width, height=height)
    fig.line(x, y, color=color, linewidth=linewidth)
    if title:
        fig.title(title)
    if xlabel:
        fig.xlabel(xlabel)
    if ylabel:
        fig.ylabel(ylabel)
    if grid:
        fig.grid(True)
    if save:
        fig.save(save)
    return fig


def scatter(x, y, *, color="coral", size=3.0, title=None,
            xlabel=None, ylabel=None, grid=False, save=None,
            width=800, height=600):
    """Quick scatter plot. Returns a Figure for further customization."""
    fig = Figure(width=width, height=height)
    fig.scatter(x, y, color=color, size=size)
    if title:
        fig.title(title)
    if xlabel:
        fig.xlabel(xlabel)
    if ylabel:
        fig.ylabel(ylabel)
    if grid:
        fig.grid(True)
    if save:
        fig.save(save)
    return fig


def imshow(data, *, vmin=None, vmax=None, cmap="viridis", title=None,
           xlabel=None, ylabel=None, save=None,
           width=800, height=600):
    """Quick heatmap of a 2D array. Returns a Figure for further customization."""
    fig = Figure(width=width, height=height)
    fig.imshow(data, vmin=vmin, vmax=vmax, cmap=cmap)
    if title:
        fig.title(title)
    if xlabel:
        fig.xlabel(xlabel)
    if ylabel:
        fig.ylabel(ylabel)
    if save:
        fig.save(save)
    return fig


def from_image(data, *, scale=None, cmap="gray", **kwargs):
    """Create a frameless figure from a 2D array. Pixel-perfect output."""
    return Figure.from_image(data, scale=scale, cmap=cmap, **kwargs)
