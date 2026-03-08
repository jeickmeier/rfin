//! SVI (Stochastic Volatility Inspired) surface calibration target.
//!
//! Calibrates a volatility surface using the SVI parameterization per expiry,
//! then interpolates across expiries to build a full grid surface.

use crate::calibration::api::schema::SviSurfaceParams;
use crate::calibration::config::CalibrationConfig;
use crate::calibration::constants::OrderedF64;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::MarketQuote;
use crate::market::quotes::vol::VolQuote;
use finstack_core::dates::{DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::Result;
use std::collections::BTreeMap;

/// Target for SVI surface calibration from option volatility quotes.
pub struct SviSurfaceTarget;

impl SviSurfaceTarget {
    /// Calibrate an SVI volatility surface from option vol quotes.
    ///
    /// Groups quotes by expiry, calibrates SVI parameters per expiry slice,
    /// interpolates parameters for target expiries, and evaluates onto a grid.
    pub fn solve(
        params: &SviSurfaceParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(VolSurface, CalibrationReport)> {
        if params.target_expiries.is_empty() || params.target_strikes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let option_quotes: Vec<&VolQuote> = quotes
            .iter()
            .filter_map(|quote| match quote {
                MarketQuote::Vol(vol_quote) => Some(vol_quote),
                _ => None,
            })
            .filter(|quote| match quote {
                VolQuote::OptionVol { underlying, .. } => {
                    underlying.as_str() == params.underlying_ticker.as_str()
                }
                VolQuote::SwaptionVol { .. } => false,
            })
            .collect();

        if option_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let spot = if let Some(spot) = params.spot_override {
            spot
        } else {
            match context.get_price(&params.underlying_ticker)? {
                MarketScalar::Price(money) => money.amount(),
                MarketScalar::Unitless(value) => *value,
            }
        };

        let discount = params
            .discount_curve_id
            .as_ref()
            .map(|curve_id| context.get_discount(curve_id))
            .transpose()?;

        let forward_fn = |t: f64| -> f64 {
            if let Some(curve) = discount.as_ref() {
                let r = curve.zero(t);
                spot * (r * t).exp()
            } else {
                spot
            }
        };

        let mut quotes_by_expiry: BTreeMap<OrderedF64, Vec<(f64, f64)>> = BTreeMap::new();
        for quote in option_quotes {
            if let VolQuote::OptionVol {
                expiry,
                strike,
                vol,
                ..
            } = quote
            {
                let t = DayCount::Act365F.year_fraction(
                    params.base_date,
                    *expiry,
                    DayCountCtx::default(),
                )?;
                if t > 0.0 {
                    quotes_by_expiry
                        .entry(t.into())
                        .or_default()
                        .push((*strike, *vol));
                }
            }
        }

        if quotes_by_expiry.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let mut params_by_expiry = BTreeMap::new();
        let mut residuals = BTreeMap::new();

        for (&expiry_key, expiry_quotes) in &quotes_by_expiry {
            if expiry_quotes.len() < 5 {
                return Err(finstack_core::Error::Validation(format!(
                    "SVI surface calibration requires at least 5 strikes per expiry; got {} at t={:.6}",
                    expiry_quotes.len(),
                    expiry_key.into_inner()
                )));
            }

            let expiry = expiry_key.into_inner();
            let forward = forward_fn(expiry);
            let strikes: Vec<f64> = expiry_quotes.iter().map(|(strike, _)| *strike).collect();
            let vols: Vec<f64> = expiry_quotes.iter().map(|(_, vol)| *vol).collect();

            let svi_params = finstack_core::math::volatility::svi::calibrate_svi(
                &strikes, &vols, forward, expiry,
            )?;

            for (idx, (strike, market_vol)) in expiry_quotes.iter().enumerate() {
                let log_moneyness = (*strike / forward).ln();
                let model_vol = svi_params.implied_vol(log_moneyness, expiry);
                residuals.insert(
                    format!("svi_t{expiry:.6}_k{strike:.6}_i{idx}"),
                    (model_vol - *market_vol).abs(),
                );
            }

            params_by_expiry.insert(expiry_key, svi_params);
        }

        let mut grid =
            Vec::with_capacity(params.target_expiries.len() * params.target_strikes.len());
        for &target_expiry in &params.target_expiries {
            let interp_params = interpolate_svi_params(target_expiry, &params_by_expiry)?;
            let forward = forward_fn(target_expiry);
            for &target_strike in &params.target_strikes {
                let log_moneyness = (target_strike / forward).ln();
                let vol = interp_params.implied_vol(log_moneyness, target_expiry);
                if !vol.is_finite() || vol <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "SVI surface produced invalid implied vol at t={target_expiry:.6}, strike={target_strike:.6}",
                    )));
                }
                grid.push(vol);
            }
        }

        let surface = VolSurface::from_grid(
            &params.surface_id,
            &params.target_expiries,
            &params.target_strikes,
            &grid,
        )?;
        let mut report = CalibrationReport::new(
            residuals,
            params_by_expiry.len(),
            true,
            "SVI surface calibration completed",
        )
        .with_model_version("SVI v1.0");
        report.update_solver_config(global_config.solver.clone());

        Ok((surface, report))
    }
}

/// Linearly interpolate SVI parameters between calibrated expiry slices.
fn interpolate_svi_params(
    target_expiry: f64,
    params_by_expiry: &BTreeMap<OrderedF64, finstack_core::math::volatility::svi::SviParams>,
) -> Result<finstack_core::math::volatility::svi::SviParams> {
    let Some((&first_key, &first_params)) = params_by_expiry.iter().next() else {
        return Err(finstack_core::Error::Input(
            finstack_core::InputError::TooFewPoints,
        ));
    };

    if params_by_expiry.len() == 1 || target_expiry <= first_key.into_inner() {
        return Ok(first_params);
    }

    let Some((&last_key, &last_params)) = params_by_expiry.iter().next_back() else {
        return Ok(first_params);
    };
    if target_expiry >= last_key.into_inner() {
        return Ok(last_params);
    }

    let mut lower = (first_key.into_inner(), first_params);
    let mut upper = (last_key.into_inner(), last_params);

    for (&expiry_key, &p) in params_by_expiry {
        let expiry = expiry_key.into_inner();
        if expiry <= target_expiry {
            lower = (expiry, p);
        }
        if expiry >= target_expiry {
            upper = (expiry, p);
            break;
        }
    }

    if (upper.0 - lower.0).abs() < f64::EPSILON {
        return Ok(lower.1);
    }

    let w = (target_expiry - lower.0) / (upper.0 - lower.0);
    Ok(finstack_core::math::volatility::svi::SviParams {
        a: lower.1.a + w * (upper.1.a - lower.1.a),
        b: lower.1.b + w * (upper.1.b - lower.1.b),
        rho: lower.1.rho + w * (upper.1.rho - lower.1.rho),
        m: lower.1.m + w * (upper.1.m - lower.1.m),
        sigma: lower.1.sigma + w * (upper.1.sigma - lower.1.sigma),
    })
}
