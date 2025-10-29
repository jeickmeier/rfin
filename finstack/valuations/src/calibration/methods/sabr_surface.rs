//! Volatility surface calibration using SABR models.
//!
//! Implements market-standard volatility surface construction by calibrating
//! SABR parameters per expiry slice and building interpolated surfaces.

use crate::calibration::quote::VolQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::common::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::dates::Date;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::prelude::Currency;
use finstack_core::Result;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Interpolation choice for volatility surfaces (currently bilinear-only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceInterp {
    /// Bilinear interpolation across expiry × strike grid
    Bilinear,
}

impl std::fmt::Display for SurfaceInterp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SurfaceInterp::Bilinear => write!(f, "bilinear"),
        }
    }
}

impl std::str::FromStr for SurfaceInterp {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "bilinear" => Ok(SurfaceInterp::Bilinear),
            other => Err(format!("Unknown surface interpolation: {}", other)),
        }
    }
}

/// Volatility surface calibrator using SABR models.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolSurfaceCalibrator {
    /// Surface identifier
    pub surface_id: String,
    /// Fixed beta parameter for SABR model
    pub beta: f64,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Target expiry grid
    pub target_expiries: Vec<f64>,
    /// Target strike grid  
    pub target_strikes: Vec<f64>,
    /// Base date for time-to-expiry calculations
    pub base_date: Date,
    /// Day count used for mapping option expiries to time-to-expiry
    pub time_dc: DayCount,
    /// Base currency for the underlying asset
    pub base_currency: Currency,
    /// Interpolation used for the output surface
    pub surface_interp: SurfaceInterp,
    /// Optional discount curve id for risk-free rates used in forward extraction
    pub discount_id: Option<String>,
}

impl VolSurfaceCalibrator {
    /// Create a new volatility surface calibrator.
    pub fn new(
        surface_id: impl Into<String>,
        beta: f64,
        target_expiries: Vec<f64>,
        target_strikes: Vec<f64>,
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
            discount_id: None,
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

    /// Set the discount curve id to use for risk-free rates when extracting forwards.
    pub fn with_discount_id(mut self, discount_id: impl Into<String>) -> Self {
        self.discount_id = Some(discount_id.into());
        self
    }

    /// Internal calibration logic with forward curve.
    fn calibrate_internal(
        &self,
        quotes: &[VolQuote],
        forward_curve: &dyn Fn(f64) -> f64, // Forward price/rate as function of time
    ) -> Result<(VolSurface, CalibrationReport)> {
        // Group quotes by time-to-expiry (years) using OrderedFloat keys (deterministic ordering)
        let mut quotes_by_expiry: BTreeMap<OrderedFloat<f64>, Vec<&VolQuote>> = BTreeMap::new();

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
        let mut sabr_params_by_expiry: BTreeMap<OrderedFloat<f64>, SABRParameters> =
            BTreeMap::new();
        let mut all_residuals = BTreeMap::new();
        let mut residual_key_counter: usize = 0;
        let sabr_calibrator = SABRCalibrator::new()
            .with_tolerance(self.config.tolerance)
            .with_max_iterations(self.config.max_iterations)
            .with_fd_gradients(self.config.use_fd_sabr_gradients);

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
                    // Validate calibrated parameters meet market standards
                    validate_sabr_params(&params)?;
                    
                    sabr_params_by_expiry.insert(time_to_expiry.into(), params.clone());

                    // Calculate residuals for this expiry
                    let model = SABRModel::new(params);
                    // Try to extract an underlying label from one of the quotes in this bucket
                    let mut underlying_label: &str = "UNDERLYING";
                    if let Some(VolQuote::OptionVol { underlying, .. }) = expiry_quotes
                        .iter()
                        .find(|q| matches!(q, VolQuote::OptionVol { .. }))
                    {
                        underlying_label = underlying.as_str();
                    }

                    for (i, &strike) in strikes.iter().enumerate() {
                        let key = format!(
                            "OPT-{}-t{:.3}y-K{:.4}-{:06}",
                            underlying_label, time_to_expiry, strike, residual_key_counter
                        );

                        let residual =
                            match model.implied_volatility(forward, strike, time_to_expiry) {
                                Ok(model_vol) => model_vol - vols[i],
                                Err(_) => crate::calibration::PENALTY,
                            };

                        residual_key_counter += 1;
                        all_residuals.insert(key, residual);
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
        sabr_params: &BTreeMap<OrderedFloat<f64>, SABRParameters>,
        forward_curve: &dyn Fn(f64) -> f64,
    ) -> Result<Vec<f64>> {
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
        sabr_params: &BTreeMap<OrderedFloat<f64>, SABRParameters>,
        target_expiry: f64,
    ) -> Result<SABRParameters> {
        // Find bracketing expiries
        let mut expiries: Vec<f64> = sabr_params.keys().map(|k| k.into_inner()).collect();
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
                // TODO: Add field context - specify underlying asset that's missing from quotes
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

        // Simple forward function using available market data
        // For equity-like underlyings, calculate F(t) = S₀ × exp((r - q) × t)
        use finstack_core::market_data::scalars::MarketScalar;

        let spot = base_context
            .price(&underlying)
            .map(|scalar| match scalar {
                MarketScalar::Price(money) => money.amount(),
                MarketScalar::Unitless(value) => *value,
            })
            .unwrap_or(100.0);

        let div_yield_key = format!("{}-DIVYIELD", underlying);
        let dividend_yield = base_context
            .price(&div_yield_key)
            .map(|scalar| match scalar {
                MarketScalar::Unitless(yield_val) => *yield_val,
                _ => 0.0,
            })
            .unwrap_or(0.0);

        // Resolve a discount curve from the context
        // Preference order: explicit id via self.discount_id → inferred "<CCY>-OIS" → first discount in context
        // For production use, always specify an explicit discount_id to avoid ambiguity in multi-currency contexts.
        let disc: std::sync::Arc<
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        > = {
            if let Some(ref id) = self.discount_id {
                base_context.get_discount(id.as_str())?
            } else {
                let inferred = format!("{}-OIS", self.base_currency);
                match base_context.get_discount(inferred.as_str()) {
                    Ok(c) => {
                        tracing::debug!(
                            "Vol surface calibration for {} using inferred discount curve: {}",
                            self.surface_id, inferred
                        );
                        c
                    },
                    Err(_) => {
                        // Fallback to first discount curve available in the context
                        let mut iter = base_context.curves_of_type("Discount");
                        if let Some((id, _storage)) = iter.next() {
                            tracing::warn!(
                                "Vol surface calibration for {} auto-selecting first available discount curve: {}. \
                                Consider specifying explicit discount_id to avoid ambiguity.",
                                self.surface_id, id
                            );
                            base_context.get_discount(id.as_str())?
                        } else {
                            return Err(finstack_core::Error::Input(
                                finstack_core::error::InputError::NotFound {
                                    id: "discount curve".to_string(),
                                },
                            ));
                        }
                    }
                }
            }
        };

        let forward_fn = move |t: f64| -> f64 {
            let risk_free_rate = disc.zero(t);
            spot * ((risk_free_rate - dividend_yield) * t).exp()
        };

        self.calibrate_internal(instruments, &forward_fn)
    }
}

// ============================================================================
// Validation Helper Functions
// ============================================================================

/// Validate SABR parameters meet market-standard bounds.
///
/// Ensures:
/// - α (alpha) > 0: Initial volatility must be positive
/// - β (beta) ∈ [0, 1]: CEV exponent must be valid
/// - ν (nu) ≥ 0: Volatility of volatility must be non-negative
/// - ρ (rho) ∈ [-1, 1]: Correlation must be valid
///
/// This provides an additional safety check on calibrated parameters, though
/// the SABRParameters::new() constructor already enforces these bounds.
fn validate_sabr_params(params: &SABRParameters) -> Result<()> {
    if params.alpha <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "Calibrated SABR α (alpha) = {:.6} is not positive",
            params.alpha
        )));
    }
    if !(0.0..=1.0).contains(&params.beta) {
        return Err(finstack_core::Error::Validation(format!(
            "Calibrated SABR β (beta) = {:.6} is not in [0, 1]",
            params.beta
        )));
    }
    if params.nu < 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "Calibrated SABR ν (nu) = {:.6} is negative",
            params.nu
        )));
    }
    if !(-1.0..=1.0).contains(&params.rho) {
        return Err(finstack_core::Error::Validation(format!(
            "Calibrated SABR ρ (rho) = {:.6} is not in [-1, 1]",
            params.rho
        )));
    }
    Ok(())
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

    #[ignore = "Calibration test: comprehensive validation"]
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

    #[ignore = "Calibration test: comprehensive validation"]
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

    #[ignore = "Calibration test: comprehensive validation"]
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

        let forward_fn = |_t: f64| 100.0; // Flat forward

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
