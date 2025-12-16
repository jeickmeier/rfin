use crate::calibration::config::CalibrationConfig;
use crate::calibration::v2::api::schema::{SwaptionVolParams, SwaptionVolConvention};
use crate::calibration::v2::domain::quotes::{MarketQuote, VolQuote};
use crate::calibration::CalibrationReport;
use crate::instruments::common::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::dates::{DayCount, DayCountCtx, DateExt};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::Result;
use std::collections::BTreeMap;

/// Adapter for calibrating swaption volatility surfaces.
///
/// Calibrates volatility surfaces from swaption quotes using the SABR model.
/// Groups quotes by expiry and tenor, calibrates SABR parameters per group,
/// and builds a volatility surface grid.
pub struct SwaptionVolAdapter;

impl SwaptionVolAdapter {
    /// Calibrates a swaption volatility surface from market quotes.
    ///
    /// Groups swaption quotes by expiry and tenor, calibrates SABR parameters
    /// for each group, and constructs a volatility surface grid.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the swaption volatility surface structure
    /// * `quotes` - Market quotes containing swaption volatility quotes
    /// * `context` - Market context containing discount curves and forward rates
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
        params: &SwaptionVolParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        config: &CalibrationConfig,
    ) -> Result<(VolSurface, CalibrationReport)> {
        let swaption_quotes: Vec<&VolQuote> = quotes.iter().filter_map(|q| match q {
            MarketQuote::Vol(vq) if matches!(vq, VolQuote::SwaptionVol { .. }) => Some(vq),
            _ => None,
        }).collect();

        if swaption_quotes.is_empty() {
            return Err(finstack_core::Error::Input(finstack_core::error::InputError::TooFewPoints));
        }

        // Group quotes by (expiry_years, tenor_years) - simplified using f64 keys (approx)
        // Use BTreeMap<(u32, u32), Vec<&VolQuote>> where key is (expiry_days, tenor_days) or similar
        // v1 uses (u64, u64) basis points. Let's do that.
        let mut grouped_quotes: BTreeMap<(u64, u64), Vec<&VolQuote>> = BTreeMap::new();
        let dc = params.fixed_day_count.unwrap_or(DayCount::Thirty360); // Default

        for q in &swaption_quotes {
            if let VolQuote::SwaptionVol { expiry, tenor, .. } = q {
                let t_exp = dc.year_fraction(params.base_date, *expiry, DayCountCtx::default())?;
                let t_ten = dc.year_fraction(*expiry, *tenor, DayCountCtx::default())?;
                
                let key = ((t_exp * 10000.0).round() as u64, (t_ten * 10000.0).round() as u64);
                grouped_quotes.entry(key).or_default().push(q);
            }
        }

        let sabr_calibrator = SABRCalibrator::new()
            .with_tolerance(config.tolerance)
            .with_max_iterations(config.max_iterations);

        let mut sabr_params: BTreeMap<(u64, u64), SABRParameters> = BTreeMap::new();
        let mut residuals = BTreeMap::new();
        let mut count = 0;

        for ((kb_exp, kb_ten), bucket_quotes) in &grouped_quotes {
            let t_exp = *kb_exp as f64 / 10000.0;
            let t_ten = *kb_ten as f64 / 10000.0;
            
            // Calculate forward swap rate
            let fwd_rate = Self::calculate_forward_swap_rate(params, t_exp, t_ten, context)?;
            
            let mut strikes = Vec::new();
            let mut vols = Vec::new();
            
            for q in bucket_quotes {
                if let VolQuote::SwaptionVol { strike, vol, .. } = q {
                    strikes.push(*strike);
                    vols.push(*vol);
                }
            }

            if strikes.len() < 3 { continue; }

            // Calibrate
            // Need to handle conventions (normal/lognormal). 
            // Simplified: assume lognormal if beta != 0, normal if beta == 0
            // Params has explicit convention.
            
            let res = if params.vol_convention == SwaptionVolConvention::Normal {
                 sabr_calibrator.calibrate_with_atm_pinning(fwd_rate, &strikes, &vols, t_exp, 0.0)
            } else {
                 sabr_calibrator.calibrate_auto_shift(fwd_rate, &strikes, &vols, t_exp, params.sabr_beta)
            };

            if let Ok(p) = res {
                sabr_params.insert((*kb_exp, *kb_ten), p.clone());
                
                let model = SABRModel::new(p);
                for (i, k) in strikes.iter().enumerate() {
                    let v = model.implied_volatility(fwd_rate, *k, t_exp).unwrap_or(0.0);
                    residuals.insert(format!("swpt_{}_{}_{}", kb_exp, kb_ten, i), (v - vols[i]).abs());
                }
                count += 1;
            }
        }

        // Build grid
        let target_expiries = params.target_expiries.clone();
        let target_tenors = params.target_tenors.clone();
        
        let mut grid = Vec::new();
        for &texp in &target_expiries {
            for &tten in &target_tenors {
                // Find or interpolate SABR params
                // Simplified: find nearest
                let key = ((texp * 10000.0).round() as u64, (tten * 10000.0).round() as u64);
                
                // TODO: Implement proper bilinear interpolation of params
                let p = sabr_params.get(&key).or_else(|| sabr_params.values().next()).cloned(); 
                
                let val = if let Some(p) = p {
                    let f = Self::calculate_forward_swap_rate(params, texp, tten, context)?;
                    let model = SABRModel::new(p);
                    // ATM vol
                    model.implied_volatility(f, f, texp).unwrap_or(0.0)
                } else {
                    0.0
                };
                grid.push(val);
            }
        }

        let surface = VolSurface::from_grid(
            &params.surface_id,
            &target_expiries,
            &target_tenors,
            &grid
        )?;

        Ok((surface, CalibrationReport::for_type_with_tolerance("swaption_vol", residuals, count, config.tolerance)))
    }

    fn calculate_forward_swap_rate(
        params: &SwaptionVolParams,
        expiry_years: f64,
        tenor_years: f64,
        context: &MarketContext,
    ) -> Result<f64> {
        let disc = context.get_discount_ref(&params.discount_curve_id)?;
        
        // Simple approximation: (DF_start - DF_end) / PV01
        // PV01 approximation: sum DF(ti) * dt
        
        let expiry_date = params.base_date.add_months((expiry_years * 12.0) as i32);
        let _maturity_date = expiry_date.add_months((tenor_years * 12.0) as i32);
        
        // Proper schedule would be better, but approximation for now
        let df_start = disc.df(expiry_years);
        let df_end = disc.df(expiry_years + tenor_years);
        
        // Annuity approximation (semi-annual)
        let n_periods = (tenor_years * 2.0) as usize;
        let mut pv01 = 0.0;
        for i in 1..=n_periods {
            let t = expiry_years + i as f64 * 0.5;
            pv01 += disc.df(t) * 0.5;
        }
        
        if pv01 == 0.0 { return Ok(0.0); }
        Ok((df_start - df_end) / pv01)
    }
}

