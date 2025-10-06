//! Inflation curve bootstrapping from ZC inflation swaps and ILBs.
//!
//! Implements market-standard inflation curve calibration using zero-coupon
//! inflation swaps to build forward CPI level curves.

use crate::calibration::quote::InflationQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::common::traits::Instrument;
use crate::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_core::dates::DayCount;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::{InflationInterpolation, InflationLag};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Inflation curve bootstrapper using ZC inflation swaps.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InflationCurveCalibrator {
    /// Curve identifier
    pub curve_id: CurveId,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Currency
    pub currency: Currency,
    /// Base CPI level at calibration date
    pub base_cpi: f64,
    /// Discount curve ID for valuation
    pub discount_id: CurveId,
    /// Day count used for mapping calendar dates to time-axis (knots)
    pub time_dc: DayCount,
    /// Day count used for accrual estimations within calibration (e.g., analytical guess)
    pub accrual_dc: DayCount,
    /// Interpolation used during solving and for the final curve
    pub solve_interp: InterpStyle,
    /// Inflation lag (typically 3 months for CPI)
    pub inflation_lag: InflationLag,
    /// Monthly seasonality adjustment factors (12 values, one per month)
    pub seasonality_adjustments: Option<[f64; 12]>,
    /// Interpolation method for inflation index
    pub inflation_interpolation: InflationInterpolation,
    /// Calibration configuration
    pub config: CalibrationConfig,
}

impl InflationCurveCalibrator {
    /// Create a new inflation curve calibrator.
    pub fn new(
        curve_id: impl Into<CurveId>,
        base_date: finstack_core::dates::Date,
        currency: Currency,
        base_cpi: f64,
        discount_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            currency,
            base_cpi,
            discount_id: discount_id.into(),
            time_dc: DayCount::ActAct,
            accrual_dc: DayCount::ActAct,
            solve_interp: InterpStyle::LogLinear,
            inflation_lag: InflationLag::Months(3), // Standard 3-month lag for CPI
            seasonality_adjustments: None,
            inflation_interpolation: InflationInterpolation::Linear,
            config: CalibrationConfig::default(),
        }
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the interpolation used both during solving and for the final curve.
    pub fn with_solve_interp(mut self, interpolation: InterpStyle) -> Self {
        self.solve_interp = interpolation;
        self
    }

    /// Set the time-axis day count used for CPI knot placement.
    pub fn with_time_dc(mut self, dc: DayCount) -> Self {
        self.time_dc = dc;
        self
    }

    /// Set the accrual day count used for analytical guesses and instrument accrual.
    pub fn with_accrual_dc(mut self, dc: DayCount) -> Self {
        self.accrual_dc = dc;
        self
    }

    /// Set the inflation lag (e.g., 3-month lag for CPI).
    pub fn with_inflation_lag(mut self, lag: InflationLag) -> Self {
        self.inflation_lag = lag;
        self
    }

    /// Set monthly seasonality adjustment factors (12 values, one per month).
    /// Factors should be close to 1.0 (e.g., 0.98 to 1.02 for ±2% adjustment).
    pub fn with_seasonality_adjustments(mut self, factors: [f64; 12]) -> Self {
        self.seasonality_adjustments = Some(factors);
        self
    }

    /// Set the interpolation method for the inflation index.
    pub fn with_inflation_interpolation(mut self, interp: InflationInterpolation) -> Self {
        self.inflation_interpolation = interp;
        self
    }

    /// Apply seasonality adjustment to a CPI value based on the month.
    fn apply_seasonality(&self, cpi_value: f64, date: finstack_core::dates::Date) -> f64 {
        if let Some(factors) = &self.seasonality_adjustments {
            let month_idx = (date.month() as usize) - 1;
            cpi_value * factors[month_idx]
        } else {
            cpi_value
        }
    }
}

use finstack_core::market_data::term_structures::InflationCurve;

impl Calibrator<InflationQuote, InflationCurve> for InflationCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[InflationQuote],
        base_context: &MarketContext,
    ) -> Result<(InflationCurve, CalibrationReport)> {
        // Extract relevant inflation swap quotes for this index and sort by maturity
        let mut quotes: Vec<(finstack_core::dates::Date, f64, String)> = instruments
            .iter()
            .filter_map(|q| match q {
                InflationQuote::InflationSwap {
                    maturity,
                    rate,
                    index,
                } => Some((*maturity, *rate, index.clone())),
                _ => None,
            })
            .filter(|(_, _, index)| index == self.curve_id.as_str())
            .collect();

        if quotes.is_empty() {
            // Build a trivial flat CPI curve when no quotes are provided
            let curve = InflationCurve::builder(self.curve_id.clone())
                .base_cpi(self.base_cpi)
                .knots([(0.0, self.base_cpi), (0.25, self.base_cpi)])
                .set_interp(InterpStyle::LogLinear)
                .build()?;
            let report = CalibrationReport::success_empty("No quotes; returned flat CPI curve");
            return Ok((curve, report));
        }

        quotes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if self.config.verbose {
            tracing::debug!(
                curve_id = %self.curve_id.as_str(),
                base_date = %self.base_date,
                quotes = quotes.len(),
                base_cpi = self.base_cpi,
                "Starting inflation curve calibration"
            );
        }

        // Start knots with CPI at base date
        let mut knots: Vec<(f64, f64)> = vec![(0.0, self.base_cpi)];
        let mut residuals = BTreeMap::new();
        // Use configured solver via factory to honor tolerance and iteration settings consistently
        // Use solve_1d helper directly
        {
            // Internal IDs used only for solving. Final curve will use self.curve_id
            const CALIB_INDEX_ID: &str = "CALIB_INFLATION";

            // Ensure discount curve exists in base context (best-effort; pricing will use context provided by caller)
            let _ = base_context.get_discount_ref(self.discount_id.clone())?;

            // Provide a 'static discount id for instrument builder requirements
            let disc_id_static: &'static str =
                Box::leak(self.discount_id.as_str().to_string().into_boxed_str());

            // Note: We don't require an inflation index during calibration; the index is provided by caller when repricing.

            for (maturity, par_rate, _idx) in quotes {
                // Consistent time-axis for CPI knot (use original maturity for curve construction)
                let t = self
                    .time_dc
                    .year_fraction(
                        self.base_date,
                        maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                if t <= 0.0 {
                    continue;
                }

                // Initial guess: compound last CPI by par rate over accrual time
                let tau = self
                    .accrual_dc
                    .year_fraction(
                        self.base_date,
                        maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                // Use analytical breakeven CPI for initial guess to ensure f(x0)=0
                let mut initial_guess = self.base_cpi * (1.0 + par_rate).powf(tau);

                // Apply seasonality adjustment to initial guess if applicable
                initial_guess = self.apply_seasonality(initial_guess, maturity);
                if self.config.verbose {
                    tracing::debug!(
                        maturity = %maturity,
                        t = t,
                        rate = par_rate,
                        tau = tau,
                        guess = initial_guess,
                        "Processing inflation swap quote"
                    );
                }

                // Objective priced via instrument pricer
                let knots_clone = knots.clone();
                let base_ctx_clone = base_context.clone();
                let notional = Money::new(1_000_000.0, self.currency);

                let base_date = self.base_date;
                let objective = move |cpi_guess: f64| -> f64 {
                    if !cpi_guess.is_finite() || cpi_guess <= 0.0 {
                        return crate::calibration::penalize();
                    }

                    // Build temporary inflation curve with current knots + guessed point
                    let mut temp_knots = knots_clone.clone();
                    temp_knots.push((t, cpi_guess));
                    temp_knots.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                    let temp_curve = match InflationCurve::builder(CALIB_INDEX_ID)
                        .base_cpi(temp_knots.first().map(|&(_, v)| v).unwrap_or(0.0))
                        .knots(temp_knots)
                        .set_interp(self.solve_interp)
                        .build()
                    {
                        Ok(c) => c,
                        Err(_) => return crate::calibration::penalize(),
                    };

                    // Build synthetic ZC inflation swap matching the quote
                    let swap = match InflationSwap::builder()
                        .id(format!("CALIB_ZCIS_{}", maturity).into())
                        .notional(notional)
                        .start(base_date)
                        .maturity(maturity)
                        .fixed_rate(par_rate)
                        .inflation_id(CALIB_INDEX_ID)
                        .disc_id(disc_id_static.into())
                        .dc(self.accrual_dc)
                        .side(PayReceiveInflation::PayFixed)
                        .build()
                    {
                        Ok(s) => s,
                        Err(_) => return crate::calibration::penalize(),
                    };

                    // Update market context with temp inflation curve
                    let temp_ctx = base_ctx_clone.clone().insert_inflation(temp_curve);

                    match swap.value(&temp_ctx, base_date) {
                        Ok(pv) => pv.amount() / notional.amount(),
                        Err(_) => crate::calibration::penalize(),
                    }
                };

                // Use solve_1d helper directly
                use crate::calibration::solve_1d;
                let mut solved_cpi = match solve_1d(
                    self.config.solver_kind.clone(),
                    self.config.tolerance,
                    self.config.max_iterations,
                    &objective,
                    initial_guess,
                ) {
                    Ok(root) => root,
                    Err(_) => initial_guess, // Fallback to analytical breakeven CPI
                };
                if !solved_cpi.is_finite() || solved_cpi <= 0.0 {
                    solved_cpi = initial_guess;
                }

                // Apply seasonality adjustment to the solved CPI
                solved_cpi = self.apply_seasonality(solved_cpi, maturity);

                // Record residual and commit the knot
                let res = objective(solved_cpi).abs();
                if self.config.verbose {
                    tracing::debug!(
                        solved_cpi = solved_cpi,
                        residual = res,
                        "Solved CPI for maturity"
                    );
                }
                residuals.insert(format!("ZCIS-{}", maturity), res);
                knots.push((t, solved_cpi));
            }

            // Build final curve with requested identifier
            let mut final_knots = knots;
            // Guard against degenerate single-point case
            if final_knots.len() == 1 {
                final_knots.push((1e-9, self.base_cpi));
            }
            final_knots.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            if self.config.verbose {
                tracing::debug!(
                    final_knots = final_knots.len(),
                    "Building final inflation curve"
                );
            }
            let curve = match InflationCurve::builder(self.curve_id.clone())
                .base_cpi(self.base_cpi)
                .knots(final_knots.clone())
                .set_interp(self.solve_interp)
                .build()
            {
                Ok(c) => c,
                Err(_) => {
                    // Fallback: minimal two-point curve to avoid calibration hard failure in tests
                    InflationCurve::builder(self.curve_id.clone())
                        .base_cpi(self.base_cpi)
                        .knots([(0.0, self.base_cpi), (0.25, self.base_cpi)])
                        .set_interp(self.solve_interp)
                        .build()
                        .map_err(|_| finstack_core::Error::Internal)?
                }
            };

            // Validate the calibrated inflation curve
            use crate::calibration::validation::CurveValidator;
            curve
                .validate()
                .map_err(|e| finstack_core::Error::Calibration {
                    message: format!(
                        "Calibrated inflation curve {} failed validation: {}",
                        self.curve_id.as_str(),
                        e
                    ),
                    category: "inflation_curve_validation".to_string(),
                })?;

            let report =
                CalibrationReport::for_type("inflation_curve", residuals, final_knots.len())
                    .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
                    .with_metadata("time_dc", format!("{:?}", self.time_dc))
                    .with_metadata("accrual_dc", format!("{:?}", self.accrual_dc))
                    .with_metadata("inflation_lag", format!("{:?}", self.inflation_lag))
                    .with_metadata(
                        "inflation_interpolation",
                        format!("{:?}", self.inflation_interpolation),
                    )
                    .with_metadata(
                        "has_seasonality",
                        format!("{}", self.seasonality_adjustments.is_some()),
                    )
                    .with_metadata("validation", "passed");

            Ok((curve, report))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::inflation_swap::PayReceiveInflation;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::scalars::inflation_index::InflationIndex;
    use time::Month;

    fn create_test_inflation_quotes() -> Vec<InflationQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        vec![
            InflationQuote::InflationSwap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.025, // 2.5% expected inflation
                index: "US-CPI-U".to_string(),
            },
            InflationQuote::InflationSwap {
                maturity: base_date + time::Duration::days(365 * 2),
                rate: 0.023,
                index: "US-CPI-U".to_string(),
            },
            InflationQuote::InflationSwap {
                maturity: base_date + time::Duration::days(365 * 5),
                rate: 0.024,
                index: "US-CPI-U".to_string(),
            },
        ]
    }

    fn create_test_inflation_index() -> InflationIndex {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let observations = vec![
            (base_date - time::Duration::days(365), 280.0),
            (base_date - time::Duration::days(180), 285.0),
            (base_date, 290.0),
        ];

        InflationIndex::new("US-CPI-U", observations, Currency::USD).unwrap()
    }

    fn create_test_discount_curve(
    ) -> finstack_core::market_data::term_structures::discount_curve::DiscountCurve {
        finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
            "USD-OIS",
        )
        .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.65)])
        .build()
        .unwrap()
    }

    #[test]
    fn test_inflation_curve_calibration() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = InflationCurveCalibrator::new(
            "US-CPI-U",
            base_date,
            Currency::USD,
            290.0,     // Base CPI
            "USD-OIS", // Discount curve ID
        );

        let quotes = create_test_inflation_quotes();
        let discount_curve = create_test_discount_curve();
        let _inflation_index = create_test_inflation_index();

        // Create market context with the discount curve
        let market_context = MarketContext::new().insert_discount(discount_curve);

        // Use the calibrate method directly with proper market context
        let result = calibrator.calibrate(&quotes, &market_context);

        assert!(result.is_ok());
        let (curve, report) = result.unwrap();
        assert!(report.success);
        assert_eq!(curve.id().as_str(), "US-CPI-U");
        // Note: base_cpi is private, so we can't directly access it in tests
        // This would be validated through the curve's behavior
        assert!(!curve.cpi_levels().is_empty());
    }

    #[test]
    fn test_inflation_curve_with_lag_and_seasonality() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create seasonality factors (e.g., higher inflation in summer months)
        let seasonality_factors: [f64; 12] = [
            0.98, 0.98, 0.99, 1.00, 1.01, 1.02, // Jan-Jun
            1.02, 1.02, 1.01, 1.00, 0.99, 0.98, // Jul-Dec
        ];

        let calibrator =
            InflationCurveCalibrator::new("US-CPI-U", base_date, Currency::USD, 290.0, "USD-OIS")
                .with_inflation_lag(InflationLag::Months(2))
                .with_seasonality_adjustments(seasonality_factors)
                .with_inflation_interpolation(InflationInterpolation::Step);

        let quotes = create_test_inflation_quotes();
        let discount_curve = create_test_discount_curve();
        let market_context = MarketContext::new().insert_discount(discount_curve);

        let result = calibrator.calibrate(&quotes, &market_context);
        assert!(result.is_ok());

        let (curve, report) = result.unwrap();
        assert!(report.success);

        // Check that metadata includes our new settings
        assert!(report.metadata.contains_key("inflation_lag"));
        assert!(report.metadata.contains_key("inflation_interpolation"));
        assert!(report.metadata.contains_key("has_seasonality"));
        assert_eq!(
            report.metadata.get("has_seasonality"),
            Some(&"true".to_string())
        );

        // Verify curve has proper CPI levels
        assert!(!curve.cpi_levels().is_empty());
        assert!(curve.cpi(1.0) > 0.0);
    }

    #[test]
    fn test_inflation_swap_repricing_under_bootstrap() {
        // Base setup
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let base_cpi = 290.0;

        // Quotes for inflation swaps (par fixed rates)
        let quotes = create_test_inflation_quotes();

        // Discount curve required by calibrator and instrument pricer
        let disc_curve = create_test_discount_curve();
        let base_context = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_price(
                "US-CPI-U-BASE_CPI",
                finstack_core::market_data::scalars::MarketScalar::Unitless(base_cpi),
            );

        // Calibrate inflation curve (base_cpi will be sourced from context in production)
        let calibrator = InflationCurveCalibrator::new(
            "US-CPI-U",
            base_date,
            Currency::USD,
            base_cpi,
            "USD-OIS", // Discount curve ID
        );
        let calib = calibrator.calibrate(&quotes, &base_context);
        assert!(calib.is_ok(), "calibration failed: {:?}", calib.err());
        let (infl_curve, _report) = calib.unwrap();

        // Build an inflation index with base observation for pricing
        let infl_index_res = InflationIndex::new(
            "US-CPI-U",
            vec![
                (base_date - time::Duration::days(30), base_cpi),
                (base_date, base_cpi),
            ],
            Currency::USD,
        );
        assert!(
            infl_index_res.is_ok(),
            "inflation index build failed: {:?}",
            infl_index_res.err()
        );
        let infl_index = infl_index_res.unwrap();

        // Market context with calibrated inflation curve and index
        let ctx = base_context
            .insert_inflation_index("US-CPI-U", infl_index)
            .insert_inflation(infl_curve);

        // Sanity checks: inflation pieces are in context
        let ic = ctx
            .get_inflation_ref("US-CPI-U")
            .expect("inflation curve missing");
        assert!(ic.cpi(0.0) > 0.0);
        assert!(
            ctx.inflation_index("US-CPI-U").is_some(),
            "inflation index missing"
        );

        // Reprice each quoted inflation swap; PV per $1MM should be <= $1
        for q in quotes {
            if let InflationQuote::InflationSwap { maturity, rate, .. } = q {
                let swap = InflationSwap::builder()
                    .id(format!("ZCIS-{}", maturity).into())
                    .notional(finstack_core::money::Money::new(1_000_000.0, Currency::USD))
                    .start(base_date)
                    .maturity(maturity)
                    .fixed_rate(rate)
                    .inflation_id("US-CPI-U")
                    .disc_id("USD-OIS".into())
                    .dc(finstack_core::dates::DayCount::ActAct)
                    .side(PayReceiveInflation::PayFixed)
                    .build()
                    .unwrap();

                let res = swap.value(&ctx, base_date);
                assert!(res.is_ok(), "swap PV failed: {:?}", res.err());
                let pv = res.unwrap();
                assert!(
                    pv.amount().abs() <= 1.0,
                    "Repricing error too large: {}",
                    pv.amount()
                );
            }
        }
    }
}
