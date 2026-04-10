use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use tiny_skia::*;

use crate::colors::paint_from_rgba;
use crate::layout::{compute_layout, format_tick, Layout};
use crate::text;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

pub struct PlotSpec {
    pub width: u32,
    pub height: u32,
    pub traces: Vec<Trace>,
    pub title: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    pub grid: bool,
}

pub enum Trace {
    Line {
        x: Vec<f64>,
        y: Vec<f64>,
        color: [u8; 4],
        linewidth: f32,
    },
    Scatter {
        x: Vec<f64>,
        y: Vec<f64>,
        color: [u8; 4],
        size: f32,
    },
    Imshow {
        data: Vec<f64>,
        rows: usize,
        cols: usize,
        vmin: f64,
        vmax: f64,
    },
}

// ---------------------------------------------------------------------------
// Coordinate transform
// ---------------------------------------------------------------------------

struct CoordTransform {
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_left: f32,
    plot_right: f32,
    plot_top: f32,
    plot_bottom: f32,
}

impl CoordTransform {
    fn data_to_pixel(&self, x: f64, y: f64) -> (f32, f32) {
        let px = self.plot_left
            + ((x - self.x_min) / (self.x_max - self.x_min)) as f32
                * (self.plot_right - self.plot_left);
        let py = self.plot_bottom
            - ((y - self.y_min) / (self.y_max - self.y_min)) as f32
                * (self.plot_bottom - self.plot_top);
        (px, py)
    }
}

// ---------------------------------------------------------------------------
// Deserialization from Python dict
// ---------------------------------------------------------------------------

impl PlotSpec {
    pub fn from_pydict(spec: &Bound<'_, PyDict>) -> PyResult<Self> {
        let width: u32 = spec
            .get_item("width")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("width"))?
            .extract()?;
        let height: u32 = spec
            .get_item("height")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("height"))?
            .extract()?;
        let title: Option<String> = spec
            .get_item("title")?
            .and_then(|v| if v.is_none() { None } else { Some(v) })
            .map(|v| v.extract())
            .transpose()?;
        let xlabel: Option<String> = spec
            .get_item("xlabel")?
            .and_then(|v| if v.is_none() { None } else { Some(v) })
            .map(|v| v.extract())
            .transpose()?;
        let ylabel: Option<String> = spec
            .get_item("ylabel")?
            .and_then(|v| if v.is_none() { None } else { Some(v) })
            .map(|v| v.extract())
            .transpose()?;
        let grid: bool = spec
            .get_item("grid")?
            .map(|v| v.extract().unwrap_or(false))
            .unwrap_or(false);

        let traces_list: Bound<'_, PyList> = spec
            .get_item("traces")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("traces"))?
            .downcast_into()?;

        let mut traces = Vec::new();
        for item in traces_list.iter() {
            let trace_dict: &Bound<'_, PyDict> = item.downcast()?;
            let kind: String = trace_dict
                .get_item("kind")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("kind"))?
                .extract()?;

            match kind.as_str() {
                "line" | "scatter" => {
                    let x: Vec<f64> = trace_dict
                        .get_item("x")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("x"))?
                        .extract()?;
                    let y: Vec<f64> = trace_dict
                        .get_item("y")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("y"))?
                        .extract()?;
                    let color_list: Vec<u8> = trace_dict
                        .get_item("color")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("color"))?
                        .extract()?;
                    let color = [
                        color_list.first().copied().unwrap_or(0),
                        color_list.get(1).copied().unwrap_or(0),
                        color_list.get(2).copied().unwrap_or(0),
                        color_list.get(3).copied().unwrap_or(255),
                    ];

                    if kind == "line" {
                        let linewidth: f32 = trace_dict
                            .get_item("linewidth")?
                            .map(|v| v.extract().unwrap_or(1.5))
                            .unwrap_or(1.5);
                        traces.push(Trace::Line {
                            x,
                            y,
                            color,
                            linewidth,
                        });
                    } else {
                        let size: f32 = trace_dict
                            .get_item("size")?
                            .map(|v| v.extract().unwrap_or(3.0))
                            .unwrap_or(3.0);
                        traces.push(Trace::Scatter {
                            x,
                            y,
                            color,
                            size,
                        });
                    }
                }
                "imshow" => {
                    let data: Vec<f64> = trace_dict
                        .get_item("data")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("data"))?
                        .extract()?;
                    let rows: usize = trace_dict
                        .get_item("rows")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("rows"))?
                        .extract()?;
                    let cols: usize = trace_dict
                        .get_item("cols")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("cols"))?
                        .extract()?;
                    let vmin: f64 = trace_dict
                        .get_item("vmin")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("vmin"))?
                        .extract()?;
                    let vmax: f64 = trace_dict
                        .get_item("vmax")?
                        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("vmax"))?
                        .extract()?;
                    traces.push(Trace::Imshow {
                        data,
                        rows,
                        cols,
                        vmin,
                        vmax,
                    });
                }
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "unknown trace kind: {other}"
                    )));
                }
            }
        }

        Ok(PlotSpec {
            width,
            height,
            traces,
            title,
            xlabel,
            ylabel,
            grid,
        })
    }
}

// ---------------------------------------------------------------------------
// Main render pipeline
// ---------------------------------------------------------------------------

pub fn render_plot(spec: &PlotSpec) -> PyResult<Vec<u8>> {
    let mut pixmap = Pixmap::new(spec.width, spec.height).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err("failed to create pixmap (invalid dimensions)")
    })?;

    // White background
    pixmap.fill(Color::WHITE);

    // Compute data bounds across all traces
    let (x_min, x_max, y_min, y_max) = data_bounds(&spec.traces);

    // Compute layout
    let layout = compute_layout(
        x_min,
        x_max,
        y_min,
        y_max,
        spec.title.is_some(),
        spec.xlabel.is_some(),
        spec.ylabel.is_some(),
    );

    let ct = CoordTransform {
        x_min,
        x_max,
        y_min,
        y_max,
        plot_left: layout.plot_left(),
        plot_right: layout.plot_right(spec.width),
        plot_top: layout.plot_top(),
        plot_bottom: layout.plot_bottom(spec.height),
    };

    // Draw components
    if spec.grid {
        draw_grid(&mut pixmap, &layout, &ct, spec.width, spec.height);
    }
    draw_axes(&mut pixmap, &layout, spec.width, spec.height);
    draw_ticks(&mut pixmap, &layout, &ct, spec.width, spec.height);

    // Draw traces (clipped to plot area)
    for trace in &spec.traces {
        match trace {
            Trace::Line {
                x,
                y,
                color,
                linewidth,
            } => draw_line(&mut pixmap, x, y, *color, *linewidth, &ct),
            Trace::Scatter {
                x,
                y,
                color,
                size,
            } => draw_scatter(&mut pixmap, x, y, *color, *size, &ct),
            Trace::Imshow {
                data,
                rows,
                cols,
                vmin,
                vmax,
            } => draw_imshow(&mut pixmap, data, *rows, *cols, *vmin, *vmax, &ct),
        }
    }

    // Draw labels
    if let Some(ref title) = spec.title {
        let cx = (layout.plot_left() + layout.plot_right(spec.width)) / 2.0;
        text::draw_text_centered(&mut pixmap, title, cx, layout.plot_top() - 12.0, 18.0, [0, 0, 0, 255]);
    }
    if let Some(ref xlabel) = spec.xlabel {
        let cx = (layout.plot_left() + layout.plot_right(spec.width)) / 2.0;
        text::draw_text_centered(
            &mut pixmap,
            xlabel,
            cx,
            layout.plot_bottom(spec.height) + 45.0,
            14.0,
            [0, 0, 0, 255],
        );
    }
    if let Some(ref ylabel) = spec.ylabel {
        let cy = (layout.plot_top() + layout.plot_bottom(spec.height)) / 2.0;
        text::draw_text_rotated(&mut pixmap, ylabel, 18.0, cy, 14.0, [0, 0, 0, 255]);
    }

    pixmap
        .encode_png()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("PNG encode failed: {e}")))
}

// ---------------------------------------------------------------------------
// Data bounds
// ---------------------------------------------------------------------------

fn data_bounds(traces: &[Trace]) -> (f64, f64, f64, f64) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;

    for trace in traces {
        match trace {
            Trace::Line { x, y, .. } | Trace::Scatter { x, y, .. } => {
                for &v in x {
                    if v < x_min { x_min = v; }
                    if v > x_max { x_max = v; }
                }
                for &v in y {
                    if v < y_min { y_min = v; }
                    if v > y_max { y_max = v; }
                }
            }
            Trace::Imshow { rows, cols, .. } => {
                // Imshow covers [0, cols] x [0, rows] in data space
                if 0.0 < x_min { x_min = 0.0; }
                if (*cols as f64) > x_max { x_max = *cols as f64; }
                if 0.0 < y_min { y_min = 0.0; }
                if (*rows as f64) > y_max { y_max = *rows as f64; }
            }
        }
    }

    // For imshow-only plots, skip padding
    let has_imshow = traces.iter().any(|t| matches!(t, Trace::Imshow { .. }));
    if has_imshow {
        return (x_min, x_max, y_min, y_max);
    }

    // Add 5% padding
    let x_pad = (x_max - x_min).abs() * 0.05;
    let y_pad = (y_max - y_min).abs() * 0.05;
    let x_pad = if x_pad < 1e-12 { 1.0 } else { x_pad };
    let y_pad = if y_pad < 1e-12 { 1.0 } else { y_pad };

    (x_min - x_pad, x_max + x_pad, y_min - y_pad, y_max + y_pad)
}

// ---------------------------------------------------------------------------
// Drawing functions
// ---------------------------------------------------------------------------

fn draw_axes(pixmap: &mut Pixmap, layout: &Layout, width: u32, height: u32) {
    let paint = paint_from_rgba([0, 0, 0, 255]);
    let mut stroke = Stroke::default();
    stroke.width = 1.5;

    let left = layout.plot_left();
    let right = layout.plot_right(width);
    let top = layout.plot_top();
    let bottom = layout.plot_bottom(height);

    // Single closed rectangle — square corners, no endpoint gaps
    let mut pb = PathBuilder::new();
    pb.move_to(left, top);
    pb.line_to(right, top);
    pb.line_to(right, bottom);
    pb.line_to(left, bottom);
    pb.close();
    if let Some(path) = pb.finish() {
        stroke.line_join = LineJoin::Miter;
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn draw_grid(pixmap: &mut Pixmap, layout: &Layout, ct: &CoordTransform, width: u32, height: u32) {
    let mut paint = paint_from_rgba([200, 200, 200, 255]);
    paint.anti_alias = false;
    let mut stroke = Stroke::default();
    stroke.width = 0.5;

    let left = layout.plot_left();
    let right = layout.plot_right(width);
    let top = layout.plot_top();
    let bottom = layout.plot_bottom(height);

    for &xv in &layout.x_ticks {
        let (px, _) = ct.data_to_pixel(xv, 0.0);
        if px > left && px < right {
            if let Some(path) = line_path(px, top, px, bottom) {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }
    for &yv in &layout.y_ticks {
        let (_, py) = ct.data_to_pixel(0.0, yv);
        if py > top && py < bottom {
            if let Some(path) = line_path(left, py, right, py) {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }
}

fn draw_ticks(
    pixmap: &mut Pixmap,
    layout: &Layout,
    ct: &CoordTransform,
    width: u32,
    height: u32,
) {
    let paint = paint_from_rgba([0, 0, 0, 255]);
    let mut stroke = Stroke::default();
    stroke.width = 1.0;

    let left = layout.plot_left();
    let right = layout.plot_right(width);
    let top = layout.plot_top();
    let bottom = layout.plot_bottom(height);
    let tick_len = 6.0;

    let x_step = if layout.x_ticks.len() >= 2 {
        (layout.x_ticks[1] - layout.x_ticks[0]).abs()
    } else {
        1.0
    };
    let y_step = if layout.y_ticks.len() >= 2 {
        (layout.y_ticks[1] - layout.y_ticks[0]).abs()
    } else {
        1.0
    };

    // X ticks
    for &xv in &layout.x_ticks {
        let (px, _) = ct.data_to_pixel(xv, 0.0);
        if px >= left && px <= right {
            if let Some(path) = line_path(px, bottom, px, bottom + tick_len) {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
            let label = format_tick(xv, x_step);
            text::draw_text_centered(pixmap, &label, px, bottom + tick_len + 16.0, 12.0, [0, 0, 0, 255]);
        }
    }

    // Y ticks
    for &yv in &layout.y_ticks {
        let (_, py) = ct.data_to_pixel(0.0, yv);
        if py >= top && py <= bottom {
            if let Some(path) = line_path(left - tick_len, py, left, py) {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
            let label = format_tick(yv, y_step);
            let tw = text::measure_text(&label, 12.0);
            text::draw_text(pixmap, &label, left - tick_len - tw - 4.0, py + 4.0, 12.0, [0, 0, 0, 255]);
        }
    }
}

fn draw_line(
    pixmap: &mut Pixmap,
    x: &[f64],
    y: &[f64],
    color: [u8; 4],
    linewidth: f32,
    ct: &CoordTransform,
) {
    if x.len() < 2 {
        return;
    }
    let mut pb = PathBuilder::new();
    let (px, py) = ct.data_to_pixel(x[0], y[0]);
    pb.move_to(px, py);
    for i in 1..x.len().min(y.len()) {
        let (px, py) = ct.data_to_pixel(x[i], y[i]);
        pb.line_to(px, py);
    }
    if let Some(path) = pb.finish() {
        let paint = paint_from_rgba(color);
        let mut stroke = Stroke::default();
        stroke.width = linewidth;
        stroke.line_cap = LineCap::Round;
        stroke.line_join = LineJoin::Round;
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn draw_scatter(
    pixmap: &mut Pixmap,
    x: &[f64],
    y: &[f64],
    color: [u8; 4],
    size: f32,
    ct: &CoordTransform,
) {
    let paint = paint_from_rgba(color);
    for i in 0..x.len().min(y.len()) {
        let (px, py) = ct.data_to_pixel(x[i], y[i]);
        let mut pb = PathBuilder::new();
        pb.push_circle(px, py, size);
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
}

fn draw_imshow(
    pixmap: &mut Pixmap,
    data: &[f64],
    rows: usize,
    cols: usize,
    vmin: f64,
    vmax: f64,
    ct: &CoordTransform,
) {
    let range = vmax - vmin;
    let range = if range.abs() < 1e-12 { 1.0 } else { range };

    for row in 0..rows {
        for col in 0..cols {
            let val = data[row * cols + col];
            let t = ((val - vmin) / range).clamp(0.0, 1.0);
            let color = viridis(t);

            // Each cell covers [col, col+1] x [row, row+1] in data space
            // Note: row 0 is at the top (highest y value)
            let (px1, py1) = ct.data_to_pixel(col as f64, (rows - row) as f64);
            let (px2, py2) = ct.data_to_pixel((col + 1) as f64, (rows - row - 1) as f64);

            let left = px1.min(px2);
            let top = py1.min(py2);
            let w = (px2 - px1).abs();
            let h = (py2 - py1).abs();

            if let Some(rect) = Rect::from_xywh(left, top, w.max(1.0), h.max(1.0)) {
                let paint = paint_from_rgba(color);
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
        }
    }
}

/// Viridis colormap: maps t in [0, 1] to RGBA.
fn viridis(t: f64) -> [u8; 4] {
    // 9-stop approximation of matplotlib's viridis
    const STOPS: [(f64, [u8; 3]); 9] = [
        (0.000, [68, 1, 84]),
        (0.125, [72, 36, 117]),
        (0.250, [64, 67, 135]),
        (0.375, [52, 94, 141]),
        (0.500, [33, 145, 140]),
        (0.625, [53, 183, 121]),
        (0.750, [109, 205, 89]),
        (0.875, [180, 222, 44]),
        (1.000, [253, 231, 37]),
    ];

    let t = t.clamp(0.0, 1.0);

    // Find the two stops to interpolate between
    let mut i = 0;
    while i < STOPS.len() - 2 && STOPS[i + 1].0 < t {
        i += 1;
    }

    let (t0, c0) = STOPS[i];
    let (t1, c1) = STOPS[i + 1];
    let f = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);

    [
        (c0[0] as f64 + (c1[0] as f64 - c0[0] as f64) * f) as u8,
        (c0[1] as f64 + (c1[1] as f64 - c0[1] as f64) * f) as u8,
        (c0[2] as f64 + (c1[2] as f64 - c0[2] as f64) * f) as u8,
        255,
    ]
}

fn line_path(x1: f32, y1: f32, x2: f32, y2: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);
    pb.finish()
}
