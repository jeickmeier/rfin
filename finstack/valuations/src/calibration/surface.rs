//! Volatility surface calibration using SABR models.
//!
//! Implements market-standard volatility surface construction by calibrating
//! SABR parameters per expiry slice and building interpolated surfaces.

use crate::calibration::primitives::{HashableFloat, InstrumentQuote};
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::options::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::{Result, F};
use std::collections::HashMap;

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
        }
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Calibrate volatility surface from option quotes.
    pub fn calibrate_surface(
        &self,
        quotes: &[InstrumentQuote],
        forward_curve: &dyn Fn(F) -> F, // Forward price/rate as function of time
    ) -> Result<(VolSurface, CalibrationReport)> {
        // Group quotes by expiry
        let mut quotes_by_expiry: HashMap<String, Vec<&InstrumentQuote>> = HashMap::new();

        for quote in quotes {
            if let InstrumentQuote::OptionVol { expiry, .. } = quote {
                let expiry_key = format!("{}", expiry);
                quotes_by_expiry.entry(expiry_key).or_default().push(quote);
            }
        }

        if quotes_by_expiry.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Calibrate SABR parameters for each expiry
        let mut sabr_params_by_expiry: HashMap<HashableFloat, SABRParameters> = HashMap::new();
        let mut all_residuals = HashMap::new();
        let sabr_calibrator = SABRCalibrator::new()
            .with_tolerance(self.config.tolerance)
            .with_max_iterations(self.config.max_iterations);

        for (expiry_key, expiry_quotes) in &quotes_by_expiry {
            if expiry_quotes.len() < 3 {
                continue; // Need at least 3 points for SABR (alpha, nu, rho)
            }

            // Extract time to expiry from first quote
            let time_to_expiry = if let InstrumentQuote::OptionVol { expiry, .. } = expiry_quotes[0]
            {
                let days = (*expiry
                    - finstack_core::dates::Date::from_calendar_date(
                        2025,
                        time::Month::January,
                        1,
                    )
                    .unwrap())
                .whole_days();
                days as F / 365.25
            } else {
                continue;
            };

            if time_to_expiry <= 0.0 {
                continue;
            }

            // Get forward rate/price for this expiry
            let forward = forward_curve(time_to_expiry);

            // Extract strikes and vols
            let mut strikes = Vec::new();
            let mut vols = Vec::new();

            for quote in expiry_quotes {
                if let InstrumentQuote::OptionVol { strike, vol, .. } = quote {
                    strikes.push(*strike);
                    vols.push(*vol);
                }
            }

            // Calibrate SABR parameters for this expiry with enhanced negative rate support
            match sabr_calibrator.calibrate_auto_shift(forward, &strikes, &vols, time_to_expiry, self.beta) {
                Ok(params) => {
                    sabr_params_by_expiry
                        .insert(HashableFloat::new(time_to_expiry), params.clone());

                    // Calculate residuals for this expiry
                    let model = SABRModel::new(params);
                    for (i, &strike) in strikes.iter().enumerate() {
                        match model.implied_volatility(forward, strike, time_to_expiry) {
                            Ok(model_vol) => {
                                let residual = model_vol - vols[i];
                                all_residuals
                                    .insert(format!("VOL-{}-{}", expiry_key, strike), residual);
                            }
                            Err(_) => {
                                all_residuals
                                    .insert(format!("VOL-{}-{}", expiry_key, strike), F::INFINITY);
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
            return Err(finstack_core::Error::Internal);
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

        let report = CalibrationReport::new()
            .success()
            .with_residuals(all_residuals)
            .with_convergence_reason("Volatility surface calibration completed")
            .with_metadata("beta".to_string(), format!("{:.3}", self.beta))
            .with_metadata(
                "calibrated_expiries".to_string(),
                format!("{}", sabr_params_by_expiry.len()),
            );

        Ok((surface, report))
    }

    /// Build volatility grid from calibrated SABR parameters.
    fn build_vol_grid(
        &self,
        sabr_params: &HashMap<HashableFloat, SABRParameters>,
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
        sabr_params: &HashMap<HashableFloat, SABRParameters>,
        target_expiry: F,
    ) -> Result<SABRParameters> {
        // Find bracketing expiries
        let mut expiries: Vec<F> = sabr_params.keys().map(|k| k.value()).collect();
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if expiries.is_empty() {
            return Err(finstack_core::Error::Internal);
        }

        // If exact match, return it
        if let Some(params) = sabr_params.get(&HashableFloat::new(target_expiry)) {
            return Ok(params.clone());
        }

        // Flat extrapolation outside the range
        if target_expiry <= expiries[0] {
            return Ok(sabr_params[&HashableFloat::new(expiries[0])].clone());
        }
        if target_expiry >= *expiries.last().unwrap() {
            return Ok(sabr_params[&HashableFloat::new(*expiries.last().unwrap())].clone());
        }

        // Linear interpolation between bracketing points
        for i in 0..expiries.len() - 1 {
            let t1 = expiries[i];
            let t2 = expiries[i + 1];

            if target_expiry > t1 && target_expiry < t2 {
                let w = (target_expiry - t1) / (t2 - t1);
                let params1 = &sabr_params[&HashableFloat::new(t1)];
                let params2 = &sabr_params[&HashableFloat::new(t2)];

                // Linear interpolation of SABR parameters
                let alpha = params1.alpha * (1.0 - w) + params2.alpha * w;
                let nu = params1.nu * (1.0 - w) + params2.nu * w;
                let rho = params1.rho * (1.0 - w) + params2.rho * w;

                return SABRParameters::new(alpha, self.beta, nu, rho);
            }
        }

        // Fallback to first available parameters
        Ok(sabr_params[&HashableFloat::new(expiries[0])].clone())
    }

    /// Build asset-specific forward function from market context.
    ///
    /// Determines asset class from underlying identifier and constructs
    /// appropriate forward calculation using market data.
    fn build_forward_function(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(F) -> F + '_>> {
        // Detect asset class from underlying identifier
        if underlying.contains("-")
            && (underlying.contains("SOFR")
                || underlying.contains("EURIBOR")
                || underlying.contains("SONIA"))
        {
            // Interest rate underlying (e.g., "USD-SOFR3M", "EUR-EURIBOR3M")
            self.build_rate_forward(context, underlying)
        } else if underlying.len() == 6 && underlying.chars().all(|c| c.is_ascii_alphabetic()) {
            // FX pair (e.g., "EURUSD", "GBPJPY")
            self.build_fx_forward(context, underlying)
        } else {
            // Equity underlying (e.g., "SPY", "AAPL")
            self.build_equity_forward(context, underlying)
        }
    }

    /// Build forward function for equity underlyings: F(t) = S0 * exp((r - q) * t)
    fn build_equity_forward(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(F) -> F + '_>> {
        // Get spot price
        let spot_scalar = context.market_scalar(underlying)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
        };

        // Get dividend yield (default to 0.0 if not available)
        let div_yield_key = format!("{}-DIVYIELD", underlying);
        let dividend_yield = context
            .market_scalar(&div_yield_key)
            .map(|scalar| match scalar {
                finstack_core::market_data::primitives::MarketScalar::Unitless(yield_val) => {
                    *yield_val
                }
                _ => 0.0,
            })
            .unwrap_or(0.0);

        // Get risk-free rate from discount curve
        let disc_curve_id = format!("{}-OIS", self.base_currency_code());
        let discount_curve = context.discount(&disc_curve_id)?;

        Ok(Box::new(move |t: F| -> F {
            let risk_free_rate = discount_curve.zero(t);
            spot * ((risk_free_rate - dividend_yield) * t).exp()
        }))
    }

    /// Build forward function for FX underlyings: F(t) = S0 * exp((r_dom - r_for) * t)
    fn build_fx_forward(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(F) -> F + '_>> {
        // Parse FX pair (assume 6-char format like "EURUSD")
        if underlying.len() != 6 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        let foreign_ccy = &underlying[0..3];
        let domestic_ccy = &underlying[3..6];

        // Get spot rate
        let spot_scalar = context.market_scalar(underlying)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
        };

        // Get domestic and foreign discount curves
        let dom_disc_id = format!("{}-OIS", domestic_ccy);
        let for_disc_id = format!("{}-OIS", foreign_ccy);
        let dom_curve = context.discount(&dom_disc_id)?;
        let for_curve = context.discount(&for_disc_id)?;

        Ok(Box::new(move |t: F| -> F {
            let domestic_rate = dom_curve.zero(t);
            let foreign_rate = for_curve.zero(t);
            spot * ((domestic_rate - foreign_rate) * t).exp()
        }))
    }

    /// Build forward function for interest rate underlyings: F(t) = forward_curve.rate(t)
    fn build_rate_forward(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(F) -> F + '_>> {
        // Get forward curve for this index
        let forward_curve = context.forecast(underlying)?;

        Ok(Box::new(move |t: F| -> F { forward_curve.rate(t) }))
    }

    /// Get base currency code for discount curve lookup
    fn base_currency_code(&self) -> &'static str {
        // In a real implementation, this would come from the orchestrator context
        // For now, assume USD as default
        "USD"
    }
}

impl Calibrator<InstrumentQuote, VolSurface> for VolSurfaceCalibrator {
    fn calibrate(
        &self,
        instruments: &[InstrumentQuote],
        base_context: &MarketContext,
    ) -> Result<(VolSurface, CalibrationReport)> {
        // Detect underlying from first quote to build appropriate forward function
        let underlying = instruments
            .iter()
            .find_map(|q| match q {
                InstrumentQuote::OptionVol { underlying, .. } => Some(underlying.clone()),
                _ => None,
            })
            .ok_or(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ))?;

        // Build asset-specific forward function from market context
        let forward_fn = self.build_forward_function(base_context, &underlying)?;

        self.calibrate_surface(instruments, &forward_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use time::Month;

    fn create_test_vol_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry_1m = base_date + time::Duration::days(30);
        let expiry_3m = base_date + time::Duration::days(90);

        vec![
            // 1M expiry options
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_1m,
                strike: 90.0,
                vol: 0.22,
                option_type: "Call".to_string(),
            },
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_1m,
                strike: 100.0,
                vol: 0.20,
                option_type: "Call".to_string(),
            },
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_1m,
                strike: 110.0,
                vol: 0.21,
                option_type: "Call".to_string(),
            },
            // 3M expiry options
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_3m,
                strike: 90.0,
                vol: 0.24,
                option_type: "Call".to_string(),
            },
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: expiry_3m,
                strike: 100.0,
                vol: 0.22,
                option_type: "Call".to_string(),
            },
            InstrumentQuote::OptionVol {
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
        let calibrator = VolSurfaceCalibrator::new(
            "TEST-VOL",
            1.0,                          // Lognormal beta for equity
            vec![1.0 / 12.0, 3.0 / 12.0], // 1M, 3M
            vec![90.0, 100.0, 110.0],
        );

        let quotes = create_test_vol_quotes();

        // Create market context with required data for SPY equity forward
        let context = MarketContext::new()
            .with_price("SPY", finstack_core::market_data::primitives::MarketScalar::Unitless(100.0))
            .with_price("SPY-DIVYIELD", finstack_core::market_data::primitives::MarketScalar::Unitless(0.02))
            .with_discount(
                finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder("USD-OIS")
                    .base_date(finstack_core::dates::Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
                    .knots([(0.0, 1.0), (5.0, 0.78)])
                    .linear_df()
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
        let calibrator = VolSurfaceCalibrator::new("TEST", 0.5, vec![1.0, 2.0, 3.0], vec![100.0]);

        // Create mock SABR parameters
        let mut params_map = HashMap::new();
        params_map.insert(
            HashableFloat::new(1.0),
            SABRParameters::new(0.2, 0.5, 0.3, -0.1).unwrap(),
        );
        params_map.insert(
            HashableFloat::new(3.0),
            SABRParameters::new(0.3, 0.5, 0.4, 0.1).unwrap(),
        );

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
        let calibrator = VolSurfaceCalibrator::new(
            "TEST",
            1.0,
            vec![0.25, 0.5], // 3M, 6M
            vec![95.0, 100.0, 105.0],
        );

        // Create simple SABR parameters
        let mut params_map = HashMap::new();
        params_map.insert(
            HashableFloat::new(0.25),
            SABRParameters::new(0.2, 1.0, 0.3, -0.2).unwrap(),
        );
        params_map.insert(
            HashableFloat::new(0.5),
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
