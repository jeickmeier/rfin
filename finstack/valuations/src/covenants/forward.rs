//! Covenant forward-projection with headroom analytics.
//!
//! This module provides generic covenant forecasting that can be driven by any
//! time-series model implementing the [`ModelTimeSeries`] trait. A thin
//! statements-specific adapter is provided behind the `statements_bridge` feature
//! so this module remains usable without introducing a crate cycle.

use crate::covenants::engine::{CovenantSpec, CovenantType, ThresholdTest};
use finstack_core::dates::{Date, PeriodId};
use finstack_core::error::Error;
use finstack_core::Result;
use finstack_core::error::InputError;
use crate::instruments::common::mc::traits::RandomStream;

/// Comparator for headroom calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparator {
    LessOrEqual,
    GreaterOrEqual,
}

/// MC configuration (subset; integrates with instruments/common/mc RNG).
#[derive(Clone, Debug, Default)]
pub struct McConfig {
    pub seed: u64,
    /// When true, uses antithetic variates (simple variance reduction).
    pub antithetic: bool,
}

/// Covenant forecast configuration.
#[derive(Clone, Debug, Default)]
pub struct CovenantForecastConfig {
    pub stochastic: bool,
    pub num_paths: usize,
    pub volatility: Option<f64>,
    pub random_seed: Option<u64>,
    pub mc: Option<McConfig>,
}

/// Forecast output with headroom analytics.
#[derive(Clone, Debug)]
pub struct CovenantForecast {
    pub covenant_id: String,
    pub test_dates: Vec<Date>,
    pub projected_values: Vec<f64>,
    pub thresholds: Vec<f64>,
    pub headroom: Vec<f64>,
    pub breach_probability: Vec<f64>,
    pub first_breach_date: Option<Date>,
    pub min_headroom_date: Date,
    pub min_headroom_value: f64,
}

impl CovenantForecast {
    /// Convenience helper to find indices with headroom under a threshold.
    pub fn warning_indices(&self, warn_threshold: f64) -> Vec<usize> {
        self.headroom
            .iter()
            .enumerate()
            .filter_map(|(i, &h)| (h < warn_threshold).then_some(i))
            .collect()
    }

    /// Render a human-readable explanation across periods.
    pub fn explain(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Covenant: {}\n", self.covenant_id));
        for i in 0..self.test_dates.len() {
            let date = self.test_dates[i];
            let value = self.projected_values[i];
            let thr = self.thresholds[i];
            let hr = self.headroom[i];
            let bp = self.breach_probability[i];
            let status = if value > thr { "BREACH" } else { "OK" };
            s.push_str(&format!(
                "{}: {:.4} (thr: {:.4}, headroom: {:+.1}%, breach prob: {:.0}%) {}\n",
                date,
                value,
                thr,
                hr * 100.0,
                bp * 100.0,
                status
            ));
        }
        s
    }

    // Polars export lives in the meta crate to avoid bringing polars into valuations.
}

/// Minimal read-only adapter to query model time-series values and map periods to dates.
pub trait ModelTimeSeries {
    fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64>;
    fn period_end_date(&self, period: &PeriodId) -> Date;
}

/// Forecast a covenant using a generic time-series model adapter.
pub fn forecast_covenant_generic<MTS: ModelTimeSeries>(
    covenant: &CovenantSpec,
    model: &MTS,
    periods: &[PeriodId],
    config: CovenantForecastConfig,
) -> Result<CovenantForecast> {
    if periods.is_empty() {
        return Err(Error::Validation("no periods provided".to_string()));
    }

    let id = covenant.covenant.description();
    let comparator = comparator_for(&covenant.covenant.covenant_type);

    // Resolve thresholds and values
    let mut test_dates: Vec<Date> = Vec::with_capacity(periods.len());
    let mut thresholds: Vec<f64> = Vec::with_capacity(periods.len());
    let mut values: Vec<f64> = Vec::with_capacity(periods.len());

    for pid in periods {
        let date = model.period_end_date(pid);
        test_dates.push(date);
        let thr = base_threshold_from_spec(&covenant.covenant.covenant_type)
            .ok_or(finstack_core::error::Error::Input(InputError::Invalid))?;
        thresholds.push(thr);

        let v = metric_value_for_spec(covenant, model, pid).ok_or_else(|| {
            Error::from(finstack_core::error::InputError::NotFound {
                id: "metric_value".to_string(),
            })
        })?;
        values.push(v);
    }

    // Deterministic headroom and breach flag
    let headroom: Vec<f64> = values
        .iter()
        .zip(thresholds.iter())
        .map(|(&v, &t)| compute_headroom(v, t, comparator))
        .collect();

    let deterministic_breach_prob: Vec<f64> = values
        .iter()
        .zip(thresholds.iter())
        .map(|(&v, &t)| match comparator {
            Comparator::LessOrEqual => (v > t) as u8 as f64,
            Comparator::GreaterOrEqual => (v < t) as u8 as f64,
        })
        .collect();

    let mut breach_probability = deterministic_breach_prob.clone();

    // Optional MC overlay (multiplicative shock to metric value)
    if config.stochastic {
        let sigma = config.volatility.unwrap_or(0.0);
        let paths = config.num_paths.max(1);
        let seed = config.random_seed.unwrap_or(42);
        let antithetic = config.mc.as_ref().map(|m| m.antithetic).unwrap_or(false);

        use crate::instruments::common::mc::rng::philox::PhiloxRng;
        let mut rng = PhiloxRng::new(seed);

        // For each test date, estimate breach probability
        for i in 0..values.len() {
            let base = values[i];
            let thr = thresholds[i];
            let mut breaches = 0usize;
            let mut buf = vec![0.0f64; 1024];
            let mut remaining = paths;
            while remaining > 0 {
                let take = remaining.min(buf.len());
                rng.fill_std_normals(&mut buf[..take]);
                for &z in &buf[..take] {
                    let shock = (sigma * z).exp(); // lognormal multiplicative shock
                    let v = base * shock;
                    let breached = match comparator {
                        Comparator::LessOrEqual => v > thr,
                        Comparator::GreaterOrEqual => v < thr,
                    };
                    if breached {
                        breaches += 1;
                    }
                    if antithetic {
                        let shock_a = (sigma * -z).exp();
                        let v_a = base * shock_a;
                        let breached_a = match comparator {
                            Comparator::LessOrEqual => v_a > thr,
                            Comparator::GreaterOrEqual => v_a < thr,
                        };
                        if breached_a {
                            breaches += 1;
                        }
                    }
                }
                remaining -= take;
            }
            breach_probability[i] = (breaches as f64) / (paths as f64 * if antithetic { 2.0 } else { 1.0 });
        }
    }

    // Summary stats
    let mut min_idx = 0usize;
    for i in 1..headroom.len() {
        if headroom[i] < headroom[min_idx] {
            min_idx = i;
        }
    }
    let min_headroom_date = test_dates[min_idx];
    let min_headroom_value = headroom[min_idx];

    let first_breach_date = (0..values.len()).find_map(|i| {
        let v = values[i];
        let t = thresholds[i];
        match comparator {
            Comparator::LessOrEqual => (v > t).then_some(test_dates[i]),
            Comparator::GreaterOrEqual => (v < t).then_some(test_dates[i]),
        }
    });

    Ok(CovenantForecast {
        covenant_id: id,
        test_dates,
        projected_values: values,
        thresholds,
        headroom,
        breach_probability,
        first_breach_date,
        min_headroom_date,
        min_headroom_value,
    })
}

fn comparator_for(cov: &CovenantType) -> Comparator {
    match cov {
        CovenantType::MaxDebtToEBITDA { .. }
        | CovenantType::MaxTotalLeverage { .. }
        | CovenantType::MaxSeniorLeverage { .. } => Comparator::LessOrEqual,
        CovenantType::MinInterestCoverage { .. }
        | CovenantType::MinFixedChargeCoverage { .. }
        | CovenantType::MinAssetCoverage { .. } => Comparator::GreaterOrEqual,
        CovenantType::Custom { test, .. } => match test {
            ThresholdTest::Maximum(_) => Comparator::LessOrEqual,
            ThresholdTest::Minimum(_) => Comparator::GreaterOrEqual,
        },
        CovenantType::Negative { .. } | CovenantType::Affirmative { .. } => Comparator::GreaterOrEqual, // Non-financial: treated as pass
    }
}

fn base_threshold_from_spec(cov: &CovenantType) -> Option<f64> {
    match cov {
        CovenantType::MaxDebtToEBITDA { threshold }
        | CovenantType::MinInterestCoverage { threshold }
        | CovenantType::MinFixedChargeCoverage { threshold }
        | CovenantType::MaxTotalLeverage { threshold }
        | CovenantType::MaxSeniorLeverage { threshold }
        | CovenantType::MinAssetCoverage { threshold } => Some(*threshold),
        CovenantType::Custom { test, .. } => match test {
            ThresholdTest::Maximum(t) | ThresholdTest::Minimum(t) => Some(*t),
        },
        _ => None,
    }
}

fn compute_headroom(value: f64, threshold: f64, cmp: Comparator) -> f64 {
    match cmp {
        Comparator::LessOrEqual => (threshold - value) / threshold,
        Comparator::GreaterOrEqual => (value - threshold) / threshold,
    }
}

fn metric_value_for_spec<MTS: ModelTimeSeries>(
    spec: &CovenantSpec,
    model: &MTS,
    period: &PeriodId,
) -> Option<f64> {
    // Prefer explicit metric_id if provided (assumed to map to model node id).
    if let Some(metric_id) = &spec.metric_id {
        let name = metric_id.as_str();
        if let Some(v) = model.get_scalar(name, period) {
            return Some(v);
        }
    }

    // Fallbacks by standard covenant types (expect nodes to exist with conventional names)
    match &spec.covenant.covenant_type {
        CovenantType::MaxDebtToEBITDA { .. } => model.get_scalar("debt_to_ebitda", period),
        CovenantType::MinInterestCoverage { .. } => model.get_scalar("interest_coverage", period),
        CovenantType::MinFixedChargeCoverage { .. } => model.get_scalar("fixed_charge_coverage", period),
        CovenantType::MaxTotalLeverage { .. } => model.get_scalar("total_leverage", period),
        CovenantType::MaxSeniorLeverage { .. } => model.get_scalar("senior_leverage", period),
        CovenantType::MinAssetCoverage { .. } => model.get_scalar("asset_coverage", period),
        CovenantType::Custom { metric, .. } => model.get_scalar(metric, period),
        CovenantType::Negative { .. } | CovenantType::Affirmative { .. } => Some(1.0),
    }
}

// Note: Statements-specific bridging lives in the `finstack` meta crate to avoid a
// dependency cycle between `valuations` and `statements`.


#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::{Date, PeriodId};
    use time::Month;

    struct MockTs {
        map: std::collections::HashMap<(String, String), f64>,
    }

    impl MockTs {
        fn new() -> Self {
            Self { map: std::collections::HashMap::new() }
        }
        fn with(mut self, node: &str, period: PeriodId, v: f64) -> Self {
            self.map.insert((node.to_string(), period.to_string()), v);
            self
        }
    }

    impl ModelTimeSeries for MockTs {
        fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64> {
            self.map.get(&(node_id.to_string(), period.to_string())).copied()
        }
        fn period_end_date(&self, period: &PeriodId) -> Date {
            // simple quarterly end approximation
            let m = [3u8, 6, 9, 12][(period.index as usize - 1).min(3)];
            Date::from_calendar_date(period.year, Month::try_from(m).unwrap(), 30).unwrap()
        }
    }

    fn q(year: i32, q: u8) -> PeriodId { PeriodId::quarter(year, q) }

    #[test]
    fn deterministic_headroom_positive_zero_breach_prob() {
        // Debt/EBITDA <= 5, actual ratio at 4 → positive headroom
        let spec = CovenantSpec::with_metric(
            crate::covenants::engine::Covenant::new(
                CovenantType::MaxDebtToEBITDA { threshold: 5.0 },
                finstack_core::dates::Frequency::quarterly(),
            ),
            crate::metrics::MetricId::custom("debt_to_ebitda"),
        );

        let periods = vec![q(2025, 1), q(2025, 2)];
        let mts = MockTs::new()
            .with("debt_to_ebitda", periods[0], 4.0)
            .with("debt_to_ebitda", periods[1], 4.2);

        let cfg = CovenantForecastConfig::default();
        let fc = forecast_covenant_generic(&spec, &mts, &periods, cfg).unwrap();

        assert!(fc.headroom.iter().all(|&h| h > 0.0));
        assert!(fc.breach_probability.iter().all(|&p| (p - 0.0).abs() < 1e-12));
        assert!(fc.first_breach_date.is_none());
    }

    #[test]
    fn stochastic_breach_probability_moves_with_vol() {
        // Debt/EBITDA <= 1.0, base ~ 1.0; with high vol, breach prob should be material
        let spec = CovenantSpec::with_metric(
            crate::covenants::engine::Covenant::new(
                CovenantType::MaxDebtToEBITDA { threshold: 1.0 },
                finstack_core::dates::Frequency::quarterly(),
            ),
            crate::metrics::MetricId::custom("debt_to_ebitda"),
        );

        let periods = vec![q(2025, 1)];
        let mts = MockTs::new().with("debt_to_ebitda", periods[0], 1.0);

        let cfg = CovenantForecastConfig {
            stochastic: true,
            num_paths: 10_000,
            volatility: Some(0.25),
            random_seed: Some(42),
            mc: Some(McConfig { seed: 42, antithetic: true }),
        };
        let fc = forecast_covenant_generic(&spec, &mts, &periods, cfg).unwrap();
        let p = fc.breach_probability[0];
        assert!(p > 0.2 && p < 0.8, "unexpected breach probability: {p}");
    }
}
