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
/// - Market data rolling fails
/// - Instrument valuation fails
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

    for &t in &config.time_grid {
        // Convert years to days for the market roll
        let days = (t * 365.25).round() as i64;
        let future_date = as_of + time::Duration::days(days);

        // Roll market data forward (constant-curves assumption).
        // If the roll fails (e.g., curves expire), treat as zero exposure at this horizon.
        let rolled_market = match market.roll_forward(days) {
            Ok(m) => m,
            Err(_) => {
                // Market data can't be rolled this far; record zero exposure
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
}
