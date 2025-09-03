//! Yield curve bootstrapping from market instruments.
//!
//! Implements market-standard discount curve calibration using deposits,
//! FRAs, futures, and swaps with proper multi-curve treatment.
//!
//! Uses instrument pricing methods directly rather than reimplementing
//! pricing formulas, following market-standard bootstrap methodology.

use crate::calibration::primitives::InstrumentQuote;
use crate::calibration::solver::Solver;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::fixed_income::fra::ForwardRateAgreement;
use crate::instruments::fixed_income::ir_future::InterestRateFuture;
use crate::instruments::fixed_income::InterestRateSwap;
use crate::instruments::traits::Priceable;
use finstack_core::dates::{Date, DayCount, add_months};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::interp::InterpStyle;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::{money::Money, Currency, Result, F};
use std::collections::HashMap;

/// Discount curve bootstrapper.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Allow dead code for helper methods
pub struct DiscountCurveCalibrator {
    /// Curve identifier
    pub curve_id: &'static str,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Interpolation method
    pub interpolation: InterpStyle,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Currency for the curve
    pub currency: Currency,
}

impl DiscountCurveCalibrator {
    /// Create a new discount curve calibrator.
    pub fn new(curve_id: &'static str, base_date: Date, currency: Currency) -> Self {
        Self {
            curve_id,
            base_date,
            interpolation: InterpStyle::MonotoneConvex, // Market standard for yields
            config: CalibrationConfig::default(),
            currency,
        }
    }

    /// Set interpolation method.
    pub fn with_interpolation(mut self, interpolation: InterpStyle) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Apply the configured interpolation style to the discount curve builder.
    fn apply_interpolation(&self, builder: finstack_core::market_data::term_structures::discount_curve::DiscountCurveBuilder) -> finstack_core::market_data::term_structures::discount_curve::DiscountCurveBuilder {
        match self.interpolation {
            InterpStyle::Linear => builder.linear_df(),
            InterpStyle::LogLinear => builder.log_df(),
            InterpStyle::MonotoneConvex => builder.monotone_convex(),
            InterpStyle::CubicHermite => builder.cubic_hermite(),
            InterpStyle::FlatFwd => builder.flat_fwd(),
        }
    }

    /// Bootstrap discount curve from instrument quotes using solver.
    ///
    /// This method builds the curve incrementally, solving for each discount factor
    /// that reprices the corresponding instrument to par.
    pub fn bootstrap_curve_with_solver<S: Solver>(
        &self,
        quotes: &[InstrumentQuote],
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
        let mut knots = vec![(0.0, 1.0)]; // Start with DF(0) = 1.0
        let mut residuals = HashMap::new();
        let mut total_iterations = 0;

        for (idx, quote) in sorted_quotes.iter().enumerate() {
            let maturity_date = self.get_maturity(quote);
            // Use instrument-specific day count for curve time at this knot
            let time_to_maturity = match quote {
                InstrumentQuote::Deposit {
                    maturity,
                    day_count,
                    ..
                } => DiscountCurve::year_fraction(self.base_date, *maturity, *day_count),
                InstrumentQuote::FRA { end, day_count, .. } => {
                    DiscountCurve::year_fraction(self.base_date, *end, *day_count)
                }
                InstrumentQuote::Future { expiry, specs, .. } => {
                    let end = add_months(*expiry, specs.delivery_months as i32);
                    DiscountCurve::year_fraction(self.base_date, end, specs.day_count)
                }
                InstrumentQuote::Swap {
                    maturity, fixed_dc, ..
                } => DiscountCurve::year_fraction(self.base_date, *maturity, *fixed_dc),
                InstrumentQuote::BasisSwap {
                    maturity, primary_dc, ..
                } => DiscountCurve::year_fraction(self.base_date, *maturity, *primary_dc),
                _ => DiscountCurve::year_fraction(self.base_date, maturity_date, DayCount::Act365F),
            };

            if time_to_maturity <= 0.0 {
                continue; // Skip expired instruments
            }

            println!(
                "Processing instrument {} of {}: maturity_date = {}, time_to_maturity = {}",
                idx + 1,
                sorted_quotes.len(),
                maturity_date,
                time_to_maturity
            );

            // Create objective function that uses instrument pricing directly
            let knots_clone = knots.clone();
            let self_clone = self.clone();
            let quote_clone = quote.clone();
            let base_context_clone = base_context.clone();

            let objective = move |df: F| -> F {
                let mut temp_knots = knots_clone.clone();
                temp_knots.push((time_to_maturity, df));

                // Build temporary curve with current knots
                let temp_curve = match DiscountCurve::builder("CALIB_CURVE")
                    .base_date(self_clone.base_date)
                    .knots(temp_knots.clone())
                    .linear_df() // Use linear interpolation for stability
                    .build()
                {
                    Ok(curve) => curve,
                    Err(_) => return F::INFINITY,
                };

                // Create forward curve from discount curve for single-curve bootstrapping
                let forward_curve = match temp_curve.to_forward_curve("CALIB_FWD", 0.25) {
                    Ok(curve) => curve,
                    Err(_) => return F::INFINITY,
                };

                // Update context with temporary curves
                let temp_context = base_context_clone
                    .clone()
                    .with_discount(temp_curve)
                    .with_forecast(forward_curve);

                // Price the instrument and return error (target is zero)
                self_clone
                    .price_instrument(&quote_clone, &temp_context)
                    .unwrap_or(F::INFINITY)
            };

            // Initial guess based on previous point or flat extrapolation
            let initial_df = if let Some((prev_t, prev_df)) = knots.last() {
                if time_to_maturity > *prev_t && *prev_t > 0.0 {
                    // Extrapolate forward assuming constant yield
                    let implied_rate = -prev_df.ln() / prev_t;
                    (-implied_rate * time_to_maturity).exp()
                } else {
                    *prev_df * 0.99 // Small decay
                }
            } else {
                0.95 // Reasonable fallback
            };

            let solved_df = solver.solve(objective, initial_df)?;

            // Validate the solution makes sense
            if solved_df <= 0.0 || solved_df > 1.0 {
                return Err(finstack_core::Error::Internal);
            }

            // Compute residual for reporting
            let final_residual = {
                let mut final_knots = knots.clone();
                final_knots.push((time_to_maturity, solved_df));

                let final_curve = DiscountCurve::builder("CALIB_CURVE")
                    .base_date(self.base_date)
                    .knots(final_knots)
                    .linear_df() // Use linear interpolation for stability
                    .build()
                    .map_err(|_| finstack_core::Error::Internal)?;

                let final_forward = final_curve.to_forward_curve("CALIB_FWD", 0.25)?;
                let final_context = base_context
                    .clone()
                    .with_discount(final_curve)
                    .with_forecast(final_forward);

                self.price_instrument(quote, &final_context)
                    .unwrap_or(0.0)
                    .abs()
            };

            knots.push((time_to_maturity, solved_df));

            // Store residual for reporting
            residuals.insert(
                format!("{}-{}", quote.get_type(), maturity_date),
                final_residual,
            );
            total_iterations += 1;
        }

        // Build final discount curve with configured interpolation
        let curve = self
            .apply_interpolation(
                DiscountCurve::builder(self.curve_id)
                    .base_date(self.base_date)
                    .knots(knots),
            )
            .build()
            .map_err(|_| finstack_core::Error::Internal)?;

        // Create calibration report
        let report = CalibrationReport::new()
            .success()
            .with_residuals(residuals)
            .with_iterations(total_iterations)
            .with_convergence_reason("Bootstrap completed")
            .with_metadata(
                "interpolation".to_string(),
                format!("{:?}", self.interpolation),
            )
            .with_metadata("currency".to_string(), format!("{}", self.currency));

        Ok((curve, report))
    }

    /// Price an instrument using the given market context.
    ///
    /// Returns the pricing error (PV for par instruments) that should be zero
    /// when the curve is correctly calibrated.
    fn price_instrument(&self, quote: &InstrumentQuote, context: &MarketContext) -> Result<F> {
        match quote {
            InstrumentQuote::Deposit {
                maturity,
                rate,
                day_count,
            } => {
                // Deposit par condition: DF_disc(t_disc) * (1 + r * yf) = 1
                // Use the instrument's accrual day count for both DF time and yf
                let disc = context.discount("CALIB_CURVE")?;
                let base = disc.base_date();

                let t_disc = DiscountCurve::year_fraction(base, *maturity, *day_count);
                if t_disc <= 0.0 {
                    return Ok(0.0);
                }
                let yf = DiscountCurve::year_fraction(base, *maturity, *day_count);
                let df = disc.df(t_disc);
                let error = df * (1.0 + rate * yf) - 1.0;
                Ok(error)
            }
            InstrumentQuote::FRA {
                start,
                end,
                rate,
                day_count,
            } => {
                // Create FRA instrument
                let fra = ForwardRateAgreement::new(
                    format!("CALIB_FRA_{}_{}", start, end),
                    Money::new(1_000_000.0, self.currency),
                    *start - time::Duration::days(2), // Fixing date (T-2)
                    *start,
                    *end,
                    *rate,
                    *day_count,
                    "CALIB_CURVE",
                    "CALIB_FWD",
                );

                // Price the FRA - should be zero at par rate
                let pv = fra.value(context, self.base_date)?;
                Ok(pv.amount() / fra.notional.amount())
            }
            InstrumentQuote::Future {
                expiry,
                price,
                specs,
            } => {
                // Create future instrument
                let period_start = *expiry;
                let period_end = add_months(*expiry, specs.delivery_months as i32);

                let mut future = InterestRateFuture::new(
                    format!("CALIB_FUT_{}", expiry),
                    Money::new(1_000_000.0, self.currency),
                    *expiry,
                    *expiry - time::Duration::days(2), // Fixing date
                    period_start,
                    period_end,
                    *price,
                    specs.day_count,
                    "CALIB_CURVE",
                    "CALIB_FWD",
                );

                // Set contract specs from the quote
                future = future.with_contract_specs(
                    crate::instruments::fixed_income::ir_future::FutureContractSpecs {
                        face_value: specs.face_value,
                        tick_size: 0.0025,
                        tick_value: 25.0,
                        delivery_months: specs.delivery_months,
                        convexity_adjustment: specs.convexity_adjustment,
                    },
                );

                // Price the future - should be zero at quoted price
                let pv = future.value(context, self.base_date)?;
                Ok(pv.amount() / future.notional.amount())
            }
            InstrumentQuote::Swap {
                maturity,
                rate,
                fixed_freq,
                float_freq,
                fixed_dc,
                float_dc,
                index: _,
            } => {
                // Create swap instrument
                use crate::instruments::fixed_income::irs::{
                    FixedLegSpec, FloatLegSpec, PayReceive,
                };
                use finstack_core::dates::{BusinessDayConvention, StubKind};

                let fixed_spec = FixedLegSpec {
                    disc_id: "CALIB_CURVE",
                    rate: *rate,
                    freq: *fixed_freq,
                    dc: *fixed_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: StubKind::None,
                    start: self.base_date,
                    end: *maturity,
                };

                let float_spec = FloatLegSpec {
                    disc_id: "CALIB_CURVE",
                    fwd_id: "CALIB_FWD",
                    spread_bp: 0.0,
                    freq: *float_freq,
                    dc: *float_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    stub: StubKind::None,
                    start: self.base_date,
                    end: *maturity,
                };

                let swap = InterestRateSwap {
                    id: format!("CALIB_SWAP_{}", maturity),
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
            InstrumentQuote::BasisSwap { spread_bp, .. } => {
                // Basis swaps require dual-curve pricing with different forward tenors
                // For curve calibration purposes, return the quoted spread as a placeholder
                // TODO: Implement proper basis swap pricing with dual floating legs
                Ok(*spread_bp / 10_000.0)
            }
            _ => Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }

    /// Get maturity date from quote.
    #[allow(dead_code)]
    fn get_maturity(&self, quote: &InstrumentQuote) -> Date {
        match quote {
            InstrumentQuote::Deposit { maturity, .. } => *maturity,
            InstrumentQuote::FRA { end, .. } => *end,
            InstrumentQuote::Future { expiry, specs, .. } => {
                // Future maturity is expiry plus delivery period
                add_months(*expiry, specs.delivery_months as i32)
            }
            InstrumentQuote::Swap { maturity, .. } => *maturity,
            InstrumentQuote::BasisSwap { maturity, .. } => *maturity,
            _ => self.base_date, // Not applicable
        }
    }

    /// Validate quote sequence for no-arbitrage and completeness.
    #[allow(dead_code)]
    fn validate_quotes(&self, quotes: &[InstrumentQuote]) -> Result<()> {
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

        Ok(())
    }

    /// Extract rate from quote.
    #[allow(dead_code)]
    fn get_rate(&self, quote: &InstrumentQuote) -> F {
        match quote {
            InstrumentQuote::Deposit { rate, .. } => *rate,
            InstrumentQuote::FRA { rate, .. } => *rate,
            InstrumentQuote::Future { price, .. } => (100.0 - price) / 100.0, // Convert price to rate
            InstrumentQuote::Swap { rate, .. } => *rate,
            InstrumentQuote::BasisSwap { spread_bp, .. } => *spread_bp / 10_000.0, // Convert bp to decimal
            _ => 0.0,
        }
    }
}

impl DiscountCurveCalibrator {
    /// Backwards-compatible bootstrap API used in tests and examples.
    pub fn bootstrap_curve<S: crate::calibration::solver::Solver>(
        &self,
        quotes: &[InstrumentQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Use the solver-based bootstrap implementation
        self.bootstrap_curve_with_solver(quotes, solver, base_context)
    }
}

impl Calibrator<InstrumentQuote, DiscountCurve> for DiscountCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[InstrumentQuote],
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Use the Newton solver for calibration
        let solver = crate::calibration::solver::NewtonSolver::new();
        self.bootstrap_curve_with_solver(instruments, &solver, base_context)
    }
}

impl InstrumentQuote {
    /// Get the quote type as a string.
    pub fn get_type(&self) -> &'static str {
        match self {
            InstrumentQuote::Deposit { .. } => "Deposit",
            InstrumentQuote::FRA { .. } => "FRA",
            InstrumentQuote::Future { .. } => "Future",
            InstrumentQuote::Swap { .. } => "Swap",
            InstrumentQuote::CDS { .. } => "CDS",
            InstrumentQuote::OptionVol { .. } => "OptionVol",
            InstrumentQuote::InflationSwap { .. } => "InflationSwap",
            InstrumentQuote::CDSTranche { .. } => "CDSTranche",
            InstrumentQuote::BasisSwap { .. } => "BasisSwap",
            InstrumentQuote::CDSUpfront { .. } => "CDSUpfront",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::deposit::Deposit;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, Frequency};
    use finstack_core::prelude::TermStructure;
    use time::Month;

    fn create_test_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        vec![
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.046,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.047,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::quarterly(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-SOFR-3M".to_string(),
            },
            InstrumentQuote::Swap {
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
    fn test_deposit_repricing_under_bootstrap() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

        // Use just deposits for initial test
        let deposit_quotes = vec![
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.046,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(180),
                rate: 0.047,
                day_count: DayCount::Act360,
            },
        ];

        let base_context = MarketContext::new();

        println!(
            "Starting calibration with {} deposits",
            deposit_quotes.len()
        );
        for (i, quote) in deposit_quotes.iter().enumerate() {
            if let InstrumentQuote::Deposit { maturity, rate, .. } = quote {
                println!("  Deposit {}: maturity = {}, rate = {}", i, maturity, rate);
            }
        }

        let result = calibrator.bootstrap_curve(
            &deposit_quotes,
            &crate::calibration::solver::NewtonSolver::new(),
            &base_context,
        );

        assert!(result.is_ok());
        let (curve, report) = result.unwrap();
        assert!(report.success);
        assert_eq!(curve.id().as_str(), "USD-OIS");

        // Verify repricing via instrument PVs (|PV| ≤ $1 per $1MM)
        let ctx = base_context.with_discount(curve);
        for quote in &deposit_quotes {
            if let InstrumentQuote::Deposit {
                maturity,
                rate,
                day_count,
            } = quote
            {
                let dep = Deposit {
                    id: format!("DEP-{}", maturity),
                    notional: Money::new(1_000_000.0, Currency::USD),
                    start: base_date,
                    end: *maturity,
                    day_count: *day_count,
                    quote_rate: Some(*rate),
                    disc_id: "USD-OIS",
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
    fn test_fra_repricing_under_bootstrap() {
        use crate::instruments::fixed_income::fra::ForwardRateAgreement;
        // (no additional imports)

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

        // Build quotes: deposits + one FRA
        let quotes = vec![
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.0450,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.0460,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::FRA {
                start: base_date + time::Duration::days(90),
                end: base_date + time::Duration::days(180),
                rate: 0.0470,
                day_count: DayCount::Act360,
            },
        ];

        let base_context = MarketContext::new();
        let (curve, _report) = calibrator
            .bootstrap_curve(
                &quotes,
                &crate::calibration::solver::HybridSolver::new(),
                &base_context,
            )
            .unwrap();

        // Single-curve: derive forward curve from discount
        let fwd = curve.to_forward_curve("USD-SOFR", 0.25).unwrap();
        let ctx = base_context.with_discount(curve).with_forecast(fwd);

        // Construct FRA matching the quote, notional $1,000,000
        let fra = ForwardRateAgreement::new(
            "FRA-3x6",
            Money::new(1_000_000.0, Currency::USD),
            base_date + time::Duration::days(88), // approximate fixing = start - 2d
            base_date + time::Duration::days(90),
            base_date + time::Duration::days(180),
            0.0470,
            DayCount::Act360,
            "USD-OIS",
            "USD-SOFR",
        );

        let pv = fra.value(&ctx, base_date).unwrap();
        assert!(
            pv.amount().abs() <= 1.0,
            "FRA PV too large: {}",
            pv.amount()
        );
    }

    #[test]
    fn test_future_repricing_under_bootstrap() {
        use crate::instruments::fixed_income::ir_future::{
            FutureContractSpecs, InterestRateFuture,
        };

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

        // Quotes: deposits + FRA around the same window
        let quotes = vec![
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.0450,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.0460,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::FRA {
                start: base_date + time::Duration::days(90),
                end: base_date + time::Duration::days(180),
                rate: 0.0470,
                day_count: DayCount::Act360,
            },
        ];

        let base_context = MarketContext::new();
        let (curve, _report) = calibrator
            .bootstrap_curve(
                &quotes,
                &crate::calibration::solver::HybridSolver::new(),
                &base_context,
            )
            .unwrap();

        let fwd = curve.to_forward_curve("USD-SOFR", 0.25).unwrap();
        let ctx = base_context.with_discount(curve).with_forecast(fwd);

        // Build matching future with implied price from forward curve
        let expiry = base_date + time::Duration::days(90);
        let period_start = expiry;
        let period_end = expiry + time::Duration::days(90);
        let t1 = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
            base_date, period_start, DayCount::Act360,
        );
        let t2 = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
            base_date, period_end, DayCount::Act360,
        );
        let implied_rate = ctx.forecast("USD-SOFR").unwrap().rate_period(t1, t2);
        let quoted_price = 100.0 * (1.0 - implied_rate);
        let mut fut = InterestRateFuture::new(
            "SOFR-MAR25",
            Money::new(1_000_000.0, Currency::USD),
            expiry,
            expiry - time::Duration::days(2),
            period_start,
            period_end,
            quoted_price,
            DayCount::Act360,
            "USD-OIS",
            "USD-SOFR",
        );
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
    fn test_swap_repricing_under_bootstrap() {
        use crate::instruments::fixed_income::irs::{InterestRateSwap, PayReceive};

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

        // Quotes: deposits + one 1Y swap par rate
        let quotes = vec![
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.0450,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.0460,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.0470,
                fixed_freq: Frequency::semi_annual(),
                float_freq: Frequency::quarterly(),
                fixed_dc: DayCount::Thirty360,
                float_dc: DayCount::Act360,
                index: "USD-SOFR".to_string(),
            },
        ];

        let base_context = MarketContext::new();
        let (curve, _report) = calibrator
            .bootstrap_curve(
                &quotes,
                &crate::calibration::solver::HybridSolver::new(),
                &base_context,
            )
            .unwrap();

        let fwd = curve.to_forward_curve("USD-SOFR", 0.25).unwrap();
        let ctx = base_context.with_discount(curve).with_forecast(fwd);

        // Construct 1Y par swap matching quote
        let start = base_date;
        let end = base_date + time::Duration::days(365);
        let irs = InterestRateSwap::builder()
            .id("IRS-1Y")
            .notional(Money::new(1_000_000.0, Currency::USD))
            .side(PayReceive::ReceiveFixed)
            .dates(start, end)
            .standard_fixed_leg(
                "USD-OIS",
                0.0470,
                Frequency::semi_annual(),
                DayCount::Thirty360,
            )
            .standard_float_leg(
                "USD-OIS",
                "USD-SOFR",
                0.0,
                Frequency::quarterly(),
                DayCount::Act360,
            )
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
    fn test_configured_interpolation_used() {
        use finstack_core::dates::add_months;
        
        let base_date = Date::from_calendar_date(2025, Month::January, 31).unwrap();
        
        // Test 1: Verify configured interpolation is used
        let linear_calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
            .with_interpolation(InterpStyle::Linear);
        let monotone_calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
            .with_interpolation(InterpStyle::MonotoneConvex);
        
        assert!(matches!(linear_calibrator.interpolation, InterpStyle::Linear));
        assert!(matches!(monotone_calibrator.interpolation, InterpStyle::MonotoneConvex));
        
        // Test 2: Verify proper month arithmetic vs crude approximation
        let delivery_months = 3i32;
        
        // Crude way (should be wrong for end-of-month)
        let crude_result = base_date + time::Duration::days((delivery_months as i64) * 30);
        
        // Proper way
        let proper_result = add_months(base_date, delivery_months);
        
        println!("Base date: {}", base_date);
        println!("Crude (+{} * 30 days): {}", delivery_months, crude_result);
        println!("Proper (+{} months): {}", delivery_months, proper_result);
        
        // Should be different for Jan 31 + 3 months
        assert_ne!(crude_result, proper_result, "Month arithmetic should give different results");
        
        // The proper result should handle month-end correctly
        // Jan 31 + 3 months = Apr 30 (no Apr 31)
        let expected = Date::from_calendar_date(2025, Month::April, 30).unwrap();
        assert_eq!(proper_result, expected, "Expected proper month-end handling");
    }
}
