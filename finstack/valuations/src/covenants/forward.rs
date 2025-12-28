//! Covenant forward-projection with headroom analytics.
//!
//! This module provides generic covenant forecasting that can be driven by any
//! time-series model implementing the [`ModelTimeSeries`] trait. A thin
//! statements-specific adapter is provided behind the `statements_bridge` feature
//! so this module remains usable without introducing a crate cycle.

use crate::covenants::engine::{
    headroom_for, BoundKind, CovenantSpec, CovenantType, SpringingCondition, ThresholdTest,
};
use finstack_core::dates::{Date, PeriodId};
use finstack_core::error::Error;
use finstack_core::error::InputError;
use finstack_core::Result;
use serde::{Deserialize, Serialize};

#[cfg(feature = "mc")]
use crate::instruments::common::mc::traits::RandomStream;

/// Comparator for headroom calculation.
/// Comparison operator for covenant threshold tests
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparator {
    /// Less than or equal to threshold (e.g., Leverage ≤ 3.0x)
    LessOrEqual,
    /// Greater than or equal to threshold (e.g., Coverage ≥ 1.2x)
    GreaterOrEqual,
}

impl From<BoundKind> for Comparator {
    fn from(kind: BoundKind) -> Self {
        match kind {
            BoundKind::AtMost => Comparator::LessOrEqual,
            BoundKind::AtLeast => Comparator::GreaterOrEqual,
        }
    }
}

/// MC configuration (subset; integrates with instruments/common/mc RNG).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McConfig {
    /// When true, uses antithetic variates (simple variance reduction).
    pub antithetic: bool,
}

/// Covenant forecast configuration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CovenantForecastConfig {
    /// Whether to use stochastic simulation (vs deterministic projection)
    pub stochastic: bool,
    /// Number of Monte Carlo paths (if stochastic)
    pub num_paths: usize,
    /// Volatility for stochastic scenarios
    pub volatility: Option<f64>,
    /// Random seed for reproducibility
    pub random_seed: Option<u64>,
    /// Monte Carlo configuration
    pub mc: Option<McConfig>,
}

/// Forecast output with headroom analytics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CovenantForecast {
    /// Covenant identifier
    pub covenant_id: String,
    /// Future test dates for covenant evaluation
    pub test_dates: Vec<Date>,
    /// Projected metric values at each test date
    pub projected_values: Vec<f64>,
    /// Covenant thresholds at each test date
    pub thresholds: Vec<f64>,
    /// Headroom (distance from breach) at each test date
    pub headroom: Vec<f64>,
    /// Probability of breach at each test date (stochastic mode)
    pub breach_probability: Vec<f64>,
    /// Date of first projected breach (if any)
    pub first_breach_date: Option<Date>,
    /// Date with minimum headroom
    pub min_headroom_date: Date,
    /// Minimum headroom value across all test dates
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

/// A projected covenant breach.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FutureBreach {
    /// Covenant identifier
    pub covenant_id: String,
    /// Date of the breach
    pub breach_date: Date,
    /// Projected value
    pub projected_value: f64,
    /// Threshold value
    pub threshold: f64,
    /// Headroom (negative means breach)
    pub headroom: f64,
    /// Probability of breach (if stochastic)
    pub breach_probability: f64,
}

/// Minimal read-only adapter to query model time-series values and map periods to dates.
pub trait ModelTimeSeries {
    /// Get scalar value for a metric node and period
    fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64>;
    /// Get end date for a given period
    fn period_end_date(&self, period: &PeriodId) -> Date;
}

/// Forecast a covenant using a generic time-series model adapter.
#[cfg_attr(not(feature = "mc"), allow(unused_variables))]
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
    let bound_kind = covenant
        .covenant
        .covenant_type
        .bound_kind()
        .ok_or(Error::from(InputError::Invalid))?;
    let base_threshold = covenant
        .covenant
        .covenant_type
        .threshold_value()
        .ok_or(Error::from(InputError::Invalid))?;

    // Resolve thresholds and values
    let mut test_dates: Vec<Date> = Vec::with_capacity(periods.len());
    let mut thresholds: Vec<f64> = Vec::with_capacity(periods.len());
    let mut values: Vec<f64> = Vec::with_capacity(periods.len());
    let mut activation_flags: Vec<bool> = Vec::with_capacity(periods.len());

    for pid in periods {
        let date = model.period_end_date(pid);
        test_dates.push(date);
        thresholds.push(base_threshold);

        let is_active =
            springing_condition_active(covenant.covenant.springing_condition.as_ref(), model, pid)?;
        activation_flags.push(is_active);

        let v = metric_value_for_spec(covenant, model, pid).ok_or_else(|| {
            Error::from(finstack_core::error::InputError::NotFound {
                id: "metric_value".to_string(),
            })
        })?;
        values.push(v);
    }

    // Deterministic headroom and breach flag
    let mut headroom: Vec<f64> = values
        .iter()
        .zip(thresholds.iter())
        .map(|(&v, &t)| headroom_for(&covenant.covenant.covenant_type, v, t))
        .collect();

    let mut deterministic_breach_prob: Vec<f64> = values
        .iter()
        .zip(thresholds.iter())
        .map(|(&v, &t)| match bound_kind {
            BoundKind::AtMost => (v > t) as u8 as f64,
            BoundKind::AtLeast => (v < t) as u8 as f64,
        })
        .collect();

    for (i, active) in activation_flags.iter().enumerate() {
        if !active {
            headroom[i] = f64::INFINITY;
            deterministic_breach_prob[i] = 0.0;
        }
    }

    #[cfg(feature = "mc")]
    let mut breach_probability = deterministic_breach_prob.clone();
    #[cfg(not(feature = "mc"))]
    let breach_probability = deterministic_breach_prob.clone();

    // Optional MC overlay (multiplicative shock to metric value)
    #[cfg(feature = "mc")]
    if config.stochastic {
        let sigma = config.volatility.unwrap_or(0.0);
        let paths = config.num_paths.max(1);
        let seed = config.random_seed.unwrap_or(42);
        let antithetic = config.mc.as_ref().map(|m| m.antithetic).unwrap_or(false);

        use crate::instruments::common::mc::rng::philox::PhiloxRng;
        let mut rng = PhiloxRng::new(seed);

        // For each test date, estimate breach probability
        for i in 0..values.len() {
            if !activation_flags[i] {
                breach_probability[i] = 0.0;
                continue;
            }
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
                    let breached = match bound_kind {
                        BoundKind::AtMost => v > thr,
                        BoundKind::AtLeast => v < thr,
                    };
                    if breached {
                        breaches += 1;
                    }
                    if antithetic {
                        let shock_a = (sigma * -z).exp();
                        let v_a = base * shock_a;
                        let breached_a = match bound_kind {
                            BoundKind::AtMost => v_a > thr,
                            BoundKind::AtLeast => v_a < thr,
                        };
                        if breached_a {
                            breaches += 1;
                        }
                    }
                }
                remaining -= take;
            }
            breach_probability[i] =
                (breaches as f64) / (paths as f64 * if antithetic { 2.0 } else { 1.0 });
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
        if !activation_flags[i] {
            return None;
        }
        match bound_kind {
            BoundKind::AtMost => (v > t).then_some(test_dates[i]),
            BoundKind::AtLeast => (v < t).then_some(test_dates[i]),
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

/// Forecast breaches for all covenants in an engine.
pub fn forecast_breaches_generic<MTS: ModelTimeSeries>(
    engine: &crate::covenants::engine::CovenantEngine,
    model: &MTS,
    periods: &[PeriodId],
    config: CovenantForecastConfig,
) -> Result<Vec<FutureBreach>> {
    let mut breaches = Vec::new();

    for spec in &engine.specs {
        // Skip inactive covenants
        if !spec.covenant.is_active {
            continue;
        }

        let forecast = forecast_covenant_generic(spec, model, periods, config.clone())?;

        for (i, &headroom) in forecast.headroom.iter().enumerate() {
            // Check for breach (headroom < 0) or high probability of breach
            let is_breach = headroom < 0.0;
            let prob = forecast.breach_probability[i];

            // We report if it's a deterministic breach OR if there's a non-zero probability in stochastic mode
            if is_breach || (config.stochastic && prob > 0.0) {
                breaches.push(FutureBreach {
                    covenant_id: forecast.covenant_id.clone(),
                    breach_date: forecast.test_dates[i],
                    projected_value: forecast.projected_values[i],
                    threshold: forecast.thresholds[i],
                    headroom,
                    breach_probability: prob,
                });
            }
        }
    }

    // Sort by date then covenant ID
    breaches.sort_by(|a, b| {
        a.breach_date
            .cmp(&b.breach_date)
            .then_with(|| a.covenant_id.cmp(&b.covenant_id))
    });

    Ok(breaches)
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
    if let Some(name) = spec.covenant.covenant_type.default_metric_name() {
        if let Some(v) = model.get_scalar(name, period) {
            return Some(v);
        }
    }

    match &spec.covenant.covenant_type {
        CovenantType::Custom { metric, .. } => model.get_scalar(metric, period),
        CovenantType::Basket { name, .. } => model.get_scalar(name, period),
        CovenantType::Negative { .. } | CovenantType::Affirmative { .. } => Some(1.0),
        _ => None,
    }
}

fn springing_condition_active<MTS: ModelTimeSeries>(
    condition: Option<&SpringingCondition>,
    model: &MTS,
    period: &PeriodId,
) -> Result<bool> {
    if let Some(cond) = condition {
        let metric_name = cond.metric_id.as_str();
        let value = model.get_scalar(metric_name, period).ok_or_else(|| {
            Error::from(InputError::NotFound {
                id: format!("springing_metric:{metric_name}"),
            })
        })?;
        let active = match cond.test {
            ThresholdTest::Maximum(threshold) => value <= threshold,
            ThresholdTest::Minimum(threshold) => value >= threshold,
        };
        Ok(active)
    } else {
        Ok(true)
    }
}

// Note: Statements-specific bridging lives in the `finstack` meta crate to avoid a
// dependency cycle between `valuations` and `statements`.

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::dates::{Date, PeriodId};
    use time::Month;

    struct MockTs {
        map: finstack_core::collections::HashMap<(String, String), f64>,
    }

    impl MockTs {
        fn new() -> Self {
            Self {
                map: finstack_core::collections::HashMap::default(),
            }
        }
        fn with(mut self, node: &str, period: PeriodId, v: f64) -> Self {
            self.map.insert((node.to_string(), period.to_string()), v);
            self
        }
    }

    impl ModelTimeSeries for MockTs {
        fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64> {
            self.map
                .get(&(node_id.to_string(), period.to_string()))
                .copied()
        }
        fn period_end_date(&self, period: &PeriodId) -> Date {
            // simple quarterly end approximation
            let m = [3u8, 6, 9, 12][(period.index as usize - 1).min(3)];
            Date::from_calendar_date(
                period.year,
                Month::try_from(m).expect("Valid month (1-12)"),
                30,
            )
            .expect("Valid test date")
        }
    }

    fn q(year: i32, q: u8) -> PeriodId {
        PeriodId::quarter(year, q)
    }

    #[test]
    fn deterministic_headroom_positive_zero_breach_prob() {
        // Debt/EBITDA <= 5, actual ratio at 4 → positive headroom
        let spec = CovenantSpec::with_metric(
            crate::covenants::engine::Covenant::new(
                CovenantType::MaxDebtToEBITDA { threshold: 5.0 },
                finstack_core::dates::Tenor::quarterly(),
            ),
            crate::metrics::MetricId::custom("debt_to_ebitda"),
        );

        let periods = vec![q(2025, 1), q(2025, 2)];
        let mts = MockTs::new().with("debt_to_ebitda", periods[0], 4.0).with(
            "debt_to_ebitda",
            periods[1],
            4.2,
        );

        let cfg = CovenantForecastConfig::default();
        let fc = forecast_covenant_generic(&spec, &mts, &periods, cfg)
            .expect("Forecast covenant should succeed in test");

        assert!(fc.headroom.iter().all(|&h| h > 0.0));
        assert!(fc
            .breach_probability
            .iter()
            .all(|&p| (p - 0.0).abs() < 1e-12));
        assert!(fc.first_breach_date.is_none());
    }

    #[test]
    #[cfg(feature = "mc")]
    fn stochastic_breach_probability_moves_with_vol() {
        // Debt/EBITDA <= 1.0, base ~ 1.0; with high vol, breach prob should be material
        let spec = CovenantSpec::with_metric(
            crate::covenants::engine::Covenant::new(
                CovenantType::MaxDebtToEBITDA { threshold: 1.0 },
                finstack_core::dates::Tenor::quarterly(),
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
            mc: Some(McConfig { antithetic: true }),
        };
        let fc = forecast_covenant_generic(&spec, &mts, &periods, cfg)
            .expect("Forecast covenant should succeed in test");
        let p = fc.breach_probability[0];
        assert!(p > 0.2 && p < 0.8, "unexpected breach probability: {p}");
    }
    #[test]
    fn test_forecast_breaches_generic() {
        use crate::covenants::engine::CovenantEngine;

        let mut engine = CovenantEngine::new();
        let covenant = crate::covenants::engine::Covenant::new(
            crate::covenants::engine::CovenantType::MaxDebtToEBITDA { threshold: 3.0 },
            finstack_core::dates::Tenor::quarterly(),
        );
        let spec = CovenantSpec {
            covenant,
            metric_id: Some(crate::metrics::MetricId::custom("NetDebtEbitda")),
            custom_evaluator: None,
        };
        engine.add_spec(spec);

        let p1 = q(2025, 1);
        let p2 = q(2025, 2);

        let mut adapter = MockTs::new();
        adapter = adapter.with("NetDebtEbitda", p1, 2.5); // Pass
        adapter = adapter.with("NetDebtEbitda", p2, 3.5); // Fail

        let periods = vec![p1, p2];
        let config = CovenantForecastConfig::default();

        let breaches = forecast_breaches_generic(&engine, &adapter, &periods, config)
            .expect("Forecast should succeed");

        assert_eq!(breaches.len(), 1);
        assert_eq!(breaches[0].covenant_id, "Debt/EBITDA <= 3.00x");
        assert_eq!(breaches[0].projected_value, 3.5);
    }
}
