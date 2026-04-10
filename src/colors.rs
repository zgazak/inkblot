use tiny_skia::{Color, Paint};

pub fn paint_from_rgba(rgba: [u8; 4]) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]));
    paint.anti_alias = true;
    paint
}

/// Look up a colormap by name. `t` is clamped to [0, 1].
pub fn colormap(name: &str, t: f64) -> [u8; 4] {
    match name {
        "gray" | "grayscale" | "grey" => grayscale(t),
        "viridis" => interpolate_lut(t, &VIRIDIS),
        "inferno" => interpolate_lut(t, &INFERNO),
        "RdYlGn" | "rdylgn" => interpolate_lut(t, &RDYLGN),
        "hot" => interpolate_lut(t, &HOT),
        _ => interpolate_lut(t, &VIRIDIS), // default fallback
    }
}

fn grayscale(t: f64) -> [u8; 4] {
    let v = (t.clamp(0.0, 1.0) * 255.0) as u8;
    [v, v, v, 255]
}

fn interpolate_lut(t: f64, stops: &[(f64, [u8; 3])]) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let mut i = 0;
    while i < stops.len() - 2 && stops[i + 1].0 < t {
        i += 1;
    }
    let (t0, c0) = stops[i];
    let (t1, c1) = stops[i + 1];
    let f = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
    [
        (c0[0] as f64 + (c1[0] as f64 - c0[0] as f64) * f) as u8,
        (c0[1] as f64 + (c1[1] as f64 - c0[1] as f64) * f) as u8,
        (c0[2] as f64 + (c1[2] as f64 - c0[2] as f64) * f) as u8,
        255,
    ]
}

const VIRIDIS: [(f64, [u8; 3]); 9] = [
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

const INFERNO: [(f64, [u8; 3]); 9] = [
    (0.000, [0, 0, 4]),
    (0.125, [40, 11, 84]),
    (0.250, [101, 21, 110]),
    (0.375, [159, 42, 99]),
    (0.500, [212, 72, 66]),
    (0.625, [245, 125, 21]),
    (0.750, [250, 193, 39]),
    (0.875, [252, 237, 105]),
    (1.000, [252, 255, 164]),
];

const RDYLGN: [(f64, [u8; 3]); 9] = [
    (0.000, [165, 0, 38]),
    (0.125, [215, 48, 39]),
    (0.250, [244, 109, 67]),
    (0.375, [253, 174, 97]),
    (0.500, [255, 255, 191]),
    (0.625, [166, 217, 106]),
    (0.750, [102, 189, 99]),
    (0.875, [26, 152, 80]),
    (1.000, [0, 104, 55]),
];

const HOT: [(f64, [u8; 3]); 5] = [
    (0.000, [0, 0, 0]),
    (0.333, [230, 0, 0]),
    (0.666, [255, 210, 0]),
    (0.833, [255, 255, 255]),
    (1.000, [255, 255, 255]),
];
