use fontdue::{Font, FontSettings};
use std::sync::OnceLock;
use tiny_skia::Pixmap;

static FONT: OnceLock<Font> = OnceLock::new();

fn get_font() -> &'static Font {
    FONT.get_or_init(|| {
        let font_data = include_bytes!("../assets/DejaVuSans.ttf");
        Font::from_bytes(font_data as &[u8], FontSettings::default())
            .expect("failed to parse embedded font")
    })
}

/// Measure the width of a string at the given font size.
pub fn measure_text(text: &str, size: f32) -> f32 {
    let font = get_font();
    text.chars()
        .map(|ch| font.rasterize(ch, size).0.advance_width)
        .sum()
}

/// Draw a horizontal text string onto the pixmap.
/// `y` is the baseline position (screen coordinates, y increases downward).
pub fn draw_text(pixmap: &mut Pixmap, text: &str, x: f32, y: f32, size: f32, color: [u8; 4]) {
    let font = get_font();
    let mut cursor_x = x;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        if !bitmap.is_empty() {
            // xmin: horizontal bearing from cursor to left edge of glyph
            // ymin: distance from baseline to bottom of glyph (positive = above baseline)
            // In screen coords: bitmap top = baseline - ymin - height
            let gx = cursor_x + metrics.xmin as f32;
            let gy = y - (metrics.ymin + metrics.height as i32) as f32;
            blit_glyph(pixmap, &bitmap, metrics.width, metrics.height, gx, gy, color);
        }
        cursor_x += metrics.advance_width;
    }
}

/// Draw text centered horizontally at (cx, y) where y is the baseline.
pub fn draw_text_centered(
    pixmap: &mut Pixmap,
    text: &str,
    cx: f32,
    y: f32,
    size: f32,
    color: [u8; 4],
) {
    let w = measure_text(text, size);
    draw_text(pixmap, text, cx - w / 2.0, y, size, color);
}

/// Draw text rotated 90 degrees counter-clockwise, centered at (cx, cy).
/// Renders text to a temporary buffer, rotates the whole thing, then blits.
pub fn draw_text_rotated(
    pixmap: &mut Pixmap,
    text: &str,
    cx: f32,
    cy: f32,
    size: f32,
    color: [u8; 4],
) {
    let font = get_font();

    // First pass: measure total dimensions of the horizontal text
    let total_width = measure_text(text, size) as u32 + 2;

    // Compute ascent/descent to size the temp buffer
    let mut ascent: i32 = 0;
    let mut descent: i32 = 0;
    for ch in text.chars() {
        let (metrics, _) = font.rasterize(ch, size);
        let top = metrics.ymin + metrics.height as i32; // above baseline
        let bottom = metrics.ymin; // below baseline if negative
        if top > ascent {
            ascent = top;
        }
        if bottom < descent {
            descent = bottom;
        }
    }

    let total_height = (ascent - descent) as u32 + 2;
    if total_width == 0 || total_height == 0 {
        return;
    }

    // Render text horizontally into a coverage buffer (single channel)
    let buf_w = total_width as usize;
    let buf_h = total_height as usize;
    let mut buf = vec![0u8; buf_w * buf_h];

    let baseline_y = ascent as f32 + 1.0; // baseline position in buffer
    let mut cursor_x: f32 = 1.0;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        if !bitmap.is_empty() {
            let gx = (cursor_x + metrics.xmin as f32) as i32;
            let gy = (baseline_y - (metrics.ymin + metrics.height as i32) as f32) as i32;
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let dx = gx + col as i32;
                    let dy = gy + row as i32;
                    if dx >= 0 && dy >= 0 && (dx as usize) < buf_w && (dy as usize) < buf_h {
                        let src = bitmap[row * metrics.width + col];
                        let dst = &mut buf[dy as usize * buf_w + dx as usize];
                        // Max blend for overlapping glyphs
                        *dst = (*dst).max(src);
                    }
                }
            }
        }
        cursor_x += metrics.advance_width;
    }

    // Rotate buffer 90 degrees CCW: (x, y) -> (y, new_w - 1 - x)
    // new dimensions: width = buf_h, height = buf_w
    let rot_w = buf_h;
    let rot_h = buf_w;
    let mut rotated = vec![0u8; rot_w * rot_h];
    for y in 0..buf_h {
        for x in 0..buf_w {
            let rx = y;
            let ry = buf_w - 1 - x;
            rotated[ry * rot_w + rx] = buf[y * buf_w + x];
        }
    }

    // Blit rotated buffer centered at (cx, cy)
    let blit_x = cx - rot_w as f32 / 2.0;
    let blit_y = cy - rot_h as f32 / 2.0;
    blit_glyph(pixmap, &rotated, rot_w, rot_h, blit_x, blit_y, color);
}

fn blit_glyph(
    pixmap: &mut Pixmap,
    bitmap: &[u8],
    w: usize,
    h: usize,
    x: f32,
    y: f32,
    color: [u8; 4],
) {
    let px = x as i32;
    let py = y as i32;
    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;
    let data = pixmap.data_mut();

    for row in 0..h {
        for col in 0..w {
            let coverage = bitmap[row * w + col];
            if coverage == 0 {
                continue;
            }
            let dx = px + col as i32;
            let dy = py + row as i32;
            if dx < 0 || dy < 0 || dx >= pw || dy >= ph {
                continue;
            }
            let idx = (dy as usize * pw as usize + dx as usize) * 4;
            let alpha = (coverage as u16 * color[3] as u16) / 255;
            let inv = 255 - alpha;
            data[idx] = ((color[0] as u16 * alpha + data[idx] as u16 * inv) / 255) as u8;
            data[idx + 1] =
                ((color[1] as u16 * alpha + data[idx + 1] as u16 * inv) / 255) as u8;
            data[idx + 2] =
                ((color[2] as u16 * alpha + data[idx + 2] as u16 * inv) / 255) as u8;
            data[idx + 3] =
                (data[idx + 3] as u16 + alpha - (data[idx + 3] as u16 * alpha) / 255) as u8;
        }
    }
}
