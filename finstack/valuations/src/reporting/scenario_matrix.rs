use crate::metrics::MetricId;
use crate::reporting::ReportComponent;
use crate::results::ValuationResult;
use serde::Serialize;
use std::fmt::Write as FmtWrite;

/// Matrix of scenario names x metrics.
///
/// Each cell is the metric value under that scenario. Includes an optional
/// base-case row for comparison and deltas from base case.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::{ScenarioMatrix, ReportComponent};
/// use finstack_valuations::results::ValuationResult;
/// use finstack_valuations::metrics::MetricId;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::create_date;
/// use indexmap::IndexMap;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let as_of = create_date(2025, Month::January, 15)?;
///
/// let mut base_measures = IndexMap::new();
/// base_measures.insert(MetricId::Dv01, 425.0);
///
/// let base = ValuationResult::stamped("BOND", as_of, Money::new(1_000_000.0, Currency::USD))
///     .with_measures(base_measures);
///
/// let mut up_measures = IndexMap::new();
/// up_measures.insert(MetricId::Dv01, 420.0);
///
/// let up = ValuationResult::stamped("BOND", as_of, Money::new(995_000.0, Currency::USD))
///     .with_measures(up_measures);
///
/// let results = vec![
///     ("Base".to_string(), base),
///     ("+100bp".to_string(), up),
/// ];
/// let metric_ids = vec![MetricId::Dv01];
///
/// let matrix = ScenarioMatrix::from_scenario_results(
///     "Rate Scenarios",
///     &results,
///     &metric_ids,
///     Some("Base"),
/// );
///
/// assert_eq!(matrix.scenario_names.len(), 2);
/// assert_eq!(matrix.base_case_index, Some(0));
/// assert!(matrix.deltas.is_some());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioMatrix {
    /// Display title.
    pub title: String,
    /// Scenario names (row labels).
    pub scenario_names: Vec<String>,
    /// Metric identifiers (column labels).
    pub metric_ids: Vec<String>,
    /// Values: `values[scenario_index][metric_index]`.
    pub values: Vec<Vec<f64>>,
    /// Index of the base-case scenario, if designated.
    pub base_case_index: Option<usize>,
    /// Differences from base case: `deltas[scenario_index][metric_index]`.
    /// `None` if no base case is designated.
    pub deltas: Option<Vec<Vec<f64>>>,
}

impl ScenarioMatrix {
    /// Build from a list of `(scenario_name, ValuationResult)` pairs and
    /// the metric IDs to extract.
    ///
    /// If `base_case_name` matches a scenario name, delta calculations are
    /// produced. Missing metrics in a result are represented as `f64::NAN`.
    pub fn from_scenario_results(
        title: impl Into<String>,
        results: &[(String, ValuationResult)],
        metric_ids: &[MetricId],
        base_case_name: Option<&str>,
    ) -> Self {
        let title = title.into();
        let scenario_names: Vec<String> = results.iter().map(|(name, _)| name.clone()).collect();
        let metric_id_strings: Vec<String> = metric_ids.iter().map(|m| m.as_str().to_string()).collect();

        let mut values = Vec::with_capacity(results.len());
        for (_, result) in results {
            let row: Vec<f64> = metric_ids
                .iter()
                .map(|mid| result.metric(mid.clone()).unwrap_or(f64::NAN))
                .collect();
            values.push(row);
        }

        let base_case_index = base_case_name
            .and_then(|name| scenario_names.iter().position(|s| s == name));

        let deltas = base_case_index.map(|base_idx| {
            let base_row = &values[base_idx];
            values
                .iter()
                .map(|row| {
                    row.iter()
                        .zip(base_row.iter())
                        .map(|(v, b)| v - b)
                        .collect()
                })
                .collect()
        });

        Self {
            title,
            scenario_names,
            metric_ids: metric_id_strings,
            values,
            base_case_index,
            deltas,
        }
    }
}

impl ReportComponent for ScenarioMatrix {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    #[allow(clippy::expect_used)]
    fn to_markdown(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "## {}\n", self.title).expect("writing to String cannot fail");

        // Header row
        write!(&mut out, "| Scenario |").expect("writing to String cannot fail");
        for mid in &self.metric_ids {
            write!(&mut out, " {} |", mid).expect("writing to String cannot fail");
        }
        out.push('\n');

        // Separator
        write!(&mut out, "|:---------|").expect("writing to String cannot fail");
        for _ in &self.metric_ids {
            write!(&mut out, "------:|").expect("writing to String cannot fail");
        }
        out.push('\n');

        // Data rows
        for (i, name) in self.scenario_names.iter().enumerate() {
            let is_base = self.base_case_index == Some(i);
            let label = if is_base {
                format!("**{}** (base)", name)
            } else {
                name.clone()
            };
            write!(&mut out, "| {} |", label).expect("writing to String cannot fail");
            for val in &self.values[i] {
                if val.is_nan() {
                    write!(&mut out, " N/A |").expect("writing to String cannot fail");
                } else {
                    write!(&mut out, " {:.4} |", val).expect("writing to String cannot fail");
                }
            }
            out.push('\n');
        }

        // Delta section
        if let Some(deltas) = &self.deltas {
            writeln!(&mut out, "\n### Deltas from Base Case\n")
                .expect("writing to String cannot fail");
            write!(&mut out, "| Scenario |").expect("writing to String cannot fail");
            for mid in &self.metric_ids {
                write!(&mut out, " {} |", mid).expect("writing to String cannot fail");
            }
            out.push('\n');
            write!(&mut out, "|:---------|").expect("writing to String cannot fail");
            for _ in &self.metric_ids {
                write!(&mut out, "------:|").expect("writing to String cannot fail");
            }
            out.push('\n');

            for (i, name) in self.scenario_names.iter().enumerate() {
                write!(&mut out, "| {} |", name).expect("writing to String cannot fail");
                for val in &deltas[i] {
                    if val.is_nan() {
                        write!(&mut out, " N/A |").expect("writing to String cannot fail");
                    } else {
                        write!(&mut out, " {:.4} |", val).expect("writing to String cannot fail");
                    }
                }
                out.push('\n');
            }
        }

        out
    }

    fn component_type(&self) -> &'static str {
        "scenario_matrix"
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use indexmap::IndexMap;
    use time::macros::date;

    fn make_result(npv: f64, dv01: f64, ytm: f64) -> ValuationResult {
        let mut measures = IndexMap::new();
        measures.insert(MetricId::Dv01, dv01);
        measures.insert(MetricId::Ytm, ytm);

        ValuationResult::stamped(
            "BOND",
            date!(2025 - 01 - 15),
            Money::new(npv, Currency::USD),
        )
        .with_measures(measures)
    }

    fn sample_results() -> Vec<(String, ValuationResult)> {
        vec![
            ("Base".to_string(), make_result(1_000_000.0, 425.0, 0.0475)),
            ("+100bp".to_string(), make_result(980_000.0, 420.0, 0.0575)),
            ("-100bp".to_string(), make_result(1_021_000.0, 430.0, 0.0375)),
        ]
    }

    #[test]
    fn basic_construction() {
        let results = sample_results();
        let metric_ids = vec![MetricId::Dv01, MetricId::Ytm];

        let matrix = ScenarioMatrix::from_scenario_results(
            "Rate Scenarios",
            &results,
            &metric_ids,
            Some("Base"),
        );

        assert_eq!(matrix.scenario_names.len(), 3);
        assert_eq!(matrix.metric_ids.len(), 2);
        assert_eq!(matrix.values.len(), 3);
        assert_eq!(matrix.base_case_index, Some(0));
    }

    #[test]
    fn deltas_from_base() {
        let results = sample_results();
        let metric_ids = vec![MetricId::Dv01, MetricId::Ytm];

        let matrix = ScenarioMatrix::from_scenario_results(
            "Test",
            &results,
            &metric_ids,
            Some("Base"),
        );

        let deltas = matrix.deltas.as_ref().expect("deltas should be present");
        // Base-to-base deltas should be 0
        assert!((deltas[0][0]).abs() < 1e-10);
        assert!((deltas[0][1]).abs() < 1e-10);
        // +100bp DV01 delta: 420 - 425 = -5
        assert!((deltas[1][0] - (-5.0)).abs() < 1e-10);
    }

    #[test]
    fn no_base_case() {
        let results = sample_results();
        let metric_ids = vec![MetricId::Dv01];

        let matrix = ScenarioMatrix::from_scenario_results(
            "Test",
            &results,
            &metric_ids,
            None,
        );

        assert!(matrix.base_case_index.is_none());
        assert!(matrix.deltas.is_none());
    }

    #[test]
    fn missing_base_case_name() {
        let results = sample_results();
        let metric_ids = vec![MetricId::Dv01];

        let matrix = ScenarioMatrix::from_scenario_results(
            "Test",
            &results,
            &metric_ids,
            Some("NonExistent"),
        );

        assert!(matrix.base_case_index.is_none());
        assert!(matrix.deltas.is_none());
    }

    #[test]
    fn missing_metric() {
        let results = sample_results();
        let metric_ids = vec![MetricId::Dv01, MetricId::custom("nonexistent")];

        let matrix = ScenarioMatrix::from_scenario_results(
            "Test",
            &results,
            &metric_ids,
            None,
        );

        // Second metric should be NaN
        assert!(matrix.values[0][1].is_nan());
    }

    #[test]
    fn single_scenario() {
        let results = vec![("Only".to_string(), make_result(1_000_000.0, 425.0, 0.0475))];
        let metric_ids = vec![MetricId::Dv01];

        let matrix = ScenarioMatrix::from_scenario_results("Single", &results, &metric_ids, None);
        assert_eq!(matrix.scenario_names.len(), 1);
        assert_eq!(matrix.values.len(), 1);
    }

    #[test]
    fn to_json_structure() {
        let results = sample_results();
        let metric_ids = vec![MetricId::Dv01, MetricId::Ytm];

        let matrix = ScenarioMatrix::from_scenario_results(
            "Rate Scenarios",
            &results,
            &metric_ids,
            Some("Base"),
        );
        let json = matrix.to_json();

        assert_eq!(json["title"], "Rate Scenarios");
        assert!(json["scenario_names"].is_array());
        assert!(json["values"].is_array());
        assert!(json["deltas"].is_array());
        assert_eq!(json["base_case_index"], 0);
    }

    #[test]
    fn to_markdown_format() {
        let results = sample_results();
        let metric_ids = vec![MetricId::Dv01, MetricId::Ytm];

        let matrix = ScenarioMatrix::from_scenario_results(
            "Rate Scenarios",
            &results,
            &metric_ids,
            Some("Base"),
        );
        let md = matrix.to_markdown();

        assert!(md.contains("## Rate Scenarios"));
        assert!(md.contains("| Scenario |"));
        assert!(md.contains("**Base** (base)"));
        assert!(md.contains("Deltas from Base Case"));
    }

    #[test]
    fn component_type_name() {
        let matrix = ScenarioMatrix::from_scenario_results(
            "Test",
            &[],
            &[],
            None,
        );
        assert_eq!(matrix.component_type(), "scenario_matrix");
    }
}
