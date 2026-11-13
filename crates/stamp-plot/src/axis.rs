//! Axis scales, tick positions and tick labels.

/// Maps data values linearly onto a span of the figure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    pub data_min: f64,
    pub data_max: f64,
    pub start: f64,
    pub end: f64,
}

impl LinearScale {
    /// Map `data_min..data_max` onto `start..end` (which may run backwards,
    /// as for a y axis that grows upwards).
    pub fn new(data_min: f64, data_max: f64, start: f64, end: f64) -> LinearScale {
        LinearScale {
            data_min,
            data_max,
            start,
            end,
        }
    }

    pub fn map(&self, value: f64) -> f64 {
        let span = self.data_max - self.data_min;
        if span == 0.0 {
            return (self.start + self.end) / 2.0;
        }
        self.start + (value - self.data_min) / span * (self.end - self.start)
    }
}

/// A step of 1, 2 or 5 times a power of ten that divides `min..max` into at
/// most `max_intervals` intervals.
fn nice_step(min: f64, max: f64, max_intervals: usize) -> f64 {
    let range = max - min;
    if !(range.is_finite() && range > 0.0) {
        return 1.0;
    }
    let raw = range / max_intervals as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    for multiple in [1.0, 2.0, 5.0, 10.0] {
        let step = multiple * magnitude;
        // Allow for rounding when the range is an exact multiple.
        if step * max_intervals as f64 >= range * (1.0 - 1e-10) {
            return step;
        }
    }
    10.0 * magnitude
}

/// Tick positions for an axis, and its limits widened to the nearest ticks.
#[derive(Debug, Clone, PartialEq)]
pub struct Ticks {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub positions: Vec<f64>,
}

impl Ticks {
    /// Ticks at round numbers, with the axis limits widened so that the data
    /// range `min..max` ends on ticks. This is how STAMP v2's plots chose
    /// their limits.
    pub fn covering(min: f64, max: f64) -> Ticks {
        let (mut min, mut max) = if min <= max { (min, max) } else { (max, min) };
        if !(min.is_finite() && max.is_finite()) {
            (min, max) = (0.0, 1.0);
        }
        if min == max {
            // A single value: centre it in a small range.
            let pad = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
            min -= pad;
            max += pad;
        }
        let mut step = nice_step(min, max, 9);
        // Widening to tick boundaries can add intervals; use a coarser step
        // if that leaves too many.
        loop {
            let low = (min / step + 1e-10).floor() * step;
            let high = (max / step - 1e-10).ceil() * step;
            let intervals = ((high - low) / step).round() as usize;
            if intervals <= 9 {
                return Ticks::within(low, high, step);
            }
            step = nice_step(low, high, 9).max(step * 2.0);
        }
    }

    /// Ticks at multiples of `step` inside fixed limits.
    pub fn within(min: f64, max: f64, step: f64) -> Ticks {
        let first = (min / step - 1e-9).ceil() as i64;
        let last = (max / step + 1e-9).floor() as i64;
        let positions = (first..=last).map(|i| i as f64 * step).collect();
        Ticks {
            min,
            max,
            step,
            positions,
        }
    }

    /// Ticks at round numbers inside fixed limits, without widening them.
    pub fn inside(min: f64, max: f64) -> Ticks {
        Ticks::within(min, max, nice_step(min, max, 9))
    }

    /// Labels for the ticks, with just enough decimals for the step.
    pub fn labels(&self) -> Vec<String> {
        let decimals = if self.step >= 1.0 || self.step <= 0.0 {
            0
        } else {
            // 0.5 -> 1, 0.25 -> 2, 0.002 -> 3
            let mut decimals = (-self.step.log10()).ceil() as usize;
            let scaled = self.step * 10f64.powi(decimals as i32);
            if (scaled - scaled.round()).abs() > 1e-6 {
                decimals += 1;
            }
            decimals.min(12)
        };
        self.positions
            .iter()
            .map(|&p| format_tick(p, decimals))
            .collect()
    }
}

/// Format a tick value with a fixed number of decimals and a true minus sign.
pub fn format_tick(value: f64, decimals: usize) -> String {
    let value = if value == 0.0 { 0.0 } else { value }; // avoid "-0"
    let text = format!("{value:.decimals$}");
    match text.strip_prefix('-') {
        Some(rest) if rest.chars().any(|c| c != '0' && c != '.') => format!("\u{2212}{rest}"),
        Some(rest) => rest.to_string(),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_maps_both_directions() {
        let x = LinearScale::new(0.0, 10.0, 100.0, 200.0);
        assert_eq!(x.map(5.0), 150.0);
        let y = LinearScale::new(-1.0, 3.0, 400.0, 0.0);
        assert_eq!(y.map(-1.0), 400.0);
        assert_eq!(y.map(3.0), 0.0);
        assert_eq!(LinearScale::new(2.0, 2.0, 0.0, 10.0).map(2.0), 5.0);
    }

    // Limits seen in STAMP v2's own example plots.
    #[test]
    fn limits_match_stamp_v2_examples() {
        // Iron mine example: effects from about -6.5 to 10.
        let t = Ticks::covering(-6.6, 10.1);
        assert_eq!((t.min, t.max), (-10.0, 15.0));
        assert_eq!(
            t.labels(),
            ["\u{2212}10", "\u{2212}5", "0", "5", "10", "15"]
        );
        // Enterotype post-hoc example: about -6 to 37.
        let t = Ticks::covering(-5.8, 37.2);
        assert_eq!((t.min, t.max), (-10.0, 40.0));
        assert_eq!(t.labels(), ["\u{2212}10", "0", "10", "20", "30", "40"]);
    }

    #[test]
    fn small_and_degenerate_ranges() {
        let t = Ticks::covering(0.012, 0.047);
        assert_eq!((t.min, t.max), (0.01, 0.05));
        assert_eq!(
            t.labels(),
            ["0.010", "0.015", "0.020", "0.025", "0.030", "0.035", "0.040", "0.045", "0.050"]
        );
        let t = Ticks::covering(-0.3, 0.42);
        assert_eq!(
            t.labels(),
            [
                "\u{2212}0.3",
                "\u{2212}0.2",
                "\u{2212}0.1",
                "0.0",
                "0.1",
                "0.2",
                "0.3",
                "0.4",
                "0.5"
            ]
        );

        let t = Ticks::covering(5.0, 5.0);
        assert!(t.min < 5.0 && t.max > 5.0 && t.positions.len() >= 2);
        let t = Ticks::covering(f64::NAN, 1.0);
        assert_eq!((t.min, t.max), (0.0, 1.0));
        assert!(Ticks::covering(3.0, -2.0).min <= -2.0);
    }

    #[test]
    fn fixed_limits_keep_their_ends() {
        let t = Ticks::inside(-3.0, 7.5);
        assert_eq!((t.min, t.max), (-3.0, 7.5));
        assert_eq!(t.positions.first(), Some(&-2.0));
        assert_eq!(t.positions.last(), Some(&6.0));
    }

    #[test]
    fn tick_formatting() {
        assert_eq!(format_tick(-0.0, 1), "0.0");
        assert_eq!(format_tick(-0.04, 1), "0.0");
        assert_eq!(format_tick(-2.5, 1), "\u{2212}2.5");
        assert_eq!(format_tick(1e-9, 0), "0");
    }
}
