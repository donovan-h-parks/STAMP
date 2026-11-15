//! How plots name the p-values they show: uncorrected, corrected for
//! testing many features, or Storey's q-values.

use stamp_core::stats::correction::Correction;

/// Which p-values a plot shows, which sets their label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PValueKind {
    /// Not corrected for multiple tests, either because no correction was
    /// chosen or because the plot shows the values before correction.
    #[default]
    Uncorrected,
    /// Corrected by a method other than Storey's.
    Corrected,
    /// Storey's q-values.
    QValue,
}

impl PValueKind {
    /// The kind of the values a correction produces.
    pub fn of(correction: Correction) -> PValueKind {
        match correction {
            Correction::None => PValueKind::Uncorrected,
            Correction::Storey => PValueKind::QValue,
            _ => PValueKind::Corrected,
        }
    }

    /// The kind of the values shown by a plot that shows corrected values
    /// if `corrected`, and the values before correction otherwise.
    pub fn shown(correction: Correction, corrected: bool) -> PValueKind {
        if corrected {
            PValueKind::of(correction)
        } else {
            PValueKind::Uncorrected
        }
    }

    /// "Uncorrected p-value", "Corrected p-value" or "q-value".
    pub fn label(self) -> &'static str {
        match self {
            PValueKind::Uncorrected => "Uncorrected p-value",
            PValueKind::Corrected => "Corrected p-value",
            PValueKind::QValue => "q-value",
        }
    }

    /// The label in the plural, for axes of many features.
    pub fn plural(self) -> &'static str {
        match self {
            PValueKind::Uncorrected => "Uncorrected p-values",
            PValueKind::Corrected => "Corrected p-values",
            PValueKind::QValue => "q-values",
        }
    }
}

/// Tooltip lines for a feature's p-value and, if a correction was applied,
/// its corrected value, both already formatted.
pub fn tooltip_lines(p_value: &str, corrected: &str, kind: PValueKind) -> String {
    match kind {
        PValueKind::Uncorrected => format!("{}: {p_value}", kind.label()),
        _ => format!(
            "{}: {p_value}\n{}: {corrected}",
            PValueKind::Uncorrected.label(),
            kind.label()
        ),
    }
}

/// A p-value as written on a plot: "Corrected p-value = 0.012", or
/// "Corrected p-value < 1e-15" for a bound. The p-value of a single test
/// (`kind` `None`) is written "p = 0.012".
pub fn p_value_text(p: &str, kind: Option<PValueKind>) -> String {
    let name = kind.map_or("p", PValueKind::label);
    if p.starts_with('<') || p.starts_with('>') {
        format!("{name} {p}")
    } else {
        format!("{name} = {p}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_the_correction() {
        assert_eq!(
            PValueKind::of(Correction::None).label(),
            "Uncorrected p-value"
        );
        for c in [
            Correction::BenjaminiHochberg,
            Correction::Bonferroni,
            Correction::HolmBonferroni,
            Correction::Sidak,
        ] {
            assert_eq!(PValueKind::of(c).label(), "Corrected p-value", "{c:?}");
        }
        assert_eq!(PValueKind::of(Correction::Storey).label(), "q-value");
        // Values shown before correction are uncorrected, whatever the
        // correction.
        assert_eq!(
            PValueKind::shown(Correction::Storey, false),
            PValueKind::Uncorrected
        );
        assert_eq!(
            PValueKind::shown(Correction::Bonferroni, true),
            PValueKind::Corrected
        );
    }

    #[test]
    fn values_are_written_with_their_name() {
        assert_eq!(p_value_text("0.012", None), "p = 0.012");
        assert_eq!(
            p_value_text("< 1e-15", Some(PValueKind::Corrected)),
            "Corrected p-value < 1e-15"
        );
    }

    #[test]
    fn tooltips_show_the_corrected_value_only_if_there_is_one() {
        assert_eq!(
            tooltip_lines("0.01", "0.01", PValueKind::Uncorrected),
            "Uncorrected p-value: 0.01"
        );
        assert_eq!(
            tooltip_lines("0.01", "0.04", PValueKind::QValue),
            "Uncorrected p-value: 0.01\nq-value: 0.04"
        );
    }
}
