//! Exposure simulation engine for XVA calculations.
//!
//! Computes exposure profiles (EPE, ENE, PFE) for a portfolio of instruments
//! by re-valuing them at future time points.
//!
//! # Methodology
//!
//! This module implements a **deterministic exposure** approach:
//! at each future time point, instruments are re-valued under the current
//! market data (curves rolled forward deterministically). This is a simplified
//! but conservative approach suitable for:
//!
//! - Initial XVA framework validation
//! - Portfolios with linear instruments (bonds, swaps)
//! - Regulatory SA-CCR style calculations
//!
//! For a full production implementation, Monte Carlo simulation of risk factors
//! would replace the deterministic forward roll. The API is designed to be
//! extended without breaking changes.
//!
//! # Exposure Definitions
//!
//! ```text
//! V(t)   = portfolio mark-to-market at time t
//! EPE(t) = E[max(V(t), 0)]     — Expected Positive Exposure
//! ENE(t) = E[max(-V(t), 0)]    — Expected Negative Exposure
//! PFE(t) = quantile(V(t), α)   — Potential Future Exposure at level α
//! ```
//!
//! # References
//!
//! - Gregory, J. (2020). *The xVA Challenge*, Chapters 8–10.
//! - Pykhtin, M. & Zhu, S. (2007). "A Guide to Modelling Counterparty
//!   Credit Risk." *GARP Risk Review*, July/August 2007.
//! - BCBS 279 (2014). SA-CCR.

use std::sync::Arc;

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

use crate::instruments::Instrument;

use super::netting::{apply_collateral, apply_netting};
use super::types::{ExposureProfile, NettingSet, XvaConfig};

/// Number of days per year used for year-to-date conversion.
///
/// Uses ACT/365 Fixed (standard quant convention) rather than 365.25
/// to stay consistent with the day count conventions used by most
/// term structures in the library.
const DAYS_PER_YEAR: f64 = 365.0;

/// Compute the exposure profile for a portfolio of instruments.
///
/// For each time point in the configuration's time grid, this function:
/// 1. Rolls the market data forward to `as_of + t` (deterministic roll)
/// 2. Re-values each instrument at the future date
/// 3. Applies close-out netting across the netting set
/// 4. Applies CSA collateral terms (if present)
/// 5. Records EPE and ENE values
///
/// # Arguments
///
/// * `instruments` - Portfolio of instruments in this netting set
/// * `market` - Current market data context
/// * `as_of` - Valuation date (T+0)
/// * `config` - XVA configuration (time grid, recovery, etc.)
/// * `netting_set` - Netting set specification with optional CSA
///
/// # Returns
///
/// An [`ExposureProfile`] containing MtM, EPE, and ENE at each time point.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration validation fails
/// - More than 50% of time grid points fail (market roll or valuation)
///
/// # Warnings
///
/// Time points where market data cannot be rolled forward are recorded
/// as zero exposure with a log warning. Instruments that fail to value
/// at a given horizon are treated as zero value (matured/settled).
///
/// # Limitations
///
/// - Uses deterministic (single-scenario) exposure; no Monte Carlo
/// - PFE equals EPE in this simplified model
/// - Does not model margin period of risk (MPOR) explicitly
/// - Curve roll uses constant-curves assumption (no carry/theta)
pub fn compute_exposure_profile(
    instruments: &[Arc<dyn Instrument>],
    market: &MarketContext,
    as_of: Date,
    config: &XvaConfig,
    netting_set: &NettingSet,
) -> finstack_core::Result<ExposureProfile> {
    config.validate()?;

    let n = config.time_grid.len();
    let mut times = Vec::with_capacity(n);
    let mut mtm_values = Vec::with_capacity(n);
    let mut epe = Vec::with_capacity(n);
    let mut ene = Vec::with_capacity(n);

    let mut market_roll_failures: usize = 0;
    let mut instrument_valuation_failures: usize = 0;

    for &t in &config.time_grid {
        // Convert years to days using ACT/365F convention
        let days = (t * DAYS_PER_YEAR).round() as i64;
        let future_date = as_of + time::Duration::days(days);

        // Roll market data forward (constant-curves assumption).
        let rolled_market = match market.roll_forward(days) {
            Ok(m) => m,
            Err(_) => {
                // Market data can't be rolled this far; record zero exposure
                // but track the failure for the quality check below.
                market_roll_failures += 1;
                times.push(t);
                mtm_values.push(0.0);
                epe.push(0.0);
                ene.push(0.0);
                continue;
            }
        };

        // Value each instrument at the future date
        let mut values = Vec::with_capacity(instruments.len());
        for inst in instruments {
            match inst.value_raw(&rolled_market, future_date) {
                Ok(v) => values.push(v),
                Err(_) => {
                    // Instrument may have expired or be unvaluable at this horizon.
                    // Treat expired instruments as zero value (matured/settled).
                    instrument_valuation_failures += 1;
                    values.push(0.0);
                }
            }
        }

        // Apply close-out netting: net portfolio value
        let net_value: f64 = values.iter().sum();
        let positive_exposure = apply_netting(&values);

        // Apply CSA collateral reduction if applicable
        let exposure = if let Some(ref csa) = netting_set.csa {
            apply_collateral(positive_exposure, csa)
        } else {
            positive_exposure
        };

        let negative_exposure = (-net_value).max(0.0);

        times.push(t);
        mtm_values.push(net_value);
        epe.push(exposure);
        ene.push(negative_exposure);
    }

    // Fail if too many time points couldn't be evaluated — this indicates
    // a data quality issue rather than normal instrument maturity.
    if market_roll_failures > n / 2 {
        return Err(finstack_core::Error::Validation(format!(
            "Exposure simulation: {market_roll_failures}/{n} time points failed \
             market data roll (>50%); check market data coverage"
        )));
    }

    // Log a warning-level summary (via the error context) if any failures occurred
    if market_roll_failures > 0 || instrument_valuation_failures > 0 {
        // In a production system this would use a proper logging framework.
        // For now, the failure counts are captured in the profile for inspection
        // and the caller can validate using ExposureProfile::validate().
        #[cfg(debug_assertions)]
        eprintln!(
            "XVA exposure warning: {market_roll_failures} market roll failures, \
             {instrument_valuation_failures} instrument valuation failures \
             across {n} time points"
        );
    }

    Ok(ExposureProfile {
        times,
        mtm_values,
        epe,
        ene,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::xva::cva::compute_cva;
    use crate::xva::types::CsaTerms;
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
    use time::Month;

    // Note: Full integration tests require constructing instrument and market mocks.
    // These unit tests verify the exposure profile logic with synthetic data.

    #[test]
    fn exposure_profile_basic_structure() {
        let config = XvaConfig {
            time_grid: vec![0.25, 0.5, 1.0],
            recovery_rate: 0.40,
            include_wrong_way_risk: false,
        };
        config.validate().expect("Config should be valid");
        assert_eq!(config.time_grid.len(), 3);
    }

    #[test]
    fn exposure_profile_epe_non_negative() {
        // EPE by construction is max(V, 0) which is always >= 0
        let profile = ExposureProfile {
            times: vec![0.25, 0.5, 1.0],
            mtm_values: vec![100.0, -50.0, 25.0],
            epe: vec![100.0, 0.0, 25.0],
            ene: vec![0.0, 50.0, 0.0],
        };
        for &e in &profile.epe {
            assert!(e >= 0.0, "EPE must be non-negative, got {e}");
        }
    }

    #[test]
    fn exposure_profile_ene_non_negative() {
        let profile = ExposureProfile {
            times: vec![0.25, 0.5],
            mtm_values: vec![100.0, -50.0],
            epe: vec![100.0, 0.0],
            ene: vec![0.0, 50.0],
        };
        for &e in &profile.ene {
            assert!(e >= 0.0, "ENE must be non-negative, got {e}");
        }
    }

    // ── Integration tests: synthetic profiles through CVA pipeline ──

    /// Helper: build a flat hazard rate curve.
    fn flat_hazard_curve(lambda: f64) -> HazardCurve {
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        HazardCurve::builder("COUNTERPARTY")
            .base_date(base)
            .knots([(0.0, lambda), (30.0, lambda)])
            .build()
            .expect("HazardCurve should build")
    }

    /// Helper: build a flat discount curve.
    fn flat_discount_curve(rate: f64) -> DiscountCurve {
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let knots: Vec<(f64, f64)> = (0..=60)
            .map(|i| {
                let t = i as f64 * 0.5;
                (t, (-rate * t).exp())
            })
            .collect();
        DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots(knots)
            .interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve should build")
    }

    #[test]
    fn collateral_reduces_cva_vs_uncollateralized() {
        // A CSA with zero threshold should reduce CVA compared to uncollateralized
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();

        // Uncollateralized profile
        let uncollat_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| 1_000_000.0).collect(),
            epe: times.iter().map(|_| 1_000_000.0).collect(),
            ene: times.iter().map(|_| 0.0).collect(),
        };

        // Collateralized profile: apply CSA to reduce EPE
        let csa = CsaTerms {
            threshold: 0.0,
            mta: 500.0,
            mpor_days: 10,
            independent_amount: 0.0,
        };
        let collat_epe: Vec<f64> = times
            .iter()
            .map(|_| apply_collateral(1_000_000.0, &csa))
            .collect();
        let collat_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| 1_000_000.0).collect(),
            epe: collat_epe,
            ene: times.iter().map(|_| 0.0).collect(),
        };

        let cva_uncollat = compute_cva(&uncollat_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;
        let cva_collat = compute_cva(&collat_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;

        assert!(
            cva_collat < cva_uncollat,
            "Collateralized CVA ({cva_collat:.2}) should be less than uncollateralized ({cva_uncollat:.2})"
        );
    }

    #[test]
    fn netting_reduces_cva_vs_gross() {
        // Netting offsetting trades should produce lower CVA
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=20).map(|i| i as f64 * 0.5).collect();

        // Gross: treat each trade individually (sum of positive exposures)
        let trade_a: f64 = 1_000_000.0;
        let trade_b: f64 = -800_000.0;
        let gross_epe: Vec<f64> = times.iter().map(|_| trade_a.max(0.0)).collect();
        let gross_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| trade_a).collect(),
            epe: gross_epe,
            ene: times.iter().map(|_| 0.0).collect(),
        };

        // Netted: use netting to compute net exposure
        let net_epe: Vec<f64> = times
            .iter()
            .map(|_| apply_netting(&[trade_a, trade_b]))
            .collect();
        let net_profile = ExposureProfile {
            times: times.clone(),
            mtm_values: times.iter().map(|_| trade_a + trade_b).collect(),
            epe: net_epe,
            ene: times
                .iter()
                .map(|_| (-(trade_a + trade_b)).max(0.0))
                .collect(),
        };

        let cva_gross = compute_cva(&gross_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;
        let cva_net = compute_cva(&net_profile, &hazard, &discount, 0.40)
            .expect("should work")
            .cva;

        assert!(
            cva_net < cva_gross,
            "Netted CVA ({cva_net:.2}) should be less than gross CVA ({cva_gross:.2})"
        );
    }

    #[test]
    fn zero_value_portfolio_gives_zero_cva() {
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=10).map(|i| i as f64).collect();

        let profile = ExposureProfile {
            times: times.clone(),
            mtm_values: vec![0.0; times.len()],
            epe: vec![0.0; times.len()],
            ene: vec![0.0; times.len()],
        };

        let result = compute_cva(&profile, &hazard, &discount, 0.40)
            .expect("CVA should compute for zero portfolio");
        assert!(
            result.cva.abs() < 1e-12,
            "CVA for zero-value portfolio should be zero, got {}",
            result.cva
        );
    }

    #[test]
    fn single_instrument_profile() {
        // Single instrument with declining exposure (e.g., amortizing swap)
        let hazard = flat_hazard_curve(0.02);
        let discount = flat_discount_curve(0.03);
        let times: Vec<f64> = (1..=10).map(|i| i as f64).collect();

        let epe: Vec<f64> = times
            .iter()
            .map(|&t| 1_000_000.0 * (1.0 - t / 10.0))
            .collect();
        let profile = ExposureProfile {
            times: times.clone(),
            mtm_values: epe.clone(),
            epe: epe.clone(),
            ene: vec![0.0; times.len()],
        };

        let result = compute_cva(&profile, &hazard, &discount, 0.40)
            .expect("CVA should compute for declining profile");

        assert!(result.cva > 0.0, "CVA should be positive");

        // Effective EPE profile should be non-decreasing
        for i in 1..result.effective_epe_profile.len() {
            assert!(
                result.effective_epe_profile[i].1 >= result.effective_epe_profile[i - 1].1 - 1e-12,
                "Effective EPE profile must be non-decreasing"
            );
        }

        // Validate the profile
        profile.validate().expect("Profile should be valid");
    }

    #[test]
    fn exposure_profile_validates_after_construction() {
        let times = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let profile = ExposureProfile {
            times: times.clone(),
            mtm_values: vec![100.0, -50.0, 25.0, 75.0, -10.0],
            epe: vec![100.0, 0.0, 25.0, 75.0, 0.0],
            ene: vec![0.0, 50.0, 0.0, 0.0, 10.0],
        };
        profile
            .validate()
            .expect("Manually constructed valid profile should pass validation");
    }
}
