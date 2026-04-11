use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use tiny_skia::*;

use crate::colors::{colormap, paint_from_rgba};
use crate::layout::{compute_layout, format_tick, Layout};
use crate::text;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

pub struct PlotSpec {
    pub width: u32,
    pub height: u32,
    pub frameless: bool,
    pub traces: Vec<Trace>,
    pub overlays: Vec<Overlay>,
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
        cmap: String,
        alpha: f32,
        origin_lower: bool,
    },
}

pub enum Overlay {
    Circle {
        x: f32,
        y: f32,
        radius: f32,
        color: [u8; 4],
        filled: bool,
        linewidth: f32,
    },
    Crosshair {
        x: f32,
        y: f32,
        size: f32,
        gap: f32,
        color: [u8; 4],
        linewidth: f32,
    },
    Polyline {
        xs: Vec<f32>,
        ys: Vec<f32>,
        color: [u8; 4],
        linewidth: f32,
        dashed: bool,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [u8; 4],
        filled: bool,
        linewidth: f32,
    },
    Polygon {
        xs: Vec<f32>,
        ys: Vec<f32>,
        color: [u8; 4],
        linewidth: f32,
        filled: bool,
    },
    Label {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color: [u8; 4],
        bg_color: Option<[u8; 4]>,
        rotation: f32,
        stroke: bool,
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

fn extract_color(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<[u8; 4]> {
    let list: Vec<u8> = dict
        .get_item(key)?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key.to_string()))?
        .extract()?;
    Ok([
        list.first().copied().unwrap_or(0),
        list.get(1).copied().unwrap_or(0),
        list.get(2).copied().unwrap_or(0),
        list.get(3).copied().unwrap_or(255),
    ])
}

fn extract_optional_color(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<[u8; 4]>> {
    match dict.get_item(key)? {
        Some(v) if !v.is_none() => {
            let list: Vec<u8> = v.extract()?;
            Ok(Some([
                list.first().copied().unwrap_or(0),
                list.get(1).copied().unwrap_or(0),
                list.get(2).copied().unwrap_or(0),
                list.get(3).copied().unwrap_or(255),
            ]))
        }
        _ => Ok(None),
    }
}

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
        let frameless: bool = spec
            .get_item("frameless")?
            .map(|v| v.extract().unwrap_or(false))
            .unwrap_or(false);
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

        // Parse traces
        let traces_list: Bound<'_, PyList> = spec
            .get_item("traces")?
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("traces"))?
            .downcast_into()?;

        let mut traces = Vec::new();
        for item in traces_list.iter() {
            let d: &Bound<'_, PyDict> = item.downcast()?;
            let kind: String = d
                .get_item("kind")?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("kind"))?
                .extract()?;

            match kind.as_str() {
                "line" | "scatter" => {
                    let x: Vec<f64> = d.get_item("x")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("x"))?.extract()?;
                    let y: Vec<f64> = d.get_item("y")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("y"))?.extract()?;
                    let color = extract_color(d, "color")?;
                    if kind == "line" {
                        let linewidth: f32 = d.get_item("linewidth")?.map(|v| v.extract().unwrap_or(1.5)).unwrap_or(1.5);
                        traces.push(Trace::Line { x, y, color, linewidth });
                    } else {
                        let size: f32 = d.get_item("size")?.map(|v| v.extract().unwrap_or(3.0)).unwrap_or(3.0);
                        traces.push(Trace::Scatter { x, y, color, size });
                    }
                }
                "imshow" => {
                    let data: Vec<f64> = d.get_item("data")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("data"))?.extract()?;
                    let rows: usize = d.get_item("rows")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("rows"))?.extract()?;
                    let cols: usize = d.get_item("cols")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("cols"))?.extract()?;
                    let vmin: f64 = d.get_item("vmin")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("vmin"))?.extract()?;
                    let vmax: f64 = d.get_item("vmax")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("vmax"))?.extract()?;
                    let cmap: String = d.get_item("cmap")?.map(|v| v.extract().unwrap_or_else(|_| "viridis".to_string())).unwrap_or_else(|| "viridis".to_string());
                    let alpha: f32 = d.get_item("alpha")?.map(|v| v.extract().unwrap_or(1.0)).unwrap_or(1.0);
                    let origin_lower: bool = d.get_item("origin_lower")?.map(|v| v.extract().unwrap_or(false)).unwrap_or(false);
                    traces.push(Trace::Imshow { data, rows, cols, vmin, vmax, cmap, alpha, origin_lower });
                }
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!("unknown trace kind: {other}")));
                }
            }
        }

        // Parse overlays
        let mut overlays = Vec::new();
        if let Some(overlays_obj) = spec.get_item("overlays")? {
            if !overlays_obj.is_none() {
                let overlays_list: Bound<'_, PyList> = overlays_obj.downcast_into()?;
                for item in overlays_list.iter() {
                    let d: &Bound<'_, PyDict> = item.downcast()?;
                    let kind: String = d.get_item("kind")?.ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("kind"))?.extract()?;

                    match kind.as_str() {
                        "circle" => {
                            overlays.push(Overlay::Circle {
                                x: d.get_item("x")?.unwrap().extract()?,
                                y: d.get_item("y")?.unwrap().extract()?,
                                radius: d.get_item("radius")?.unwrap().extract()?,
                                color: extract_color(d, "color")?,
                                filled: d.get_item("filled")?.map(|v| v.extract().unwrap_or(false)).unwrap_or(false),
                                linewidth: d.get_item("linewidth")?.map(|v| v.extract().unwrap_or(1.5)).unwrap_or(1.5),
                            });
                        }
                        "crosshair" => {
                            overlays.push(Overlay::Crosshair {
                                x: d.get_item("x")?.unwrap().extract()?,
                                y: d.get_item("y")?.unwrap().extract()?,
                                size: d.get_item("size")?.map(|v| v.extract().unwrap_or(15.0)).unwrap_or(15.0),
                                gap: d.get_item("gap")?.map(|v| v.extract().unwrap_or(3.0)).unwrap_or(3.0),
                                color: extract_color(d, "color")?,
                                linewidth: d.get_item("linewidth")?.map(|v| v.extract().unwrap_or(1.5)).unwrap_or(1.5),
                            });
                        }
                        "polyline" => {
                            overlays.push(Overlay::Polyline {
                                xs: d.get_item("xs")?.unwrap().extract()?,
                                ys: d.get_item("ys")?.unwrap().extract()?,
                                color: extract_color(d, "color")?,
                                linewidth: d.get_item("linewidth")?.map(|v| v.extract().unwrap_or(1.0)).unwrap_or(1.0),
                                dashed: d.get_item("dashed")?.map(|v| v.extract().unwrap_or(false)).unwrap_or(false),
                            });
                        }
                        "label" => {
                            overlays.push(Overlay::Label {
                                x: d.get_item("x")?.unwrap().extract()?,
                                y: d.get_item("y")?.unwrap().extract()?,
                                text: d.get_item("text")?.unwrap().extract()?,
                                font_size: d.get_item("font_size")?.map(|v| v.extract().unwrap_or(12.0)).unwrap_or(12.0),
                                color: extract_color(d, "color")?,
                                bg_color: extract_optional_color(d, "bg_color")?,
                                rotation: d.get_item("rotation")?.map(|v| v.extract().unwrap_or(0.0)).unwrap_or(0.0),
                                stroke: d.get_item("stroke")?.map(|v| v.extract().unwrap_or(false)).unwrap_or(false),
                            });
                        }
                        "rect" => {
                            overlays.push(Overlay::Rect {
                                x: d.get_item("x")?.unwrap().extract()?,
                                y: d.get_item("y")?.unwrap().extract()?,
                                w: d.get_item("w")?.unwrap().extract()?,
                                h: d.get_item("h")?.unwrap().extract()?,
                                color: extract_color(d, "color")?,
                                filled: d.get_item("filled")?.map(|v| v.extract().unwrap_or(false)).unwrap_or(false),
                                linewidth: d.get_item("linewidth")?.map(|v| v.extract().unwrap_or(1.5)).unwrap_or(1.5),
                            });
                        }
                        "polygon" => {
                            overlays.push(Overlay::Polygon {
                                xs: d.get_item("xs")?.unwrap().extract()?,
                                ys: d.get_item("ys")?.unwrap().extract()?,
                                color: extract_color(d, "color")?,
                                linewidth: d.get_item("linewidth")?.map(|v| v.extract().unwrap_or(1.5)).unwrap_or(1.5),
                                filled: d.get_item("filled")?.map(|v| v.extract().unwrap_or(false)).unwrap_or(false),
                            });
                        }
                        other => {
                            return Err(pyo3::exceptions::PyValueError::new_err(format!("unknown overlay kind: {other}")));
                        }
                    }
                }
            }
        }

        Ok(PlotSpec {
            width,
            height,
            frameless,
            traces,
            overlays,
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

    if spec.frameless {
        // Frameless: black background, no margins, image fills entire pixmap
        pixmap.fill(Color::BLACK);

        let ct = CoordTransform {
            x_min: 0.0,
            x_max: spec.width as f64,
            y_min: 0.0,
            y_max: spec.height as f64,
            plot_left: 0.0,
            plot_right: spec.width as f32,
            plot_top: 0.0,
            plot_bottom: spec.height as f32,
        };

        for trace in &spec.traces {
            match trace {
                Trace::Imshow { data, rows, cols, vmin, vmax, cmap, alpha, origin_lower } => {
                    draw_imshow(&mut pixmap, data, *rows, *cols, *vmin, *vmax, cmap, *alpha, *origin_lower, &ct);
                }
                Trace::Line { x, y, color, linewidth } => {
                    draw_line(&mut pixmap, x, y, *color, *linewidth, &ct);
                }
                Trace::Scatter { x, y, color, size } => {
                    draw_scatter(&mut pixmap, x, y, *color, *size, &ct);
                }
            }
        }
    } else {
        // Normal mode: white background, margins, axes
        pixmap.fill(Color::WHITE);

        let (x_min, x_max, y_min, y_max) = data_bounds(&spec.traces);
        let layout = compute_layout(
            x_min, x_max, y_min, y_max,
            spec.title.is_some(), spec.xlabel.is_some(), spec.ylabel.is_some(),
        );

        let ct = CoordTransform {
            x_min, x_max, y_min, y_max,
            plot_left: layout.plot_left(),
            plot_right: layout.plot_right(spec.width),
            plot_top: layout.plot_top(),
            plot_bottom: layout.plot_bottom(spec.height),
        };

        if spec.grid {
            draw_grid(&mut pixmap, &layout, &ct, spec.width, spec.height);
        }
        draw_axes(&mut pixmap, &layout, spec.width, spec.height);
        draw_ticks(&mut pixmap, &layout, &ct, spec.width, spec.height);

        for trace in &spec.traces {
            match trace {
                Trace::Line { x, y, color, linewidth } => draw_line(&mut pixmap, x, y, *color, *linewidth, &ct),
                Trace::Scatter { x, y, color, size } => draw_scatter(&mut pixmap, x, y, *color, *size, &ct),
                Trace::Imshow { data, rows, cols, vmin, vmax, cmap, alpha, origin_lower } => draw_imshow(&mut pixmap, data, *rows, *cols, *vmin, *vmax, cmap, *alpha, *origin_lower, &ct),
            }
        }

        if let Some(ref title) = spec.title {
            let cx = (layout.plot_left() + layout.plot_right(spec.width)) / 2.0;
            text::draw_text_centered(&mut pixmap, title, cx, layout.plot_top() - 12.0, 18.0, [0, 0, 0, 255]);
        }
        if let Some(ref xlabel) = spec.xlabel {
            let cx = (layout.plot_left() + layout.plot_right(spec.width)) / 2.0;
            text::draw_text_centered(&mut pixmap, xlabel, cx, layout.plot_bottom(spec.height) + 45.0, 14.0, [0, 0, 0, 255]);
        }
        if let Some(ref ylabel) = spec.ylabel {
            let cy = (layout.plot_top() + layout.plot_bottom(spec.height)) / 2.0;
            text::draw_text_rotated(&mut pixmap, ylabel, 18.0, cy, 14.0, [0, 0, 0, 255]);
        }
    }

    // Draw overlays on top of everything
    for overlay in &spec.overlays {
        match overlay {
            Overlay::Circle { x, y, radius, color, filled, linewidth } => {
                draw_overlay_circle(&mut pixmap, *x, *y, *radius, *color, *filled, *linewidth);
            }
            Overlay::Crosshair { x, y, size, gap, color, linewidth } => {
                draw_overlay_crosshair(&mut pixmap, *x, *y, *size, *gap, *color, *linewidth);
            }
            Overlay::Polyline { xs, ys, color, linewidth, dashed } => {
                draw_overlay_polyline(&mut pixmap, xs, ys, *color, *linewidth, *dashed);
            }
            Overlay::Rect { x, y, w, h, color, filled, linewidth } => {
                draw_overlay_rect(&mut pixmap, *x, *y, *w, *h, *color, *filled, *linewidth);
            }
            Overlay::Polygon { xs, ys, color, linewidth, filled } => {
                draw_overlay_polygon(&mut pixmap, xs, ys, *color, *linewidth, *filled);
            }
            Overlay::Label { x, y, text, font_size, color, bg_color, rotation, stroke } => {
                draw_overlay_label(&mut pixmap, *x, *y, text, *font_size, *color, *bg_color, *rotation, *stroke);
            }
        }
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
                for &v in x { if v < x_min { x_min = v; } if v > x_max { x_max = v; } }
                for &v in y { if v < y_min { y_min = v; } if v > y_max { y_max = v; } }
            }
            Trace::Imshow { rows, cols, .. } => {
                if 0.0 < x_min { x_min = 0.0; }
                if (*cols as f64) > x_max { x_max = *cols as f64; }
                if 0.0 < y_min { y_min = 0.0; }
                if (*rows as f64) > y_max { y_max = *rows as f64; }
            }
        }
    }

    let has_imshow = traces.iter().any(|t| matches!(t, Trace::Imshow { .. }));
    if has_imshow {
        return (x_min, x_max, y_min, y_max);
    }

    let x_pad = (x_max - x_min).abs() * 0.05;
    let y_pad = (y_max - y_min).abs() * 0.05;
    let x_pad = if x_pad < 1e-12 { 1.0 } else { x_pad };
    let y_pad = if y_pad < 1e-12 { 1.0 } else { y_pad };
    (x_min - x_pad, x_max + x_pad, y_min - y_pad, y_max + y_pad)
}

// ---------------------------------------------------------------------------
// Axes drawing (normal mode only)
// ---------------------------------------------------------------------------

fn draw_axes(pixmap: &mut Pixmap, layout: &Layout, width: u32, height: u32) {
    let paint = paint_from_rgba([0, 0, 0, 255]);
    let mut stroke = Stroke::default();
    stroke.width = 1.5;

    let left = layout.plot_left();
    let right = layout.plot_right(width);
    let top = layout.plot_top();
    let bottom = layout.plot_bottom(height);

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

fn draw_ticks(pixmap: &mut Pixmap, layout: &Layout, ct: &CoordTransform, width: u32, height: u32) {
    let paint = paint_from_rgba([0, 0, 0, 255]);
    let mut stroke = Stroke::default();
    stroke.width = 1.0;

    let left = layout.plot_left();
    let right = layout.plot_right(width);
    let top = layout.plot_top();
    let bottom = layout.plot_bottom(height);
    let tick_len = 6.0;

    let x_step = if layout.x_ticks.len() >= 2 { (layout.x_ticks[1] - layout.x_ticks[0]).abs() } else { 1.0 };
    let y_step = if layout.y_ticks.len() >= 2 { (layout.y_ticks[1] - layout.y_ticks[0]).abs() } else { 1.0 };

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

// ---------------------------------------------------------------------------
// Trace drawing
// ---------------------------------------------------------------------------

fn draw_line(pixmap: &mut Pixmap, x: &[f64], y: &[f64], color: [u8; 4], linewidth: f32, ct: &CoordTransform) {
    if x.len() < 2 { return; }
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

fn draw_scatter(pixmap: &mut Pixmap, x: &[f64], y: &[f64], color: [u8; 4], size: f32, ct: &CoordTransform) {
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
    pixmap: &mut Pixmap, data: &[f64], rows: usize, cols: usize,
    vmin: f64, vmax: f64, cmap: &str, alpha: f32, origin_lower: bool,
    ct: &CoordTransform,
) {
    let range = if (vmax - vmin).abs() < 1e-12 { 1.0 } else { vmax - vmin };
    let alpha_u8 = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
    let is_opaque = alpha_u8 == 255;

    for row in 0..rows {
        for col in 0..cols {
            let val = data[row * cols + col];
            if !val.is_finite() { continue; } // NaN/Inf → transparent

            let t = ((val - vmin) / range).clamp(0.0, 1.0);
            let mut color = colormap(cmap, t);
            color[3] = alpha_u8;

            // Row mapping: origin_lower means row 0 is at the bottom
            let data_row = if origin_lower { row } else { rows - 1 - row };
            let (px1, py1) = ct.data_to_pixel(col as f64, (data_row + 1) as f64);
            let (px2, py2) = ct.data_to_pixel((col + 1) as f64, data_row as f64);

            let left = px1.min(px2);
            let top = py1.min(py2);
            let w = (px2 - px1).abs();
            let h = (py2 - py1).abs();

            if is_opaque {
                if let Some(rect) = Rect::from_xywh(left, top, w.max(0.5), h.max(0.5)) {
                    let paint = paint_from_rgba(color);
                    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                }
            } else {
                // Alpha-blended: write directly to pixel buffer
                let px_x = left as i32;
                let px_y = top as i32;
                let pw = pixmap.width() as i32;
                let ph = pixmap.height() as i32;
                let w_i = w.ceil() as i32;
                let h_i = h.ceil() as i32;
                let pdata = pixmap.data_mut();
                let a = alpha_u8 as u16;
                let inv = 255 - a;
                for dy in 0..h_i {
                    for dx in 0..w_i {
                        let fx = px_x + dx;
                        let fy = px_y + dy;
                        if fx >= 0 && fy >= 0 && fx < pw && fy < ph {
                            let idx = (fy as usize * pw as usize + fx as usize) * 4;
                            pdata[idx]     = ((color[0] as u16 * a + pdata[idx] as u16 * inv) / 255) as u8;
                            pdata[idx + 1] = ((color[1] as u16 * a + pdata[idx + 1] as u16 * inv) / 255) as u8;
                            pdata[idx + 2] = ((color[2] as u16 * a + pdata[idx + 2] as u16 * inv) / 255) as u8;
                            pdata[idx + 3] = 255;
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Overlay drawing
// ---------------------------------------------------------------------------

fn draw_overlay_circle(pixmap: &mut Pixmap, x: f32, y: f32, radius: f32, color: [u8; 4], filled: bool, linewidth: f32) {
    let mut pb = PathBuilder::new();
    pb.push_circle(x, y, radius);
    if let Some(path) = pb.finish() {
        if filled {
            let paint = paint_from_rgba(color);
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        } else {
            // Black outline for contrast
            if linewidth >= 1.0 {
                let outline_paint = paint_from_rgba([0, 0, 0, color[3] / 2]);
                let mut outline_stroke = Stroke::default();
                outline_stroke.width = linewidth + 2.0;
                pixmap.stroke_path(&path, &outline_paint, &outline_stroke, Transform::identity(), None);
            }
            let paint = paint_from_rgba(color);
            let mut stroke = Stroke::default();
            stroke.width = linewidth;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn draw_overlay_crosshair(pixmap: &mut Pixmap, x: f32, y: f32, size: f32, gap: f32, color: [u8; 4], linewidth: f32) {
    let arms: [(f32, f32, f32, f32); 4] = [
        (x, y - size, x, y - gap),  // top
        (x, y + gap, x, y + size),  // bottom
        (x - size, y, x - gap, y),  // left
        (x + gap, y, x + size, y),  // right
    ];

    for (x1, y1, x2, y2) in &arms {
        if let Some(path) = line_path(*x1, *y1, *x2, *y2) {
            // Black outline
            let outline_paint = paint_from_rgba([0, 0, 0, color[3] / 2]);
            let mut outline_stroke = Stroke::default();
            outline_stroke.width = linewidth + 2.0;
            outline_stroke.line_cap = LineCap::Round;
            pixmap.stroke_path(&path, &outline_paint, &outline_stroke, Transform::identity(), None);

            // Colored stroke
            let paint = paint_from_rgba(color);
            let mut stroke = Stroke::default();
            stroke.width = linewidth;
            stroke.line_cap = LineCap::Round;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn draw_overlay_polyline(pixmap: &mut Pixmap, xs: &[f32], ys: &[f32], color: [u8; 4], linewidth: f32, dashed: bool) {
    if xs.len() < 2 { return; }
    let mut pb = PathBuilder::new();
    pb.move_to(xs[0], ys[0]);
    for i in 1..xs.len().min(ys.len()) {
        pb.line_to(xs[i], ys[i]);
    }
    if let Some(path) = pb.finish() {
        let paint = paint_from_rgba(color);
        let mut stroke = Stroke::default();
        stroke.width = linewidth;
        stroke.line_cap = LineCap::Round;
        stroke.line_join = LineJoin::Round;
        if dashed {
            stroke.dash = StrokeDash::new(vec![8.0, 5.0], 0.0);
        }
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn draw_overlay_rect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: [u8; 4], filled: bool, linewidth: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x, y);
    pb.line_to(x + w, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    if let Some(path) = pb.finish() {
        if filled {
            let paint = paint_from_rgba(color);
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        } else {
            let paint = paint_from_rgba(color);
            let mut stroke = Stroke::default();
            stroke.width = linewidth;
            stroke.line_join = LineJoin::Miter;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn draw_overlay_polygon(pixmap: &mut Pixmap, xs: &[f32], ys: &[f32], color: [u8; 4], linewidth: f32, filled: bool) {
    if xs.len() < 3 { return; }
    let mut pb = PathBuilder::new();
    pb.move_to(xs[0], ys[0]);
    for i in 1..xs.len().min(ys.len()) {
        pb.line_to(xs[i], ys[i]);
    }
    pb.close();
    if let Some(path) = pb.finish() {
        if filled {
            let paint = paint_from_rgba(color);
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        } else {
            // Outline stroke for contrast
            let outline_paint = paint_from_rgba([0, 0, 0, color[3] / 2]);
            let mut outline_stroke = Stroke::default();
            outline_stroke.width = linewidth + 2.0;
            outline_stroke.line_join = LineJoin::Miter;
            pixmap.stroke_path(&path, &outline_paint, &outline_stroke, Transform::identity(), None);

            let paint = paint_from_rgba(color);
            let mut stroke = Stroke::default();
            stroke.width = linewidth;
            stroke.line_join = LineJoin::Miter;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn draw_overlay_label(
    pixmap: &mut Pixmap, x: f32, y: f32, label_text: &str,
    font_size: f32, color: [u8; 4], bg_color: Option<[u8; 4]>, _rotation: f32,
    stroke: bool,
) {
    let tw = text::measure_text(label_text, font_size);
    let pad = 3.0;

    // Background box
    if let Some(bg) = bg_color {
        let bx = x - pad;
        let by = y - font_size - pad;
        let bw = tw + pad * 2.0;
        let bh = font_size + pad * 2.0;
        if let Some(rect) = Rect::from_xywh(bx, by, bw, bh) {
            let paint = paint_from_rgba(bg);
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }

    // Text stroke outline for contrast (draw black text underneath)
    if stroke {
        text::draw_text(pixmap, label_text, x - 1.0, y, font_size, [0, 0, 0, color[3]]);
        text::draw_text(pixmap, label_text, x + 1.0, y, font_size, [0, 0, 0, color[3]]);
        text::draw_text(pixmap, label_text, x, y - 1.0, font_size, [0, 0, 0, color[3]]);
        text::draw_text(pixmap, label_text, x, y + 1.0, font_size, [0, 0, 0, color[3]]);
    }

    text::draw_text(pixmap, label_text, x, y, font_size, color);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn line_path(x1: f32, y1: f32, x2: f32, y2: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);
    pb.finish()
}
