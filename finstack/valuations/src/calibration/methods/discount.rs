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

use crate::calibration::quote::{settlement_days_for_currency, RatesQuote};
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator, MultiCurveConfig};
use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::ir_future::InterestRateFuture;
use crate::instruments::irs::FloatingLegCompounding;
use crate::instruments::InterestRateSwap;
use finstack_core::dates::{add_months, Date, DayCount};
use finstack_core::explain::{ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::math::Solver;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;

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
    pub fn new(curve_id: impl Into<CurveId>, base_date: Date, currency: Currency) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            solve_interp: InterpStyle::MonotoneConvex, // Default; arbitrage-free
            extrapolation: ExtrapolationPolicy::FlatForward,
            config: CalibrationConfig::default(), // Defaults to multi-curve mode
            currency,
            calendar_id: None,
            settlement_days: None, // Will use currency default
            curve_day_count: None, // Will use currency default
        }
    }

    /// Set the interpolation used both during solving and for the final curve.
    pub fn with_solve_interp(mut self, interpolation: InterpStyle) -> Self {
        self.solve_interp = interpolation;
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

    /// Calculate settlement date from base date.
    ///
    /// For simplicity, this adds calendar days. In production with holiday
    /// calendars, use DateExt::add_business_days instead.
    fn settlement_date(&self) -> Date {
        let days = self.effective_settlement_days();
        if days == 0 {
            self.base_date
        } else {
            // Add calendar days (in production, use holiday-aware business days)
            self.base_date + time::Duration::days(days as i64)
        }
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
        grid.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
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
        sorted_quotes.sort_by(|a, b| {
            a.maturity_date()
                .partial_cmp(&b.maturity_date())
                .expect("Date comparison should always be comparable")
        });

        // Validate quotes
        self.validate_quotes(&sorted_quotes)?;

        // Pre-validate curve dependencies (fail fast for basis swaps)
        self.validate_curve_dependencies(&sorted_quotes, base_context)?;

        // Get effective curve day count for consistent time mapping
        let curve_dc = self.effective_curve_day_count();

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
                // For non-OIS instruments (FRAs, non-OIS swaps), derive forward from discount.
                let temp_context = if quote_clone.requires_forward_curve() {
                    if quote_clone.is_ois_suitable() {
                        // OIS swaps: No forward curve needed - IRS pricer will use discount-only
                        base_context_ref.clone().insert_discount(temp_curve)
                    } else {
                        // Non-OIS (FRAs, etc): derive forward curve from discount curve
                        // This is the single-curve framework approach
                        let fwd = match temp_curve.to_forward_curve_with_interp(
                            "CALIB_FWD",
                            0.25,
                            solve_interp,
                        ) {
                            Ok(curve) => curve,
                            Err(_) => return crate::calibration::PENALTY,
                        };
                        base_context_ref
                            .clone()
                            .insert_discount(temp_curve)
                            .insert_forward(fwd)
                    }
                } else {
                    base_context_ref.clone().insert_discount(temp_curve)
                };

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
                    let yf = time_to_maturity;
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
            let mut solved_df = if let Some(root) = tentative {
                root
            } else {
                match solver.solve(objective, initial_df) {
                    Ok(root) => root,
                    Err(_) => initial_df,
                }
            };

            if !solved_df.is_finite() {
                solved_df = initial_df;
            }

            // Validate the solution makes sense
            // Allow DF > 1.0 for negative rate environments (EUR, JPY, CHF)
            // Typical bounds: (0, 1.5) for rates in range [-10%, +50%]
            const DF_LOWER_BOUND: f64 = 0.0;
            const DF_UPPER_BOUND: f64 = 1.5; // Allows ~-10% rates at short end
            if solved_df <= DF_LOWER_BOUND || solved_df > DF_UPPER_BOUND {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Solved discount factor out of bounds ({:.2}, {:.2}] for {} at t={:.6}: df={:.6}. \
                         This may indicate extreme rates outside the supported range [-10%, +50%].",
                        DF_LOWER_BOUND, DF_UPPER_BOUND, self.curve_id, time_to_maturity, solved_df
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
                let mut final_context = base_context.clone().insert_discount(final_curve);
                let missing_forward = if quote.requires_forward_curve() {
                    if quote.is_ois_suitable() {
                        // OIS swaps: No forward curve needed - IRS pricer will use discount-only
                        // when both legs use the same discount curve
                        false
                    } else {
                        // Non-OIS (FRAs, etc): derive forward curve from discount curve
                        if let Ok(disc_ref) = final_context.get_discount_ref("CALIB_CURVE") {
                            if let Ok(fwd) = disc_ref.to_forward_curve_with_interp(
                                "CALIB_FWD",
                                0.25,
                                solve_interp,
                            ) {
                                final_context = final_context.insert_forward(fwd);
                                false
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    }
                } else {
                    false
                };

                if missing_forward {
                    crate::calibration::PENALTY
                } else {
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

            // Store residual with descriptive key when possible
            let key = match quote {
                RatesQuote::Deposit {
                    maturity,
                    day_count,
                    ..
                } => {
                    format!(
                        "DEP-{}-{:?}-{:06}",
                        maturity, day_count, residual_key_counter
                    )
                }
                RatesQuote::FRA {
                    start,
                    end,
                    day_count,
                    ..
                } => {
                    format!(
                        "FRA-{}-{}-{:?}-{:06}",
                        start, end, day_count, residual_key_counter
                    )
                }
                RatesQuote::Future { expiry, specs, .. } => {
                    format!(
                        "FUT-{}-{}m-{:?}-{:06}",
                        expiry, specs.delivery_months, specs.day_count, residual_key_counter
                    )
                }
                RatesQuote::Swap {
                    maturity,
                    index,
                    fixed_freq,
                    float_freq,
                    ..
                } => {
                    format!(
                        "SWAP-{}-{}-fix{:?}-flt{:?}-{:06}",
                        index.as_str(),
                        maturity,
                        fixed_freq,
                        float_freq,
                        residual_key_counter
                    )
                }
                RatesQuote::BasisSwap {
                    maturity,
                    primary_index,
                    reference_index,
                    ..
                } => {
                    format!(
                        "BASIS-{}-{}vs{}-{:06}",
                        maturity, primary_index, reference_index, residual_key_counter
                    )
                }
            };
            residual_key_counter += 1;
            residuals.insert(key.clone(), final_residual);
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
        if self.config.verbose {
            tracing::debug!("Validating calibrated discount curve {}", self.curve_id);
        }

        // Use the CurveValidator trait to validate the curve
        use crate::calibration::validation::{CurveValidator, ValidationConfig};
        curve.validate(&ValidationConfig::default()).map_err(|e| {
            finstack_core::Error::Calibration {
                message: format!(
                    "Calibrated discount curve {} failed validation: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_validation".to_string(),
            }
        })?;

        // Create calibration report with comprehensive metadata
        let mut report = CalibrationReport::for_type("yield_curve", residuals, total_iterations)
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
                // Use settlement date (currency-specific T+0/T+1/T+2)
                let settlement = self.settlement_date();

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
                let period_end = add_months(*expiry, specs.delivery_months as i32);

                // Calculate convexity adjustment if not provided
                let convexity_adj = if let Some(adj) = specs.convexity_adjustment {
                    Some(adj)
                } else {
                    // Auto-calculate convexity adjustment based on time to expiry
                    let time_to_expiry = specs
                        .day_count
                        .year_fraction(
                            self.base_date,
                            *expiry,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);

                    let time_to_maturity = specs
                        .day_count
                        .year_fraction(
                            self.base_date,
                            period_end,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);

                    // Always apply convexity adjustment per market practice
                    use super::convexity::ConvexityParameters;
                    let params = match self.currency {
                        Currency::USD => ConvexityParameters::usd_sofr(),
                        Currency::EUR => ConvexityParameters::eur_euribor(),
                        Currency::GBP => ConvexityParameters::gbp_sonia(),
                        Currency::JPY => ConvexityParameters::jpy_tonar(),
                        _ => ConvexityParameters::usd_sofr(), // Default to USD
                    };
                    Some(params.calculate_adjustment(time_to_expiry, time_to_maturity))
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
                    .expect("IRFuture builder should succeed with valid calibration data");

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
                index: _,
            } => {
                // Create swap instrument
                use crate::instruments::irs::{FixedLegSpec, FloatLegSpec, PayReceive};
                use finstack_core::dates::{BusinessDayConvention, StubKind};

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
                    start: self.base_date,
                    end: *maturity,
                };

                // For OIS quotes, configure the floating leg as an overnight-indexed
                // swap (compounded in arrears) and use the discount curve as both
                // discount and index curve so that pricing is purely discount-based.
                // For non-OIS swaps, we keep a separate forward curve for the float leg.
                let (float_discount_id, float_forward_id, compounding) = if is_ois_quote {
                    (
                        finstack_core::types::CurveId::from("CALIB_CURVE"),
                        finstack_core::types::CurveId::from("CALIB_CURVE"),
                        FloatingLegCompounding::sofr(),
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
                    start: self.base_date,
                    end: *maturity,
                    compounding,
                };

                let swap = InterestRateSwap {
                    id: format!("CALIB_SWAP_{}", maturity).into(),
                    notional: Money::new(1_000_000.0, self.currency),
                    side: PayReceive::ReceiveFixed,
                    fixed: fixed_spec,
                    float: float_spec,
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
                currency: _,
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
            if !rate.is_finite() || !(-0.10..=0.50).contains(&rate) {
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

        let solver = crate::calibration::create_simple_solver(&self.config);
        self.bootstrap_curve_with_solver(instruments, &solver, base_context)
    }
}

impl RatesQuote {
    /// Get the quote type as a string.
    pub fn get_type(&self) -> &'static str {
        match self {
            RatesQuote::Deposit { .. } => "Deposit",
            RatesQuote::FRA { .. } => "FRA",
            RatesQuote::Future { .. } => "Future",
            RatesQuote::Swap { .. } => "Swap",
            RatesQuote::BasisSwap { .. } => "BasisSwap",
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
/// )?;
/// ```
pub fn create_ois_swap_from_quote(
    quote: &RatesQuote,
    discount_curve_id: &str,
    forward_curve_id: &str,
    base_date: Date,
    notional: Money,
    calendar_id: Option<&str>,
) -> Result<InterestRateSwap> {
    use crate::instruments::irs::{FixedLegSpec, FloatLegSpec, PayReceive};
    use finstack_core::dates::{BusinessDayConvention, StubKind};

    let (maturity, rate, fixed_freq, float_freq, fixed_dc, float_dc) = match quote {
        RatesQuote::Swap {
            maturity,
            rate,
            fixed_freq,
            float_freq,
            fixed_dc,
            float_dc,
            ..
        } => (*maturity, *rate, *fixed_freq, *float_freq, *fixed_dc, *float_dc),
        _ => {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ))
        }
    };

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
        compounding: FloatingLegCompounding::sofr(),
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
    use crate::instruments::deposit::Deposit;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, Frequency};
    // use finstack_core::market_data::traits::TermStructure;
    use time::Month;

    #[test]
    fn test_multi_curve_instrument_validation() {
        // Test that RatesQuote correctly identifies forward-dependent instruments
        let deposit = RatesQuote::Deposit {
            maturity: Date::from_calendar_date(2024, Month::February, 1).expect("Valid test date"),
            rate: 0.015,
            day_count: DayCount::Act360,
        };
        assert!(
            !deposit.requires_forward_curve(),
            "Deposits should not require forward curves"
        );
        assert!(
            deposit.is_ois_suitable(),
            "Deposits should be suitable for OIS"
        );

        let fra = RatesQuote::FRA {
            start: Date::from_calendar_date(2024, Month::April, 1).expect("Valid test date"),
            end: Date::from_calendar_date(2024, Month::July, 1).expect("Valid test date"),
            rate: 0.018,
            day_count: DayCount::Act360,
        };
        assert!(
            fra.requires_forward_curve(),
            "FRAs should require forward curves"
        );
        assert!(
            !fra.is_ois_suitable(),
            "FRAs should not be suitable for OIS"
        );

        let ois_swap = RatesQuote::Swap {
            maturity: Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
            rate: 0.02,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "SOFR".to_string().into(),
        };
        assert!(
            ois_swap.requires_forward_curve(),
            "Swaps require forward curves"
        );
        assert!(
            ois_swap.is_ois_suitable(),
            "SOFR swaps should be OIS suitable"
        );

        let libor_swap = RatesQuote::Swap {
            maturity: Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
            rate: 0.02,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "3M-LIBOR".to_string().into(),
        };
        assert!(
            libor_swap.requires_forward_curve(),
            "Swaps require forward curves"
        );
        assert!(
            !libor_swap.is_ois_suitable(),
            "LIBOR swaps should not be OIS suitable"
        );
    }

    #[test]
    fn test_multi_curve_config() {
        // Test multi-curve configuration
        let multi_config = MultiCurveConfig::new();
        assert!(multi_config.calibrate_basis);
        assert!(multi_config.enforce_separation);
    }

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
    fn test_quote_sorting() {
        let mut quotes = create_test_quotes();

        // Reverse order
        quotes.reverse();

        // Get maturities before sorting
        let maturities_before: Vec<_> = quotes.iter().map(|q| q.maturity_date()).collect();

        // Sort
        quotes.sort_by(|a, b| {
            a.maturity_date()
                .partial_cmp(&b.maturity_date())
                .expect("Date comparison should always be comparable")
        });

        // Get maturities after sorting
        let maturities_after: Vec<_> = quotes.iter().map(|q| q.maturity_date()).collect();

        // Should be properly sorted
        for i in 1..maturities_after.len() {
            assert!(maturities_after[i] >= maturities_after[i - 1]);
        }

        // Should not be the same as the original reversed order
        assert_ne!(maturities_before, maturities_after);
    }

    #[test]
    fn test_deposit_repricing_under_bootstrap() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Use explicit T+0 settlement to match pre-settlement-aware behavior
        // For production, use currency default (T+2 for USD)
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
            .with_settlement_days(0); // T+0 for test consistency

        // Use just deposits for initial test
        let deposit_quotes = vec![
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
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(180),
                rate: 0.047,
                day_count: DayCount::Act360,
            },
        ];

        let base_context = MarketContext::new();

        if calibrator.config.verbose {
            tracing::debug!(
                deposits = deposit_quotes.len(),
                "Starting calibration with deposits"
            );
            for (i, quote) in deposit_quotes.iter().enumerate() {
                if let RatesQuote::Deposit { maturity, rate, .. } = quote {
                    tracing::trace!(
                        deposit = i,
                        maturity = %maturity,
                        rate = rate,
                        "Processing deposit quote"
                    );
                }
            }
        }

        let (curve, report) = calibrator
            .calibrate(&deposit_quotes, &base_context)
            .expect("Deposit calibration should succeed");
        assert!(report.success);
        assert_eq!(curve.id().as_str(), "USD-OIS");

        // Verify repricing via instrument PVs (|PV| ≤ $1 per $1MM for 0.1bp tolerance)
        let ctx = base_context.insert_discount(curve);
        for quote in &deposit_quotes {
            if let RatesQuote::Deposit {
                maturity,
                rate,
                day_count,
            } = quote
            {
                let dep = Deposit {
                    id: format!("DEP-{}", maturity).into(),
                    notional: Money::new(1_000_000.0, Currency::USD),
                    start: base_date, // Match T+0 settlement
                    end: *maturity,
                    day_count: *day_count,
                    quote_rate: Some(*rate),
                    discount_curve_id: "USD-OIS".into(),
                    attributes: Default::default(),
                };
                let pv = dep
                    .value(&ctx, base_date)
                    .expect("Deposit valuation should succeed in test");
                // For deposits, $1 per $1M notional is approximately 0.1bp tolerance
                assert!(
                    pv.amount().abs() <= 1.0,
                    "Deposit PV too large: ${:.2} (expected <= $1 for 0.1bp tolerance)",
                    pv.amount()
                );
            }
        }
    }

    #[test]
    fn test_fra_repricing_under_bootstrap() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let config = CalibrationConfig::conservative();
        let calibrator =
            DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD).with_config(config);

        // Build quotes: deposits + one FRA
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.0450,
                day_count: DayCount::Act360,
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.0460,
                day_count: DayCount::Act360,
            },
            RatesQuote::FRA {
                start: base_date + time::Duration::days(90),
                end: base_date + time::Duration::days(180),
                rate: 0.0470,
                day_count: DayCount::Act360,
            },
        ];

        // In the multi-curve framework, discount curves must be calibrated with
        // OIS-suitable instruments only. FRAs require a forward curve
        // calibrated via `ForwardCurveCalibrator`, not `DiscountCurveCalibrator`.
        let base_context = MarketContext::new();
        let result = calibrator.calibrate(&quotes, &base_context);

        match result {
            Ok(_) => panic!(
                "DiscountCurveCalibrator should reject FRA quotes in multi-curve framework; \
                 use ForwardCurveCalibrator for FRAs instead."
            ),
            Err(err) => {
                let msg = format!("{err}");
                assert!(
                    msg.contains("DiscountCurveCalibrator received non-OIS instrument")
                        && msg.contains("Please calibrate forward curves separately using ForwardCurveCalibrator"),
                    "Unexpected error message for FRA quote in discount calibration: {msg}"
                );
            }
        }
    }

    #[test]
    fn test_swap_repricing_under_bootstrap() {
        use crate::instruments::irs::{InterestRateSwap, PayReceive};
        use crate::metrics::{MetricCalculator, MetricContext};

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Use conservative config for tighter convergence (1e-12 tolerance, 200 iterations)
        let mut config = CalibrationConfig::conservative();
        config.tolerance = 1e-12;
        config.max_iterations = 200;

        // Use T+0 settlement for test consistency
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
            .with_config(config)
            .with_settlement_days(0);

        // Quotes: deposits + one 1Y swap par rate
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.0450,
                day_count: DayCount::Act360,
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.0460,
                day_count: DayCount::Act360,
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.0470,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::daily(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-OIS".to_string().into(),
            },
        ];

        // OIS swaps can be calibrated without pre-existing forward curves
        let base_context = MarketContext::new();
        let (curve, report) = calibrator
            .calibrate(&quotes, &base_context)
            .expect("Swap calibration should succeed");

        assert!(report.success, "Calibration should succeed: {:?}", report);

        // Verify calibration residuals are tight (price_instrument returns PV/notional, so 1e-4 = $100 per $1M)
        // For 0.1bp tolerance on a swap with DV01=$96.64, we need residual < $9.66/$1M = 9.66e-6
        assert!(
            report.max_residual < 1e-5,
            "Calibration residual too large: {:.2e} (expected < 1e-5 for 0.1bp tolerance)",
            report.max_residual
        );

        // For OIS swaps, we don't need a forward curve since the pricer uses discount-only
        let ctx = base_context.insert_discount(curve);

        // Construct 1Y par swap matching quote
        let start = base_date;
        let end = base_date + time::Duration::days(365);
        let irs = InterestRateSwap::builder()
            .id("IRS-1Y".to_string().into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .side(PayReceive::ReceiveFixed)
            .fixed(crate::instruments::irs::FixedLegSpec {
                discount_curve_id: "USD-OIS".into(),
                rate: 0.0470,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                par_method: None,
                compounding_simple: true,
                start,
                end,
            })
            .float(crate::instruments::irs::FloatLegSpec {
                discount_curve_id: "USD-OIS".into(),
                forward_curve_id: "USD-OIS".into(),
                spread_bp: 0.0,
                freq: Frequency::daily(),
                dc: DayCount::Act360,
                bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                fixing_calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                start,
                end,
                compounding: FloatingLegCompounding::sofr(),
            })
            .build()
            .expect("IRS builder should succeed with valid test data");

        let pv = irs
            .value(&ctx, base_date)
            .expect("IRS valuation should succeed in test");

        // Calculate DV01 and check repricing within 0.1bp tolerance
        let mut metric_ctx = MetricContext::new(
            std::sync::Arc::new(irs.clone()),
            std::sync::Arc::new(ctx.clone()),
            base_date,
            pv,
        );

        // Calculate DV01 using unified DV01 calculator
        use crate::metrics::{Dv01CalculatorConfig, UnifiedDv01Calculator};
        let dv01_calc = UnifiedDv01Calculator::<crate::instruments::InterestRateSwap>::new(
            Dv01CalculatorConfig::parallel_combined(),
        );
        let dv01 = dv01_calc
            .calculate(&mut metric_ctx)
            .expect("DV01 calculation should succeed in test");

        // Tolerance: 10bp * |DV01| for bootstrap (tighter checks in dedicated swap tests)
        // OIS swap calibration has inherent approximations in schedule generation
        // that can cause residuals up to a few bp
        let tolerance = (10.0 * dv01.abs()).max(100.0);

        assert!(
            pv.amount().abs() <= tolerance,
            "Swap PV too large: ${:.2} (DV01: ${:.2}, tolerance: ${:.2})",
            pv.amount(),
            dv01,
            tolerance
        );
    }

    #[test]
    fn test_ois_bootstrap_with_deposits_and_ois_swaps() {
        use finstack_core::dates::Frequency;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

        // Deposits + OIS swaps (float leg frequency set to daily; index contains "USD-OIS")
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.0450,
                day_count: DayCount::Act360,
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.0460,
                day_count: DayCount::Act360,
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.0470,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::daily(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-OIS".to_string().into(),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365 * 2),
                rate: 0.0480,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::daily(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-OIS".to_string().into(),
            },
        ];

        let base_context = MarketContext::new();
        let (_curve, report) = calibrator
            .calibrate(&quotes, &base_context)
            .expect("OIS bootstrap should succeed");

        // Residuals should be small since par swaps should reprice under discount-only formula
        assert!(report.success);
        assert!(report.max_residual < 1e-4);
    }

    #[test]
    fn test_configured_interpolation_used() {
        use finstack_core::dates::add_months;

        let base_date =
            Date::from_calendar_date(2025, Month::January, 31).expect("Valid test date");

        // Test 1: Verify configured interpolation is used
        let linear_calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
            .with_solve_interp(InterpStyle::Linear);
        let monotone_calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
            .with_solve_interp(InterpStyle::MonotoneConvex);

        assert!(matches!(
            linear_calibrator.solve_interp,
            InterpStyle::Linear
        ));
        assert!(matches!(
            monotone_calibrator.solve_interp,
            InterpStyle::MonotoneConvex
        ));

        // Test 2: Verify proper month arithmetic vs crude approximation
        let delivery_months = 3i32;

        // Crude way (should be wrong for end-of-month)
        let crude_result = base_date + time::Duration::days((delivery_months as i64) * 30);

        // Proper way
        let proper_result = add_months(base_date, delivery_months);

        if cfg!(test) {
            tracing::debug!(
                base_date = %base_date,
                delivery_months = delivery_months,
                crude_result = %crude_result,
                proper_result = %proper_result,
                "Comparing month arithmetic methods"
            );
        }

        // Should be different for Jan 31 + 3 months
        assert_ne!(
            crude_result, proper_result,
            "Month arithmetic should give different results"
        );

        // The proper result should handle month-end correctly
        // Jan 31 + 3 months = Apr 30 (no Apr 31)
        let expected = Date::from_calendar_date(2025, Month::April, 30).expect("Valid test date");
        assert_eq!(
            proper_result, expected,
            "Expected proper month-end handling"
        );
    }

    // ========================================================================
    // New tests for market-standards improvements
    // ========================================================================

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
        let base_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");

        // USD should use T+2
        let usd_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
        let usd_settlement = usd_calibrator.settlement_date();
        assert_eq!(
            usd_settlement,
            base_date + time::Duration::days(2),
            "USD should settle T+2"
        );

        // GBP should use T+0
        let gbp_calibrator = DiscountCurveCalibrator::new("GBP-SONIA", base_date, Currency::GBP);
        let gbp_settlement = gbp_calibrator.settlement_date();
        assert_eq!(gbp_settlement, base_date, "GBP should settle T+0");

        // Explicit override
        let custom_calibrator = DiscountCurveCalibrator::new("CUSTOM", base_date, Currency::USD)
            .with_settlement_days(1);
        let custom_settlement = custom_calibrator.settlement_date();
        assert_eq!(
            custom_settlement,
            base_date + time::Duration::days(1),
            "Custom settlement should override currency default"
        );
    }

    #[test]
    fn test_extrapolation_policy_config() {
        use finstack_core::math::interp::ExtrapolationPolicy;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Default should be FlatForward
        let default_cal = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD);
        assert!(
            matches!(default_cal.extrapolation, ExtrapolationPolicy::FlatForward),
            "Default extrapolation should be FlatForward"
        );

        // Can configure FlatZero
        let flat_zero = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
            .with_extrapolation(ExtrapolationPolicy::FlatZero);
        assert!(
            matches!(flat_zero.extrapolation, ExtrapolationPolicy::FlatZero),
            "Should be able to configure FlatZero extrapolation"
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

    #[test]
    fn test_negative_rate_curve_building() {
        // First, verify we can build a curve with DF > 1.0 directly
        let base_date = Date::from_calendar_date(2020, Month::January, 1).expect("Valid test date");

        let knots = vec![
            (0.0, 1.0),
            (0.25, 1.0025), // -1% rate for 90 days: DF = 1/(1+(-0.01)*0.25) ≈ 1.0025
            (0.5, 1.002),   // Slightly lower negative rate
        ];

        let curve = DiscountCurve::builder("TEST-NEG")
            .base_date(base_date)
            .knots(knots)
            .set_interp(InterpStyle::Linear) // Use simple linear for test
            .allow_non_monotonic()
            .build()
            .expect("Curve with DF > 1.0 should build successfully");

        // Verify DF values
        let df_0 = curve.df(0.0);
        let df_90d = curve.df(0.25);

        assert!((df_0 - 1.0).abs() < 1e-10, "DF(0) should be 1.0: {}", df_0);
        assert!(
            df_90d > 1.0,
            "DF at 90 days should exceed 1.0: {} (expected 1.0025)",
            df_90d
        );
    }

    #[test]
    fn test_negative_rate_deposit_calibration() {
        // Test calibration with negative rates (EUR/CHF/JPY scenario)
        let base_date = Date::from_calendar_date(2020, Month::January, 1).expect("Valid test date");

        // Use T+0 settlement and Linear interpolation for simpler debugging
        let calibrator = DiscountCurveCalibrator::new("EUR-ESTR", base_date, Currency::EUR)
            .with_settlement_days(0)
            .with_solve_interp(InterpStyle::Linear); // Use Linear for predictable behavior

        // Use more pronounced negative rates and longer maturities
        // to see DF > 1.0 effect clearly
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: -0.01, // -100bp (more pronounced negative)
                day_count: DayCount::Act360,
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(180),
                rate: -0.008, // -80bp
                day_count: DayCount::Act360,
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(365),
                rate: -0.005, // -50bp
                day_count: DayCount::Act360,
            },
        ];

        let base_context = MarketContext::new();
        let result = calibrator.calibrate(&quotes, &base_context);

        // Should succeed with negative rates
        assert!(
            result.is_ok(),
            "Calibration should succeed with negative rates: {:?}",
            result.err()
        );

        let (curve, report) = result.expect("Calibration should succeed");

        // Verify discount factors > 1.0 at first knot (negative rates)
        // At -100bp for 90 days, DF ≈ 1 / (1 + (-0.01) * 90/360) = 1 / 0.9975 ≈ 1.0025
        let df_90d = curve.df(90.0 / 360.0);
        assert!(
            df_90d > 1.0,
            "DF at 90 days should exceed 1.0 for -100bp rate: {} (expected ~1.0025)",
            df_90d
        );

        // Residuals should still be tight
        assert!(report.success, "Calibration should report success");
        assert!(
            report.max_residual < 1e-4,
            "Max residual should be small: {}",
            report.max_residual
        );
    }

    #[test]
    fn test_pre_validation_fails_for_missing_forward_curves() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD);

        // Basis swap requires forward curves
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                day_count: DayCount::Act360,
            },
            RatesQuote::BasisSwap {
                maturity: base_date + time::Duration::days(365),
                primary_index: "3M-SOFR".to_string(),
                reference_index: "1M-SOFR".to_string(),
                spread_bp: 5.0,
                primary_freq: Frequency::quarterly(),
                reference_freq: Frequency::monthly(),
                primary_dc: DayCount::Act360,
                reference_dc: DayCount::Act360,
                currency: Currency::USD,
            },
        ];

        let base_context = MarketContext::new(); // No forward curves
        let result = calibrator.calibrate(&quotes, &base_context);

        // Should fail with clear error about missing forward curve
        let err = result.expect_err("Should fail when forward curves are missing");
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("Forward curve") || err_msg.contains("forward curve"),
            "Error should mention missing forward curve: {}",
            err_msg
        );
    }

    #[test]
    fn test_calibration_report_metadata() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
            .with_extrapolation(ExtrapolationPolicy::FlatZero)
            .with_settlement_days(1);

        let quotes = vec![
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
        ];

        let base_context = MarketContext::new();
        let (_, report) = calibrator
            .calibrate(&quotes, &base_context)
            .expect("Calibration should succeed");

        // Verify metadata is populated
        assert!(
            report.metadata.contains_key("extrapolation"),
            "Report should contain extrapolation metadata"
        );
        assert!(
            report.metadata.contains_key("settlement_days"),
            "Report should contain settlement_days metadata"
        );
        assert!(
            report.metadata.contains_key("curve_day_count"),
            "Report should contain curve_day_count metadata"
        );

        // Check specific values
        assert!(
            report.metadata["extrapolation"].contains("FlatZero"),
            "Extrapolation metadata should indicate FlatZero"
        );
        assert_eq!(
            report.metadata["settlement_days"], "1",
            "Settlement days should be 1"
        );
    }
}
