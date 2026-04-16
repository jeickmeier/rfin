use crate::factor_model::sensitivity::SensitivityMatrix;
use crate::reporting::metrics_table::MetricUnit;
use crate::reporting::ReportComponent;
use serde::Serialize;
use std::fmt::Write as FmtWrite;

/// 2D grid of position x factor sensitivities, suitable for heatmap
/// or pivot-table rendering.
///
/// Wraps the existing [`SensitivityMatrix`] from the factor model module,
/// adding row/column totals and a grand total for summary views.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::{SensitivityGrid, MetricUnit, ReportComponent};
/// use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;
/// use finstack_core::factor_model::FactorId;
///
/// let mut matrix = SensitivityMatrix::zeros(
///     vec!["pos-1".into(), "pos-2".into()],
///     vec![FactorId::new("1Y"), FactorId::new("5Y")],
/// );
/// matrix.set_delta(0, 0, 100.0);
/// matrix.set_delta(0, 1, 200.0);
/// matrix.set_delta(1, 0, -50.0);
/// matrix.set_delta(1, 1, 150.0);
///
/// let grid = SensitivityGrid::from_sensitivity_matrix(
///     &matrix,
///     "DV01 by Tenor",
///     MetricUnit::CurrencyPerBp,
/// );
///
/// assert_eq!(grid.row_labels.len(), 2);
/// assert_eq!(grid.col_labels.len(), 2);
/// assert!((grid.grand_total - 400.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct SensitivityGrid {
    /// Display title for the grid.
    pub title: String,
    /// Position identifiers (row labels).
    pub row_labels: Vec<String>,
    /// Factor identifiers (column labels, e.g., tenor buckets).
    pub col_labels: Vec<String>,
    /// Row-major values: `values[row][col]`.
    pub values: Vec<Vec<f64>>,
    /// Sum across columns per row.
    pub row_totals: Vec<f64>,
    /// Sum across rows per column.
    pub col_totals: Vec<f64>,
    /// Sum of all values.
    pub grand_total: f64,
    /// Unit annotation for the values.
    pub unit: MetricUnit,
}

impl SensitivityGrid {
    /// Build from a [`SensitivityMatrix`].
    ///
    /// Extracts position IDs as row labels, factor IDs as column labels,
    /// and computes row totals, column totals, and a grand total.
    pub fn from_sensitivity_matrix(
        matrix: &SensitivityMatrix,
        title: impl Into<String>,
        unit: MetricUnit,
    ) -> Self {
        let n_pos = matrix.n_positions();
        let n_fac = matrix.n_factors();

        let row_labels: Vec<String> = matrix.position_ids().to_vec();
        let col_labels: Vec<String> = matrix.factor_ids().iter().map(|f| f.to_string()).collect();

        let mut values = Vec::with_capacity(n_pos);
        let mut row_totals = Vec::with_capacity(n_pos);
        let mut col_totals = vec![0.0; n_fac];

        for pos_idx in 0..n_pos {
            let row: Vec<f64> = (0..n_fac).map(|fac_idx| matrix.delta(pos_idx, fac_idx)).collect();
            let row_sum: f64 = row.iter().sum();
            for (fac_idx, &val) in row.iter().enumerate() {
                col_totals[fac_idx] += val;
            }
            row_totals.push(row_sum);
            values.push(row);
        }

        let grand_total: f64 = row_totals.iter().sum();

        Self {
            title: title.into(),
            row_labels,
            col_labels,
            values,
            row_totals,
            col_totals,
            grand_total,
            unit,
        }
    }
}

impl ReportComponent for SensitivityGrid {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    #[allow(clippy::expect_used)]
    fn to_markdown(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "## {}\n", self.title).expect("writing to String cannot fail");

        // Header row
        write!(&mut out, "| Position |").expect("writing to String cannot fail");
        for label in &self.col_labels {
            write!(&mut out, " {} |", label).expect("writing to String cannot fail");
        }
        writeln!(&mut out, " Total |").expect("writing to String cannot fail");

        // Separator
        write!(&mut out, "|:---------|").expect("writing to String cannot fail");
        for _ in &self.col_labels {
            write!(&mut out, "------:|").expect("writing to String cannot fail");
        }
        writeln!(&mut out, "------:|").expect("writing to String cannot fail");

        // Data rows
        for (i, label) in self.row_labels.iter().enumerate() {
            write!(&mut out, "| {} |", label).expect("writing to String cannot fail");
            for val in &self.values[i] {
                write!(&mut out, " {:.2} |", val).expect("writing to String cannot fail");
            }
            writeln!(&mut out, " {:.2} |", self.row_totals[i])
                .expect("writing to String cannot fail");
        }

        // Totals row
        write!(&mut out, "| **Total** |").expect("writing to String cannot fail");
        for val in &self.col_totals {
            write!(&mut out, " **{:.2}** |", val).expect("writing to String cannot fail");
        }
        writeln!(&mut out, " **{:.2}** |", self.grand_total)
            .expect("writing to String cannot fail");

        out
    }

    fn component_type(&self) -> &'static str {
        "sensitivity_grid"
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_core::factor_model::FactorId;

    fn sample_matrix() -> SensitivityMatrix {
        let mut matrix = SensitivityMatrix::zeros(
            vec!["pos-1".into(), "pos-2".into()],
            vec![FactorId::new("1Y"), FactorId::new("5Y"), FactorId::new("10Y")],
        );
        matrix.set_delta(0, 0, 100.0);
        matrix.set_delta(0, 1, 200.0);
        matrix.set_delta(0, 2, 50.0);
        matrix.set_delta(1, 0, -30.0);
        matrix.set_delta(1, 1, 150.0);
        matrix.set_delta(1, 2, 80.0);
        matrix
    }

    #[test]
    fn from_sensitivity_matrix_basic() {
        let matrix = sample_matrix();
        let grid =
            SensitivityGrid::from_sensitivity_matrix(&matrix, "DV01", MetricUnit::CurrencyPerBp);

        assert_eq!(grid.row_labels, vec!["pos-1", "pos-2"]);
        assert_eq!(grid.col_labels, vec!["1Y", "5Y", "10Y"]);
        assert_eq!(grid.values.len(), 2);
        assert_eq!(grid.values[0].len(), 3);
    }

    #[test]
    fn row_and_col_totals() {
        let matrix = sample_matrix();
        let grid =
            SensitivityGrid::from_sensitivity_matrix(&matrix, "DV01", MetricUnit::CurrencyPerBp);

        // Row totals: pos-1 = 100+200+50 = 350, pos-2 = -30+150+80 = 200
        assert!((grid.row_totals[0] - 350.0).abs() < 1e-10);
        assert!((grid.row_totals[1] - 200.0).abs() < 1e-10);

        // Col totals: 1Y = 100-30 = 70, 5Y = 200+150 = 350, 10Y = 50+80 = 130
        assert!((grid.col_totals[0] - 70.0).abs() < 1e-10);
        assert!((grid.col_totals[1] - 350.0).abs() < 1e-10);
        assert!((grid.col_totals[2] - 130.0).abs() < 1e-10);

        // Grand total = 350+200 = 550
        assert!((grid.grand_total - 550.0).abs() < 1e-10);
    }

    #[test]
    fn to_json_structure() {
        let matrix = sample_matrix();
        let grid =
            SensitivityGrid::from_sensitivity_matrix(&matrix, "DV01", MetricUnit::CurrencyPerBp);
        let json = grid.to_json();

        assert_eq!(json["title"], "DV01");
        assert!(json["row_labels"].is_array());
        assert!(json["col_labels"].is_array());
        assert!(json["values"].is_array());
        assert!(json["row_totals"].is_array());
        assert!(json["col_totals"].is_array());
    }

    #[test]
    fn to_markdown_format() {
        let matrix = sample_matrix();
        let grid =
            SensitivityGrid::from_sensitivity_matrix(&matrix, "DV01", MetricUnit::CurrencyPerBp);
        let md = grid.to_markdown();

        assert!(md.contains("## DV01"));
        assert!(md.contains("pos-1"));
        assert!(md.contains("pos-2"));
        assert!(md.contains("1Y"));
        assert!(md.contains("**Total**"));
    }

    #[test]
    fn single_position_single_factor() {
        let mut matrix =
            SensitivityMatrix::zeros(vec!["pos-1".into()], vec![FactorId::new("Rates")]);
        matrix.set_delta(0, 0, 42.0);

        let grid =
            SensitivityGrid::from_sensitivity_matrix(&matrix, "Simple", MetricUnit::Currency);

        assert_eq!(grid.row_labels.len(), 1);
        assert_eq!(grid.col_labels.len(), 1);
        assert!((grid.grand_total - 42.0).abs() < 1e-10);
    }

    #[test]
    fn zero_matrix() {
        let matrix = SensitivityMatrix::zeros(
            vec!["pos-1".into(), "pos-2".into()],
            vec![FactorId::new("A"), FactorId::new("B")],
        );
        let grid =
            SensitivityGrid::from_sensitivity_matrix(&matrix, "Zeros", MetricUnit::CurrencyPerBp);

        assert!((grid.grand_total).abs() < 1e-15);
        for t in &grid.row_totals {
            assert!(t.abs() < 1e-15);
        }
        for t in &grid.col_totals {
            assert!(t.abs() < 1e-15);
        }
    }

    #[test]
    fn component_type_name() {
        let matrix =
            SensitivityMatrix::zeros(vec!["pos-1".into()], vec![FactorId::new("Rates")]);
        let grid =
            SensitivityGrid::from_sensitivity_matrix(&matrix, "Test", MetricUnit::Currency);
        assert_eq!(grid.component_type(), "sensitivity_grid");
    }
}
