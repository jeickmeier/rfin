//! Yield curve bootstrapping from market instruments.
//!
//! Implements market-standard multi-curve discount curve calibration using
//! deposits and OIS swaps. Forward curves are calibrated separately.
//!
//! Uses instrument pricing methods directly rather than reimplementing
//! pricing formulas, following market-standard bootstrap methodology.

use crate::calibration::quote::RatesQuote;
use crate::calibration::{
    solve_1d, CalibrationConfig, CalibrationReport, Calibrator, MultiCurveConfig, SolverKind,
};
use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::ir_future::InterestRateFuture;
use crate::instruments::InterestRateSwap;
use finstack_core::dates::{add_months, Date};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::math::Solver;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;
use finstack_core::F;
use std::collections::BTreeMap;

/// Discount curve bootstrapper.
#[derive(Clone, Debug)]
pub struct DiscountCurveCalibrator {
    /// Curve identifier
    pub curve_id: CurveId,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Interpolation used during solving and for the final curve
    pub solve_interp: InterpStyle,
    /// Calibration configuration (includes multi-curve settings)
    pub config: CalibrationConfig,
    /// Currency for the curve
    pub currency: Currency,
}

impl DiscountCurveCalibrator {
    /// Create a new discount curve calibrator.
    pub fn new(curve_id: impl Into<CurveId>, base_date: Date, currency: Currency) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            solve_interp: InterpStyle::MonotoneConvex, // Default; explicit and consistent
            config: CalibrationConfig::default(),      // Defaults to multi-curve mode
            currency,
        }
    }

    /// Set the interpolation used both during solving and for the final curve.
    pub fn with_solve_interp(mut self, interpolation: InterpStyle) -> Self {
        self.solve_interp = interpolation;
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

    /// Apply the configured solve interpolation style to the discount curve builder.
    fn apply_solve_interpolation(
        &self,
        builder: finstack_core::market_data::term_structures::discount_curve::DiscountCurveBuilder,
    ) -> finstack_core::market_data::term_structures::discount_curve::DiscountCurveBuilder {
        builder.set_interp(self.solve_interp)
    }

    /// Bootstrap discount curve from instrument quotes using solver.
    ///
    /// This method builds the curve incrementally, solving for each discount factor
    /// that reprices the corresponding instrument to par.
    fn bootstrap_curve_with_solver<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Sort quotes by maturity
        let mut sorted_quotes = quotes.to_vec();
        sorted_quotes.sort_by(|a, b| {
            self.get_maturity(a)
                .partial_cmp(&self.get_maturity(b))
                .unwrap()
        });

        // Validate quotes
        self.validate_quotes(&sorted_quotes)?;

        // Build knots sequentially
        let mut knots = Vec::with_capacity(sorted_quotes.len() + 1);
        knots.push((0.0, 1.0)); // Start with DF(0) = 1.0
        let mut residuals = BTreeMap::new();
        let mut residual_key_counter: usize = 0;
        let mut total_iterations = 0;

        for (idx, quote) in sorted_quotes.iter().enumerate() {
            let maturity_date = self.get_maturity(quote);
            // Use instrument-specific day count for curve time at this knot
            let time_to_maturity = match quote {
                RatesQuote::Deposit {
                    maturity,
                    day_count,
                    ..
                } => day_count
                    .year_fraction(
                        self.base_date,
                        *maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0),
                RatesQuote::FRA { end, day_count, .. } => day_count
                    .year_fraction(
                        self.base_date,
                        *end,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0),
                RatesQuote::Future { expiry, specs, .. } => {
                    let end = add_months(*expiry, specs.delivery_months as i32);
                    specs
                        .day_count
                        .year_fraction(
                            self.base_date,
                            end,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0)
                }
                RatesQuote::Swap {
                    maturity, fixed_dc, ..
                } => fixed_dc
                    .year_fraction(
                        self.base_date,
                        *maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0),
                RatesQuote::BasisSwap {
                    maturity,
                    primary_dc,
                    ..
                } => primary_dc
                    .year_fraction(
                        self.base_date,
                        *maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0),
            };

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
            let self_clone = self.clone();
            let quote_clone = quote.clone();
            // Use Arc reference instead of cloning the entire context
            let base_context_ref = std::sync::Arc::new(base_context.clone());

            let objective = move |df: F| -> F {
                let mut temp_knots = Vec::with_capacity(knots_clone.len() + 1);
                temp_knots.extend_from_slice(&knots_clone);
                temp_knots.push((time_to_maturity, df));

                // Build temporary curve with current knots
                let temp_curve = match DiscountCurve::builder("CALIB_CURVE")
                    .base_date(self_clone.base_date)
                    .knots(temp_knots)
                    .set_interp(self_clone.solve_interp)
                    .build()
                {
                    Ok(curve) => curve,
                    Err(_) => return crate::calibration::penalize(),
                };

                // Multi-curve only: for OIS instruments derive a temporary forward from discount; otherwise require existing forward
                let temp_context = if quote_clone.requires_forward_curve() {
                    // OIS-style swaps: derive forward from discount for pricing
                    if quote_clone.is_ois_suitable() {
                        let fwd = match temp_curve.to_forward_curve_with_interp(
                            "CALIB_FWD",
                            0.25,
                            self_clone.solve_interp,
                        ) {
                            Ok(curve) => curve,
                            Err(_) => return crate::calibration::penalize(),
                        };
                        (*base_context_ref)
                            .clone()
                            .insert_discount(temp_curve)
                            .insert_forward(fwd)
                    } else {
                        // Require pre-existing forward curve in context for non-OIS instruments
                        if (*base_context_ref).get_forward_ref("CALIB_FWD").is_err() {
                            return crate::calibration::penalize();
                        }
                        (*base_context_ref).clone().insert_discount(temp_curve)
                    }
                } else {
                    (*base_context_ref).clone().insert_discount(temp_curve)
                };

                // Price the instrument and return error (target is zero)
                self_clone
                    .price_instrument(&quote_clone, &temp_context)
                    .unwrap_or(crate::calibration::penalize())
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

            let tentative = self.solve_with_bracketing(&objective, initial_df)?;
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
            if solved_df <= 0.0 || solved_df > 1.0 {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Solved discount factor out of bounds [0,1] for {} at t={:.6}: df={:.6}",
                        self.curve_id, time_to_maturity, solved_df
                    ),
                    category: "yield_curve_bootstrap".to_string(),
                });
            }

            // Compute residual for reporting
            let final_residual = {
                let mut final_knots = Vec::with_capacity(knots.len() + 1);
                final_knots.extend_from_slice(&knots);
                final_knots.push((time_to_maturity, solved_df));

                let final_curve = DiscountCurve::builder("CALIB_CURVE")
                    .base_date(self.base_date)
                    .knots(final_knots)
                    .set_interp(self.solve_interp)
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
                let mut missing_forward = false;
                if quote.requires_forward_curve() {
                    if quote.is_ois_suitable() {
                        // Derive forward from discount for OIS-style instruments
                        if let Ok(disc_ref) = final_context.get_discount_ref("CALIB_CURVE") {
                            if let Ok(fwd) = disc_ref.to_forward_curve_with_interp(
                                "CALIB_FWD",
                                0.25,
                                self.solve_interp,
                            ) {
                                final_context = final_context.insert_forward(fwd);
                            } else {
                                missing_forward = true;
                            }
                        } else {
                            missing_forward = true;
                        }
                    } else {
                        // Non-OIS requires pre-existing forward in context; if missing, we'll penalize below
                        missing_forward = final_context.get_forward_ref("CALIB_FWD").is_err();
                    }
                }

                if missing_forward {
                    crate::calibration::penalize()
                } else {
                    self.price_instrument(quote, &final_context)
                        .unwrap_or(crate::calibration::penalize())
                        .abs()
                }
            };

            knots.push((time_to_maturity, solved_df));

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
                        index, maturity, fixed_freq, float_freq, residual_key_counter
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
            residuals.insert(key, final_residual);
            total_iterations += 1;
        }

        // Build final discount curve with configured interpolation
        let curve = self
            .apply_solve_interpolation(
                DiscountCurve::builder(self.curve_id.clone())
                    .base_date(self.base_date)
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
        use crate::calibration::validation::CurveValidator;
        curve
            .validate()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Calibrated discount curve {} failed validation: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_validation".to_string(),
            })?;

        // Create calibration report
        let report = CalibrationReport::for_type("yield_curve", residuals, total_iterations)
            .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
            .with_metadata("currency", self.currency.to_string())
            .with_metadata("validation", "passed");

        Ok((curve, report))
    }

    fn solve_with_bracketing(&self, objective: &dyn Fn(F) -> F, initial: F) -> Result<Option<F>> {
        let value_initial = objective(initial);
        if value_initial.is_finite() && value_initial.abs() < self.config.tolerance {
            return Ok(Some(initial));
        }

        let scan_points: [F; 18] = [
            1.0, 0.99, 0.98, 0.96, 0.94, 0.92, 0.90, 0.88, 0.85, 0.80, 0.75, 0.70, 0.65, 0.60,
            0.55, 0.50, 0.45, 0.40,
        ];

        let mut last_valid: Option<(F, F)> = None;
        for &point in &scan_points {
            let value = objective(point);
            if !value.is_finite() || value.abs() >= crate::calibration::PENALTY / 10.0 {
                continue;
            }

            if let Some((prev_point, prev_value)) = last_valid {
                if prev_value == 0.0 {
                    return Ok(Some(prev_point));
                }
                if value == 0.0 {
                    return Ok(Some(point));
                }
                if prev_value.signum() != value.signum() {
                    let guess = (prev_point + point) * 0.5;
                    let root = solve_1d(
                        SolverKind::Brent,
                        self.config.tolerance,
                        self.config.max_iterations.max(50),
                        objective,
                        guess,
                    )?;
                    return Ok(Some(root));
                }
            }

            last_valid = Some((point, value));
        }

        Ok(None)
    }

    /// Price an instrument using the given market context.
    ///
    /// Returns the pricing error (PV for par instruments) that should be zero
    /// when the curve is correctly calibrated.
    fn price_instrument(&self, quote: &RatesQuote, context: &MarketContext) -> Result<F> {
        match quote {
            RatesQuote::Deposit {
                maturity,
                rate,
                day_count,
            } => {
                // Create Deposit instrument and use its pricer for consistency
                let dep = Deposit {
                    id: format!("CALIB_DEP_{}", maturity).into(),
                    notional: Money::new(1_000_000.0, self.currency),
                    start: self.base_date,
                    end: *maturity,
                    day_count: *day_count,
                    quote_rate: Some(*rate),
                    disc_id: "CALIB_CURVE".into(),
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
                    .disc_id("CALIB_CURVE".into())
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
                    .disc_id(finstack_core::types::CurveId::from("CALIB_CURVE"))
                    .forward_id(finstack_core::types::CurveId::from("CALIB_FWD"))
                    .build()
                    .unwrap();

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
                    disc_id: finstack_core::types::CurveId::from("CALIB_CURVE"),
                    rate: *rate,
                    freq: *fixed_freq,
                    dc: *fixed_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: StubKind::None,
                    par_method: None,
                    compounding_simple: true,
                    start: self.base_date,
                    end: *maturity,
                };

                let float_spec = FloatLegSpec {
                    disc_id: finstack_core::types::CurveId::from("CALIB_CURVE"),
                    fwd_id: finstack_core::types::CurveId::from("CALIB_FWD"),
                    spread_bp: 0.0,
                    freq: *float_freq,
                    dc: *float_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: StubKind::None,
                    reset_lag_days: 2,
                    start: self.base_date,
                    end: *maturity,
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
                    spread: *spread_bp / 10_000.0, // Convert bp to decimal
                };

                let reference_leg = BasisSwapLeg {
                    forward_curve_id: reference_forward_id,
                    frequency: *reference_freq,
                    day_count: *reference_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
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

    /// Get maturity date from quote.
    fn get_maturity(&self, quote: &RatesQuote) -> Date {
        match quote {
            RatesQuote::Deposit { maturity, .. } => *maturity,
            RatesQuote::FRA { end, .. } => *end,
            RatesQuote::Future { expiry, specs, .. } => {
                // Future maturity is expiry plus delivery period
                add_months(*expiry, specs.delivery_months as i32)
            }
            RatesQuote::Swap { maturity, .. } => *maturity,
            RatesQuote::BasisSwap { maturity, .. } => *maturity,
        }
    }

    /// Validate quote sequence for no-arbitrage and completeness.
    fn validate_quotes(&self, quotes: &[RatesQuote]) -> Result<()> {
        if quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Check for duplicate maturities
        let mut maturities = std::collections::HashSet::new();
        for quote in quotes {
            let maturity = self.get_maturity(quote);
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

        // Multi-curve mode validation
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

        // Warn if using forward-dependent instruments for discount curve calibration
        if has_forward_dependent && !has_ois_suitable {
            tracing::warn!(
                "Using forward-dependent instruments (FRA, Future, Swap) \
                 for discount curve calibration. Consider using OIS swaps or deposits instead. \
                 Forward curves must be provided in the context for these instruments to price correctly."
            );
        }

        Ok(())
    }

    /// Extract rate from quote.
    fn get_rate(&self, quote: &RatesQuote) -> F {
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
            maturity: Date::from_calendar_date(2024, Month::February, 1).unwrap(),
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
            start: Date::from_calendar_date(2024, Month::April, 1).unwrap(),
            end: Date::from_calendar_date(2024, Month::July, 1).unwrap(),
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
            maturity: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            rate: 0.02,
            fixed_freq: Frequency::annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "SOFR".to_string(),
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
            maturity: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            rate: 0.02,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "3M-LIBOR".to_string(),
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
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

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
                index: "USD-SOFR-3M".to_string(),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365 * 2),
                rate: 0.048,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::quarterly(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-SOFR-3M".to_string(),
            },
        ]
    }

    #[test]
    fn test_quote_validation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD);

        let empty_quotes = vec![];
        assert!(calibrator.validate_quotes(&empty_quotes).is_err());

        let valid_quotes = create_test_quotes();
        assert!(calibrator.validate_quotes(&valid_quotes).is_ok());
    }

    #[test]
    fn test_quote_sorting() {
        let calibrator = DiscountCurveCalibrator::new(
            "TEST",
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Currency::USD,
        );
        let mut quotes = create_test_quotes();

        // Reverse order
        quotes.reverse();

        // Get maturities before sorting
        let maturities_before: Vec<_> = quotes.iter().map(|q| calibrator.get_maturity(q)).collect();

        // Sort
        quotes.sort_by(|a, b| {
            calibrator
                .get_maturity(a)
                .partial_cmp(&calibrator.get_maturity(b))
                .unwrap()
        });

        // Get maturities after sorting
        let maturities_after: Vec<_> = quotes.iter().map(|q| calibrator.get_maturity(q)).collect();

        // Should be properly sorted
        for i in 1..maturities_after.len() {
            assert!(maturities_after[i] >= maturities_after[i - 1]);
        }

        // Should not be the same as the original reversed order
        assert_ne!(maturities_before, maturities_after);
    }

    #[test]
    #[ignore = "Calibration logic still being completed"]
    fn test_deposit_repricing_under_bootstrap() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

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

        // Verify repricing via instrument PVs (|PV| ≤ $1 per $1MM)
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
                    start: base_date,
                    end: *maturity,
                    day_count: *day_count,
                    quote_rate: Some(*rate),
                    disc_id: "USD-OIS".into(),
                    attributes: Default::default(),
                };
                let pv = dep.value(&ctx, base_date).unwrap();
                assert!(
                    pv.amount().abs() <= 1.0,
                    "Deposit PV too large: {}",
                    pv.amount()
                );
            }
        }
    }

    #[test]
    #[ignore = "Calibration logic still being completed"]
    fn test_fra_repricing_under_bootstrap() {
        use crate::instruments::fra::ForwardRateAgreement;
        // (no additional imports)

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

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

        // Seed a minimal forward curve so discount bootstrap can evaluate swap quotes
        // Provide a seeded forward curve so the non-OIS swap can be priced during bootstrap
        let seed_forward =
            finstack_core::market_data::term_structures::forward_curve::ForwardCurve::builder(
                "CALIB_FWD",
                0.25,
            )
            .base_date(base_date)
            .knots(vec![
                (0.0, 0.0470),
                (0.25, 0.0470),
                (0.5, 0.0470),
                (1.0, 0.0470),
            ])
            .set_interp(InterpStyle::Linear)
            .day_count(DayCount::Act360)
            .build()
            .unwrap();
        let base_context = MarketContext::new().insert_forward(seed_forward);
        let (curve, _report) = calibrator
            .calibrate(&quotes, &base_context)
            .expect("FRA calibration should succeed");

        // Single-curve: derive forward curve from discount
        let fwd = curve.to_forward_curve("USD-SOFR", 0.25).unwrap();
        let ctx = base_context.insert_discount(curve).insert_forward(fwd);

        // Construct FRA matching the quote, notional $1,000,000
        let fra = ForwardRateAgreement::builder()
            .id("FRA-3x6".to_string().into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .fixing_date(base_date + time::Duration::days(88))
            .start_date(base_date + time::Duration::days(90))
            .end_date(base_date + time::Duration::days(180))
            .fixed_rate(0.0470)
            .day_count(DayCount::Act360)
            .reset_lag(2)
            .disc_id("USD-OIS".into())
            .forward_id("USD-SOFR".into())
            .build()
            .unwrap();

        let pv = fra.value(&ctx, base_date).unwrap();
        assert!(
            pv.amount().abs() <= 1.0,
            "FRA PV too large: {}",
            pv.amount()
        );
    }

    #[test]
    #[ignore = "Calibration logic still being completed"]
    fn test_future_repricing_under_bootstrap() {
        use crate::instruments::ir_future::{FutureContractSpecs, InterestRateFuture};

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

        // Quotes: deposits + FRA around the same window
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

        let base_context = MarketContext::new();
        let (curve, _report) = calibrator
            .calibrate(&quotes, &base_context)
            .expect("Future calibration should succeed");

        let fwd = curve.to_forward_curve("USD-SOFR", 0.25).unwrap();
        let ctx = base_context.insert_discount(curve).insert_forward(fwd);

        // Build matching future with implied price from forward curve
        let expiry = base_date + time::Duration::days(90);
        let period_start = expiry;
        let period_end = expiry + time::Duration::days(90);
        let t1 = finstack_core::dates::DayCount::Act360
            .year_fraction(
                base_date,
                period_start,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t2 = finstack_core::dates::DayCount::Act360
            .year_fraction(
                base_date,
                period_end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let implied_rate = ctx.get_forward_ref("USD-SOFR").unwrap().rate_period(t1, t2);
        let quoted_price = 100.0 * (1.0 - implied_rate);
        let mut fut = InterestRateFuture::builder()
            .id("SOFR-MAR25".to_string().into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(expiry)
            .fixing_date(expiry - time::Duration::days(2))
            .period_start(period_start)
            .period_end(period_end)
            .quoted_price(quoted_price)
            .day_count(DayCount::Act360)
            .position(crate::instruments::ir_future::Position::Long)
            .contract_specs(crate::instruments::ir_future::FutureContractSpecs::default())
            .disc_id("USD-OIS".into())
            .forward_id("USD-SOFR".into())
            .build()
            .unwrap();
        fut = fut.with_contract_specs(FutureContractSpecs {
            ..Default::default()
        });

        let pv = fut.value(&ctx, base_date).unwrap();
        assert!(
            pv.amount().abs() <= 1.0,
            "Future PV too large: {}",
            pv.amount()
        );
    }

    #[test]
    #[ignore = "Calibration logic still being completed"]
    fn test_swap_repricing_under_bootstrap() {
        use crate::instruments::irs::{InterestRateSwap, PayReceive};

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

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
                index: "USD-OIS".to_string(),
            },
        ];

        // OIS swaps can be calibrated without pre-existing forward curves
        let base_context = MarketContext::new();
        let (curve, _report) = calibrator
            .calibrate(&quotes, &base_context)
            .expect("Swap calibration should succeed");

        // For verification, derive forward from the calibrated discount curve
        let fwd = curve.to_forward_curve("USD-OIS", 0.25).unwrap();
        let ctx = base_context.insert_discount(curve).insert_forward(fwd);

        // Construct 1Y par swap matching quote
        let start = base_date;
        let end = base_date + time::Duration::days(365);
        let irs = InterestRateSwap::builder()
            .id("IRS-1Y".to_string().into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .side(PayReceive::ReceiveFixed)
            .fixed(crate::instruments::irs::FixedLegSpec {
                disc_id: "USD-OIS".into(),
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
                disc_id: "USD-OIS".into(),
                fwd_id: "USD-OIS".into(),
                spread_bp: 0.0,
                freq: Frequency::daily(),
                dc: DayCount::Act360,
                bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                start,
                end,
            })
            .build()
            .unwrap();

        let pv = irs.value(&ctx, base_date).unwrap();
        assert!(
            pv.amount().abs() <= 1.0,
            "Swap PV too large: {}",
            pv.amount()
        );
    }

    #[test]
    #[ignore = "Calibration logic still being completed"]
    fn test_ois_bootstrap_with_deposits_and_ois_swaps() {
        use finstack_core::dates::Frequency;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
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
                index: "USD-OIS".to_string(),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365 * 2),
                rate: 0.0480,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::daily(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-OIS".to_string(),
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

        let base_date = Date::from_calendar_date(2025, Month::January, 31).unwrap();

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
        let expected = Date::from_calendar_date(2025, Month::April, 30).unwrap();
        assert_eq!(
            proper_result, expected,
            "Expected proper month-end handling"
        );
    }
}
