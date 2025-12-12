//! Yield curve bootstrapping from market instruments.
//!
//! Implements market-standard multi-curve discount curve calibration using
//! deposits and OIS swaps. Forward curves are calibrated separately.
//!
//! Uses instrument pricing methods directly rather than reimplementing
//! pricing formulas, following market-standard bootstrap methodology.
//!
//! # Features
//!
//! - **Adaptive scan grid**: Supports negative rate environments (DF > 1.0)
//! - **Settlement conventions**: Currency-specific T+0/T+2 handling
//! - **Day-count alignment**: Validates consistency between quotes and curve
//! - **Pre-validation**: Checks curve dependencies before bootstrap starts
//! - **Extrapolation policy**: Configurable flat-forward or flat-zero extrapolation
//!
//! # Market Conventions
//!
//! Default settlement by currency:
//! - **USD/EUR/JPY/CHF**: T+2
//! - **GBP**: T+0
//! - **AUD/CAD**: T+1

use crate::calibration::config::CalibrationMethod;
use crate::calibration::quote::{
    default_calendar_for_currency, settlement_days_for_currency, RatesQuote,
};
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator, MultiCurveConfig};
use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::ir_future::InterestRateFuture;
use crate::instruments::irs::FloatingLegCompounding;
use crate::instruments::InterestRateSwap;
use finstack_core::dates::{
    adjust, BusinessDayConvention, CalendarRegistry, Date, DateExt, DayCount,
};
use finstack_core::explain::{ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::math::{MultiSolver, Solver};
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::{CurveId, IndexId};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Discount curve bootstrapper with market-standard conventions.
///
/// Implements sequential bootstrapping for OIS discount curves from deposits
/// and overnight-indexed swaps. Supports negative rate environments and
/// configurable settlement/extrapolation conventions.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::calibration::methods::DiscountCurveCalibrator;
/// use finstack_core::currency::Currency;
///
/// let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
///     .with_extrapolation(ExtrapolationPolicy::FlatForward)
///     .with_settlement_days(2);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscountCurveCalibrator {
    /// Curve identifier
    pub curve_id: CurveId,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Interpolation used during solving and for the final curve
    pub solve_interp: InterpStyle,
    /// Extrapolation policy for the final curve
    #[serde(default = "default_extrapolation")]
    pub extrapolation: ExtrapolationPolicy,
    /// Calibration configuration (includes multi-curve settings)
    pub config: CalibrationConfig,
    /// Calibration method (bootstrap vs global solve)
    #[serde(default)]
    pub calibration_method: CalibrationMethod,
    /// Currency for the curve
    pub currency: Currency,
    /// Optional calendar identifier for schedule generation
    pub calendar_id: Option<String>,
    /// Settlement lag in business days (None = use currency default)
    #[serde(default)]
    pub settlement_days: Option<i32>,
    /// Day count for curve time (None = use currency default)
    #[serde(default)]
    pub curve_day_count: Option<DayCount>,
    /// Payment delay in business days after period end (default: 0).
    ///
    /// Bloomberg OIS swaps typically use 2 business days payment delay.
    /// Set to 2 for accurate Bloomberg curve matching.
    #[serde(default)]
    pub payment_delay_days: i32,
    /// Allow calendar-day settlement fallback when the calendar cannot be resolved.
    ///
    /// When `false` (default), missing calendars are treated as an input error to
    /// avoid silently misaligning spot/settlement conventions.
    #[serde(default)]
    pub allow_calendar_fallback: bool,
}

fn default_extrapolation() -> ExtrapolationPolicy {
    ExtrapolationPolicy::FlatForward
}

impl DiscountCurveCalibrator {
    /// Create a new discount curve calibrator with currency-appropriate defaults.
    ///
    /// Default settings:
    /// - Interpolation: MonotoneConvex (arbitrage-free forwards)
    /// - Extrapolation: FlatForward (standard for risk)
    /// - Settlement: Currency-specific (T+2 for USD/EUR, T+0 for GBP)
    /// - Day count: Currency-specific (ACT/360 for USD/EUR, ACT/365 for GBP)
    /// - Payment delay: 0 (set to 2 for Bloomberg OIS matching)
    pub fn new(curve_id: impl Into<CurveId>, base_date: Date, currency: Currency) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            solve_interp: InterpStyle::MonotoneConvex, // Default; arbitrage-free
            extrapolation: ExtrapolationPolicy::FlatForward,
            config: CalibrationConfig::default(), // Defaults to multi-curve mode
            calibration_method: CalibrationMethod::default(),
            currency,
            calendar_id: None,
            settlement_days: None, // Will use currency default
            curve_day_count: None, // Will use currency default
            payment_delay_days: 0, // Default 0; set to 2 for Bloomberg OIS
            allow_calendar_fallback: false,
        }
    }

    /// Set the interpolation used both during solving and for the final curve.
    pub fn with_solve_interp(mut self, interpolation: InterpStyle) -> Self {
        self.solve_interp = interpolation;
        self
    }

    /// Select calibration method (bootstrap vs global solve).
    pub fn with_calibration_method(mut self, method: CalibrationMethod) -> Self {
        self.calibration_method = method;
        self
    }

    /// Set the extrapolation policy for the final curve.
    ///
    /// - `FlatForward`: Constant forward rate beyond last knot (standard for risk)
    /// - `FlatZero`: Constant zero rate beyond last knot (some regulatory uses)
    pub fn with_extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Set multi-curve framework configuration.
    pub fn with_multi_curve_config(mut self, multi_curve_config: MultiCurveConfig) -> Self {
        self.config.multi_curve = multi_curve_config;
        self
    }

    /// Set an optional calendar identifier for schedule generation.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Set explicit settlement days (overrides currency default).
    ///
    /// Market conventions:
    /// - USD/EUR/JPY/CHF: 2 days
    /// - GBP: 0 days (same-day settlement)
    /// - AUD/CAD: 1 day
    pub fn with_settlement_days(mut self, days: i32) -> Self {
        self.settlement_days = Some(days);
        self
    }

    /// Set explicit day count for curve time (overrides currency default).
    pub fn with_curve_day_count(mut self, day_count: DayCount) -> Self {
        self.curve_day_count = Some(day_count);
        self
    }

    /// Set payment delay in business days after period end.
    ///
    /// Bloomberg OIS swaps typically use 2 business days payment delay.
    /// Set to 2 for accurate Bloomberg curve matching.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    ///     .with_payment_delay(2);  // Match Bloomberg OIS convention
    /// ```
    pub fn with_payment_delay(mut self, days: i32) -> Self {
        self.payment_delay_days = days;
        self
    }

    /// Allow (or disallow) calendar-day settlement fallback when a calendar cannot be resolved.
    ///
    /// For production calibration, keep this `false` to avoid silent date shifts.
    pub fn with_allow_calendar_fallback(mut self, allow: bool) -> Self {
        self.allow_calendar_fallback = allow;
        self
    }

    /// Get effective settlement days (explicit or currency default).
    fn effective_settlement_days(&self) -> i32 {
        self.settlement_days
            .unwrap_or_else(|| settlement_days_for_currency(self.currency))
    }

    /// Get effective day count for curve time (explicit or currency default).
    fn effective_curve_day_count(&self) -> DayCount {
        self.curve_day_count.unwrap_or_else(|| {
            crate::calibration::quote::standard_day_count_for_currency(self.currency)
        })
    }

    /// Validate a calibrated discount curve using auto-detected rate environment.
    ///
    /// Automatically detects negative rate environments (EUR/CHF/JPY) by checking
    /// the short-end zero rate, and applies appropriate validation rules.
    fn validate_calibrated_curve(&self, curve: &DiscountCurve) -> Result<()> {
        use crate::calibration::validation::{CurveValidator, ValidationConfig};

        if self.config.verbose {
            tracing::debug!("Validating calibrated discount curve {}", self.curve_id);
        }

        // Auto-detect negative rate environment by checking short-end zero rate
        let short_rate = curve.zero(0.25);
        let validation_config = if short_rate < 0.0 {
            // Negative rate environment (EUR/CHF/JPY) - allow non-monotone DFs
            ValidationConfig::negative_rates()
        } else {
            // Positive rate environment - enforce strict monotonicity
            ValidationConfig::default()
        };

        curve
            .validate(&validation_config)
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Calibrated discount curve {} failed validation: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_validation".to_string(),
            })
    }

    /// Calculate settlement date from base date using business-day calendar.
    ///
    /// Uses the configured calendar (or currency default) to properly compute
    /// the spot/settlement date by adding business days and adjusting to the
    /// next business day if needed.
    ///
    /// # Market Conventions
    ///
    /// - USD/EUR/JPY/CHF: T+2 business days
    /// - GBP: T+0 (same-day settlement)
    /// - AUD/CAD: T+1 business day
    ///
    /// The result is adjusted using Modified Following convention to ensure
    /// the settlement date falls on a valid business day.
    fn settlement_date(&self) -> finstack_core::Result<Date> {
        let days = self.effective_settlement_days();

        // Resolve calendar: explicit calendar_id or currency default
        let calendar_id = self
            .calendar_id
            .as_deref()
            .unwrap_or_else(|| default_calendar_for_currency(self.currency));

        let registry = CalendarRegistry::global();

        // If we have a valid calendar, use business-day arithmetic
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            if days == 0 {
                // T+0: just ensure base_date is a business day
                adjust(self.base_date, BusinessDayConvention::Following, calendar)
            } else {
                // Add business days and adjust result
                let spot = self.base_date.add_business_days(days, calendar)?;
                // Final adjustment ensures we land on a business day
                adjust(spot, BusinessDayConvention::ModifiedFollowing, calendar)
            }
        } else if self.allow_calendar_fallback {
            // Fallback: calendar not found, use calendar-day addition with warning.
            // This should only be used for prototyping/backward compatibility.
            tracing::warn!(
                calendar_id = calendar_id,
                currency = ?self.currency,
                "Calendar not found, falling back to calendar-day settlement"
            );
            Ok(if days == 0 {
                self.base_date
            } else {
                self.base_date + time::Duration::days(days as i64)
            })
        } else {
            Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: format!("calendar '{}'", calendar_id),
                },
            ))
        }
    }

    /// Market-standard OIS compounding preset inferred from the quote's index id.
    ///
    /// This is intentionally heuristic and should be kept consistent with `RatesQuote::is_ois_suitable()`.
    fn ois_compounding_for_index(index: &IndexId, currency: Currency) -> FloatingLegCompounding {
        let upper = index.as_str().to_ascii_uppercase();

        // Index-name driven overrides.
        if upper.contains("SONIA") {
            return FloatingLegCompounding::sonia();
        }
        if upper.contains("ESTR") || upper.contains("€STR") {
            return FloatingLegCompounding::estr();
        }
        if upper.contains("TONA") || upper.contains("TONAR") {
            return FloatingLegCompounding::tona();
        }
        if upper.contains("SOFR") {
            return FloatingLegCompounding::sofr();
        }

        // Currency fallback for generic ids like "USD-OIS".
        match currency {
            Currency::GBP => FloatingLegCompounding::sonia(),
            Currency::EUR => FloatingLegCompounding::estr(),
            Currency::JPY => FloatingLegCompounding::tona(),
            _ => FloatingLegCompounding::sofr(),
        }
    }

    /// Compute maturity-aware discount-factor bounds implied by configured rate bounds.
    fn df_bounds_for_time(&self, t: f64) -> (f64, f64) {
        // Guard against degenerate maturities.
        let t = t.max(1e-12);

        // DF(t) = exp(-z(t) * t). Using configured bounds as a coarse guard.
        let bounds = &self.config.rate_bounds;
        let df_a = (-bounds.max_rate * t).exp();
        let df_b = (-bounds.min_rate * t).exp();

        let mut lo = df_a.min(df_b);
        let mut hi = df_a.max(df_b);

        // Hard guards: avoid zeros/NaNs and prevent numeric overflow in extreme stress settings.
        const DF_HARD_MIN: f64 = 1e-12;
        const DF_HARD_MAX: f64 = 1e6;
        if !lo.is_finite() || lo <= 0.0 {
            lo = DF_HARD_MIN;
        }
        if !hi.is_finite() || hi <= 0.0 {
            hi = DF_HARD_MAX;
        }
        lo = lo.max(DF_HARD_MIN);
        hi = hi.min(DF_HARD_MAX).max(lo * 1.000_000_1);

        (lo, hi)
    }

    /// Apply the configured solve interpolation style to the discount curve builder.
    fn apply_solve_interpolation(
        &self,
        builder: finstack_core::market_data::term_structures::discount_curve::DiscountCurveBuilder,
    ) -> finstack_core::market_data::term_structures::discount_curve::DiscountCurveBuilder {
        builder.set_interp(self.solve_interp)
    }

    /// Generate an adaptive scan grid for discount factor solving.
    ///
    /// Unlike the fixed grid, this adapts to the expected DF range based on
    /// the initial guess. Critical for negative rate environments where DF > 1.0.
    fn adaptive_scan_grid(initial_df: f64) -> Vec<f64> {
        // Center the grid around the initial guess
        let center = initial_df.clamp(0.3, 1.3);

        // For negative rates (DF > 1), extend upward
        let (min_df, max_df) = if initial_df > 0.98 {
            // Potentially negative rates - extend to DF > 1.0
            (0.85, 1.15)
        } else if initial_df < 0.5 {
            // High rates / long maturity - extend downward
            (0.2, 1.0)
        } else {
            // Normal range
            (0.4, 1.05)
        };

        // Generate grid with finer resolution near center
        let mut grid = Vec::with_capacity(30);

        // Coarse grid covering full range
        let step = (max_df - min_df) / 15.0;
        for i in 0..=15 {
            let df = max_df - i as f64 * step;
            if df > 0.0 && df <= 1.5 {
                grid.push(df);
            }
        }

        // Finer grid near center
        let fine_step = 0.01;
        for i in -5..=5 {
            let df = center + i as f64 * fine_step;
            if df > 0.0 && df <= 1.5 && !grid.iter().any(|&x| (x - df).abs() < 0.005) {
                grid.push(df);
            }
        }

        // Sort descending (from 1.0+ downward)
        grid.sort_by(|a, b| b.total_cmp(a));
        grid.dedup_by(|a, b| (*a - *b).abs() < 0.001);
        grid
    }

    /// Pre-validate that all required curves exist for the quote set.
    ///
    /// Fails fast with a clear error if dependencies are missing, rather than
    /// returning PENALTY values during bootstrap.
    fn validate_curve_dependencies(
        &self,
        quotes: &[RatesQuote],
        context: &MarketContext,
    ) -> Result<()> {
        for quote in quotes {
            if let RatesQuote::BasisSwap {
                primary_index,
                reference_index,
                ..
            } = quote
            {
                let primary_fwd = format!("FWD_{}", primary_index);
                let ref_fwd = format!("FWD_{}", reference_index);

                if context.get_forward_ref(&primary_fwd).is_err() {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: format!(
                                "Forward curve '{}' required for basis swap calibration. \
                                 Please calibrate the forward curve first.",
                                primary_fwd
                            ),
                        },
                    ));
                }
                if context.get_forward_ref(&ref_fwd).is_err() {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: format!(
                                "Forward curve '{}' required for basis swap calibration. \
                                 Please calibrate the forward curve first.",
                                ref_fwd
                            ),
                        },
                    ));
                }
            }
        }
        Ok(())
    }

    /// Bootstrap discount curve from instrument quotes using solver.
    ///
    /// This method builds the curve incrementally, solving for each discount factor
    /// that reprices the corresponding instrument to par.
    ///
    /// # Features
    ///
    /// - **Adaptive scan grid**: Supports negative rate environments (DF > 1.0)
    /// - **Pre-validation**: Checks curve dependencies before bootstrap starts
    /// - **Day-count alignment**: Uses curve day count for consistent time mapping
    fn bootstrap_curve_with_solver<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Sort quotes by maturity
        let mut sorted_quotes = quotes.to_vec();
        sorted_quotes.sort_by_key(RatesQuote::maturity_date);

        // Validate quotes
        self.validate_quotes(&sorted_quotes)?;

        // Pre-validate curve dependencies (fail fast for basis swaps)
        self.validate_curve_dependencies(&sorted_quotes, base_context)?;

        // Get effective curve day count for consistent time mapping
        let curve_dc = self.effective_curve_day_count();
        let settlement = self.settlement_date()?;

        // Build knots sequentially
        let mut knots = Vec::with_capacity(sorted_quotes.len() + 1);
        knots.push((0.0, 1.0)); // Start with DF(0) = 1.0
        let mut residuals = BTreeMap::new();
        let mut residual_key_counter: usize = 0;
        let mut total_iterations = 0;

        // Initialize explanation trace if requested
        let mut trace = if self.config.explain.enabled {
            Some(ExplanationTrace::new("calibration"))
        } else {
            None
        };

        for (idx, quote) in sorted_quotes.iter().enumerate() {
            let maturity_date = quote.maturity_date();
            // Use CURVE day count for consistent time mapping (not instrument day count)
            // This ensures all knots are on the same time basis for interpolation
            let time_to_maturity = curve_dc
                .year_fraction(
                    self.base_date,
                    maturity_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);

            if time_to_maturity <= 0.0 {
                continue; // Skip expired instruments
            }

            if self.config.verbose {
                tracing::debug!(
                    instrument = idx + 1,
                    total = sorted_quotes.len(),
                    maturity_date = %maturity_date,
                    time_to_maturity = time_to_maturity,
                    "Processing instrument for bootstrap"
                );
            }

            // Create objective function that uses instrument pricing directly
            // Clone only the necessary data, not the entire context (for performance)
            let knots_clone = knots.clone();
            let quote_clone = quote.clone();
            // Capture context by reference; clone only once per evaluation (not twice)
            let base_context_ref = base_context;
            let base_date = self.base_date;
            let solve_interp = self.solve_interp;

            // Capture curve_dc for the closure
            let curve_day_count = curve_dc;

            let objective = move |df: f64| -> f64 {
                let mut temp_knots = Vec::with_capacity(knots_clone.len() + 1);
                temp_knots.extend_from_slice(&knots_clone);
                temp_knots.push((time_to_maturity, df));

                // Build temporary curve with current knots
                // Use the same day count as the final curve for consistency
                // Allow non-monotonic during solving to support negative rates (DF > 1.0)
                // Final validation is done after bootstrap completes
                let temp_curve = match DiscountCurve::builder("CALIB_CURVE")
                    .base_date(base_date)
                    .day_count(curve_day_count)
                    .knots(temp_knots)
                    .set_interp(solve_interp)
                    .allow_non_monotonic() // Allow DF > 1.0 for negative rate environments
                    .build()
                {
                    Ok(curve) => curve,
                    Err(_) => return crate::calibration::PENALTY,
                };

                // Multi-curve only: for OIS instruments we DON'T need a forward curve since
                // the IRS pricer will use discount-only pricing when both legs use the same curve.
                // Non-OIS forward-dependent instruments are not supported by this calibrator.
                if quote_clone.requires_forward_curve() && !quote_clone.is_ois_suitable() {
                    return crate::calibration::PENALTY;
                }
                let temp_context = base_context_ref.clone().insert_discount(temp_curve);

                // Price the instrument and return error (target is zero)
                self.price_instrument(&quote_clone, &temp_context)
                    .unwrap_or(crate::calibration::PENALTY)
            };

            // Initial guess
            // For deposits, use DF ≈ 1 / (1 + r * yf). For others, use
            // extrapolation from previous point with a constant yield.
            let initial_df = match quote {
                RatesQuote::Deposit { .. } => {
                    let r = self.get_rate(quote);
                    let yf = match quote {
                        RatesQuote::Deposit {
                            maturity,
                            day_count,
                            ..
                        } => day_count
                            .year_fraction(
                                settlement,
                                *maturity,
                                finstack_core::dates::DayCountCtx::default(),
                            )
                            .unwrap_or(time_to_maturity)
                            .max(1e-6),
                        _ => time_to_maturity.max(1e-6),
                    };
                    1.0 / (1.0 + r * yf)
                }
                _ => {
                    if let Some((prev_t, prev_df)) = knots.last() {
                        if time_to_maturity > *prev_t && *prev_t > 0.0 {
                            // Extrapolate forward assuming constant yield
                            let implied_rate = -prev_df.ln() / prev_t;
                            (-implied_rate * time_to_maturity).exp()
                        } else {
                            *prev_df * 0.99 // Small decay
                        }
                    } else {
                        0.95 // Reasonable fallback
                    }
                }
            };

            // Use adaptive scan grid based on initial guess
            // This supports negative rate environments where DF > 1.0
            let scan_grid = Self::adaptive_scan_grid(initial_df);
            let tentative = crate::calibration::bracket_solve_1d(
                &objective,
                initial_df,
                &scan_grid,
                self.config.tolerance,
                self.config.max_iterations,
            )?;
            let solved_df = if let Some(root) = tentative {
                root
            } else {
                // Only attempt a direct solve if we have at least one reasonable objective value.
                let v0 = objective(initial_df);
                if !v0.is_finite() || v0.abs() >= crate::calibration::PENALTY / 10.0 {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Bootstrap objective invalid/penalized for {} at t={:.6} (initial_df={:.6}, value={:?}). \
                             This usually indicates inconsistent conventions (calendar/settlement) or unsupported instrument set.",
                            self.curve_id,
                            time_to_maturity,
                            initial_df,
                            v0
                        ),
                        category: "yield_curve_bootstrap".to_string(),
                    });
                }

                solver.solve(objective, initial_df).map_err(|e| {
                    finstack_core::Error::Calibration {
                        message: format!(
                            "Bootstrap solver failed for {} at t={:.6} (initial_df={:.6}): {}",
                            self.curve_id, time_to_maturity, initial_df, e
                        ),
                        category: "yield_curve_bootstrap".to_string(),
                    }
                })?
            };

            if !solved_df.is_finite() {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap produced non-finite discount factor for {} at t={:.6}: df={:?}",
                        self.curve_id, time_to_maturity, solved_df
                    ),
                    category: "yield_curve_bootstrap".to_string(),
                });
            }

            // Validate the solution against maturity-aware DF bounds implied by configured rate bounds.
            let (df_lower, df_upper) = self.df_bounds_for_time(time_to_maturity);
            if solved_df < df_lower || solved_df > df_upper {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Solved discount factor out of bounds [{:.6}, {:.6}] implied by rate bounds [{:.4}, {:.4}] for {} at t={:.6}: df={:.6}.",
                        df_lower,
                        df_upper,
                        self.config.rate_bounds.min_rate,
                        self.config.rate_bounds.max_rate,
                        self.curve_id,
                        time_to_maturity,
                        solved_df
                    ),
                    category: "yield_curve_bootstrap".to_string(),
                });
            }

            // Compute residual for reporting
            let final_residual = {
                let mut final_knots = Vec::with_capacity(knots.len() + 1);
                final_knots.extend_from_slice(&knots);
                final_knots.push((time_to_maturity, solved_df));

                // Allow non-monotonic during residual calculation for negative rate support
                // Use the same day count as the final curve for consistency
                let final_curve = DiscountCurve::builder("CALIB_CURVE")
                    .base_date(base_date)
                    .day_count(curve_dc)
                    .knots(final_knots)
                    .set_interp(solve_interp)
                    .allow_non_monotonic()
                    .build()
                    .map_err(|e| finstack_core::Error::Calibration {
                        message: format!(
                            "temp DiscountCurve build failed for {}: {}",
                            self.curve_id, e
                        ),
                        category: "yield_curve_bootstrap".to_string(),
                    })?;

                // Build final pricing context
                if quote.requires_forward_curve() && !quote.is_ois_suitable() {
                    crate::calibration::PENALTY
                } else {
                    let final_context = base_context.clone().insert_discount(final_curve);
                    self.price_instrument(quote, &final_context)
                        .unwrap_or(crate::calibration::PENALTY)
                        .abs()
                }
            };

            knots.push((time_to_maturity, solved_df));

            // Skip recording penalty placeholders; only keep real residuals
            if !(final_residual.is_finite()
                && final_residual.abs() < crate::calibration::PENALTY * 0.5)
            {
                // Do not count this residual; continue bootstrapping
                continue;
            }

            // Store residual with descriptive key
            let key = quote.format_residual_key(residual_key_counter);
            residual_key_counter += 1;
            residuals.insert(key, final_residual);
            total_iterations += 1;

            // Add trace entry if explanation is enabled
            if let Some(ref mut t) = trace {
                let converged = final_residual.abs() < self.config.tolerance;
                t.push(
                    TraceEntry::CalibrationIteration {
                        iteration: total_iterations,
                        residual: final_residual,
                        knots_updated: vec![format!("{:.4}y", time_to_maturity)],
                        converged,
                    },
                    self.config.explain.max_entries,
                );
            }
        }

        // Build final discount curve with configured interpolation and extrapolation
        // Allow non-monotonic to support negative rate environments (DF > 1.0)
        let curve = self
            .apply_solve_interpolation(
                DiscountCurve::builder(self.curve_id.to_owned())
                    .base_date(self.base_date)
                    .day_count(self.effective_curve_day_count())
                    .extrapolation(self.extrapolation)
                    .allow_non_monotonic() // Support negative rates
                    .knots(knots),
            )
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "final DiscountCurve build failed for {}: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_bootstrap".to_string(),
            })?;

        // Validate the calibrated curve
        self.validate_calibrated_curve(&curve)?;

        // Create calibration report with comprehensive metadata
        let mut report = CalibrationReport::for_type_with_tolerance(
            "yield_curve",
            residuals,
            total_iterations,
            self.config.tolerance,
        )
        .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
        .with_metadata("extrapolation", format!("{:?}", self.extrapolation))
        .with_metadata("currency", self.currency.to_string())
        .with_metadata(
            "curve_day_count",
            format!("{:?}", self.effective_curve_day_count()),
        )
        .with_metadata(
            "settlement_days",
            self.effective_settlement_days().to_string(),
        )
        .with_metadata("validation", "passed");

        // Attach explanation trace if present
        if let Some(explanation) = trace {
            report = report.with_explanation(explanation);
        }

        Ok((curve, report))
    }

    /// Global solve for discount factors using multi-dimensional solver.
    ///
    /// Uses Levenberg-Marquardt to minimize pricing residuals across all instruments
    /// simultaneously. This provides an optional alternative to sequential bootstrap.
    fn calibrate_global(
        &self,
        quotes: &[RatesQuote],
        base_context: &MarketContext,
        _use_analytical_jacobian: bool,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        use finstack_core::error::InputError;

        // Sort quotes by maturity and validate dependencies
        let mut sorted_quotes = quotes.to_vec();
        sorted_quotes.sort_by_key(RatesQuote::maturity_date);
        self.validate_quotes(&sorted_quotes)?;
        self.validate_curve_dependencies(&sorted_quotes, base_context)?;

        let curve_dc = self.effective_curve_day_count();
        let settlement = self.settlement_date()?;

        // Build time grid and initial guesses
        let mut times: Vec<f64> = Vec::new();
        let mut initials: Vec<f64> = Vec::new();
        let mut active_quotes: Vec<RatesQuote> = Vec::new();

        for quote in sorted_quotes {
            let time_to_maturity = curve_dc
                .year_fraction(
                    self.base_date,
                    quote.maturity_date(),
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            if time_to_maturity <= 0.0 {
                continue;
            }

            let init_df = match &quote {
                RatesQuote::Deposit {
                    maturity,
                    day_count,
                    ..
                } => {
                    // Deposits are quoted on the accrual from *settlement* → maturity.
                    // Use the quote's day-count to build a better initial guess.
                    let r = self.get_rate(&quote);
                    let yf = day_count
                        .year_fraction(
                            settlement,
                            *maturity,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(time_to_maturity)
                        .max(1e-6);
                    1.0 / (1.0 + r * yf)
                }
                _ => {
                    let r = self.get_rate(&quote);
                    (-r * time_to_maturity).exp()
                }
            };

            times.push(time_to_maturity);
            initials.push(init_df.clamp(1e-6, 1.5));
            active_quotes.push(quote);
        }

        if active_quotes.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }

        // Seed the global solve with a high-quality initial guess.
        //
        // Using the bootstrapped curve as the starting point dramatically improves
        // convergence and typically yields the same (or near-identical) curve when
        // the instrument set is exactly solvable (par instruments).
        let bootstrap_solver = crate::calibration::create_simple_solver(&self.config);
        if let Ok((boot_curve, _)) =
            self.bootstrap_curve_with_solver(&active_quotes, &bootstrap_solver, base_context)
        {
            for (i, t) in times.iter().enumerate() {
                initials[i] = boot_curve.df(*t).clamp(1e-6, 1.5);
            }
        }

        let solver = self.config.create_lm_solver();
        let base_context_clone = base_context.clone();
        let solve_interp = self.solve_interp;
        let curve_day_count = curve_dc;
        let base_date = self.base_date;

        let residuals = |params: &[f64], resid: &mut [f64]| {
            // Build curve once per parameter vector.
            //
            // For interpolation styles like MonotoneConvex we must ensure a monotone
            // discount-factor sequence; otherwise the interpolator construction fails
            // and the residuals become constant penalties.
            let mut knots = Vec::with_capacity(params.len() + 1);
            knots.push((0.0, 1.0));

            let mut prev = 1.0;
            for (t, &raw_df) in times.iter().zip(params.iter()) {
                // Keep DF positive; for some interpolators we also project onto a
                // (weakly) non-increasing sequence to avoid builder failures.
                let mut df = raw_df.clamp(1e-8, 1.5);
                match solve_interp {
                    finstack_core::math::interp::InterpStyle::MonotoneConvex
                    | finstack_core::math::interp::InterpStyle::CubicHermite => {
                        if df > prev {
                            df = prev;
                        }
                        prev = df;
                    }
                    _ => {
                        // No monotonic projection for other styles.
                        prev = df;
                    }
                }
                knots.push((*t, df));
            }

            let temp_curve = match DiscountCurve::builder("CALIB_CURVE")
                .base_date(base_date)
                .day_count(curve_day_count)
                .knots(knots)
                .set_interp(solve_interp)
                .allow_non_monotonic()
                .build()
            {
                Ok(curve) => curve,
                Err(_) => {
                    for r in resid.iter_mut() {
                        *r = crate::calibration::PENALTY;
                    }
                    return;
                }
            };

            let temp_context = base_context_clone.clone().insert_discount(temp_curve);

            for (i, quote) in active_quotes.iter().enumerate() {
                resid[i] = self
                    .price_instrument(quote, &temp_context)
                    .unwrap_or(crate::calibration::PENALTY);
            }
        };

        let solved = solver.solve_system(residuals, &initials)?;

        // Build final knots and curve
        let mut final_knots = Vec::with_capacity(solved.len() + 1);
        final_knots.push((0.0, 1.0));
        for (t, df) in times.iter().zip(solved.iter()) {
            final_knots.push((*t, df.clamp(1e-8, 1.5)));
        }

        let curve = self
            .apply_solve_interpolation(
                DiscountCurve::builder(self.curve_id.to_owned())
                    .base_date(self.base_date)
                    .day_count(curve_dc)
                    .extrapolation(self.extrapolation)
                    .allow_non_monotonic()
                    .knots(final_knots.clone()),
            )
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "global DiscountCurve build failed for {}: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_global_solve".to_string(),
            })?;

        // Validate calibrated curve
        self.validate_calibrated_curve(&curve)?;

        // Compute residuals for report
        let mut residuals_map = BTreeMap::new();
        let mut total_iterations = 0;
        // NOTE: `price_instrument` uses internal CALIB_* curve ids for repricing.
        // The global solver produces the final curve with the user-facing id
        // (`self.curve_id`), so we also build an equivalent curve under the
        // calibration id for residual reporting.
        let pricing_curve = self
            .apply_solve_interpolation(
                DiscountCurve::builder("CALIB_CURVE")
                    .base_date(self.base_date)
                    .day_count(curve_dc)
                    .extrapolation(self.extrapolation)
                    .allow_non_monotonic()
                    .knots(final_knots.clone()),
            )
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("global pricing DiscountCurve build failed: {}", e),
                category: "yield_curve_global_solve_pricing_curve".to_string(),
            })?;

        let final_context = base_context.clone().insert_discount(pricing_curve);
        for (idx, quote) in active_quotes.iter().enumerate() {
            let ctx = final_context.clone();
            let residual = self
                .price_instrument(quote, &ctx)
                .unwrap_or(crate::calibration::PENALTY)
                .abs();
            residuals_map.insert(format!("GLOBAL-{:06}", idx), residual);
            total_iterations += 1;
        }

        let report = CalibrationReport::for_type_with_tolerance(
            "yield_curve_global",
            residuals_map,
            total_iterations,
            self.config.tolerance,
        )
        .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
        .with_metadata("extrapolation", format!("{:?}", self.extrapolation))
        .with_metadata("currency", self.currency.to_string())
        .with_metadata(
            "curve_day_count",
            format!("{:?}", self.effective_curve_day_count()),
        )
        .with_metadata("method", "global_solve");

        Ok((curve, report))
    }

    /// Price an instrument using the given market context.
    ///
    /// Returns the pricing error (PV for par instruments) that should be zero
    /// when the curve is correctly calibrated.
    ///
    /// # Settlement Handling
    ///
    /// Deposits use currency-specific settlement dates:
    /// - USD/EUR/JPY/CHF: T+2
    /// - GBP: T+0 (same-day settlement)
    /// - AUD/CAD: T+1
    fn price_instrument(&self, quote: &RatesQuote, context: &MarketContext) -> Result<f64> {
        let is_ois_quote = quote.is_ois_suitable();
        match quote {
            RatesQuote::Deposit {
                maturity,
                rate,
                day_count,
            } => {
                // Use settlement date (currency-specific T+0/T+1/T+2) with business-day calendar
                let settlement = self.settlement_date()?;

                // Create Deposit instrument with proper settlement
                let dep = Deposit {
                    id: format!("CALIB_DEP_{}", maturity).into(),
                    notional: Money::new(1_000_000.0, self.currency),
                    start: settlement, // Use settlement date, not base date
                    end: *maturity,
                    day_count: *day_count,
                    quote_rate: Some(*rate),
                    discount_curve_id: "CALIB_CURVE".into(),
                    attributes: Default::default(),
                    spot_lag_days: None,
                    bdc: None,
                    calendar_id: None,
                };

                // Price the deposit - should be zero at par rate
                let pv = dep.value(context, self.base_date)?;
                Ok(pv.amount() / dep.notional.amount())
            }
            RatesQuote::FRA {
                start,
                end,
                rate,
                day_count,
            } => {
                // Create FRA instrument via builder
                let fixing_date = if *start >= self.base_date + time::Duration::days(2) {
                    *start - time::Duration::days(2)
                } else {
                    self.base_date
                };

                let fra = match ForwardRateAgreement::builder()
                    .id(format!("CALIB_FRA_{}_{}", start, end).into())
                    .notional(Money::new(1_000_000.0, self.currency))
                    .fixing_date(fixing_date)
                    .start_date(*start)
                    .end_date(*end)
                    .fixed_rate(*rate)
                    .day_count(*day_count)
                    .reset_lag(2)
                    .discount_curve_id("CALIB_CURVE".into())
                    .forward_id("CALIB_FWD".into())
                    .build()
                {
                    Ok(fra) => fra,
                    Err(_) => return Err(finstack_core::Error::Internal),
                };

                // Price the FRA - should be zero at par rate
                let pv = fra.value(context, self.base_date)?;
                Ok(pv.amount() / fra.notional.amount())
            }
            RatesQuote::Future {
                expiry,
                price,
                specs,
            } => {
                // Create future instrument
                let period_start = *expiry;
                let period_end = expiry.add_months(specs.delivery_months as i32);

                // Calculate convexity adjustment if not provided
                let convexity_adj = if let Some(adj) = specs.convexity_adjustment {
                    Some(adj)
                } else {
                    // Auto-calculate convexity adjustment using currency-specific parameters
                    use super::convexity::ConvexityParameters;
                    let params = ConvexityParameters::for_currency(self.currency);
                    Some(params.calculate_for_future(
                        self.base_date,
                        *expiry,
                        period_end,
                        specs.day_count,
                    ))
                };

                let mut future = InterestRateFuture::builder()
                    .id(format!("CALIB_FUT_{}", expiry).into())
                    .notional(Money::new(1_000_000.0, self.currency))
                    .expiry_date(*expiry)
                    .fixing_date(*expiry - time::Duration::days(2))
                    .period_start(period_start)
                    .period_end(period_end)
                    .quoted_price(*price)
                    .day_count(specs.day_count)
                    .position(crate::instruments::ir_future::Position::Long)
                    .contract_specs(crate::instruments::ir_future::FutureContractSpecs::default())
                    .discount_curve_id(finstack_core::types::CurveId::from("CALIB_CURVE"))
                    .forward_id(finstack_core::types::CurveId::from("CALIB_FWD"))
                    .build()
                    .map_err(|e| finstack_core::Error::Calibration {
                        message: format!("IRFuture builder failed for expiry {}: {}", expiry, e),
                        category: "yield_curve_bootstrap".to_string(),
                    })?;

                // Set contract specs from the quote with calculated convexity
                future = future.with_contract_specs(
                    crate::instruments::ir_future::FutureContractSpecs {
                        face_value: specs.face_value,
                        tick_size: 0.0025,
                        tick_value: 6.25,
                        delivery_months: specs.delivery_months,
                        convexity_adjustment: convexity_adj,
                    },
                );

                // Price the future - should be zero at quoted price
                let pv = future.value(context, self.base_date)?;
                Ok(pv.amount() / future.notional.amount())
            }
            RatesQuote::Swap {
                maturity,
                rate,
                fixed_freq,
                float_freq,
                fixed_dc,
                float_dc,
                index,
                ..
            } => {
                // Create swap instrument
                // Swaps start at settlement date (T+2), not at base_date
                use crate::instruments::irs::{FixedLegSpec, FloatLegSpec, PayReceive};
                use finstack_core::dates::{BusinessDayConvention, StubKind};

                let settlement = self.settlement_date()?;

                let fixed_spec = FixedLegSpec {
                    discount_curve_id: finstack_core::types::CurveId::from("CALIB_CURVE"),
                    rate: *rate,
                    freq: *fixed_freq,
                    dc: *fixed_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: self.calendar_id.clone(),
                    stub: StubKind::None,
                    par_method: None,
                    compounding_simple: true,
                    start: settlement, // Use settlement date (T+2), not base_date
                    end: *maturity,
                    payment_delay_days: self.payment_delay_days,
                };

                // For OIS quotes, configure the floating leg as an overnight-indexed
                // swap (compounded in arrears) and use the discount curve as both
                // discount and index curve so that pricing is purely discount-based.
                // For non-OIS swaps, we keep a separate forward curve for the float leg.
                let (float_discount_id, float_forward_id, compounding) = if is_ois_quote {
                    (
                        finstack_core::types::CurveId::from("CALIB_CURVE"),
                        finstack_core::types::CurveId::from("CALIB_CURVE"),
                        Self::ois_compounding_for_index(index, self.currency),
                    )
                } else {
                    (
                        finstack_core::types::CurveId::from("CALIB_CURVE"),
                        finstack_core::types::CurveId::from("CALIB_FWD"),
                        FloatingLegCompounding::Simple,
                    )
                };

                let float_spec = FloatLegSpec {
                    discount_curve_id: float_discount_id,
                    forward_curve_id: float_forward_id,
                    spread_bp: 0.0,
                    freq: *float_freq,
                    dc: *float_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: self.calendar_id.clone(),
                    fixing_calendar_id: self.calendar_id.clone(),
                    stub: StubKind::None,
                    reset_lag_days: 2,
                    start: settlement, // Use settlement date (T+2), not base_date
                    end: *maturity,
                    compounding,
                    payment_delay_days: self.payment_delay_days,
                };

                let swap = InterestRateSwap {
                    id: format!("CALIB_SWAP_{}", maturity).into(),
                    notional: Money::new(1_000_000.0, self.currency),
                    side: PayReceive::ReceiveFixed,
                    fixed: fixed_spec,
                    float: float_spec,
                    margin_spec: None,
                    attributes: Default::default(),
                };

                // Price the swap - should be zero at par rate
                let pv = swap.value(context, self.base_date)?;
                Ok(pv.amount() / swap.notional.amount())
            }
            RatesQuote::BasisSwap {
                maturity,
                primary_index,
                reference_index,
                spread_bp,
                primary_freq,
                reference_freq,
                primary_dc,
                reference_dc,
                ..
            } => {
                // Import BasisSwap types
                use crate::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
                use finstack_core::dates::BusinessDayConvention;

                // In multi-curve mode, basis swaps contribute to tenor basis calibration
                // Extract tenor information from index names (e.g., "3M-SOFR" -> 3M)
                let primary_forward_id = format!("FWD_{}", primary_index).into();
                let reference_forward_id = format!("FWD_{}", reference_index).into();

                // Store string references for later use in checks
                let primary_fwd_str = format!("FWD_{}", primary_index);
                let reference_fwd_str = format!("FWD_{}", reference_index);

                // Create basis swap instrument
                let primary_leg = BasisSwapLeg {
                    forward_curve_id: primary_forward_id,
                    frequency: *primary_freq,
                    day_count: *primary_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    payment_lag_days: 0,
                    reset_lag_days: 0,
                    spread: *spread_bp / 10_000.0, // Convert bp to decimal
                };

                let reference_leg = BasisSwapLeg {
                    forward_curve_id: reference_forward_id,
                    frequency: *reference_freq,
                    day_count: *reference_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    payment_lag_days: 0,
                    reset_lag_days: 0,
                    spread: 0.0,
                };

                let basis_swap = BasisSwap::new(
                    format!("CALIB_BASIS_{}_{}", primary_index, reference_index),
                    Money::new(1_000_000.0, self.currency),
                    self.base_date,
                    *maturity,
                    primary_leg,
                    reference_leg,
                    "CALIB_CURVE",
                );

                // Check if forward curves exist for pricing
                if context.get_forward_ref(&primary_fwd_str).is_err()
                    || context.get_forward_ref(&reference_fwd_str).is_err()
                {
                    // Forward curves not yet calibrated — surface a typed error instead of placeholder value
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::NotFound {
                            id: "forward curves".to_string(),
                        },
                    ));
                }

                // Price the basis swap - should be zero at market spread
                let pv = basis_swap.value(context, self.base_date)?;
                Ok(pv.amount() / basis_swap.notional.amount())
            }
        }
    }

    /// Validate quote sequence for no-arbitrage and completeness.
    ///
    /// ## Multi-Curve Framework Guidance
    ///
    /// **Appropriate for discount curve calibration:**
    /// - OIS swaps (e.g., SOFR, ESTR, SONIA): overnight compounded, collateral-aligned
    /// - Deposits: short-end risk-free rates
    ///
    /// **Not recommended for discount curves (use dedicated forward curve calibration):**
    /// - FRAs: reference LIBOR/term rates, require forward curve for pricing
    /// - Futures: reference term rates, convexity-adjusted
    /// - Tenor swaps (3M, 6M LIBOR-based): require forward curves per tenor
    /// - Basis swaps: used for cross-tenor calibration, not discount
    ///
    /// This validator warns (not errors) when forward-dependent instruments are used,
    /// allowing flexibility while alerting to potential misuse. In strict mode (future),
    /// this could be configured to error instead.
    fn validate_quotes(&self, quotes: &[RatesQuote]) -> Result<()> {
        if quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Check for duplicate maturities
        let mut maturities = std::collections::HashSet::new();
        for quote in quotes {
            let maturity = quote.maturity_date();
            if !maturities.insert(maturity) {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        // Check rates are reasonable (basic sanity check)
        for quote in quotes {
            let rate = self.get_rate(quote);
            if !rate.is_finite() || !self.config.rate_bounds.contains(rate) {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        // Multi-curve mode validation: warn if mixing instrument types inappropriately
        let mut has_forward_dependent = false;
        let mut has_ois_suitable = false;

        for quote in quotes {
            if quote.requires_forward_curve() {
                has_forward_dependent = true;
            }
            if quote.is_ois_suitable() {
                has_ois_suitable = true;
            }
        }

        // Enforce separation if configured: do not allow forward-dependent instruments for discount curve
        if has_forward_dependent && !has_ois_suitable {
            if self.config.multi_curve.enforce_separation {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            } else {
                tracing::warn!(
                    "Discount curve calibration using forward-dependent instruments (FRA, Future, non-OIS Swap). \
                     Best practice: use OIS swaps (SOFR/ESTR/SONIA) or deposits for discount curves, \
                     and calibrate forward curves separately."
                );
            }
        }

        Ok(())
    }

    /// Extract rate from quote.
    fn get_rate(&self, quote: &RatesQuote) -> f64 {
        match quote {
            RatesQuote::Deposit { rate, .. } => *rate,
            RatesQuote::FRA { rate, .. } => *rate,
            RatesQuote::Future { price, .. } => (100.0 - price) / 100.0, // Convert price to rate
            RatesQuote::Swap { rate, .. } => *rate,
            RatesQuote::BasisSwap { spread_bp, .. } => *spread_bp / 10_000.0, // Convert bp to decimal
        }
    }
}

impl Calibrator<RatesQuote, DiscountCurve> for DiscountCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[RatesQuote],
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Validate multi-curve integrity: Discount curves should only be calibrated with OIS-suitable instruments
        // Non-OIS instruments (FRAs, Futures, LIBOR swaps) require forward curves with proper basis spreads
        for quote in instruments {
            if !quote.is_ois_suitable() && quote.requires_forward_curve() {
                return Err(finstack_core::Error::Validation(
                    format!(
                        "DiscountCurveCalibrator received non-OIS instrument: {}. \
                         Non-OIS instruments ({:?}) violate multi-curve principles by implying zero basis spread. \
                         Please calibrate forward curves separately using ForwardCurveCalibrator with appropriate basis spreads.",
                        quote.get_type(),
                        quote
                    ),
                ));
            }
        }

        match self.calibration_method {
            CalibrationMethod::Bootstrap => {
                let solver = crate::calibration::create_simple_solver(&self.config);
                self.bootstrap_curve_with_solver(instruments, &solver, base_context)
            }
            CalibrationMethod::GlobalSolve {
                use_analytical_jacobian,
            } => self.calibrate_global(instruments, base_context, use_analytical_jacobian),
        }
    }
}

// =============================================================================
// Shared Swap Construction Helper
// =============================================================================

/// Creates an OIS swap from a rates quote with specified curve IDs.
///
/// This helper ensures identical swap construction between calibration and
/// repricing tests, eliminating schedule alignment issues that would otherwise
/// cause repricing errors.
///
/// # Arguments
///
/// * `quote` - The swap quote containing rate and frequency parameters
/// * `discount_curve_id` - Curve ID for discounting (e.g., "USD-OIS")
/// * `forward_curve_id` - Curve ID for forward projection (same as discount for OIS)
/// * `base_date` - Start date of the swap
/// * `notional` - Notional amount
/// * `calendar_id` - Optional calendar for schedule generation
///
/// # Returns
///
/// An `InterestRateSwap` configured identically to calibration instruments.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::calibration::methods::create_ois_swap_from_quote;
///
/// let swap = create_ois_swap_from_quote(
///     &quote,
///     "USD-OIS",
///     "USD-OIS",
///     base_date,
///     Money::new(1_000_000.0, Currency::USD),
///     None,
///     0, // payment_delay_days
/// )?;
/// ```
pub fn create_ois_swap_from_quote(
    quote: &RatesQuote,
    discount_curve_id: &str,
    forward_curve_id: &str,
    base_date: Date,
    notional: Money,
    calendar_id: Option<&str>,
    payment_delay_days: i32,
) -> Result<InterestRateSwap> {
    use crate::instruments::irs::{FixedLegSpec, FloatLegSpec, PayReceive};
    use finstack_core::dates::{BusinessDayConvention, StubKind};

    let (maturity, rate, fixed_freq, float_freq, fixed_dc, float_dc, index) = match quote {
        RatesQuote::Swap {
            maturity,
            rate,
            fixed_freq,
            float_freq,
            fixed_dc,
            float_dc,
            index,
            ..
        } => (
            *maturity,
            *rate,
            *fixed_freq,
            *float_freq,
            *fixed_dc,
            *float_dc,
            index,
        ),
        _ => {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ))
        }
    };

    let currency = notional.currency();
    let compounding = DiscountCurveCalibrator::ois_compounding_for_index(index, currency);

    let fixed_spec = FixedLegSpec {
        discount_curve_id: CurveId::from(discount_curve_id),
        rate,
        freq: fixed_freq,
        dc: fixed_dc,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: calendar_id.map(String::from),
        stub: StubKind::None,
        par_method: None,
        compounding_simple: true,
        start: base_date,
        end: maturity,
        payment_delay_days,
    };

    let float_spec = FloatLegSpec {
        discount_curve_id: CurveId::from(discount_curve_id),
        forward_curve_id: CurveId::from(forward_curve_id),
        spread_bp: 0.0,
        freq: float_freq,
        dc: float_dc,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: calendar_id.map(String::from),
        fixing_calendar_id: calendar_id.map(String::from),
        stub: StubKind::None,
        reset_lag_days: 2,
        start: base_date,
        end: maturity,
        compounding,
        payment_delay_days,
    };

    InterestRateSwap::builder()
        .id(format!("SWAP-{}", maturity).into())
        .notional(notional)
        .side(PayReceive::ReceiveFixed)
        .fixed(fixed_spec)
        .float(float_spec)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, Frequency};
    use time::Month;

    fn create_test_quotes() -> Vec<RatesQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                day_count: DayCount::Act360,
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.046,
                day_count: DayCount::Act360,
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.047,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::quarterly(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-SOFR-3M".to_string().into(),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365 * 2),
                rate: 0.048,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::quarterly(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-SOFR-3M".to_string().into(),
            },
        ]
    }

    #[test]
    fn test_quote_validation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD);

        let empty_quotes = vec![];
        assert!(calibrator.validate_quotes(&empty_quotes).is_err());

        let valid_quotes = create_test_quotes();
        assert!(calibrator.validate_quotes(&valid_quotes).is_ok());
    }

    #[test]
    fn test_adaptive_scan_grid_normal_rates() {
        // Normal positive rates environment
        let grid = DiscountCurveCalibrator::adaptive_scan_grid(0.95);

        // Grid should be sorted descending
        for i in 1..grid.len() {
            assert!(
                grid[i] <= grid[i - 1],
                "Grid should be sorted descending: {} > {}",
                grid[i],
                grid[i - 1]
            );
        }

        // Should contain values around 0.95
        assert!(
            grid.iter().any(|&x| (x - 0.95).abs() < 0.05),
            "Grid should contain values near initial guess"
        );

        // Should have at least 10 points for good coverage
        assert!(
            grid.len() >= 10,
            "Grid should have sufficient points: {}",
            grid.len()
        );
    }

    #[test]
    fn test_adaptive_scan_grid_negative_rates() {
        // Negative rate environment where DF > 1.0
        let grid = DiscountCurveCalibrator::adaptive_scan_grid(1.02);

        // Grid should include values > 1.0
        assert!(
            grid.iter().any(|&x| x > 1.0),
            "Grid should include DF > 1.0 for negative rates"
        );

        // Grid should still be bounded
        assert!(
            grid.iter().all(|&x| x > 0.0 && x <= 1.5),
            "Grid should be bounded between 0 and 1.5"
        );
    }

    #[test]
    fn test_adaptive_scan_grid_high_rates() {
        // High rate environment where DF is low
        let grid = DiscountCurveCalibrator::adaptive_scan_grid(0.4);

        // Grid should extend to low values
        assert!(
            grid.iter().any(|&x| x < 0.5),
            "Grid should include low DF values for high rates"
        );
    }

    #[test]
    fn test_settlement_days_by_currency() {
        use crate::calibration::quote::settlement_days_for_currency;

        // Standard T+2 currencies
        assert_eq!(settlement_days_for_currency(Currency::USD), 2);
        assert_eq!(settlement_days_for_currency(Currency::EUR), 2);
        assert_eq!(settlement_days_for_currency(Currency::JPY), 2);

        // GBP is T+0
        assert_eq!(settlement_days_for_currency(Currency::GBP), 0);

        // AUD/CAD are T+1
        assert_eq!(settlement_days_for_currency(Currency::AUD), 1);
        assert_eq!(settlement_days_for_currency(Currency::CAD), 1);
    }

    #[test]
    fn test_calibrator_settlement_date() {
        // Wednesday, January 15, 2025 - a normal business day
        let base_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");

        // USD should use T+2 business days: Wed -> Fri (Jan 17)
        let usd_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
        let usd_settlement = usd_calibrator
            .settlement_date()
            .expect("Settlement should succeed");
        assert_eq!(
            usd_settlement,
            Date::from_calendar_date(2025, Month::January, 17).expect("Valid date"),
            "USD should settle T+2 business days (Wed -> Fri)"
        );

        // GBP should use T+0 (same business day)
        let gbp_calibrator = DiscountCurveCalibrator::new("GBP-SONIA", base_date, Currency::GBP);
        let gbp_settlement = gbp_calibrator
            .settlement_date()
            .expect("Settlement should succeed");
        assert_eq!(gbp_settlement, base_date, "GBP should settle T+0");

        // Explicit override: T+1
        let custom_calibrator = DiscountCurveCalibrator::new("CUSTOM", base_date, Currency::USD)
            .with_settlement_days(1);
        let custom_settlement = custom_calibrator
            .settlement_date()
            .expect("Settlement should succeed");
        assert_eq!(
            custom_settlement,
            Date::from_calendar_date(2025, Month::January, 16).expect("Valid date"),
            "Custom settlement should override currency default (T+1)"
        );
    }

    #[test]
    fn test_calibrator_settlement_date_over_weekend() {
        // Friday, January 17, 2025
        let base_date =
            Date::from_calendar_date(2025, Month::January, 17).expect("Valid test date");

        // USD T+2 from Friday should skip weekend AND MLK Day (Jan 20):
        // Fri Jan 17 + 2 business days = Wed Jan 22 (skipping Sat 18, Sun 19, MLK Day Mon 20)
        let usd_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
        let usd_settlement = usd_calibrator
            .settlement_date()
            .expect("Settlement should succeed");
        assert_eq!(
            usd_settlement,
            Date::from_calendar_date(2025, Month::January, 22).expect("Valid date"),
            "USD T+2 from Friday should land on Wednesday (skipping weekend and MLK Day)"
        );
    }

    #[test]
    fn test_calibrator_settlement_date_over_holiday() {
        // Monday, December 23, 2024 - before Christmas
        let base_date =
            Date::from_calendar_date(2024, Month::December, 23).expect("Valid test date");

        // USD T+2 from Dec 23 should skip Christmas (Dec 25) and land on Dec 26
        let usd_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
        let usd_settlement = usd_calibrator
            .settlement_date()
            .expect("Settlement should succeed");

        // Expected: Dec 23 + 2 business days = Dec 26 (skipping Dec 25 Christmas)
        assert_eq!(
            usd_settlement,
            Date::from_calendar_date(2024, Month::December, 26).expect("Valid date"),
            "USD T+2 from Dec 23 should skip Christmas and land on Dec 26"
        );
    }

    #[test]
    fn test_curve_day_count_config() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // USD default should be ACT/360
        let usd_cal = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
        assert!(
            matches!(usd_cal.effective_curve_day_count(), DayCount::Act360),
            "USD default day count should be ACT/360"
        );

        // GBP default should be ACT/365F
        let gbp_cal = DiscountCurveCalibrator::new("GBP-SONIA", base_date, Currency::GBP);
        assert!(
            matches!(gbp_cal.effective_curve_day_count(), DayCount::Act365F),
            "GBP default day count should be ACT/365F"
        );

        // Can override
        let custom = DiscountCurveCalibrator::new("CUSTOM", base_date, Currency::USD)
            .with_curve_day_count(DayCount::Act365F);
        assert!(
            matches!(custom.effective_curve_day_count(), DayCount::Act365F),
            "Should be able to override curve day count"
        );
    }

    #[test]
    fn test_ois_index_registry() {
        use crate::calibration::quote::{is_overnight_index, lookup_index_info, RateIndexFamily};

        // Overnight indices
        assert!(is_overnight_index("SOFR"), "SOFR should be overnight");
        assert!(
            is_overnight_index("USD-SOFR"),
            "USD-SOFR should be overnight"
        );
        assert!(is_overnight_index("SONIA"), "SONIA should be overnight");
        assert!(is_overnight_index("ESTR"), "ESTR should be overnight");
        assert!(
            is_overnight_index("EUR-€STR"),
            "EUR-€STR should be overnight"
        );
        assert!(is_overnight_index("TONA"), "TONA should be overnight");

        // Term indices (NOT overnight)
        assert!(!is_overnight_index("LIBOR"), "LIBOR should be term");
        assert!(!is_overnight_index("3M-LIBOR"), "3M-LIBOR should be term");
        assert!(!is_overnight_index("EURIBOR"), "EURIBOR should be term");
        assert!(
            !is_overnight_index("6M-EURIBOR"),
            "6M-EURIBOR should be term"
        );

        // Edge cases - should NOT match
        assert!(
            !is_overnight_index("RANDOM-INDEX"),
            "Unknown index should default to not-overnight"
        );

        // Verify lookup returns correct info
        let sofr_info = lookup_index_info("SOFR").expect("SOFR should be in registry");
        assert_eq!(sofr_info.family, RateIndexFamily::Overnight);
        assert_eq!(sofr_info.currency, Currency::USD);
        assert_eq!(sofr_info.settlement_days, 2);

        let sonia_info = lookup_index_info("SONIA").expect("SONIA should be in registry");
        assert_eq!(sonia_info.family, RateIndexFamily::Overnight);
        assert_eq!(sonia_info.currency, Currency::GBP);
        assert_eq!(sonia_info.settlement_days, 0); // GBP settles T+0
    }
}
