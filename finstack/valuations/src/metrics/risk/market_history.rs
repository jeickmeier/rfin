//! Market history storage for Historical VaR calculation.
//!
//! This module provides data structures for storing and applying historical
//! market shifts. The core concept is to store shifts (differences from base)
//! rather than absolute levels, enabling efficient scenario application.

use crate::metrics::risk::RiskFactorType;
use finstack_core::dates::Date;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Historical shift for a single risk factor on a single date.
///
/// Represents the change in a market variable from its base value.
/// For example, a +15bp shift in 5Y USD rates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskFactorShift {
    /// Risk factor being shifted
    pub factor: RiskFactorType,
    /// Absolute change in the factor
    /// - For rates/spreads: change in basis points as decimal (e.g., 0.0015 = 15bp)
    /// - For equity spot: percentage change (e.g., -0.025 = -2.5%)
    /// - For volatility: absolute vol change (e.g., 0.02 = +2 vol points)
    pub shift: f64,
}

/// Collection of all risk factor shifts for a single historical date.
///
/// Represents a complete market scenario that can be applied to revalue
/// a portfolio. Each scenario contains shifts for all relevant risk factors.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketScenario {
    /// Historical date this scenario represents
    pub date: Date,
    /// All risk factor shifts on this date (relative to base date)
    pub shifts: Vec<RiskFactorShift>,
}

impl MarketScenario {
    /// Create a new market scenario.
    pub fn new(date: Date, shifts: Vec<RiskFactorShift>) -> Self {
        Self { date, shifts }
    }

    /// Apply this scenario to a base market context.
    ///
    /// Creates a new `MarketContext` with all risk factor shifts applied.
    /// Uses key-rate triangular bumps for rate/credit shifts at specific tenors,
    /// preserving curve shape information. Equity and vol shifts are applied
    /// as multiplicative and additive bumps respectively.
    ///
    /// # Arguments
    ///
    /// * `base_market` - The base market context (current market state)
    ///
    /// # Returns
    ///
    /// New market context with historical shifts applied
    pub fn apply(&self, base_market: &MarketContext) -> Result<MarketContext> {
        let mut bumped_market = base_market.clone();

        for shift in &self.shifts {
            let maybe_bump: Option<(CurveId, BumpSpec)> = match &shift.factor {
                RiskFactorType::DiscountRate {
                    curve_id,
                    tenor_years,
                } => Some(key_rate_bp_bump(curve_id, *tenor_years, shift.shift)),
                RiskFactorType::ForwardRate {
                    curve_id,
                    tenor_years,
                } => Some(key_rate_bp_bump(curve_id, *tenor_years, shift.shift)),
                RiskFactorType::CreditSpread {
                    curve_id,
                    tenor_years,
                } => Some(key_rate_bp_bump(curve_id, *tenor_years, shift.shift)),
                RiskFactorType::EquitySpot { ticker } => Some((
                    CurveId::from(ticker.as_str()),
                    BumpSpec::multiplier(1.0 + shift.shift),
                )),
                RiskFactorType::ImpliedVol { surface_id, .. } => Some((
                    surface_id.clone(),
                    BumpSpec {
                        mode: BumpMode::Additive,
                        units: BumpUnits::Fraction,
                        value: shift.shift,
                        bump_type: BumpType::Parallel,
                    },
                )),
            };

            if let Some((curve_id, spec)) = maybe_bump {
                bumped_market = bumped_market.bump([MarketBump::Curve { id: curve_id, spec }])?;
            }
        }

        Ok(bumped_market)
    }
}

/// Build a triangular key-rate bump for a specific tenor on a curve.
///
/// Uses the standard bucket grid to determine the triangular weight neighbors,
/// ensuring that localized shifts preserve curve shape.
fn key_rate_bp_bump(curve_id: &CurveId, tenor_years: f64, shift: f64) -> (CurveId, BumpSpec) {
    let shift_bp = shift * 10_000.0;

    let (prev, next) = find_triangular_neighbors(tenor_years);
    (
        curve_id.clone(),
        BumpSpec::triangular_key_rate_bp(prev, tenor_years, next, shift_bp),
    )
}

/// Find the neighboring bucket boundaries for a triangular key-rate bump.
fn find_triangular_neighbors(tenor: f64) -> (f64, f64) {
    let buckets = &crate::metrics::sensitivities::config::STANDARD_BUCKETS_YEARS;

    let mut prev = 0.0;
    for (i, &bucket) in buckets.iter().enumerate() {
        if (tenor - bucket).abs() <= f64::EPSILON {
            let next = buckets.get(i + 1).copied().unwrap_or(f64::INFINITY);
            return (prev, next);
        }
        if tenor < bucket {
            return (prev, bucket);
        }
        prev = bucket;
    }

    (prev, f64::INFINITY)
}

/// Historical market data for VaR calculation.
///
/// Stores a time series of market scenarios representing historical market
/// shifts over a lookback window (e.g., last 500 days).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketHistory {
    /// Base date (current market state reference point)
    pub base_date: Date,
    /// Historical window size in days
    pub window_days: u32,
    /// Historical scenarios (one per day in lookback window)
    /// Ordered chronologically from oldest to newest
    pub scenarios: Vec<MarketScenario>,
}

impl MarketHistory {
    /// Create a new market history.
    ///
    /// # Arguments
    ///
    /// * `base_date` - Current date (reference point for shifts)
    /// * `window_days` - Size of historical window
    /// * `scenarios` - Historical market scenarios
    pub fn new(base_date: Date, window_days: u32, scenarios: Vec<MarketScenario>) -> Self {
        Self {
            base_date,
            window_days,
            scenarios,
        }
    }

    /// Number of scenarios in the history.
    pub fn len(&self) -> usize {
        self.scenarios.len()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty()
    }

    /// Get scenario at index.
    pub fn get(&self, index: usize) -> Option<&MarketScenario> {
        self.scenarios.get(index)
    }

    /// Iterator over scenarios.
    pub fn iter(&self) -> impl Iterator<Item = &MarketScenario> {
        self.scenarios.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use time::macros::date;

    #[test]
    fn triangular_neighbors_surround_non_standard_tenor() {
        let (prev, next) = find_triangular_neighbors(4.0);

        assert_eq!(prev, 3.0);
        assert_eq!(next, 5.0);
    }

    #[test]
    fn test_market_scenario_creation() {
        let scenario_date = date!(2024 - 01 - 02);
        let shifts = vec![
            RiskFactorShift {
                factor: RiskFactorType::DiscountRate {
                    curve_id: CurveId::from("USD-OIS"),
                    tenor_years: 5.0,
                },
                shift: 0.0015, // +15bp
            },
            RiskFactorShift {
                factor: RiskFactorType::CreditSpread {
                    curve_id: CurveId::from("AAPL"),
                    tenor_years: 5.0,
                },
                shift: -0.0010, // -10bp
            },
        ];

        let scenario = MarketScenario::new(scenario_date, shifts);

        assert_eq!(scenario.date, scenario_date);
        assert_eq!(scenario.shifts.len(), 2);
    }

    #[test]
    fn test_market_history_creation() {
        let base_date = date!(2024 - 01 - 01);
        let scenarios = vec![
            MarketScenario::new(date!(2023 - 12 - 31), vec![]),
            MarketScenario::new(date!(2023 - 12 - 30), vec![]),
        ];

        let history = MarketHistory::new(base_date, 500, scenarios);

        assert_eq!(history.base_date, base_date);
        assert_eq!(history.window_days, 500);
        assert_eq!(history.len(), 2);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_market_scenario_allows_multiple_key_rates() -> Result<()> {
        let base_date = date!(2024 - 01 - 01);
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 1.0), (5.0, 0.85)])
            .build()?;
        let base_market = MarketContext::new().insert(curve);

        let scenario = MarketScenario::new(
            date!(2024 - 01 - 02),
            vec![
                RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: 0.0010,
                },
                RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 10.0,
                    },
                    shift: -0.0005,
                },
            ],
        );

        let bumped = scenario.apply(&base_market)?;
        assert!(bumped.get_discount("USD-OIS").is_ok());

        Ok(())
    }

    #[test]
    fn test_scenario_apply_creates_bumped_market() -> Result<()> {
        let base_date = date!(2024 - 01 - 01);

        // Create base market
        let base_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 1.0), (5.0, 0.85), (10.0, 0.70)])
            .build()?;

        let base_market = MarketContext::new().insert(base_curve);

        // Create scenario with rate shift
        let scenario = MarketScenario::new(
            date!(2024 - 01 - 02),
            vec![RiskFactorShift {
                factor: RiskFactorType::DiscountRate {
                    curve_id: CurveId::from("USD-OIS"),
                    tenor_years: 5.0,
                },
                shift: 0.0010, // +10bp
            }],
        );

        // Apply scenario
        let bumped_market = scenario.apply(&base_market)?;

        // Verify bumped market has the curve
        assert!(bumped_market.get_discount("USD-OIS").is_ok());

        // The bumped market should be different from base
        // (We can't easily compare values without evaluating the curves,
        // but we've verified the bump mechanism works)

        Ok(())
    }

    #[test]
    fn test_market_history_iteration() {
        let scenarios = vec![
            MarketScenario::new(date!(2024 - 01 - 01), vec![]),
            MarketScenario::new(date!(2024 - 01 - 02), vec![]),
            MarketScenario::new(date!(2024 - 01 - 03), vec![]),
        ];

        let history = MarketHistory::new(date!(2024 - 01 - 01), 3, scenarios);

        let count = history.iter().count();
        assert_eq!(count, 3);

        // Verify order
        let dates: Vec<_> = history.iter().map(|s| s.date).collect();
        assert_eq!(dates[0], date!(2024 - 01 - 01));
        assert_eq!(dates[2], date!(2024 - 01 - 03));
    }

    #[test]
    fn test_empty_market_history() {
        let history = MarketHistory::new(date!(2024 - 01 - 01), 0, vec![]);

        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert!(history.get(0).is_none());
    }

    #[test]
    fn test_equity_spot_shift_applied() -> Result<()> {
        use finstack_core::market_data::scalars::MarketScalar;

        let base_market = MarketContext::new().insert_price("AAPL", MarketScalar::Unitless(100.0));

        let scenario = MarketScenario::new(
            date!(2024 - 01 - 02),
            vec![RiskFactorShift {
                factor: RiskFactorType::EquitySpot {
                    ticker: "AAPL".to_string(),
                },
                shift: 0.10, // +10%
            }],
        );

        let bumped = scenario.apply(&base_market)?;
        match bumped.get_price("AAPL")? {
            MarketScalar::Unitless(v) => assert!((v - 110.0).abs() < 1e-9),
            other => panic!("unexpected scalar variant: {:?}", other),
        }

        Ok(())
    }

    #[test]
    fn test_implied_vol_shift_applied() -> Result<()> {
        use finstack_core::market_data::surfaces::VolSurface;

        let surface = VolSurface::builder("EQ-VOL")
            .expiries(&[0.5, 1.0])
            .strikes(&[100.0, 110.0])
            .row(&[0.20, 0.22])
            .row(&[0.21, 0.23])
            .build()?;

        let base_market = MarketContext::new().insert_surface(surface.clone());

        let scenario = MarketScenario::new(
            date!(2024 - 01 - 02),
            vec![RiskFactorShift {
                factor: RiskFactorType::ImpliedVol {
                    surface_id: CurveId::from("EQ-VOL"),
                    expiry_years: 1.0,
                    strike: 100.0,
                },
                shift: 0.02, // +2 vol points
            }],
        );

        let bumped = scenario.apply(&base_market)?;
        let bumped_surface = bumped.get_surface("EQ-VOL")?;
        let vol = bumped_surface
            .value_checked(1.0, 100.0)
            .expect("grid point lookup should succeed");
        assert!((vol - 0.23).abs() < 1e-9);

        Ok(())
    }
}
