use crate::metrics::MetricId;
use crate::reporting::ReportComponent;
use crate::results::ValuationResult;
use serde::Serialize;
use std::fmt::Write as FmtWrite;

/// Conditional direction annotation for a metric value.
///
/// Used as metadata for downstream rendering decisions (e.g., color coding).
/// Components produce this as data; actual color choices belong to the
/// rendering layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Value is favorable (e.g., positive P&L).
    Positive,
    /// Value is adverse (e.g., negative P&L).
    Negative,
    /// Value is neutral or directionless.
    Neutral,
}

/// Unit annotation for a metric value.
///
/// Consumers use this to decide formatting (e.g., append "bps", show "%").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricUnit {
    /// Absolute currency amount.
    Currency,
    /// Currency per basis-point shift (e.g., DV01).
    CurrencyPerBp,
    /// Percentage (0.05 = 5%).
    Percent,
    /// Dimensionless ratio.
    Ratio,
    /// Basis points.
    BasisPoints,
    /// Time in years.
    Years,
    /// No unit / raw number.
    Dimensionless,
}

/// A single metric row in a [`MetricsTable`].
#[derive(Debug, Clone, Serialize)]
pub struct MetricRow {
    /// Metric identifier string (e.g., "dv01", "ytm").
    pub metric_id: String,
    /// Raw numeric value.
    pub value: f64,
    /// Unit annotation for formatting decisions.
    pub unit: MetricUnit,
    /// Directional annotation for conditional formatting.
    pub direction: Direction,
}

/// Structured key-value export of computed metrics from a [`ValuationResult`].
///
/// Each row is one metric with its identifier, raw value, unit annotation,
/// and directional metadata. Constructed via [`MetricsTable::from_valuation_result`].
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::{MetricsTable, ReportComponent};
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
/// let pv = Money::new(1_000_000.0, Currency::USD);
///
/// let mut measures = IndexMap::new();
/// measures.insert(MetricId::Dv01, 425.0);
/// measures.insert(MetricId::Ytm, 0.0475);
///
/// let result = ValuationResult::stamped("BOND-001", as_of, pv)
///     .with_measures(measures);
///
/// let table = MetricsTable::from_valuation_result(&result);
/// assert_eq!(table.instrument_id, "BOND-001");
/// assert_eq!(table.rows.len(), 2);
///
/// let json = table.to_json();
/// assert_eq!(json["currency"], "USD");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct MetricsTable {
    /// Instrument identifier from the source [`ValuationResult`].
    pub instrument_id: String,
    /// Valuation date as ISO 8601 string.
    pub as_of: String,
    /// Currency code.
    pub currency: String,
    /// Net present value.
    pub npv: f64,
    /// Individual metric rows.
    pub rows: Vec<MetricRow>,
}

impl MetricsTable {
    /// Build from a [`ValuationResult`].
    ///
    /// Iterates the measures map and resolves unit/direction from
    /// [`MetricId`] metadata. Metrics are emitted in their original
    /// insertion order.
    pub fn from_valuation_result(result: &ValuationResult) -> Self {
        let rows = result
            .measures
            .iter()
            .map(|(id, &value)| MetricRow {
                metric_id: id.as_str().to_string(),
                value,
                unit: infer_unit(id),
                direction: infer_direction(value),
            })
            .collect();

        Self {
            instrument_id: result.instrument_id.clone(),
            as_of: result.as_of.to_string(),
            currency: result.value.currency().to_string(),
            npv: result.value.amount(),
            rows,
        }
    }
}

impl ReportComponent for MetricsTable {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    #[allow(clippy::expect_used)]
    fn to_markdown(&self) -> String {
        let mut out = String::new();
        writeln!(
            &mut out,
            "## Metrics: {} (as of {})",
            self.instrument_id, self.as_of
        )
        .expect("writing to String cannot fail");
        writeln!(&mut out, "**NPV**: {:.2} {}\n", self.npv, self.currency)
            .expect("writing to String cannot fail");

        writeln!(&mut out, "| Metric | Value | Unit | Direction |")
            .expect("writing to String cannot fail");
        writeln!(&mut out, "|:-------|------:|:-----|:----------|")
            .expect("writing to String cannot fail");

        for row in &self.rows {
            let unit_str = match row.unit {
                MetricUnit::Currency => "currency",
                MetricUnit::CurrencyPerBp => "currency/bp",
                MetricUnit::Percent => "%",
                MetricUnit::Ratio => "ratio",
                MetricUnit::BasisPoints => "bps",
                MetricUnit::Years => "years",
                MetricUnit::Dimensionless => "-",
            };
            let dir_str = match row.direction {
                Direction::Positive => "positive",
                Direction::Negative => "negative",
                Direction::Neutral => "neutral",
            };
            writeln!(
                &mut out,
                "| {} | {:.6} | {} | {} |",
                row.metric_id, row.value, unit_str, dir_str
            )
            .expect("writing to String cannot fail");
        }

        out
    }

    fn component_type(&self) -> &'static str {
        "metrics_table"
    }
}

/// Infer the unit for a metric based on its identifier.
///
/// Uses naming conventions: metrics ending in `01` are currency-per-bp,
/// `ytm`/`oas`/rate metrics are percentages, duration/life metrics are
/// years, etc.
fn infer_unit(id: &MetricId) -> MetricUnit {
    let s = id.as_str();
    if s.ends_with("01") || s.starts_with("dv01") || s.starts_with("cs01") {
        MetricUnit::CurrencyPerBp
    } else if s.contains("duration")
        || s.contains("wal")
        || s == "weighted_avg_life"
        || s == "time_to_maturity"
    {
        MetricUnit::Years
    } else if s.contains("ytm")
        || s.contains("oas")
        || s.contains("spread")
        || s.contains("yield")
        || s.contains("rate")
        || s == "default_probability"
    {
        MetricUnit::Percent
    } else if s.contains("convexity") || s == "tvpi_lp" || s == "dpi_lp" || s == "rvpi_lp" {
        MetricUnit::Ratio
    } else if s == "delta"
        || s == "gamma"
        || s == "vega"
        || s == "theta"
        || s == "rho"
        || s == "npv"
        || s.contains("pnl")
    {
        MetricUnit::Currency
    } else {
        MetricUnit::Dimensionless
    }
}

/// Infer direction from value sign.
fn infer_direction(value: f64) -> Direction {
    if value > 0.0 {
        Direction::Positive
    } else if value < 0.0 {
        Direction::Negative
    } else {
        Direction::Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use indexmap::IndexMap;
    use time::macros::date;

    fn sample_result() -> ValuationResult {
        let mut measures = IndexMap::new();
        measures.insert(MetricId::Dv01, 425.0);
        measures.insert(MetricId::Ytm, 0.0475);
        measures.insert(MetricId::DurationMod, 4.25);

        ValuationResult::stamped(
            "BOND-001",
            date!(2025 - 01 - 15),
            Money::new(1_000_000.0, Currency::USD),
        )
        .with_measures(measures)
    }

    #[test]
    fn from_valuation_result_basic() {
        let result = sample_result();
        let table = MetricsTable::from_valuation_result(&result);

        assert_eq!(table.instrument_id, "BOND-001");
        assert_eq!(table.currency, "USD");
        assert!((table.npv - 1_000_000.0).abs() < 1e-10);
        assert_eq!(table.rows.len(), 3);
    }

    #[test]
    fn to_json_roundtrip() {
        let result = sample_result();
        let table = MetricsTable::from_valuation_result(&result);
        let json = table.to_json();

        assert_eq!(json["instrument_id"], "BOND-001");
        assert_eq!(json["currency"], "USD");
        assert!(json["rows"].is_array());
        assert_eq!(json["rows"].as_array().expect("should be array").len(), 3);
    }

    #[test]
    fn to_markdown_contains_headers() {
        let result = sample_result();
        let table = MetricsTable::from_valuation_result(&result);
        let md = table.to_markdown();

        assert!(md.contains("## Metrics: BOND-001"));
        assert!(md.contains("| Metric |"));
        assert!(md.contains("dv01"));
        assert!(md.contains("ytm"));
    }

    #[test]
    fn empty_measures() {
        let result = ValuationResult::stamped(
            "EMPTY",
            date!(2025 - 01 - 01),
            Money::new(0.0, Currency::EUR),
        );
        let table = MetricsTable::from_valuation_result(&result);

        assert_eq!(table.rows.len(), 0);
        let json = table.to_json();
        assert_eq!(json["rows"].as_array().expect("should be array").len(), 0);
    }

    #[test]
    fn unit_inference() {
        assert_eq!(infer_unit(&MetricId::Dv01), MetricUnit::CurrencyPerBp);
        assert_eq!(infer_unit(&MetricId::Cs01), MetricUnit::CurrencyPerBp);
        assert_eq!(infer_unit(&MetricId::Ytm), MetricUnit::Percent);
        assert_eq!(infer_unit(&MetricId::DurationMod), MetricUnit::Years);
        assert_eq!(infer_unit(&MetricId::Convexity), MetricUnit::Ratio);
    }

    #[test]
    fn direction_inference() {
        assert_eq!(infer_direction(100.0), Direction::Positive);
        assert_eq!(infer_direction(-50.0), Direction::Negative);
        assert_eq!(infer_direction(0.0), Direction::Neutral);
    }

    #[test]
    fn component_type_name() {
        let result = sample_result();
        let table = MetricsTable::from_valuation_result(&result);
        assert_eq!(table.component_type(), "metrics_table");
    }
}
