use crate::calibration::config::CalibrationConfig;
use crate::calibration::v2::api::schema::VolSurfaceParams;
use crate::calibration::v2::domain::quotes::{MarketQuote, VolQuote};
use crate::calibration::CalibrationReport;
use crate::instruments::common::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::Result;
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

/// Adapter for calibrating option volatility surfaces.
///
/// Calibrates volatility surfaces from option quotes using the SABR model.
/// Groups quotes by expiry, calibrates SABR parameters per expiry, and builds
/// a volatility surface grid.
pub struct VolSurfaceAdapter;

impl VolSurfaceAdapter {
    /// Calibrates an option volatility surface from market quotes.
    ///
    /// Groups option quotes by expiry, calibrates SABR parameters for each
    /// expiry, and constructs a volatility surface grid.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the volatility surface structure
    /// * `quotes` - Market quotes containing option volatility quotes
    /// * `context` - Market context containing spot prices, discount curves, and dividend yields
    /// * `config` - Calibration configuration settings
    ///
    /// # Returns
    ///
    /// A tuple containing the calibrated volatility surface and calibration report.
    ///
    /// # Errors
    ///
    /// Returns an error if insufficient quotes are provided or calibration fails.
    pub fn calibrate(
        params: &VolSurfaceParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        config: &CalibrationConfig,
    ) -> Result<(VolSurface, CalibrationReport)> {
        // Filter quotes
        let vol_quotes: Vec<&VolQuote> = quotes
            .iter()
            .filter_map(|q| match q {
                MarketQuote::Vol(vq) => Some(vq),
                _ => None,
            })
            .collect();

        if vol_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Group by expiry (year fraction)
        let mut quotes_by_expiry: BTreeMap<OrderedFloat<f64>, Vec<&VolQuote>> = BTreeMap::new();
        // We need day count for time conversion. Default to Act365F for vol surfaces if not specified?
        // Params doesn't specify DC. v1 uses Act365F.
        let time_dc = finstack_core::dates::DayCount::Act365F;

        for q in &vol_quotes {
            if let VolQuote::OptionVol { expiry, .. } = q {
                let t = time_dc.year_fraction(
                    params.base_date,
                    *expiry,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                if t > 0.0 {
                    quotes_by_expiry.entry(t.into()).or_default().push(q);
                }
            }
        }

        // Forward function
        // Need spot and dividend yield
        let spot = if let Some(s) = params.spot_override {
            s
        } else {
            let scalar = context.price(&params.underlying_id).map_err(|_| {
                finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                    id: params.underlying_id.clone(),
                })
            })?;
            match scalar {
                MarketScalar::Price(m) => m.amount(),
                MarketScalar::Unitless(v) => *v,
            }
        };

        // Resolve discount curve
        let disc_id = params
            .discount_curve_id
            .clone()
            .ok_or(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid, // Should specify discount curve
            ))?;
        let discount = context.get_discount_ref(&disc_id)?;

        // Dividend yield
        let div_yield = if let Some(d) = params.dividend_yield_override {
            d
        } else {
            // Try looking up underlying-DIVYIELD
            let key = format!("{}-DIVYIELD", params.underlying_id);
            if let Ok(MarketScalar::Unitless(v)) = context.price(&key) {
                *v
            } else {
                0.0
            }
        };

        let forward_fn = |t: f64| -> f64 {
            let r = discount.zero(t); // Continuously compounded zero rate
            spot * ((r - div_yield) * t).exp()
        };

        // Calibrate SABR per expiry
        let sabr_calibrator = SABRCalibrator::new()
            .with_tolerance(config.tolerance)
            .with_max_iterations(config.max_iterations);

        let mut sabr_params_by_expiry: BTreeMap<OrderedFloat<f64>, SABRParameters> =
            BTreeMap::new();
        let mut residuals = BTreeMap::new();
        let mut total_iterations = 0;

        for (t_key, expiry_quotes) in &quotes_by_expiry {
            let t = t_key.into_inner();
            let f = forward_fn(t);

            let mut strikes = Vec::new();
            let mut vols = Vec::new();

            for q in expiry_quotes {
                if let VolQuote::OptionVol { strike, vol, .. } = q {
                    strikes.push(*strike);
                    vols.push(*vol);
                }
            }

            if strikes.len() < 3 {
                continue; // Need at least 3 points
            }

            match sabr_calibrator.calibrate_auto_shift(f, &strikes, &vols, t, params.beta) {
                Ok(p) => {
                    sabr_params_by_expiry.insert(*t_key, p.clone());

                    // Residuals
                    let model = SABRModel::new(p);
                    for (i, k) in strikes.iter().enumerate() {
                        let model_vol = model.implied_volatility(f, *k, t).unwrap_or(0.0);
                        let res = (model_vol - vols[i]).abs();
                        residuals.insert(format!("opt_vol_t{:.2}_k{:.2}", t, k), res);
                    }
                    total_iterations += 1;
                }
                Err(_) => {
                    // Log failure?
                }
            }
        }

        // Build grid
        // Use params.target_expiries and target_strikes
        let mut grid = Vec::new();
        let mut failed = false;

        for &t in &params.target_expiries {
            let f = forward_fn(t);
            // Interpolate SABR params for t
            // Simple nearest or linear. v1 has interpolation logic.
            // For brevity, using nearest valid calibration or linear if possible.
            // Let's implement simple linear interpolation of params.

            let p = Self::interpolate_params(t, &sabr_params_by_expiry)?;
            let model = SABRModel::new(p);

            for &k in &params.target_strikes {
                match model.implied_volatility(f, k, t) {
                    Ok(v) => grid.push(v),
                    Err(_) => {
                        grid.push(0.0);
                        failed = true;
                    }
                }
            }
        }

        if failed {
            return Err(finstack_core::Error::Calibration {
                message: "Failed to build vol surface grid".to_string(),
                category: "vol_surface".to_string(),
            });
        }

        let surface = VolSurface::from_grid(
            &params.surface_id,
            &params.target_expiries,
            &params.target_strikes,
            &grid,
        )?;

        let report = CalibrationReport::for_type_with_tolerance(
            "vol_surface",
            residuals,
            total_iterations,
            config.tolerance,
        );

        Ok((surface, report))
    }

    fn interpolate_params(
        t: f64,
        params: &BTreeMap<OrderedFloat<f64>, SABRParameters>,
    ) -> Result<SABRParameters> {
        if params.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "No calibrated SABR parameters".to_string(),
                category: "vol_surface".to_string(),
            });
        }

        // Find neighbors
        let mut before = None;
        let mut after = None;

        for (&kt, p) in params {
            let kt_f = kt.into_inner();
            if kt_f <= t {
                before = Some((kt_f, p));
            }
            if kt_f >= t && after.is_none() {
                after = Some((kt_f, p));
            }
        }

        match (before, after) {
            (Some((t1, p1)), Some((t2, p2))) if t1 != t2 => {
                let w = (t - t1) / (t2 - t1);
                // Linear interp of parameters
                Ok(SABRParameters {
                    alpha: p1.alpha * (1.0 - w) + p2.alpha * w,
                    beta: p1.beta, // assume constant
                    nu: p1.nu * (1.0 - w) + p2.nu * w,
                    rho: p1.rho * (1.0 - w) + p2.rho * w,
                    shift: p1.shift, // assume constant or interpolate
                })
            }
            (Some((_, p)), _) => Ok(p.clone()),
            (_, Some((_, p))) => Ok(p.clone()),
            _ => unreachable!(),
        }
    }
}
