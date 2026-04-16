use crate::reporting::metrics_table::Direction;
use crate::reporting::ReportComponent;
use serde::Serialize;
use std::fmt::Write as FmtWrite;

/// A single step in a [`WaterfallData`] visualization.
#[derive(Debug, Clone, Serialize)]
pub struct WaterfallStep {
    /// Factor or category label.
    pub label: String,
    /// Signed contribution value.
    pub value: f64,
    /// Running total after this step.
    pub cumulative: f64,
    /// Directional annotation.
    pub direction: Direction,
    /// Absolute contribution as a fraction of total change.
    pub pct_of_total: f64,
}

/// Ordered steps for waterfall chart rendering.
///
/// Each step represents a factor's contribution to the total P&L change.
/// Includes a residual term for any unexplained portion.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::{WaterfallData, ReportComponent};
///
/// let waterfall = WaterfallData::from_attribution(
///     "P&L Attribution",
///     "USD",
///     1_000_000.0,
///     1_050_000.0,
///     &[
///         ("Rates".to_string(), 30_000.0),
///         ("Credit".to_string(), 25_000.0),
///         ("FX".to_string(), -5_000.0),
///     ],
/// );
///
/// assert_eq!(waterfall.steps.len(), 3);
/// assert!((waterfall.total_change - 50_000.0).abs() < 1e-10);
/// assert!((waterfall.residual - 0.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct WaterfallData {
    /// Display title.
    pub title: String,
    /// Currency code.
    pub currency: String,
    /// Opening value (start of waterfall).
    pub start_value: f64,
    /// Closing value (end of waterfall).
    pub end_value: f64,
    /// Ordered attribution steps.
    pub steps: Vec<WaterfallStep>,
    /// Unexplained portion (total_change - sum of steps).
    pub residual: f64,
    /// `end_value - start_value`.
    pub total_change: f64,
}

impl WaterfallData {
    /// Build from attribution factor contributions.
    ///
    /// Steps are ordered as specified in the input. A residual is computed
    /// as the difference between the total change and the sum of all
    /// factor contributions.
    pub fn from_attribution(
        title: impl Into<String>,
        currency: impl Into<String>,
        start_value: f64,
        end_value: f64,
        factor_contributions: &[(String, f64)],
    ) -> Self {
        let title = title.into();
        let currency = currency.into();
        let total_change = end_value - start_value;
        let abs_total = total_change.abs();

        let mut cumulative = start_value;
        let mut steps = Vec::with_capacity(factor_contributions.len());

        for (label, &value) in factor_contributions
            .iter()
            .map(|(label, value)| (label, value))
        {
            cumulative += value;
            let direction = if value > 0.0 {
                Direction::Positive
            } else if value < 0.0 {
                Direction::Negative
            } else {
                Direction::Neutral
            };
            let pct_of_total = if abs_total > 0.0 {
                value.abs() / abs_total
            } else {
                0.0
            };

            steps.push(WaterfallStep {
                label: label.clone(),
                value,
                cumulative,
                direction,
                pct_of_total,
            });
        }

        let explained: f64 = factor_contributions.iter().map(|(_, v)| v).sum();
        let residual = total_change - explained;

        Self {
            title,
            currency,
            start_value,
            end_value,
            steps,
            residual,
            total_change,
        }
    }
}

impl ReportComponent for WaterfallData {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    #[allow(clippy::expect_used)]
    fn to_markdown(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "## {}\n", self.title).expect("writing to String cannot fail");
        writeln!(
            &mut out,
            "**Start**: {:.2} {} | **End**: {:.2} {} | **Change**: {:.2} {}\n",
            self.start_value,
            self.currency,
            self.end_value,
            self.currency,
            self.total_change,
            self.currency,
        )
        .expect("writing to String cannot fail");

        writeln!(
            &mut out,
            "| Factor | Contribution | Cumulative | % of Total | Direction |"
        )
        .expect("writing to String cannot fail");
        writeln!(
            &mut out,
            "|:-------|-------------:|-----------:|-----------:|:----------|"
        )
        .expect("writing to String cannot fail");

        for step in &self.steps {
            let dir_str = match step.direction {
                Direction::Positive => "positive",
                Direction::Negative => "negative",
                Direction::Neutral => "neutral",
            };
            writeln!(
                &mut out,
                "| {} | {:.2} | {:.2} | {:.1}% | {} |",
                step.label,
                step.value,
                step.cumulative,
                step.pct_of_total * 100.0,
                dir_str,
            )
            .expect("writing to String cannot fail");
        }

        if self.residual.abs() > 1e-10 {
            writeln!(&mut out, "| *Residual* | {:.2} | | | |", self.residual)
                .expect("writing to String cannot fail");
        }

        out
    }

    fn component_type(&self) -> &'static str {
        "waterfall_data"
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn sample_waterfall() -> WaterfallData {
        WaterfallData::from_attribution(
            "P&L Attribution",
            "USD",
            1_000_000.0,
            1_050_000.0,
            &[
                ("Rates".to_string(), 30_000.0),
                ("Credit".to_string(), 25_000.0),
                ("FX".to_string(), -5_000.0),
            ],
        )
    }

    #[test]
    fn basic_construction() {
        let w = sample_waterfall();
        assert_eq!(w.steps.len(), 3);
        assert!((w.total_change - 50_000.0).abs() < 1e-10);
        assert!((w.start_value - 1_000_000.0).abs() < 1e-10);
        assert!((w.end_value - 1_050_000.0).abs() < 1e-10);
    }

    #[test]
    fn residual_zero_when_explained() {
        let w = sample_waterfall();
        assert!((w.residual).abs() < 1e-10);
    }

    #[test]
    fn residual_nonzero_when_unexplained() {
        let w = WaterfallData::from_attribution(
            "Partial",
            "USD",
            100.0,
            200.0,
            &[("Factor A".to_string(), 80.0)],
        );
        assert!((w.residual - 20.0).abs() < 1e-10);
    }

    #[test]
    fn cumulative_tracking() {
        let w = sample_waterfall();
        // Start: 1_000_000
        // After Rates (+30k): 1_030_000
        // After Credit (+25k): 1_055_000
        // After FX (-5k): 1_050_000
        assert!((w.steps[0].cumulative - 1_030_000.0).abs() < 1e-10);
        assert!((w.steps[1].cumulative - 1_055_000.0).abs() < 1e-10);
        assert!((w.steps[2].cumulative - 1_050_000.0).abs() < 1e-10);
    }

    #[test]
    fn pct_of_total() {
        let w = sample_waterfall();
        // Rates: |30000| / |50000| = 0.6
        assert!((w.steps[0].pct_of_total - 0.6).abs() < 1e-10);
        // Credit: |25000| / |50000| = 0.5
        assert!((w.steps[1].pct_of_total - 0.5).abs() < 1e-10);
        // FX: |-5000| / |50000| = 0.1
        assert!((w.steps[2].pct_of_total - 0.1).abs() < 1e-10);
    }

    #[test]
    fn direction_assignment() {
        let w = sample_waterfall();
        assert_eq!(w.steps[0].direction, Direction::Positive);
        assert_eq!(w.steps[1].direction, Direction::Positive);
        assert_eq!(w.steps[2].direction, Direction::Negative);
    }

    #[test]
    fn zero_total_change() {
        let w = WaterfallData::from_attribution(
            "Flat",
            "EUR",
            100.0,
            100.0,
            &[("A".to_string(), 10.0), ("B".to_string(), -10.0)],
        );
        assert!((w.total_change).abs() < 1e-15);
        // pct_of_total should be 0 when total_change is 0
        assert!((w.steps[0].pct_of_total).abs() < 1e-15);
    }

    #[test]
    fn negative_start_value() {
        let w = WaterfallData::from_attribution(
            "Negative",
            "USD",
            -100.0,
            -50.0,
            &[("Gain".to_string(), 50.0)],
        );
        assert!((w.total_change - 50.0).abs() < 1e-10);
        assert!((w.steps[0].cumulative - (-50.0)).abs() < 1e-10);
    }

    #[test]
    fn single_step() {
        let w = WaterfallData::from_attribution(
            "Single",
            "USD",
            0.0,
            100.0,
            &[("Only".to_string(), 100.0)],
        );
        assert_eq!(w.steps.len(), 1);
        assert!((w.steps[0].pct_of_total - 1.0).abs() < 1e-10);
    }

    #[test]
    fn empty_steps() {
        let w = WaterfallData::from_attribution("Empty", "USD", 100.0, 200.0, &[]);
        assert_eq!(w.steps.len(), 0);
        assert!((w.residual - 100.0).abs() < 1e-10);
    }

    #[test]
    fn to_json_structure() {
        let w = sample_waterfall();
        let json = w.to_json();

        assert_eq!(json["title"], "P&L Attribution");
        assert_eq!(json["currency"], "USD");
        assert!(json["steps"].is_array());
        assert_eq!(json["steps"].as_array().expect("should be array").len(), 3);
        assert_eq!(json["steps"][0]["label"], "Rates");
    }

    #[test]
    fn to_markdown_format() {
        let w = sample_waterfall();
        let md = w.to_markdown();

        assert!(md.contains("## P&L Attribution"));
        assert!(md.contains("**Start**"));
        assert!(md.contains("| Factor |"));
        assert!(md.contains("Rates"));
        assert!(md.contains("Credit"));
        assert!(md.contains("FX"));
    }

    #[test]
    fn markdown_shows_residual() {
        let w = WaterfallData::from_attribution(
            "Partial",
            "USD",
            100.0,
            200.0,
            &[("Factor A".to_string(), 80.0)],
        );
        let md = w.to_markdown();
        assert!(md.contains("Residual"));
    }

    #[test]
    fn markdown_hides_zero_residual() {
        let w = sample_waterfall();
        let md = w.to_markdown();
        assert!(!md.contains("Residual"));
    }

    #[test]
    fn component_type_name() {
        let w = sample_waterfall();
        assert_eq!(w.component_type(), "waterfall_data");
    }
}
