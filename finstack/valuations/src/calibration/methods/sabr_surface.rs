//! Volatility surface calibration using SABR models.
//!
//! Implements market-standard volatility surface construction by calibrating
//! SABR parameters per expiry slice and building interpolated surfaces.

use crate::calibration::quote::VolQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::dates::Date;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::prelude::Currency;
use finstack_core::{Result, F};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

/// Interpolation choice for volatility surfaces (currently bilinear-only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceInterp {
    /// Bilinear interpolation across expiry × strike grid
    Bilinear,
}

/// Volatility surface calibrator using SABR models.
#[derive(Clone, Debug)]
pub struct VolSurfaceCalibrator {
    /// Surface identifier
    pub surface_id: String,
    /// Fixed beta parameter for SABR model
    pub beta: F,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Target expiry grid
    pub target_expiries: Vec<F>,
    /// Target strike grid  
    pub target_strikes: Vec<F>,
    /// Base date for time-to-expiry calculations
    pub base_date: Date,
    /// Day count used for mapping option expiries to time-to-expiry
    pub time_dc: DayCount,
    /// Base currency for equity forward calculation (used by auto_forward)
    pub base_currency: Currency,
    /// Interpolation used for the output surface
    pub surface_interp: SurfaceInterp,
}

impl VolSurfaceCalibrator {
    /// Create a new volatility surface calibrator.
    pub fn new(
        surface_id: impl Into<String>,
        beta: F,
        target_expiries: Vec<F>,
        target_strikes: Vec<F>,
    ) -> Self {
        Self {
            surface_id: surface_id.into(),
            beta,
            config: CalibrationConfig::default(),
            target_expiries,
            target_strikes,
            base_date: Date::from_calendar_date(1970, time::Month::January, 1).unwrap(),
            time_dc: DayCount::Act365F,
            base_currency: Currency::USD,
            surface_interp: SurfaceInterp::Bilinear,
        }
    }

    /// Set the base date for time-to-expiry calculations.
    pub fn with_base_date(mut self, base_date: Date) -> Self {
        self.base_date = base_date;
        self
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the base currency used when building the forward function for equities.
    pub fn with_base_currency(mut self, base_currency: Currency) -> Self {
        self.base_currency = base_currency;
        self
    }

    /// Set the interpolation used for the final surface.
    pub fn with_surface_interp(mut self, interp: SurfaceInterp) -> Self {
        self.surface_interp = interp;
        self
    }

    /// Set the time-axis day count used for expiries.
    pub fn with_time_dc(mut self, dc: DayCount) -> Self {
        self.time_dc = dc;
        self
    }

    /// Internal calibration logic with forward curve.
    fn calibrate_internal(
        &self,
        quotes: &[VolQuote],
        forward_curve: &dyn Fn(F) -> F, // Forward price/rate as function of time
    ) -> Result<(VolSurface, CalibrationReport)> {
        // Group quotes by time-to-expiry (years) using OrderedFloat keys (deterministic ordering)
        let mut quotes_by_expiry: BTreeMap<OrderedFloat<F>, Vec<&VolQuote>> = BTreeMap::new();

        for quote in quotes {
            if let VolQuote::OptionVol { expiry, .. } = quote {
                let t = self
                    .time_dc
                    .year_fraction(
                        self.base_date,
                        *expiry,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                if t > 0.0 {
                    quotes_by_expiry.entry(t.into()).or_default().push(quote);
                }
            }
        }

        if quotes_by_expiry.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Calibrate SABR parameters for each expiry
        let mut sabr_params_by_expiry: BTreeMap<OrderedFloat<F>, SABRParameters> = BTreeMap::new();
        let mut all_residuals = BTreeMap::new();
        let mut residual_key_counter: usize = 0;
        let sabr_calibrator = SABRCalibrator::new()
            .with_tolerance(self.config.tolerance)
            .with_max_iterations(self.config.max_iterations);

        for (t_key, expiry_quotes) in &quotes_by_expiry {
            if expiry_quotes.len() < 3 {
                continue; // Need at least 3 points for SABR (alpha, nu, rho)
            }

            // Use grouped time-to-expiry key
            let time_to_expiry = t_key.into_inner();

            if time_to_expiry <= 0.0 {
                continue;
            }

            // Get forward rate/price for this expiry
            let forward = forward_curve(time_to_expiry);

            // Extract strikes and vols
            let mut strikes = Vec::with_capacity(expiry_quotes.len());
            let mut vols = Vec::with_capacity(expiry_quotes.len());

            for quote in expiry_quotes {
                if let VolQuote::OptionVol { strike, vol, .. } = quote {
                    strikes.push(*strike);
                    vols.push(*vol);
                }
            }

            // Calibrate SABR parameters for this expiry with enhanced negative rate support
            // Use analytical derivatives for better performance
            match sabr_calibrator.calibrate_auto_shift_with_derivatives(
                forward,
                &strikes,
                &vols,
                time_to_expiry,
                self.beta,
            ) {
                Ok(params) => {
                    sabr_params_by_expiry.insert(time_to_expiry.into(), params.clone());

                    // Calculate residuals for this expiry
                    let model = SABRModel::new(params);
                    for (i, &strike) in strikes.iter().enumerate() {
                        match model.implied_volatility(forward, strike, time_to_expiry) {
                            Ok(model_vol) => {
                                let residual = model_vol - vols[i];
                                let key = format!("{:06}", residual_key_counter);
                                residual_key_counter += 1;
                                all_residuals.insert(key, residual);
                            }
                            Err(_) => {
                                let key = format!("{:06}", residual_key_counter);
                                residual_key_counter += 1;
                                all_residuals.insert(key, crate::calibration::penalize());
                            }
                        }
                    }
                }
                Err(_) => {
                    // Failed to calibrate this expiry - skip
                    continue;
                }
            }
        }

        if sabr_params_by_expiry.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "No SABR expiries calibrated; check quotes or forward function"
                    .to_string(),
                category: "vol_surface_calibration".to_string(),
            });
        }

        // Build volatility surface on target grid
        let vol_grid = self.build_vol_grid(&sabr_params_by_expiry, forward_curve)?;

        // Create VolSurface with provided identifier
        let surface = VolSurface::from_grid(
            &self.surface_id,
            &self.target_expiries,
            &self.target_strikes,
            &vol_grid,
        )?;

        // Validate the calibrated volatility surface
        use crate::calibration::validation::SurfaceValidator;
        surface
            .validate()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Calibrated volatility surface {} failed validation: {}",
                    self.surface_id, e
                ),
                category: "vol_surface_validation".to_string(),
            })?;

        let report = CalibrationReport::for_type(
            "volatility_surface",
            all_residuals,
            sabr_params_by_expiry.len(), // Use number of calibrated expiries as iteration count
        )
        .with_metadata("beta", format!("{:.3}", self.beta))
        .with_metadata(
            "calibrated_expiries",
            sabr_params_by_expiry.len().to_string(),
        )
        .with_metadata("surface_interp", format!("{:?}", self.surface_interp))
        .with_metadata("time_dc", format!("{:?}", self.time_dc))
        .with_metadata("validation", "passed");

        Ok((surface, report))
    }

    /// Build volatility grid from calibrated SABR parameters.
    fn build_vol_grid(
        &self,
        sabr_params: &BTreeMap<OrderedFloat<F>, SABRParameters>,
        forward_curve: &dyn Fn(F) -> F,
    ) -> Result<Vec<F>> {
        let mut vol_grid =
            Vec::with_capacity(self.target_expiries.len() * self.target_strikes.len());

        for &expiry in &self.target_expiries {
            let forward = forward_curve(expiry);

            // Find SABR parameters for this expiry (interpolate if needed)
            let params = self.interpolate_sabr_params(sabr_params, expiry)?;
            let model = SABRModel::new(params);

            for &strike in &self.target_strikes {
                let vol = model
                    .implied_volatility(forward, strike, expiry)
                    .unwrap_or(0.20); // Fallback volatility
                vol_grid.push(vol);
            }
        }

        Ok(vol_grid)
    }

    /// Interpolate SABR parameters between calibrated expiries.
    fn interpolate_sabr_params(
        &self,
        sabr_params: &BTreeMap<OrderedFloat<F>, SABRParameters>,
        target_expiry: F,
    ) -> Result<SABRParameters> {
        // Find bracketing expiries
        let mut expiries: Vec<F> = sabr_params.keys().map(|k| k.into_inner()).collect();
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if expiries.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "SABR parameter map empty during interpolation".to_string(),
                category: "vol_surface_interpolation".to_string(),
            });
        }

        // If exact match, return it
        if let Some(params) = sabr_params.get(&target_expiry.into()) {
            return Ok(params.clone());
        }

        // Flat extrapolation outside the range
        if target_expiry <= expiries[0] {
            return Ok(sabr_params[&expiries[0].into()].clone());
        }
        if target_expiry >= *expiries.last().unwrap() {
            return Ok(sabr_params[&(*expiries.last().unwrap()).into()].clone());
        }

        // Linear interpolation between bracketing points
        for i in 0..expiries.len() - 1 {
            let t1 = expiries[i];
            let t2 = expiries[i + 1];

            if target_expiry > t1 && target_expiry < t2 {
                let w = (target_expiry - t1) / (t2 - t1);
                let params1 = &sabr_params[&t1.into()];
                let params2 = &sabr_params[&t2.into()];

                // Linear interpolation of SABR parameters
                let alpha = params1.alpha * (1.0 - w) + params2.alpha * w;
                let nu = params1.nu * (1.0 - w) + params2.nu * w;
                let rho = params1.rho * (1.0 - w) + params2.rho * w;

                return SABRParameters::new(alpha, self.beta, nu, rho);
            }
        }

        // Fallback to first available parameters
        Ok(sabr_params[&expiries[0].into()].clone())
    }
}

impl Calibrator<VolQuote, VolSurface> for VolSurfaceCalibrator {
    fn calibrate(
        &self,
        instruments: &[VolQuote],
        base_context: &MarketContext,
    ) -> Result<(VolSurface, CalibrationReport)> {
        // Explicitly reject swaptions: this calibrator is equity/FX-style and
        // assumes forward extraction from spot/dividend/discount, not swaption-aware forwards
        if instruments
            .iter()
            .any(|q| matches!(q, VolQuote::SwaptionVol { .. }))
        {
            return Err(finstack_core::Error::Calibration {
                message: "SwaptionVol quotes are not supported by VolSurfaceCalibrator. Use a dedicated swaption calibrator.".to_string(),
                category: "vol_surface_calibration".to_string(),
            });
        }

        // Detect underlying from first quote to build appropriate forward function
        let underlying = instruments
            .iter()
            .find_map(|q| match q {
                VolQuote::OptionVol { underlying, .. } => Some(underlying.clone()),
                _ => None,
            })
            .ok_or(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ))?;

        // Validate all option vol quotes share the same underlying
        let mismatch = instruments.iter().find(|q| match q {
            VolQuote::OptionVol { underlying: u, .. } => u != &underlying,
            _ => false,
        });
        if mismatch.is_some() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        // Build asset-specific forward function from market context using configured base currency
        let forward_fn = base_context.auto_forward(&underlying, self.base_currency)?;

        self.calibrate_internal(instruments, &forward_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn create_test_vol_quotes() -> Vec<VolQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry_1m = base_date + time::Duration::days(30);
        let expiry_3m = base_date + time::Duration::days(90);

        vec![
            // 1M expiry options
            VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_1m,
                strike: 90.0,
                vol: 0.22,
                option_type: "Call".to_string(),
            },
            VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_1m,
                strike: 100.0,
                vol: 0.20,
                option_type: "Call".to_string(),
            },
            VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_1m,
                strike: 110.0,
                vol: 0.21,
                option_type: "Call".to_string(),
            },
            // 3M expiry options
            VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_3m,
                strike: 90.0,
                vol: 0.24,
                option_type: "Call".to_string(),
            },
            VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_3m,
                strike: 100.0,
                vol: 0.22,
                option_type: "Call".to_string(),
            },
            VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_3m,
                strike: 110.0,
                vol: 0.23,
                option_type: "Call".to_string(),
            },
        ]
    }

    #[test]
    fn test_vol_surface_calibration() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = VolSurfaceCalibrator::new(
            "TEST-VOL",
            1.0,                          // Lognormal beta for equity
            vec![1.0 / 12.0, 3.0 / 12.0], // 1M, 3M
            vec![90.0, 100.0, 110.0],
        )
        .with_base_date(base_date);

        let quotes = create_test_vol_quotes();

        // Create market context with required data for SPY equity forward
        let context = MarketContext::new()
            .insert_price("SPY", finstack_core::market_data::scalars::MarketScalar::Unitless(100.0))
            .insert_price("SPY-DIVYIELD", finstack_core::market_data::scalars::MarketScalar::Unitless(0.02))
            .insert_discount(
                finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder("USD-OIS")
                    .base_date(base_date)
                    .knots([(0.0, 1.0), (5.0, 0.78)])
                    .set_interp(InterpStyle::Linear)
                    .build()
                    .unwrap()
            );

        let result = calibrator.calibrate(&quotes, &context);

        assert!(result.is_ok());
        let (surface, report) = result.unwrap();
        assert!(report.success);
        assert_eq!(surface.id().as_str(), "TEST-VOL");
        assert_eq!(surface.expiries().len(), 2);
        assert_eq!(surface.strikes().len(), 3);
    }

    #[test]
    fn test_sabr_parameter_interpolation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = VolSurfaceCalibrator::new("TEST", 0.5, vec![1.0, 2.0, 3.0], vec![100.0])
            .with_base_date(base_date);

        // Create mock SABR parameters
        let mut params_map = BTreeMap::new();
        params_map.insert(
            1.0.into(),
            SABRParameters::new(0.2, 0.5, 0.3, -0.1).unwrap(),
        );
        params_map.insert(3.0.into(), SABRParameters::new(0.3, 0.5, 0.4, 0.1).unwrap());

        // Test interpolation at t=2.0 (midpoint)
        let interp_params = calibrator
            .interpolate_sabr_params(&params_map, 2.0)
            .unwrap();

        // Should be average of endpoints
        assert!((interp_params.alpha - 0.25).abs() < 1e-10);
        assert!((interp_params.nu - 0.35).abs() < 1e-10);
        assert!((interp_params.rho - 0.0).abs() < 1e-10);

        // Test extrapolation below range
        let extrap_low = calibrator
            .interpolate_sabr_params(&params_map, 0.5)
            .unwrap();
        assert!((extrap_low.alpha - 0.2).abs() < 1e-10);

        // Test extrapolation above range
        let extrap_high = calibrator
            .interpolate_sabr_params(&params_map, 4.0)
            .unwrap();
        assert!((extrap_high.alpha - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_vol_grid_construction() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = VolSurfaceCalibrator::new(
            "TEST",
            1.0,
            vec![0.25, 0.5], // 3M, 6M
            vec![95.0, 100.0, 105.0],
        )
        .with_base_date(base_date);

        // Create simple SABR parameters
        let mut params_map = BTreeMap::new();
        params_map.insert(
            0.25.into(),
            SABRParameters::new(0.2, 1.0, 0.3, -0.2).unwrap(),
        );
        params_map.insert(
            0.5.into(),
            SABRParameters::new(0.25, 1.0, 0.35, -0.1).unwrap(),
        );

        let forward_fn = |_t: F| 100.0; // Flat forward

        let vol_grid = calibrator.build_vol_grid(&params_map, &forward_fn).unwrap();

        // Should have 2 expiries × 3 strikes = 6 values
        assert_eq!(vol_grid.len(), 6);

        // All vols should be positive
        for vol in &vol_grid {
            assert!(*vol > 0.0);
            assert!(*vol < 2.0); // Reasonable vol range
        }
    }
}
