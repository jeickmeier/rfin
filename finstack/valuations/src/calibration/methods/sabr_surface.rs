//! Volatility surface calibration using SABR models.
//!
//! Implements market-standard volatility surface construction by calibrating
//! SABR parameters per expiry slice and building interpolated surfaces.

use crate::calibration::quotes::VolQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::common::models::{SABRCalibrator, SABRModel, SABRParameters};
use finstack_core::config::FinstackConfig;
use finstack_core::dates::Date;
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::prelude::Currency;
use finstack_core::types::CurveId;
use finstack_core::Result;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Interpolation choice for volatility surfaces (currently bilinear-only).
/// Note: This is metadata for now; `VolSurface` manages its own interpolation.
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
    pub discount_id: Option<CurveId>,
    /// Optional spot override used for forward construction.
    ///
    /// When set, this value is used instead of reading spot from `MarketContext`.
    #[serde(default)]
    pub spot_override: Option<f64>,
    /// Optional dividend yield override used for forward construction.
    ///
    /// When set, this value is used instead of reading dividend yield from `MarketContext`.
    /// Dividend yield is expected in decimal terms (e.g. 0.02 for 2%).
    #[serde(default)]
    pub dividend_yield_override: Option<f64>,
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
            base_date: Date::from_calendar_date(1970, time::Month::January, 1)
                .expect("Epoch date (1970-01-01) should always be valid"),
            time_dc: DayCount::Act365F,
            base_currency: Currency::USD,
            surface_interp: SurfaceInterp::Bilinear,
            discount_id: None,
            spot_override: None,
            dividend_yield_override: None,
        }
    }

    /// Set the base date for time-to-expiry calculations.
    pub fn with_base_date(mut self, base_date: Date) -> Self {
        self.base_date = base_date;
        self
    }

    /// Set calibration configuration from a `FinstackConfig`.
    ///
    /// Resolves `CalibrationConfig` from `FinstackConfig.extensions["valuations.calibration.v1"]`.
    pub fn with_finstack_config(mut self, cfg: &FinstackConfig) -> Result<Self> {
        self.config = CalibrationConfig::from_finstack_config_or_default(cfg)?;
        Ok(self)
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
    pub fn with_discount_id(mut self, discount_id: impl Into<CurveId>) -> Self {
        self.discount_id = Some(discount_id.into());
        self
    }

    /// Set a spot override for forward construction.
    ///
    /// This is useful when spot is not present in `MarketContext`, or when you want
    /// to explicitly control the forward construction inputs.
    pub fn with_spot_override(mut self, spot: f64) -> Self {
        self.spot_override = Some(spot);
        self
    }

    /// Set a dividend yield override for forward construction.
    ///
    /// Yield is expected in decimal terms (e.g. 0.02 for 2%).
    pub fn with_dividend_yield_override(mut self, dividend_yield: f64) -> Self {
        self.dividend_yield_override = Some(dividend_yield);
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

        let mut skipped_insufficient_quotes: Vec<f64> = Vec::new();
        let mut skipped_failed_calibration: Vec<f64> = Vec::new();

        for (t_key, expiry_quotes) in &quotes_by_expiry {
            if expiry_quotes.len() < 3 {
                skipped_insufficient_quotes.push(t_key.into_inner());
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
                    skipped_failed_calibration.push(time_to_expiry);
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

        // Validate the calibrated volatility surface using configured thresholds/policies.
        use crate::calibration::validation::SurfaceValidator;
        surface.validate(&self.config.validation).map_err(|e| {
            finstack_core::Error::Calibration {
                message: format!(
                    "Calibrated volatility surface {} failed validation: {}",
                    self.surface_id, e
                ),
                category: "vol_surface_validation".to_string(),
            }
        })?;

        let mut report = CalibrationReport::for_type_with_tolerance(
            "volatility_surface",
            all_residuals,
            sabr_params_by_expiry.len(), // Use number of calibrated expiries as iteration count
            self.config.tolerance,
        )
        .with_metadata("beta", format!("{:.3}", self.beta))
        .with_metadata(
            "calibrated_expiries",
            sabr_params_by_expiry.len().to_string(),
        )
        .with_metadata(
            "skipped_expiries_insufficient_quotes",
            skipped_insufficient_quotes.len().to_string(),
        )
        .with_metadata(
            "skipped_expiries_failed_calibration",
            skipped_failed_calibration.len().to_string(),
        )
        .with_metadata(
            "skipped_expiries_insufficient_quotes_t",
            format!(
                "[{}]",
                skipped_insufficient_quotes
                    .iter()
                    .take(10)
                    .map(|t| format!("{:.6}", t))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
        .with_metadata(
            "skipped_expiries_failed_calibration_t",
            format!(
                "[{}]",
                skipped_failed_calibration
                    .iter()
                    .take(10)
                    .map(|t| format!("{:.6}", t))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
        .with_metadata("surface_interp", format!("{:?}", self.surface_interp))
        .with_metadata("time_dc", format!("{:?}", self.time_dc))
        .with_metadata("validation", "passed");

        // Market-standard reporting: a partially calibrated surface is not a "success".
        // Surface can be returned for diagnostics/use in Warn-mode workflows, but success should be false.
        if !skipped_insufficient_quotes.is_empty() || !skipped_failed_calibration.is_empty() {
            report.success = false;
            report.convergence_reason = format!(
                "Surface calibration incomplete: calibrated_expiries={}, skipped_insufficient_quotes={}, skipped_failed_calibration={}",
                sabr_params_by_expiry.len(),
                skipped_insufficient_quotes.len(),
                skipped_failed_calibration.len()
            );
            report = report.with_metadata("partial_calibration", "true");
        }

        Ok((surface, report))
    }

    /// Build volatility grid from calibrated SABR parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if SABR implied volatility cannot be computed for any
    /// expiry/strike combination. Silent fallbacks are not allowed as they can
    /// mask calibration failures and produce invalid risk surfaces.
    fn build_vol_grid(
        &self,
        sabr_params: &BTreeMap<OrderedFloat<f64>, SABRParameters>,
        forward_curve: &dyn Fn(f64) -> f64,
    ) -> Result<Vec<f64>> {
        let mut vol_grid =
            Vec::with_capacity(self.target_expiries.len() * self.target_strikes.len());
        let mut failed_points: Vec<(f64, f64)> = Vec::new();

        for &expiry in &self.target_expiries {
            let forward = forward_curve(expiry);

            // Find SABR parameters for this expiry (interpolate if needed)
            let params = self.interpolate_sabr_params(sabr_params, expiry)?;
            let model = SABRModel::new(params);

            for &strike in &self.target_strikes {
                match model.implied_volatility(forward, strike, expiry) {
                    Ok(vol) => vol_grid.push(vol),
                    Err(_) => {
                        // Track the failed point for error reporting
                        failed_points.push((expiry, strike));
                        // Push a placeholder to maintain grid structure; will error below
                        vol_grid.push(f64::NAN);
                    }
                }
            }
        }

        // Fail calibration if any SABR inversions failed
        if !failed_points.is_empty() {
            let failed_desc: Vec<String> = failed_points
                .iter()
                .take(10) // Limit error message size
                .map(|(t, k)| format!("T={:.4}y, K={:.2}", t, k))
                .collect();
            let suffix = if failed_points.len() > 10 {
                format!(" (and {} more)", failed_points.len() - 10)
            } else {
                String::new()
            };
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "SABR implied volatility failed at {} point(s): [{}]{}. \
                    Check that strikes are not too far OTM/ITM for the given forward and SABR parameters.",
                    failed_points.len(),
                    failed_desc.join(", "),
                    suffix
                ),
                category: "vol_surface_sabr_inversion".to_string(),
            });
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
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if expiries.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "SABR parameter map empty during interpolation".to_string(),
                category: "vol_surface_interpolation".to_string(),
            });
        }

        if target_expiry <= expiries[0] {
            return Ok(sabr_params[&expiries[0].into()].clone());
        }
        let last_expiry = *expiries
            .last()
            .ok_or_else(|| finstack_core::Error::Calibration {
                message: "SABR expiries vector became empty after validation".to_string(),
                category: "vol_surface_interpolation".to_string(),
            })?;
        if target_expiry >= last_expiry {
            return Ok(sabr_params[&last_expiry.into()].clone());
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

        let spot = match self.spot_override {
            Some(val) => val,
            None => base_context
                .price(underlying.as_ref())
                .map(|scalar| match scalar {
                    MarketScalar::Price(money) => money.amount(),
                    MarketScalar::Unitless(value) => *value,
                })
                .map_err(|_| {
                    finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                        id: format!("spot price for {}", underlying.as_ref()),
                    })
                })?,
        };
        if !spot.is_finite() || spot <= 0.0 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        let div_yield_key = format!("{}-DIVYIELD", underlying);
        let dividend_yield = match self.dividend_yield_override {
            Some(val) => val,
            None => {
                let scalar = base_context.price(&div_yield_key).map_err(|_| {
                    finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                        id: format!("dividend yield {}", div_yield_key),
                    })
                })?;
                match scalar {
                    MarketScalar::Unitless(yield_val) => *yield_val,
                    _ => {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::Invalid,
                        ))
                    }
                }
            }
        };
        if !dividend_yield.is_finite() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        // Resolve a discount curve from the context
        // Preference order: explicit id via self.discount_id → inferred "<CCY>-OIS" → first discount in context
        // For production use, always specify an explicit discount_id to avoid ambiguity in multi-currency contexts.
        let (disc, used_discount_id): (
            std::sync::Arc<
                finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
            >,
            String,
        ) = {
            if let Some(ref id) = self.discount_id {
                (
                    base_context.get_discount(id.as_str())?,
                    id.as_str().to_string(),
                )
            } else {
                // If there is exactly one discount in context, use it; otherwise require explicit id
                let mut iter = base_context.curves_of_type("Discount");
                let first = iter.next();
                if let Some((id, _)) = first {
                    if iter.next().is_none() {
                        (
                            base_context.get_discount(id.as_str())?,
                            id.as_str().to_string(),
                        )
                    } else {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::NotFound {
                                id: "discount_id (ambiguous)".to_string(),
                            },
                        ));
                    }
                } else {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: "discount curve".to_string(),
                        },
                    ));
                }
            }
        };

        let forward_fn = move |t: f64| -> f64 {
            let risk_free_rate = disc.zero(t);
            spot * ((risk_free_rate - dividend_yield) * t).exp()
        };

        let (surface, mut report) = self.calibrate_internal(instruments, &forward_fn)?;
        report.update_metadata("underlying", underlying.as_ref().to_string());
        report.update_metadata("spot", format!("{:.10}", spot));
        report.update_metadata("dividend_yield", format!("{:.10}", dividend_yield));
        report.update_metadata("discount_id", used_discount_id);
        report.update_metadata("forward_model", "equity_forward_exp(r-q)t".to_string());
        Ok((surface, report))
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
