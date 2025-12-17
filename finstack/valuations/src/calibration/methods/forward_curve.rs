//! Forward curve bootstrapping from market instruments using OIS discounting.
//!
//! This module provides calibration for tenor-specific forward curves (e.g., 1M, 3M, 6M SOFR)
//! in a multi-curve framework where discounting is performed using a separate OIS curve.
//!
//! # Multi-Curve Framework
//!
//! Post-2008 market practice requires separate curves for discounting (OIS) and forward
//! rate projection. This calibrator builds forward curves using:
//! - FRA quotes (short end)
//! - Interest rate futures (with convexity adjustment)
//! - Swap quotes (tenor-specific)
//! - Basis swap quotes (relative to another forward curve)
//!
//! # Basis Swap Calibration
//!
//! **Important**: When using basis swap quotes, the reference leg's forward curve must
//! already exist in the provided `MarketContext`. For example, to calibrate a 3M SOFR
//! forward curve using 3M vs 6M basis swaps, the 6M SOFR forward curve must be
//! pre-calibrated and present in the context.
//!
//! For simultaneous multi-curve calibration, consider using a `MultiCurveCalibrator`
//! (if available) or calibrating curves in sequence from the most liquid to least liquid.
//!
//! # Rate Bounds
//!
//! Forward rates are validated against configurable bounds that vary by currency:
//! - Developed markets (USD, GBP): typically [-2%, 50%]
//! - Negative rate environments (EUR, JPY, CHF): [-5%, 30%]
//! - Emerging markets (TRY, ARS): [-5%, 200%]
//!
//! Use `with_finstack_config()` with a `FinstackConfig` containing calibration extensions to set appropriate bounds.
//!
//! # Convexity Adjustment
//!
//! For interest rate futures, convexity adjustments are applied automatically using
//! currency-specific Hull-White/Ho-Lee parameters. For production use with calibrated
//! parameters, use `with_convexity_params()` to override defaults.
//!
//! # Examples
//!
//! ## Basic USD Forward Curve
//!
//! ```ignore
//! use finstack_valuations::calibration::methods::ForwardCurveCalibrator;
//! use finstack_valuations::calibration::CalibrationConfig;
//!
//! let calibrator = ForwardCurveCalibrator::new(
//!     "USD-SOFR-3M-FWD",
//!     0.25,
//!     base_date,
//!     Currency::USD,
//!     "USD-OIS-DISC",
//! );
//!
//! let (curve, report) = calibrator.calibrate(&quotes, &context)?;
//! ```
//!
//! ## Emerging Market with Custom Bounds
//!
//! ```ignore
//! use finstack_core::config::FinstackConfig;
//! use finstack_valuations::calibration::methods::ForwardCurveCalibrator;
//! use finstack_valuations::calibration::CALIBRATION_CONFIG_KEY_V1;
//!
//! // Create calibrator for emerging market with appropriate rate bounds
//! let mut cfg = FinstackConfig::default();
//! cfg.extensions.insert(
//!     CALIBRATION_CONFIG_KEY_V1,
//!     serde_json::json!({
//!         "rate_bounds_policy": "explicit",
//!         "rate_bounds": { "min_rate": -0.05, "max_rate": 2.00 }
//!     })
//! );
//!
//! let calibrator = ForwardCurveCalibrator::new(
//!     "TRY-TRLIBOR-3M-FWD",
//!     0.25,
//!     base_date,
//!     Currency::TRY,
//!     "TRY-OIS-DISC",
//! )
//! .with_finstack_config(&cfg)?;
//!
//! let (curve, report) = calibrator.calibrate(&quotes, &context)?;
//! ```

use crate::calibration::{
    config::{CalibrationConfig, ValidationMode},
    pricing::conventions as conv,
    pricing::{CalibrationPricer, RatesQuoteUseCase},
    quotes::{InstrumentConventions, RatesQuote},
    report::CalibrationReport,
    traits::Calibrator,
};
use finstack_core::{
    config::FinstackConfig,
    currency::Currency,
    dates::{Date, DayCountCtx, Tenor},
    explain::TraceEntry,
    market_data::{context::MarketContext, term_structures::forward_curve::ForwardCurve},
    math::{interp::InterpStyle, Solver},
    types::CurveId,
    Result,
};
use serde::{Deserialize, Serialize};

/// Forward curve calibrator for multi-curve bootstrapping.
///
/// Calibrates a tenor-specific forward curve (e.g., 3M SOFR) using market instruments
/// while discounting with a separate OIS curve.
///
/// # Market Standards
///
/// This calibrator follows ISDA conventions for:
/// - **FRA fixing dates**: T-2 business days before spot date (configurable via `reset_lag`)
/// - **Convexity adjustment**: Hull-White/Ho-Lee model with currency-specific parameters
/// - **Rate bounds**: Currency-appropriate defaults from `CalibrationConfig`
///
/// # Convexity Adjustment
///
/// For interest rate futures, convexity adjustments are applied automatically
/// using currency-specific Hull-White/Ho-Lee parameters. Override with
/// `with_convexity_params()` for custom calibration or when calibrated parameters
/// are available from swaption volatility surfaces.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForwardCurveCalibrator {
    /// Forward curve identifier
    pub fwd_curve_id: CurveId,
    /// Tenor in years (e.g., 0.25 for 3M, 0.5 for 6M)
    pub tenor_years: f64,
    /// Base date for the curve
    pub base_date: Date,
    /// Currency
    pub currency: Currency,
    /// Discount curve identifier for PV calculations
    pub discount_curve_id: CurveId,
    /// Interpolation style for forward rates
    pub solve_interp: InterpStyle,
    /// Calibration configuration (includes rate bounds, solver settings)
    pub config: CalibrationConfig,
    /// Allow calendar-day fallback when the calendar cannot be resolved.
    ///
    /// When `false` (default), missing calendars are treated as an input error to
    /// avoid silently misaligning spot/settlement conventions.
    #[serde(default)]
    pub allow_calendar_fallback: bool,
    /// Optional custom convexity parameters for futures pricing.
    /// If None, uses currency-specific defaults from `ConvexityParameters`.
    #[serde(default)]
    pub convexity_params: Option<crate::calibration::pricing::ConvexityParameters>,
}

impl ForwardCurveCalibrator {
    /// Create a new forward curve calibrator.
    ///
    /// # Arguments
    /// * `fwd_curve_id` - Identifier for the forward curve being calibrated
    /// * `tenor_years` - Tenor of the forward rate (e.g., 0.25 for 3M)
    /// * `base_date` - Valuation date / curve base date
    /// * `currency` - Currency (used for defaults and convexity parameters)
    /// * `discount_curve_id` - OIS discount curve for present value calculations
    ///
    /// # Defaults
    /// - Time day-count: ACT/360 for USD/EUR/CHF, ACT/365F for GBP/JPY/others
    /// - Rate bounds: Currency-appropriate defaults (e.g., wider for EM currencies)
    /// - Convexity: Currency-specific Hull-White parameters
    /// - Reset lag: 2 business days (ISDA standard)
    /// - Calendar: None (use `with_calendar_id()` for business day adjustments)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let calibrator = ForwardCurveCalibrator::new(
    ///     "USD-SOFR-3M-FWD",
    ///     0.25,  // 3M tenor
    ///     base_date,
    ///     Currency::USD,
    ///     "USD-OIS-DISC",
    /// )
    /// .with_calendar_id("USD");  // Enable business day adjustments
    /// ```
    pub fn new(
        fwd_curve_id: impl Into<CurveId>,
        tenor_years: f64,
        base_date: Date,
        currency: Currency,
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        // Use currency-appropriate rate bounds
        let config = CalibrationConfig::default().with_rate_bounds_for_currency(currency);
        Self {
            fwd_curve_id: fwd_curve_id.into(),
            tenor_years,
            base_date,
            currency,
            discount_curve_id: discount_curve_id.into(),
            solve_interp: InterpStyle::Linear,
            config,
            allow_calendar_fallback: false,
            convexity_params: None,
        }
    }

    /// Set the interpolation style for forward rates.
    pub fn with_solve_interp(mut self, interp: InterpStyle) -> Self {
        self.solve_interp = interp;
        self
    }

    /// Set calibration configuration from a `FinstackConfig`.
    ///
    /// Resolves `CalibrationConfig` from `FinstackConfig.extensions["valuations.calibration.v1"]`.
    /// If not present, uses `CalibrationConfig::default()`.
    ///
    /// # Errors
    ///
    /// Returns an error if the extension is malformed.
    pub fn with_finstack_config(mut self, cfg: &FinstackConfig) -> Result<Self> {
        self.config = CalibrationConfig::from_finstack_config_or_default(cfg)?;
        Ok(self)
    }

    /// Allow (or disallow) calendar-day fallback when a calendar cannot be resolved.
    ///
    /// For production calibration, keep this `false` to avoid silent date shifts.
    #[must_use]
    pub fn with_allow_calendar_fallback(mut self, allow: bool) -> Self {
        self.allow_calendar_fallback = allow;
        self
    }

    /// Set custom convexity parameters for futures pricing.
    ///
    /// Override the default currency-specific convexity adjustment calculation.
    /// Useful when you have calibrated Hull-White parameters from swaption volatility.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use finstack_valuations::calibration::methods::convexity::{ConvexityParameters, VolatilitySource};
    ///
    /// let calibrator = ForwardCurveCalibrator::new(...)
    ///     .with_convexity_params(
    ///         ConvexityParameters::usd_sofr()
    ///             .with_mean_reversion(0.05)
    ///             .with_volatility(VolatilitySource::custom(0.0085))
    ///     );
    /// ```
    #[must_use]
    pub fn with_convexity_params(
        mut self,
        params: crate::calibration::pricing::ConvexityParameters,
    ) -> Self {
        self.convexity_params = Some(params);
        self
    }

    fn make_pricer(&self) -> CalibrationPricer {
        let mut pricer = CalibrationPricer::for_forward_curve(
            self.base_date,
            self.fwd_curve_id.clone(),
            self.discount_curve_id.clone(),
            self.tenor_years,
        )
        .with_allow_calendar_fallback(self.allow_calendar_fallback)
        // Forward curve calibration uses spot-starting swaps (settlement date)
        .with_use_settlement_start(true)
        .with_verbose(self.config.verbose);

        if let Some(ref params) = self.convexity_params {
            pricer = pricer.with_convexity_params(params.clone());
        }
        pricer
    }

    /// Bootstrap the forward curve with the given solver.
    ///
    /// Uses sequential bootstrapping where each quote adds a knot to the curve.
    /// The solver finds the forward rate that prices the instrument to par.
    /// Bootstrap the forward curve with the given solver.
    ///
    /// Uses sequential bootstrapping where each quote adds a knot to the curve.
    /// The solver finds the forward rate that prices the instrument to par.
    fn bootstrap_curve_with_solver<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(ForwardCurve, CalibrationReport)> {
        // Extended scan grid for root bracketing
        let _scan = self.build_scan_grid();
        let config = self.config.clone();

        // Validate quotes
        self.validate_quotes(quotes)?;

        // Ensure discount curve exists
        let _ = base_context.get_discount_ref(self.discount_curve_id.as_ref())?;

        // Reuse a single pricer instance for convention resolution during filtering
        let pricer_for_filter = self.make_pricer();

        // Filter and sort quotes
        // ... (reuse filtering logic) ...
        let mut sorted_quotes: Vec<RatesQuote> = quotes
            .iter()
            .filter(|q| match q {
                RatesQuote::Deposit { .. } => false,
                RatesQuote::Swap {
                    float_leg_conventions,
                    ..
                } => {
                    let index = float_leg_conventions
                        .index
                        .as_ref()
                        .map(|i| i.as_str())
                        .unwrap_or("");
                    let resolved = conv::resolve_swap_conventions(&pricer_for_filter, q, self.currency);
                    let float_freq = resolved.map(|r| r.float_freq).unwrap_or_else(|_| {
                        crate::calibration::quotes::InstrumentConventions::default_float_leg_frequency(
                            self.currency,
                        )
                    });
                    self.matches_tenor(index, &float_freq)
                }
                RatesQuote::BasisSwap {
                    conventions,
                    primary_leg_conventions,
                    reference_leg_conventions,
                    ..
                } => {
                    let currency = conventions.currency.unwrap_or(self.currency);
                    let resolved = conv::resolve_basis_swap_conventions(&pricer_for_filter, q, currency);
                    let (primary_freq, reference_freq) = resolved
                        .map(|r| (r.primary_freq, r.reference_freq))
                        .unwrap_or_else(|_| {
                            (
                                crate::calibration::quotes::InstrumentConventions::default_float_leg_frequency(currency),
                                crate::calibration::quotes::InstrumentConventions::default_float_leg_frequency(currency),
                            )
                        });
                    let p_idx = primary_leg_conventions
                        .index
                        .as_ref()
                        .map(|i| i.as_str())
                        .unwrap_or("");
                    let r_idx = reference_leg_conventions
                        .index
                        .as_ref()
                        .map(|i| i.as_str())
                        .unwrap_or("");
                    self.matches_tenor(p_idx, &primary_freq)
                        || self.matches_tenor(r_idx, &reference_freq)
                }
                _ => true,
            })
            .cloned()
            .collect();
        sorted_quotes.sort_by_key(|q| q.maturity_date());

        if sorted_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Filter duplicates and prepare for bootstrapper
        let time_dc = super::discount::default_curve_day_count(self.currency);
        let mut unique_quotes = Vec::with_capacity(sorted_quotes.len());
        let mut seen_times = Vec::new();

        for quote in sorted_quotes {
            let knot_date = quote.maturity_date();
            let t = time_dc.year_fraction(self.base_date, knot_date, DayCountCtx::default())?;
            let collision_tol = self.scale_aware_tolerance(t);
            if !seen_times
                .iter()
                .any(|&seen_t: &f64| (seen_t - t).abs() < collision_tol)
            {
                unique_quotes.push(quote);
                seen_times.push(t);
            }
        }

        if unique_quotes.is_empty() {
            // Edge case where all were duplicates? Unlikely if sorted_quotes was not empty.
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Setup Bootstrapper
        let target = ForwardBootstrapper {
            calibrator: self,
            base_context: std::rc::Rc::new(std::cell::RefCell::new(base_context.clone())),
            time_dc,
        };

        // Bootstrap
        let trace = if self.config.explain.enabled {
            Some(finstack_core::explain::ExplanationTrace::new(
                "forward_curve_calibration",
            ))
        } else {
            None
        };
        let (curve, report) =
            crate::calibration::methods::common::bootstrapper::SequentialBootstrapper::bootstrap(
                &target,
                &unique_quotes,
                Vec::new(),
                &config,
                trace,
            )?;

        // Validation and Reporting details
        // `SequentialBootstrapper` returns report but we might want to enrich it.
        // Also ensure final curve has anchor (it should).

        // Re-validate against strict mode if needed
        use crate::calibration::validation::CurveValidator;
        let mut validation_status = "passed";
        let mut validation_error: Option<String> = None;
        if let Err(e) = curve.validate(&self.config.validation) {
            validation_status = "failed";
            validation_error = Some(e.to_string());
            match self.config.validation_mode {
                ValidationMode::Warn => {
                    tracing::warn!("Calibrated forward curve failed validation: {}", e);
                }
                ValidationMode::Error => {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Calibrated forward curve {} failed validation: {}",
                            self.fwd_curve_id.as_str(),
                            e
                        ),
                        category: "forward_curve_validation".to_string(),
                    });
                }
            }
        }

        let report = report
            .with_metadata("curve_id", self.fwd_curve_id.to_string())
            .with_metadata("tenor_years", self.tenor_years.to_string())
            .with_metadata("interp", format!("{:?}", self.solve_interp))
            .with_metadata("discount_curve", self.discount_curve_id.to_string())
            .with_metadata("time_dc", format!("{:?}", time_dc))
            .with_metadata("validation", validation_status)
            .with_validation_result(validation_status == "passed", validation_error);

        // Jacobian calculation if needed
        let report = if self.config.explain.enabled && report.explanation.is_some() {
            let mut r = report;
            let jacobian = self.calculate_jacobian(&unique_quotes, &curve, base_context, solver)?;
            if let Some(t) = &mut r.explanation {
                t.push(jacobian, self.config.explain.max_entries);
            }
            r
        } else {
            report
        };

        Ok((curve, report))
    }
}

// ForwardBootstrapper implementation
struct ForwardBootstrapper<'a> {
    calibrator: &'a ForwardCurveCalibrator,
    base_context: std::rc::Rc<std::cell::RefCell<MarketContext>>,
    time_dc: finstack_core::dates::DayCount,
}

impl<'a> crate::calibration::methods::common::bootstrapper::BootstrapTarget
    for ForwardBootstrapper<'a>
{
    type Quote = RatesQuote;
    type Curve = ForwardCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let knot_date = quote.maturity_date();
        self.time_dc.year_fraction(
            self.calibrator.base_date,
            knot_date,
            finstack_core::dates::DayCountCtx::default(),
        )
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        let mut full_knots = knots.to_vec();

        // Ensure anchor logic
        if full_knots.is_empty() {
            full_knots.push((0.0, 0.02)); // Fallback if strictly empty
        } else {
            // Logic from ensure_anchor: derive from first knot if > tolerance
            if full_knots[0].0 > self.calibrator.config.tolerance {
                full_knots.insert(0, (0.0, full_knots[0].1));
            }
        }

        ForwardCurve::builder(
            self.calibrator.fwd_curve_id.to_owned(),
            self.calibrator.tenor_years,
        )
        .base_date(self.calibrator.base_date)
        .knots(full_knots)
        .set_interp(self.calibrator.solve_interp)
        .day_count(self.time_dc)
        .build()
        .map_err(|e| finstack_core::Error::Calibration {
            message: format!("Failed to build temp forward curve: {}", e),
            category: "bootstrapping".to_string(),
        })
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        {
            let mut ctx = self.base_context.borrow_mut();
            ctx.insert_mut(std::sync::Arc::new(curve.clone()));
        }
        let ctx = self.base_context.borrow();

        let pricer = self.calibrator.make_pricer();
        let pv = pricer.price_instrument(quote, self.calibrator.currency, &ctx)?;

        // Keep signed residual so root finder can detect sign changes
        Ok(pv)
    }

    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        // We need context for discount curve fallback
        let ctx = self.base_context.borrow();

        // Reuse logic from get_initial_guess
        match quote {
            RatesQuote::FRA { rate, .. } => Ok(*rate),
            RatesQuote::Future { price, specs, .. } => {
                let implied_rate = (100.0 - price) / 100.0;
                if let Some(adj) = specs.convexity_adjustment {
                    Ok(implied_rate + adj)
                } else {
                    Ok(implied_rate)
                }
            }
            RatesQuote::Swap { rate, .. } => Ok(*rate),
            _ => {
                let g = previous_knots.last().map(|(_, fwd)| *fwd).or_else(|| {
                    let t = self.calibrator.tenor_years.max(1.0 / 12.0);
                    ctx.get_discount_ref(self.calibrator.discount_curve_id.as_ref())
                        .ok()
                        .map(|disc_curve| disc_curve.zero(t))
                });
                g.ok_or_else(|| finstack_core::Error::Calibration {
                    message: "Unable to derive initial forward rate guess".into(),
                    category: "bootstrapping".to_string(),
                })
            }
        }
    }

    fn validate_knot(&self, time: f64, value: f64) -> Result<()> {
        if !value.is_finite() {
            return Err(finstack_core::Error::Calibration {
                message: format!("Non-finite forward rate at t={:.6}", time),
                category: "bootstrapping".to_string(),
            });
        }
        let bounds = self
            .calibrator
            .config
            .effective_rate_bounds(self.calibrator.currency);
        if !bounds.contains(value) {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Solved forward rate out of bounds for {} at t={:.6}: {:.4}%",
                    self.calibrator.fwd_curve_id,
                    time,
                    value * 100.0
                ),
                category: "bootstrapping".to_string(),
            });
        }
        Ok(())
    }
}

impl ForwardCurveCalibrator {
    /// Check if an index/frequency matches our tenor.
    fn matches_tenor(&self, index: &str, freq: &Tenor) -> bool {
        let tol = self.config.tolerance;
        // Map tenor_years to standard tenor strings with epsilon comparison
        let tenor_str = match self.tenor_years {
            x if (x - 1.0 / 12.0).abs() < tol => "1M",
            x if (x - 0.25).abs() < tol => "3M",
            x if (x - 0.5).abs() < tol => "6M",
            x if (x - 1.0).abs() < tol => "12M",
            _ => return false,
        };

        // Tokenize on non-alphanumerics to avoid substring traps ("13M" contains "3M")
        let normalized = index.to_uppercase();
        let tokens_match = normalized
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|token| token == tenor_str);

        tokens_match || self.frequency_matches_tenor(freq)
    }

    /// Check if frequency matches tenor.
    fn frequency_matches_tenor(&self, freq: &Tenor) -> bool {
        match freq {
            freq if freq.unit == finstack_core::dates::TenorUnit::Months => {
                let freq_years = freq.count as f64 / 12.0;
                (freq_years - self.tenor_years).abs() < self.config.tolerance
            }
            _ => false,
        }
    }

    /// Validate quotes for calibration.
    ///
    /// Delegates to the unified `CalibrationPricer::validate_rates_quotes` for:
    /// - Non-empty check
    /// - Duplicate (type, maturity) detection
    /// - Rate bounds validation against configured limits
    /// - Maturity > base_date validation
    /// - Forward curve specific: no Deposit quotes allowed
    ///
    /// Additionally performs calibrator-specific day-count consistency warnings (non-fatal).
    fn validate_quotes(&self, quotes: &[RatesQuote]) -> Result<()> {
        let bounds = self.config.effective_rate_bounds(self.currency);

        // Use unified validation (handles non-empty, bounds, maturity, Deposit rejection)
        CalibrationPricer::validate_rates_quotes(
            quotes,
            &bounds,
            self.base_date,
            RatesQuoteUseCase::ForwardCurve,
        )?;

        // Calibrator-specific: day-count consistency warnings (non-fatal, not in unified validator)
        for quote in quotes {
            self.check_daycount_consistency(quote);
        }

        Ok(())
    }

    /// Check day-count consistency between quote and calibrator settings.
    ///
    /// Emits warnings (not errors) when potential mismatches are detected.
    fn check_daycount_consistency(&self, quote: &RatesQuote) {
        match quote {
            RatesQuote::FRA { conventions, .. } => {
                // FRA day-count should typically match tenor conventions
                let day_count = conventions.day_count.unwrap_or_else(|| {
                    InstrumentConventions::default_money_market_day_count(self.currency)
                });
                let time_dc = super::discount::default_curve_day_count(self.currency);
                if day_count != time_dc && self.config.verbose {
                    tracing::warn!(
                        fra_dc = ?day_count,
                        calibrator_dc = ?time_dc,
                        explicit_dc = ?conventions.day_count,
                        "FRA day-count differs from calibrator time day-count. \
                        This is usually fine as they serve different purposes \
                        (accrual vs curve time-axis)."
                    );
                }
            }
            RatesQuote::Swap {
                fixed_leg_conventions,
                float_leg_conventions,
                ..
            } => {
                // Get conventions with currency defaults
                let float_freq = float_leg_conventions.payment_frequency.unwrap_or_else(|| {
                    InstrumentConventions::default_float_leg_frequency(self.currency)
                });
                let fixed_dc = fixed_leg_conventions.day_count.unwrap_or_else(|| {
                    InstrumentConventions::default_fixed_leg_day_count(self.currency)
                });
                let float_dc = float_leg_conventions.day_count.unwrap_or_else(|| {
                    InstrumentConventions::default_float_leg_day_count(self.currency)
                });
                let time_dc = super::discount::default_curve_day_count(self.currency);

                // Check float leg frequency matches calibrator tenor
                if !self.frequency_matches_tenor(&float_freq) && self.config.verbose {
                    tracing::warn!(
                        swap_float_freq = ?float_freq,
                        calibrator_tenor = self.tenor_years,
                        "Swap float leg frequency doesn't match calibrator tenor. \
                        Ensure this swap is appropriate for this forward curve."
                    );
                }

                // Log day-count info for diagnostics
                if self.config.verbose {
                    tracing::debug!(
                        fixed_dc = ?fixed_dc,
                        float_dc = ?float_dc,
                        time_dc = ?time_dc,
                        "Swap day-count conventions"
                    );
                }
            }
            _ => {}
        }
    }

    /// Build a dynamic scan grid for the solver based on configured rate bounds.
    ///
    /// The grid is denser near zero and extends to cover the full rate bounds range.
    fn build_scan_grid(&self) -> Vec<f64> {
        let bounds = self.config.effective_rate_bounds(self.currency);

        // Core grid: dense near zero, sparser at extremes
        let mut grid = vec![
            -0.10, -0.05, -0.03, -0.02, -0.01, -0.005, 0.0, 0.002, 0.005, 0.01, 0.015, 0.02, 0.025,
            0.03, 0.035, 0.04, 0.045, 0.05, 0.06, 0.075, 0.10, 0.125, 0.15, 0.20, 0.25, 0.30, 0.40,
            0.50,
        ];

        // Extend for high rate environments (EM currencies)
        if bounds.max_rate > 0.50 {
            grid.extend_from_slice(&[0.60, 0.75, 1.00]);
        }
        if bounds.max_rate > 1.00 {
            grid.extend_from_slice(&[1.25, 1.50, 1.75, 2.00]);
        }

        // Extend for deep negative rates (EUR/JPY/CHF)
        if bounds.min_rate < -0.05 {
            grid.insert(0, -0.15);
            grid.insert(0, -0.20);
        }

        // Filter to only include points within bounds (with margin)
        grid.into_iter()
            .filter(|&r| r >= bounds.min_rate - 0.05 && r <= bounds.max_rate + 0.05)
            .collect()
    }

    /// Calculate scale-aware tolerance for knot collision detection.
    ///
    /// Uses relative tolerance for longer tenors to avoid floating-point precision issues.
    #[inline]
    fn scale_aware_tolerance(&self, knot_time: f64) -> f64 {
        // Base tolerance scaled by (1 + t) for scale awareness
        // Minimum of base tolerance ensures precision for short tenors
        (self.config.tolerance * (1.0 + knot_time)).max(self.config.tolerance)
    }

    /// Calculate the Jacobian matrix (sensitivity of curve points to input quotes).
    ///
    /// Uses a bump-and-rebuild approach:
    /// 1. Perturb each input quote by 1bp
    /// 2. Re-calibrate the curve
    /// 3. Measure change in curve knots
    fn calculate_jacobian<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        base_curve: &ForwardCurve,
        base_context: &MarketContext,
        solver: &S,
    ) -> Result<TraceEntry> {
        let bump_size = 0.0001; // 1bp
        let mut sensitivity_matrix = Vec::with_capacity(quotes.len());
        let row_labels: Vec<String> = quotes
            .iter()
            .enumerate()
            .map(|(i, q)| q.format_residual_key(i, self.currency))
            .collect();
        let col_labels: Vec<String> = base_curve
            .knots()
            .iter()
            .map(|t| format!("t={:.4}", t))
            .collect();

        // Base knots (excluding anchor if it wasn't part of the solve, but here we just take all knots)
        // Note: The anchor at t=0 is usually derived or fixed. We include all knots in the sensitivity matrix.
        let base_knots: Vec<f64> = base_curve.forwards().to_vec();

        for (i, quote) in quotes.iter().enumerate() {
            // 1. Bump quote
            let bumped_quote = quote.bump(bump_size);
            let mut bumped_quotes = quotes.to_vec();
            bumped_quotes[i] = bumped_quote;

            // 2. Re-calibrate (disable explanation to avoid recursion)
            // We need a clone of self with explanation disabled
            let mut sub_calibrator = self.clone();
            sub_calibrator.config.explain.enabled = false;

            // We use the internal bootstrap method directly
            let (bumped_curve, _) =
                sub_calibrator.bootstrap_curve_with_solver(&bumped_quotes, solver, base_context)?;

            // 3. Calculate sensitivities
            let mut row_sensitivities = Vec::with_capacity(base_knots.len());

            // Match knots by time (assuming same grid structure, which should hold for small bumps)
            // If the grid changes (e.g. adaptive knots), this simple mapping might fail,
            // but for standard bootstrapping the knot times are determined by quote maturities.
            for (j, base_rate) in base_knots.iter().enumerate() {
                if j < bumped_curve.knots().len() {
                    let bumped_rate = bumped_curve.forwards()[j];
                    let sensitivity = (bumped_rate - base_rate) / bump_size;
                    row_sensitivities.push(sensitivity);
                } else {
                    row_sensitivities.push(0.0);
                }
            }
            sensitivity_matrix.push(row_sensitivities);
        }

        Ok(TraceEntry::Jacobian {
            row_labels,
            col_labels,
            sensitivity_matrix,
        })
    }
}

impl Calibrator<RatesQuote, ForwardCurve> for ForwardCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[RatesQuote],
        base_context: &MarketContext,
    ) -> Result<(ForwardCurve, CalibrationReport)> {
        let solver = crate::calibration::create_simple_solver(&self.config);
        self.bootstrap_curve_with_solver(instruments, &solver, base_context)
    }
}
