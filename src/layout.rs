/// Layout computation: margins, tick positions, plot area.

pub struct Layout {
    pub margin_left: f32,
    pub margin_right: f32,
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub x_ticks: Vec<f64>,
    pub y_ticks: Vec<f64>,
}

impl Layout {
    pub fn plot_left(&self) -> f32 {
        self.margin_left
    }
    pub fn plot_right(&self, width: u32) -> f32 {
        width as f32 - self.margin_right
    }
    pub fn plot_top(&self) -> f32 {
        self.margin_top
    }
    pub fn plot_bottom(&self, height: u32) -> f32 {
        height as f32 - self.margin_bottom
    }
}

pub fn compute_layout(
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    has_title: bool,
    has_xlabel: bool,
    has_ylabel: bool,
) -> Layout {
    let margin_left = if has_ylabel { 80.0 } else { 60.0 };
    let margin_right = 30.0;
    let margin_top = if has_title { 50.0 } else { 25.0 };
    let margin_bottom = if has_xlabel { 65.0 } else { 45.0 };

    let x_ticks = nice_ticks(x_min, x_max, 6);
    let y_ticks = nice_ticks(y_min, y_max, 5);

    Layout {
        margin_left,
        margin_right,
        margin_top,
        margin_bottom,
        x_ticks,
        y_ticks,
    }
}

/// Heckbert "nice numbers" algorithm for clean tick values.
pub fn nice_ticks(min_val: f64, max_val: f64, target_count: usize) -> Vec<f64> {
    if (max_val - min_val).abs() < 1e-12 {
        return vec![min_val];
    }

    let range = nice_num(max_val - min_val, false);
    let step = nice_num(range / target_count as f64, true);

    let start = (min_val / step).floor() * step;
    let stop = (max_val / step).ceil() * step;

    let mut ticks = Vec::new();
    let mut v = start;
    while v <= stop + step * 0.5e-6 {
        ticks.push(v);
        v += step;
    }
    ticks
}

fn nice_num(x: f64, round: bool) -> f64 {
    let exp = x.log10().floor();
    let frac = x / 10.0_f64.powf(exp);
    let nice = if round {
        if frac < 1.5 {
            1.0
        } else if frac < 3.0 {
            2.0
        } else if frac < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if frac <= 1.0 {
        1.0
    } else if frac <= 2.0 {
        2.0
    } else if frac <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * 10.0_f64.powf(exp)
}

/// Format a tick value nicely (remove trailing zeros).
pub fn format_tick(value: f64, step: f64) -> String {
    if step >= 1.0 && value == value.round() {
        format!("{}", value as i64)
    } else {
        // Determine decimal places from step size
        let decimals = (-step.log10().floor()).max(0.0) as usize + 1;
        format!("{:.prec$}", value, prec = decimals)
    }
}
