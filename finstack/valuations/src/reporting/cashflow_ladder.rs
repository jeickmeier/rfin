use crate::reporting::ReportComponent;
use finstack_core::dates::Date;
use serde::Serialize;
use std::fmt::Write as FmtWrite;

/// Bucketing frequency for cashflow aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BucketFrequency {
    /// Monthly buckets.
    Monthly,
    /// Quarterly buckets.
    Quarterly,
    /// Semi-annual buckets.
    SemiAnnual,
    /// Annual buckets.
    Annual,
}

/// A single time bucket in a [`CashflowLadder`].
#[derive(Debug, Clone, Serialize)]
pub struct CashflowBucket {
    /// Human-readable label (e.g., "2025-Q1", "2025-03", "2025").
    pub label: String,
    /// Bucket start date (ISO 8601).
    pub start_date: String,
    /// Bucket end date (ISO 8601).
    pub end_date: String,
    /// Total principal in this bucket.
    pub principal: f64,
    /// Total interest in this bucket.
    pub interest: f64,
    /// Principal + interest.
    pub total: f64,
    /// Number of individual cashflows aggregated into this bucket.
    pub count: usize,
    /// Cumulative principal from the first bucket through this one.
    pub cumulative_principal: f64,
}

/// Time-bucketed cashflow summary.
///
/// Groups individual cashflows into calendar periods (monthly, quarterly,
/// annual) with subtotals. Constructed via [`CashflowLadder::from_cashflows`].
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::{CashflowLadder, BucketFrequency, ReportComponent};
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cashflows = vec![
///     (create_date(2025, Month::March, 15)?, 0.0, 2500.0),
///     (create_date(2025, Month::June, 15)?, 0.0, 2500.0),
///     (create_date(2025, Month::September, 15)?, 0.0, 2500.0),
///     (create_date(2025, Month::December, 15)?, 100_000.0, 2500.0),
/// ];
///
/// let ladder = CashflowLadder::from_cashflows(
///     "BOND-001",
///     "USD",
///     &cashflows,
///     BucketFrequency::Quarterly,
/// );
///
/// assert_eq!(ladder.buckets.len(), 4);
/// assert!((ladder.total - 110_000.0).abs() < 1e-10);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct CashflowLadder {
    /// Instrument identifier.
    pub instrument_id: String,
    /// Currency code.
    pub currency: String,
    /// How cashflows are bucketed.
    pub bucket_frequency: BucketFrequency,
    /// Ordered list of time buckets.
    pub buckets: Vec<CashflowBucket>,
    /// Grand total of all cashflows (principal + interest).
    pub total: f64,
    /// Weighted average life in years.
    pub weighted_avg_life: f64,
}

impl CashflowLadder {
    /// Build from a slice of `(date, principal, interest)` tuples.
    ///
    /// Cashflows are sorted by date, then aggregated into calendar buckets
    /// according to the specified frequency. Empty input produces a ladder
    /// with no buckets and zero totals.
    pub fn from_cashflows(
        instrument_id: impl Into<String>,
        currency: impl Into<String>,
        cashflows: &[(Date, f64, f64)],
        frequency: BucketFrequency,
    ) -> Self {
        let instrument_id = instrument_id.into();
        let currency = currency.into();

        if cashflows.is_empty() {
            return Self {
                instrument_id,
                currency,
                bucket_frequency: frequency,
                buckets: Vec::new(),
                total: 0.0,
                weighted_avg_life: 0.0,
            };
        }

        // Sort cashflows by date
        let mut sorted: Vec<(Date, f64, f64)> = cashflows.to_vec();
        sorted.sort_by_key(|(d, _, _)| *d);

        let first_date = sorted[0].0;

        // Group into buckets
        let mut bucket_map: Vec<(String, String, String, f64, f64, usize)> = Vec::new();

        for &(date, principal, interest) in &sorted {
            let (label, start, end) = bucket_key(date, frequency);

            if let Some(last) = bucket_map.last_mut() {
                if last.0 == label {
                    last.3 += principal;
                    last.4 += interest;
                    last.5 += 1;
                    continue;
                }
            }

            bucket_map.push((label, start, end, principal, interest, 1));
        }

        // Build buckets with cumulative tracking
        let mut cumulative_principal = 0.0;
        let mut buckets = Vec::with_capacity(bucket_map.len());

        for (label, start_date, end_date, principal, interest, count) in bucket_map {
            cumulative_principal += principal;
            let total = principal + interest;

            buckets.push(CashflowBucket {
                label,
                start_date,
                end_date,
                principal,
                interest,
                total,
                count,
                cumulative_principal,
            });
        }

        let total: f64 = buckets.iter().map(|b| b.total).sum();

        // WAL = sum(principal_i * time_i) / sum(principal_i)
        let total_principal: f64 = sorted.iter().map(|(_, p, _)| *p).sum();
        let weighted_avg_life = if total_principal > 0.0 {
            let wal_numerator: f64 = sorted
                .iter()
                .map(|(d, p, _)| {
                    let days = (*d - first_date).whole_days() as f64;
                    let years = days / 365.25;
                    p * years
                })
                .sum();
            wal_numerator / total_principal
        } else {
            0.0
        };

        Self {
            instrument_id,
            currency,
            bucket_frequency: frequency,
            buckets,
            total,
            weighted_avg_life,
        }
    }
}

/// Generate a bucket key (label, start_date, end_date) for a given date and frequency.
fn bucket_key(date: Date, frequency: BucketFrequency) -> (String, String, String) {
    let year = date.year();
    let month = date.month() as u8;

    match frequency {
        BucketFrequency::Monthly => {
            let label = format!("{}-{:02}", year, month);
            let start = format!("{}-{:02}-01", year, month);
            let days_in_month = date.month().length(year);
            let end = format!("{}-{:02}-{:02}", year, month, days_in_month);
            (label, start, end)
        }
        BucketFrequency::Quarterly => {
            let quarter = (month - 1) / 3 + 1;
            let label = format!("{}-Q{}", year, quarter);
            let start_month = (quarter - 1) * 3 + 1;
            let end_month = quarter * 3;
            let end_month_enum = time::Month::try_from(end_month).unwrap_or(time::Month::December);
            let days_in_end = end_month_enum.length(year);
            let start = format!("{}-{:02}-01", year, start_month);
            let end = format!("{}-{:02}-{:02}", year, end_month, days_in_end);
            (label, start, end)
        }
        BucketFrequency::SemiAnnual => {
            let half = if month <= 6 { 1 } else { 2 };
            let label = format!("{}-H{}", year, half);
            let (start_month, end_month) = if half == 1 { (1, 6) } else { (7, 12) };
            let end_month_enum = time::Month::try_from(end_month).unwrap_or(time::Month::December);
            let days_in_end = end_month_enum.length(year);
            let start = format!("{}-{:02}-01", year, start_month);
            let end = format!("{}-{:02}-{:02}", year, end_month, days_in_end);
            (label, start, end)
        }
        BucketFrequency::Annual => {
            let label = format!("{}", year);
            let start = format!("{}-01-01", year);
            let end = format!("{}-12-31", year);
            (label, start, end)
        }
    }
}

impl ReportComponent for CashflowLadder {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    #[allow(clippy::expect_used)]
    fn to_markdown(&self) -> String {
        let mut out = String::new();
        writeln!(
            &mut out,
            "## Cashflow Ladder: {} ({})\n",
            self.instrument_id, self.currency
        )
        .expect("writing to String cannot fail");

        writeln!(
            &mut out,
            "| Period | Principal | Interest | Total | Cum. Principal |"
        )
        .expect("writing to String cannot fail");
        writeln!(
            &mut out,
            "|:-------|----------:|---------:|------:|---------------:|"
        )
        .expect("writing to String cannot fail");

        for bucket in &self.buckets {
            writeln!(
                &mut out,
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} |",
                bucket.label,
                bucket.principal,
                bucket.interest,
                bucket.total,
                bucket.cumulative_principal,
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "\n**Total**: {:.2}", self.total)
            .expect("writing to String cannot fail");
        writeln!(&mut out, "**WAL**: {:.2} years", self.weighted_avg_life)
            .expect("writing to String cannot fail");

        out
    }

    fn component_type(&self) -> &'static str {
        "cashflow_ladder"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::create_date;
    use time::Month;

    fn sample_cashflows() -> Vec<(Date, f64, f64)> {
        vec![
            (
                create_date(2025, Month::March, 15).expect("valid date"),
                0.0,
                2500.0,
            ),
            (
                create_date(2025, Month::June, 15).expect("valid date"),
                0.0,
                2500.0,
            ),
            (
                create_date(2025, Month::September, 15).expect("valid date"),
                0.0,
                2500.0,
            ),
            (
                create_date(2025, Month::December, 15).expect("valid date"),
                100_000.0,
                2500.0,
            ),
        ]
    }

    #[test]
    fn quarterly_bucketing() {
        let cfs = sample_cashflows();
        let ladder =
            CashflowLadder::from_cashflows("BOND-001", "USD", &cfs, BucketFrequency::Quarterly);

        assert_eq!(ladder.buckets.len(), 4);
        assert_eq!(ladder.buckets[0].label, "2025-Q1");
        assert_eq!(ladder.buckets[1].label, "2025-Q2");
        assert_eq!(ladder.buckets[2].label, "2025-Q3");
        assert_eq!(ladder.buckets[3].label, "2025-Q4");
    }

    #[test]
    fn annual_bucketing() {
        let cfs = sample_cashflows();
        let ladder =
            CashflowLadder::from_cashflows("BOND-001", "USD", &cfs, BucketFrequency::Annual);

        assert_eq!(ladder.buckets.len(), 1);
        assert_eq!(ladder.buckets[0].label, "2025");
        assert!((ladder.buckets[0].principal - 100_000.0).abs() < 1e-10);
        assert!((ladder.buckets[0].interest - 10_000.0).abs() < 1e-10);
    }

    #[test]
    fn total_and_wal() {
        let cfs = sample_cashflows();
        let ladder =
            CashflowLadder::from_cashflows("BOND-001", "USD", &cfs, BucketFrequency::Quarterly);

        assert!((ladder.total - 110_000.0).abs() < 1e-10);
        // WAL should be ~0.75 years (Dec 15 - Mar 15 ~= 275 days / 365.25)
        assert!(ladder.weighted_avg_life > 0.5);
        assert!(ladder.weighted_avg_life < 1.0);
    }

    #[test]
    fn empty_cashflows() {
        let ladder = CashflowLadder::from_cashflows("EMPTY", "EUR", &[], BucketFrequency::Monthly);

        assert_eq!(ladder.buckets.len(), 0);
        assert!((ladder.total).abs() < 1e-15);
        assert!((ladder.weighted_avg_life).abs() < 1e-15);
    }

    #[test]
    fn single_cashflow() {
        let cfs = vec![(
            create_date(2025, Month::June, 15).expect("valid date"),
            50_000.0,
            1_000.0,
        )];
        let ladder =
            CashflowLadder::from_cashflows("SINGLE", "USD", &cfs, BucketFrequency::Quarterly);

        assert_eq!(ladder.buckets.len(), 1);
        assert!((ladder.buckets[0].total - 51_000.0).abs() < 1e-10);
        // WAL = 0 because there is only one date
        assert!((ladder.weighted_avg_life).abs() < 1e-10);
    }

    #[test]
    fn cumulative_principal() {
        let cfs = vec![
            (
                create_date(2025, Month::March, 15).expect("valid date"),
                10_000.0,
                500.0,
            ),
            (
                create_date(2025, Month::June, 15).expect("valid date"),
                20_000.0,
                400.0,
            ),
            (
                create_date(2025, Month::September, 15).expect("valid date"),
                30_000.0,
                300.0,
            ),
        ];
        let ladder = CashflowLadder::from_cashflows("CUM", "USD", &cfs, BucketFrequency::Quarterly);

        assert!((ladder.buckets[0].cumulative_principal - 10_000.0).abs() < 1e-10);
        assert!((ladder.buckets[1].cumulative_principal - 30_000.0).abs() < 1e-10);
        assert!((ladder.buckets[2].cumulative_principal - 60_000.0).abs() < 1e-10);
    }

    #[test]
    fn to_json_structure() {
        let cfs = sample_cashflows();
        let ladder =
            CashflowLadder::from_cashflows("BOND-001", "USD", &cfs, BucketFrequency::Quarterly);
        let json = ladder.to_json();

        assert_eq!(json["instrument_id"], "BOND-001");
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["bucket_frequency"], "quarterly");
        assert!(json["buckets"].is_array());
        assert_eq!(
            json["buckets"].as_array().expect("should be array").len(),
            4
        );
    }

    #[test]
    fn to_markdown_format() {
        let cfs = sample_cashflows();
        let ladder =
            CashflowLadder::from_cashflows("BOND-001", "USD", &cfs, BucketFrequency::Quarterly);
        let md = ladder.to_markdown();

        assert!(md.contains("## Cashflow Ladder: BOND-001"));
        assert!(md.contains("| Period |"));
        assert!(md.contains("2025-Q1"));
        assert!(md.contains("**Total**"));
        assert!(md.contains("**WAL**"));
    }

    #[test]
    fn component_type_name() {
        let ladder = CashflowLadder::from_cashflows("X", "USD", &[], BucketFrequency::Annual);
        assert_eq!(ladder.component_type(), "cashflow_ladder");
    }
}
