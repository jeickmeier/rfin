//! Volatility surface shock adapter.
//!
//! Supports both parallel and bucketed volatility adjustments that power the
//! `OperationSpec::VolSurfaceParallelPct` and `OperationSpec::VolSurfaceBucketPct`
//! variants. The helpers rebuild the vol surface from the shocked grid so that
//! the resulting object remains Serde-friendly and deterministic.
//!
//! # Arbitrage Detection
//!
//! The [`check_arbitrage`] function can validate vol surface grids for common
//! arbitrage violations:
//! - **Calendar spread**: Total variance must be non-decreasing in expiry
//! - **Positive vol**: All volatilities must be positive
//!
//! Since vol bumps are applied via `MarketBump` (delegated to the market context),
//! arbitrage validation should be performed by the caller after scenario application
//! if needed. Large negative percentage shocks (e.g., -50%) on short-dated options
//! may produce non-positive vols.
//!
//! # Example
//!
//! ```rust,ignore
//! use finstack_scenarios::adapters::vol::{check_arbitrage, ArbitrageViolation};
//!
//! // After applying vol shocks, validate the resulting surface:
//! let violations = check_arbitrage(&expiries, &strikes, &vols);
//! for v in &violations {
//!     eprintln!("Warning: {}", v);
//! }
//! ```

use crate::error::Result;
use crate::utils::parse_tenor_to_years_with_context;
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_core::market_data::bumps::{
    BumpMode, BumpSpec, BumpType, BumpUnits, Bumpable, MarketBump,
};
use finstack_core::market_data::surfaces::VolSurface;

/// Arbitrage violation types detected in volatility surfaces.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ArbitrageViolation {
    /// Calendar spread arbitrage: total variance decreases with expiry at given strike
    CalendarSpread {
        /// Strike level where violation was detected
        strike: f64,
        /// Expiry time in years where violation was detected
        expiry: f64,
        /// Total variance at previous expiry
        prev_variance: f64,
        /// Total variance at current expiry (lower than prev, indicating arbitrage)
        curr_variance: f64,
    },
    /// Negative or zero volatility detected
    NonPositiveVol {
        /// Expiry time in years where violation was detected
        expiry: f64,
        /// Strike level where violation was detected
        strike: f64,
        /// The non-positive volatility value
        vol: f64,
    },
}

impl std::fmt::Display for ArbitrageViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArbitrageViolation::CalendarSpread {
                strike,
                expiry,
                prev_variance,
                curr_variance,
            } => write!(
                f,
                "Calendar spread arbitrage at strike={:.2}, expiry={:.4}Y: \
                 total variance decreased from {:.6} to {:.6}",
                strike, expiry, prev_variance, curr_variance
            ),
            ArbitrageViolation::NonPositiveVol {
                expiry,
                strike,
                vol,
            } => write!(
                f,
                "Non-positive vol at expiry={:.4}Y, strike={:.2}: vol={:.6}",
                expiry, strike, vol
            ),
        }
    }
}

/// Check a vol surface grid for arbitrage violations.
///
/// Validates the surface for:
/// - **Calendar spread arbitrage**: Total variance must be non-decreasing in expiry.
///   If variance decreases, a calendar spread can be constructed for risk-free profit.
/// - **Non-positive volatility**: All vols must be positive. Negative or zero vols
///   indicate either data errors or excessive negative shocks.
///
/// # Arguments
/// - `expiries`: Expiry times in years (must be strictly ascending)
/// - `strikes`: Strike levels
/// - `vols`: 2D grid of volatilities indexed as `vols[expiry_idx][strike_idx]`
///
/// # Returns
/// Vector of detected arbitrage violations (empty if surface is clean). If
/// `expiries` is not strictly ascending, a synthetic
/// [`ArbitrageViolation::CalendarSpread`] violation is returned with variance
/// fields set to NaN and further checks are short-circuited; this prevents
/// silent false-negatives when the caller accidentally passes an unsorted
/// grid.
///
/// # Example
///
/// ```rust
/// use finstack_scenarios::adapters::vol::{check_arbitrage, ArbitrageViolation};
///
/// let expiries = vec![0.25, 0.5, 1.0];
/// let strikes = vec![100.0];
/// let vols = vec![
///     vec![0.20], // 0.25Y
///     vec![0.19], // 0.5Y
///     vec![0.18], // 1Y
/// ];
///
/// let violations = check_arbitrage(&expiries, &strikes, &vols);
/// assert!(violations.is_empty(), "Surface should be arbitrage-free");
/// ```
pub fn check_arbitrage(
    expiries: &[f64],
    strikes: &[f64],
    vols: &[Vec<f64>],
) -> Vec<ArbitrageViolation> {
    let mut violations = Vec::new();

    // Calendar-spread arbitrage is defined in terms of cumulative total
    // variance along the expiry axis. If expiries are not strictly ascending
    // (or contain NaN), the variance comparison below is meaningless, so we
    // surface a diagnostic rather than returning "no violations" and
    // misleading the caller.
    for win in expiries.windows(2) {
        let (prev, next) = (win[0], win[1]);
        if !(prev.is_finite() && next.is_finite() && next > prev) {
            violations.push(ArbitrageViolation::CalendarSpread {
                strike: f64::NAN,
                expiry: next,
                prev_variance: f64::NAN,
                curr_variance: f64::NAN,
            });
            return violations;
        }
    }

    // Check calendar spread arbitrage for each strike
    for (strike_idx, &strike) in strikes.iter().enumerate() {
        let mut prev_var = 0.0;

        for (exp_idx, &expiry) in expiries.iter().enumerate() {
            if exp_idx >= vols.len() || strike_idx >= vols[exp_idx].len() {
                continue;
            }

            let vol = vols[exp_idx][strike_idx];

            // Check for non-positive vol
            if vol <= 0.0 {
                violations.push(ArbitrageViolation::NonPositiveVol {
                    expiry,
                    strike,
                    vol,
                });
                continue;
            }

            // Check total variance is non-decreasing
            let total_var = vol * vol * expiry;
            if total_var < prev_var - 1e-8 {
                violations.push(ArbitrageViolation::CalendarSpread {
                    strike,
                    expiry,
                    prev_variance: prev_var,
                    curr_variance: total_var,
                });
            }
            prev_var = total_var;
        }
    }

    violations
}

fn surface_grid(surface: &VolSurface) -> Result<Vec<Vec<f64>>> {
    surface
        .expiries()
        .iter()
        .map(|&expiry| {
            surface
                .strikes()
                .iter()
                .map(|&strike| {
                    surface
                        .value_checked(expiry, strike)
                        .map_err(crate::error::Error::from)
                })
                .collect()
        })
        .collect()
}

fn arbitrage_warnings_for_surface(surface_id: &str, surface: &VolSurface) -> Result<Vec<String>> {
    let vols = surface_grid(surface)?;
    Ok(
        check_arbitrage(surface.expiries(), surface.strikes(), &vols)
            .into_iter()
            .map(|violation| {
                format!("Vol surface '{surface_id}' post-shock arbitrage warning: {violation}")
            })
            .collect(),
    )
}

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::spec::OperationSpec;

/// Adapter for volatility surface operations.
pub struct VolAdapter;

/// Threshold for warning about large negative vol shocks that may cause arbitrage.
/// A -50% shock could produce non-positive vols for low-vol points.
const LARGE_NEGATIVE_VOL_SHOCK_PCT: f64 = -50.0;

impl ScenarioAdapter for VolAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::VolSurfaceParallelPct {
                surface_id, pct, ..
            } => {
                // NOTE: `surface_kind` is informational metadata; lookup is by surface_id only.
                let mut effects = Vec::new();
                let surface = ctx.market.get_surface(surface_id.as_str())?;

                // Warn about potentially problematic vol shocks
                if *pct <= LARGE_NEGATIVE_VOL_SHOCK_PCT {
                    effects.push(ScenarioEffect::Warning(format!(
                        "Vol surface '{}': Large negative shock ({:.1}%) may produce \
                         non-positive vols or calendar spread arbitrage. Consider using \
                         check_arbitrage() to validate post-shock surface.",
                        surface_id, pct
                    )));
                }

                // Build the BumpSpec once so the preview and the dispatched
                // MarketBump cannot diverge. A multiplicative factor of
                // `1 + pct/100` is numerically equivalent to an additive
                // percent bump of `pct` (which is how
                // `MarketBump::VolBucketPct { expiries: None, strikes: None, .. }`
                // is routed internally), so either routing produces the same
                // shocked vol surface.
                let parallel_spec = BumpSpec {
                    mode: BumpMode::Multiplicative,
                    units: BumpUnits::Factor,
                    value: 1.0 + (pct / 100.0),
                    bump_type: BumpType::Parallel,
                };

                let preview = surface.as_ref().apply_bump(parallel_spec)?;
                for warning in arbitrage_warnings_for_surface(surface_id, &preview)? {
                    effects.push(ScenarioEffect::Warning(warning));
                }

                let bump = MarketBump::Curve {
                    id: finstack_core::types::CurveId::from(surface_id.as_str()),
                    spec: parallel_spec,
                };
                effects.push(ScenarioEffect::MarketBump(bump));

                Ok(Some(effects))
            }
            OperationSpec::VolSurfaceBucketPct {
                surface_id,
                tenors,
                strikes,
                pct,
                ..
            } => {
                // NOTE: `surface_kind` is informational metadata; lookup is by surface_id only.
                // The market context stores all vol surfaces in a single collection keyed by ID.
                let mut warnings = Vec::new();
                let surface = ctx.market.get_surface(surface_id.as_str())?;

                // Warn about potentially problematic vol shocks
                if *pct <= LARGE_NEGATIVE_VOL_SHOCK_PCT {
                    warnings.push(format!(
                        "Vol surface '{}': Large negative bucket shock ({:.1}%) may produce \
                         non-positive vols or calendar spread arbitrage. Consider using \
                         check_arbitrage() to validate post-shock surface.",
                        surface_id, pct
                    ));
                }

                let exp_years = if let Some(t) = tenors {
                    let parsed: std::result::Result<Vec<f64>, _> = t
                        .iter()
                        .map(|s| {
                            parse_tenor_to_years_with_context(
                                s,
                                ctx.as_of,
                                ctx.calendar,
                                BusinessDayConvention::Unadjusted,
                                DayCount::Act365F,
                            )
                        })
                        .collect();
                    Some(parsed?)
                } else {
                    None
                };

                let preview = surface
                    .apply_bucket_bump(exp_years.as_deref(), strikes.as_deref(), *pct)
                    .ok_or_else(|| {
                        finstack_core::Error::from(finstack_core::InputError::DimensionMismatch)
                    })?;
                warnings.extend(arbitrage_warnings_for_surface(surface_id, &preview)?);

                let bump = MarketBump::VolBucketPct {
                    surface_id: finstack_core::types::CurveId::from(surface_id.as_str()),
                    expiries: exp_years,
                    strikes: strikes.clone(),
                    pct: *pct,
                };

                let mut effects = vec![ScenarioEffect::MarketBump(bump)];
                for w in warnings {
                    effects.push(ScenarioEffect::Warning(w));
                }

                Ok(Some(effects))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ExecutionContext;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::surfaces::VolSurface;
    use time::macros::date;

    #[test]
    fn test_arbitrage_detection_calendar_spread() {
        // Create a surface with calendar spread arbitrage
        // Vol decreases with expiry (arbitrage)
        let expiries = vec![0.25, 0.5, 1.0];
        let strikes = vec![100.0];
        let vols = vec![
            vec![0.3],  // 0.25Y: var = 0.3^2 * 0.25 = 0.0225
            vec![0.2],  // 0.5Y: var = 0.2^2 * 0.5 = 0.02 < 0.0225 (arbitrage!)
            vec![0.15], // 1Y: var = 0.15^2 * 1.0 = 0.0225
        ];

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert!(!violations.is_empty());
        assert!(matches!(
            &violations[0],
            ArbitrageViolation::CalendarSpread { .. }
        ));
    }

    #[test]
    fn test_arbitrage_detection_non_positive() {
        let expiries = vec![0.5];
        let strikes = vec![100.0, 110.0];
        let vols = vec![vec![0.2, -0.1]]; // Negative vol

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert!(!violations.is_empty());
        assert!(matches!(
            &violations[0],
            ArbitrageViolation::NonPositiveVol { .. }
        ));
    }

    #[test]
    fn test_arbitrage_rejects_unsorted_expiries() {
        let expiries = vec![1.0, 0.5]; // descending, not ascending
        let strikes = vec![100.0];
        let vols = vec![vec![0.20], vec![0.25]];

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert_eq!(violations.len(), 1);
        assert!(matches!(
            &violations[0],
            ArbitrageViolation::CalendarSpread { prev_variance, curr_variance, .. }
                if prev_variance.is_nan() && curr_variance.is_nan()
        ));
    }

    #[test]
    fn test_arbitrage_rejects_duplicate_expiries() {
        let expiries = vec![0.5, 0.5]; // duplicates violate strict ordering
        let strikes = vec![100.0];
        let vols = vec![vec![0.20], vec![0.25]];

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_no_arbitrage_clean_surface() {
        // Create a clean surface with no arbitrage
        let expiries = vec![0.25, 0.5, 1.0];
        let strikes = vec![90.0, 100.0, 110.0];
        let vols = vec![
            vec![0.25, 0.20, 0.22], // 0.25Y
            vec![0.24, 0.19, 0.21], // 0.5Y (variance increases)
            vec![0.22, 0.18, 0.20], // 1Y (variance increases)
        ];

        let violations = check_arbitrage(&expiries, &strikes, &vols);
        assert!(violations.is_empty());
    }

    /// End-to-end check that `VolSurfaceParallelPct` routes its `MarketBump`
    /// through `MarketContext::bump` and actually mutates the stored surface.
    /// This pins the semantics of the multiplicative factor (`1 + pct/100`)
    /// against the surface's own `apply_bump` implementation and ensures that
    /// the `MarketBump::Curve` variant correctly dispatches to surfaces (not
    /// just term-structure curves).
    #[test]
    fn test_vol_surface_parallel_pct_integration() -> crate::error::Result<()> {
        use crate::engine::ScenarioEngine;
        use crate::spec::{ScenarioSpec, VolSurfaceKind};

        let surface = VolSurface::builder("VOL")
            .expiries(&[0.5, 1.0])
            .strikes(&[100.0])
            .row(&[0.20])
            .row(&[0.22])
            .build()
            .map_err(crate::error::Error::from)?;
        let mut market = MarketContext::new().insert_surface(surface);
        let mut model = finstack_statements::FinancialModelSpec::new("test", vec![]);

        let scenario = ScenarioSpec {
            id: "vol_parallel".into(),
            name: None,
            description: None,
            operations: vec![OperationSpec::VolSurfaceParallelPct {
                surface_id: "VOL".into(),
                surface_kind: VolSurfaceKind::Equity,
                pct: 10.0,
            }],
            priority: 0,
            resolution_mode: Default::default(),
        };

        let engine = ScenarioEngine::new();
        {
            let mut ctx = ExecutionContext {
                market: &mut market,
                model: &mut model,
                instruments: None,
                rate_bindings: None,
                calendar: None,
                as_of: date!(2025 - 01 - 01),
            };
            let report = engine.apply(&scenario, &mut ctx)?;
            assert_eq!(report.operations_applied, 1);
        }

        let bumped = market.get_surface("VOL")?;
        let v_05 = bumped
            .value_checked(0.5, 100.0)
            .map_err(crate::error::Error::from)?;
        let v_10 = bumped
            .value_checked(1.0, 100.0)
            .map_err(crate::error::Error::from)?;
        assert!(
            (v_05 - 0.22).abs() < 1e-10,
            "expected 0.20 * 1.10 = 0.22, got {v_05}"
        );
        assert!(
            (v_10 - 0.242).abs() < 1e-10,
            "expected 0.22 * 1.10 = 0.242, got {v_10}"
        );
        Ok(())
    }

    #[test]
    fn test_bucket_shock_warns_on_post_bump_arbitrage() -> crate::error::Result<()> {
        let surface = VolSurface::builder("VOL")
            .expiries(&[0.25, 0.5])
            .strikes(&[100.0])
            .row(&[0.30])
            .row(&[0.22])
            .build()
            .map_err(crate::error::Error::from)?;
        let mut market = MarketContext::new().insert_surface(surface);
        let mut model = finstack_statements::FinancialModelSpec::new("test", vec![]);
        let ctx = ExecutionContext {
            market: &mut market,
            model: &mut model,
            instruments: None,
            rate_bindings: None,
            calendar: None,
            as_of: date!(2025 - 01 - 01),
        };

        let effects = VolAdapter
            .try_generate_effects(
                &OperationSpec::VolSurfaceBucketPct {
                    surface_id: "VOL".into(),
                    surface_kind: crate::spec::VolSurfaceKind::Equity,
                    tenors: Some(vec!["6M".into()]),
                    strikes: None,
                    pct: -30.0,
                },
                &ctx,
            )?
            .ok_or_else(|| crate::error::Error::Internal("vol op should be handled".to_string()))?;

        assert!(effects.iter().any(|effect| matches!(
            effect,
            ScenarioEffect::Warning(message) if message.contains("post-shock arbitrage warning")
        )));
        Ok(())
    }
}
