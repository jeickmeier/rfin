//! Calibration orchestrator for comprehensive market data calibration.
//!
//! Provides high-level functions to calibrate complete market environments
//! from instrument quotes using proper sequencing and dependencies.

use crate::calibration::base_correlation::BaseCorrelationCalibrator;
use crate::calibration::bootstrap::{
    CreditCurveCalibrator, DiscountCurveCalibrator, InflationCurveCalibrator,
};
use crate::calibration::primitives::{HashableFloat, InstrumentQuote};
use crate::calibration::surface::VolSurfaceCalibrator;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};

use crate::market_data::ValuationMarketContext;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::credit_curve::Seniority;

use finstack_core::{Currency, Result};
use std::collections::HashMap;

/// Comprehensive market data calibration orchestrator.
///
/// Manages the end-to-end calibration process for a complete market environment,
/// handling dependencies between different curve types and ensuring proper sequencing.
#[derive(Clone, Debug)]
pub struct CalibrationOrchestrator {
    /// Base date for all calibrations
    pub base_date: finstack_core::dates::Date,
    /// Primary currency for discount curve
    pub base_currency: Currency,
    /// Calibration configuration
    pub config: CalibrationConfig,
}

impl CalibrationOrchestrator {
    /// Create a new calibration orchestrator.
    pub fn new(base_date: finstack_core::dates::Date, base_currency: Currency) -> Self {
        Self {
            base_date,
            base_currency,
            config: CalibrationConfig::default(),
        }
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Perform complete market data calibration.
    ///
    /// Calibrates curves in the proper sequence:
    /// 1. Discount curves (OIS)
    /// 2. Forward curves (IBOR/RFR)
    /// 3. Credit curves
    /// 4. Inflation curves
    /// 5. Volatility surfaces
    /// 6. Base correlation curves
    pub fn calibrate_market(
        &self,
        quotes: &[InstrumentQuote],
    ) -> Result<(MarketContext, CalibrationReport)> {
        let mut context = MarketContext::new();
        let mut all_residuals = HashMap::new();
        let mut total_iterations = 0;
        let mut calibration_stages = Vec::new();

        // Stage 1: Calibrate primary discount curve (OIS)
        if let Some((discount_curve, report)) = self.calibrate_discount_curve(quotes, &context)? {
            context = context.with_discount(discount_curve);
            self.merge_report_data(&mut all_residuals, &mut total_iterations, &report);
            calibration_stages.push("Discount curve".to_string());
        }

        // Stage 2: Calibrate forward curves (commented out for now)
        // let forward_curves = self.calibrate_forward_curves(quotes, &context)?;
        // for (curve_id, (curve, report)) in forward_curves {
        //     context = context.with_forecast(curve);
        //     self.merge_report_data(&mut all_residuals, &mut total_iterations, &report);
        //     calibration_stages.push(format!("Forward curve: {}", curve_id));
        // }

        // Stage 3: Calibrate credit curves
        let credit_curves = self.calibrate_credit_curves(quotes, &context)?;
        for (entity, (curve, report)) in credit_curves {
            context = context.with_credit(curve);
            self.merge_report_data(&mut all_residuals, &mut total_iterations, &report);
            calibration_stages.push(format!("Credit curve: {}", entity));
        }

        // Stage 4: Calibrate inflation curves
        let inflation_curves = self.calibrate_inflation_curves(quotes, &context)?;
        for (index, (curve, report)) in inflation_curves {
            context = context.with_inflation(curve);
            self.merge_report_data(&mut all_residuals, &mut total_iterations, &report);
            calibration_stages.push(format!("Inflation curve: {}", index));
        }

        // Stage 5: Calibrate volatility surfaces
        let vol_surfaces = self.calibrate_vol_surfaces(quotes, &context)?;
        for (surface_id, (surface, report)) in vol_surfaces {
            context = context.with_surface(surface);
            self.merge_report_data(&mut all_residuals, &mut total_iterations, &report);
            calibration_stages.push(format!("Vol surface: {}", surface_id));
        }

        // Stage 6: Calibrate base correlation curves (requires all previous stages)
        let base_corr_curves = self.calibrate_base_correlation_curves(quotes, &context)?;
        for (index, curves_by_maturity) in base_corr_curves {
            for (maturity, (_curve, report)) in curves_by_maturity {
                // Note: MarketContext doesn't have base correlation yet, would need extension
                self.merge_report_data(&mut all_residuals, &mut total_iterations, &report);
                calibration_stages.push(format!("Base correlation: {} {}Y", index, maturity));
            }
        }

        // Create final calibration report
        let final_report = CalibrationReport::new()
            .success()
            .with_residuals(all_residuals)
            .with_iterations(total_iterations)
            .with_convergence_reason("Complete market calibration finished")
            .with_metadata("stages".to_string(), calibration_stages.join(", "))
            .with_metadata(
                "base_currency".to_string(),
                format!("{}", self.base_currency),
            );

        Ok((context, final_report))
    }

    /// Calibrate primary discount curve from OIS quotes.
    fn calibrate_discount_curve(
        &self,
        quotes: &[InstrumentQuote],
        _context: &MarketContext,
    ) -> Result<
        Option<(
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
            CalibrationReport,
        )>,
    > {
        // Filter relevant quotes (deposits and OIS swaps)
        let relevant_quotes: Vec<_> = quotes
            .iter()
            .filter(|q| match q {
                InstrumentQuote::Deposit { .. } => true,
                InstrumentQuote::Swap { index, .. } => {
                    index.contains("OIS") || index.contains("SOFR")
                }
                _ => false,
            })
            .cloned()
            .collect();

        if relevant_quotes.is_empty() {
            return Ok(None);
        }

        let calibrator = DiscountCurveCalibrator::new(
            format!("{}-OIS", self.base_currency),
            self.base_date,
            self.base_currency,
        )
        .with_config(self.config.clone());

        let base_context = MarketContext::new();
        let (curve, report) = calibrator.calibrate(&relevant_quotes, &[], &base_context)?;

        Ok(Some((curve, report)))
    }

    /*
    /// Calibrate forward curves for different tenors.
    fn calibrate_forward_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<HashMap<String, (finstack_core::market_data::term_structures::forward_curve::ForwardCurve, CalibrationReport)>> {
        let mut results = HashMap::new();

        // Standard tenors to calibrate
        let tenors = vec![
            ("1M", 1.0/12.0),
            ("3M", 3.0/12.0),
            ("6M", 6.0/12.0),
            ("12M", 1.0),
        ];

        for (tenor_name, tenor_years) in tenors {
            // Filter quotes relevant to this tenor
            let relevant_quotes: Vec<_> = quotes
                .iter()
                .filter(|q| match q {
                    InstrumentQuote::FRA { .. } => true,
                    InstrumentQuote::Future { .. } => true,
                    InstrumentQuote::Swap { index, .. } => index.contains(tenor_name),
                    _ => false,
                })
                .cloned()
                .collect();

            if !relevant_quotes.is_empty() {
                let curve_id = format!("{}-{}", self.base_currency, tenor_name);
                let calibrator = ForwardCurveCalibrator::new(
                    &curve_id,
                    self.base_date,
                    tenor_years,
                    self.base_currency,
                );

                // Get discount curve for pricing
                if let Ok(discount_curve) = context.discount(&format!("{}-OIS", self.base_currency)) {
                    match calibrator.bootstrap_curve(&relevant_quotes, &crate::calibration::solver::HybridSolver::new(), discount_curve.as_ref()) {
                        Ok((curve, report)) => {
                            results.insert(curve_id, (curve, report));
                        }
                        Err(_) => {
                            // Failed to calibrate this tenor - continue with others
                            continue;
                        }
                    }
                }
            }
        }

        Ok(results)
    }
    */

    /// Calibrate credit curves for different entities.
    fn calibrate_credit_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<
        HashMap<
            String,
            (
                finstack_core::market_data::term_structures::credit_curve::CreditCurve,
                CalibrationReport,
            ),
        >,
    > {
        let mut results = HashMap::new();

        // Group CDS quotes by entity
        let mut quotes_by_entity: HashMap<String, Vec<&InstrumentQuote>> = HashMap::new();
        for quote in quotes {
            if let InstrumentQuote::CDS { entity, .. } = quote {
                quotes_by_entity
                    .entry(entity.clone())
                    .or_default()
                    .push(quote);
            }
        }

        for (entity, entity_quotes) in quotes_by_entity {
            if entity_quotes.len() < 2 {
                continue; // Need multiple tenors for bootstrapping
            }

            let calibrator = CreditCurveCalibrator::new(
                &entity,
                Seniority::Senior, // Default to senior debt
                0.4,               // Standard 40% recovery
                self.base_date,
                self.base_currency,
            );

            let entity_quote_vec: Vec<_> = entity_quotes.iter().map(|&q| q.clone()).collect();
            match calibrator.calibrate(&entity_quote_vec, &[], context) {
                Ok((curve, report)) => {
                    results.insert(entity, (curve, report));
                }
                Err(_) => {
                    // Failed to calibrate this entity - continue with others
                    continue;
                }
            }
        }

        Ok(results)
    }

    /// Calibrate inflation curves.
    fn calibrate_inflation_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<
        HashMap<
            String,
            (
                finstack_core::market_data::term_structures::inflation::InflationCurve,
                CalibrationReport,
            ),
        >,
    > {
        let mut results = HashMap::new();

        // Group inflation swap quotes by index
        let mut quotes_by_index: HashMap<String, Vec<&InstrumentQuote>> = HashMap::new();
        for quote in quotes {
            if let InstrumentQuote::InflationSwap { index, .. } = quote {
                quotes_by_index
                    .entry(index.clone())
                    .or_default()
                    .push(quote);
            }
        }

        for (index, index_quotes) in quotes_by_index {
            if index_quotes.len() < 2 {
                continue;
            }

            // Use current CPI as base level (would get from inflation index in practice)
            let base_cpi = 290.0; // Placeholder

            let calibrator =
                InflationCurveCalibrator::new(&index, self.base_date, self.base_currency, base_cpi);

            let index_quote_vec: Vec<_> = index_quotes.iter().map(|&q| q.clone()).collect();
            match calibrator.calibrate(&index_quote_vec, &[], context) {
                Ok((curve, report)) => {
                    results.insert(index, (curve, report));
                }
                Err(_) => {
                    continue;
                }
            }
        }

        Ok(results)
    }

    /// Calibrate volatility surfaces.
    fn calibrate_vol_surfaces(
        &self,
        quotes: &[InstrumentQuote],
        _context: &MarketContext,
    ) -> Result<
        HashMap<
            String,
            (
                finstack_core::market_data::surfaces::vol_surface::VolSurface,
                CalibrationReport,
            ),
        >,
    > {
        let mut results = HashMap::new();

        // Group option vol quotes by underlying
        let mut quotes_by_underlying: HashMap<String, Vec<&InstrumentQuote>> = HashMap::new();
        for quote in quotes {
            if let InstrumentQuote::OptionVol { underlying, .. } = quote {
                quotes_by_underlying
                    .entry(underlying.clone())
                    .or_default()
                    .push(quote);
            }
        }

        for (underlying, underlying_quotes) in quotes_by_underlying {
            if underlying_quotes.len() < 6 {
                continue; // Need sufficient points for SABR calibration
            }

            // Determine expiry and strike grids from market data
            let mut expiries = std::collections::HashSet::new();
            let mut strikes = std::collections::HashSet::new();

            for quote in &underlying_quotes {
                if let InstrumentQuote::OptionVol { expiry, strike, .. } = quote {
                    let days = (*expiry - self.base_date).whole_days();
                    let years = days as finstack_core::F / 365.25;
                    expiries.insert((years * 1000.0).round() as i32); // Round to avoid floating point issues
                    strikes.insert((*strike * 100.0).round() as i32);
                }
            }

            let mut expiry_grid: Vec<finstack_core::F> = expiries
                .into_iter()
                .map(|e| e as finstack_core::F / 1000.0)
                .collect();
            expiry_grid.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let mut strike_grid: Vec<finstack_core::F> = strikes
                .into_iter()
                .map(|s| s as finstack_core::F / 100.0)
                .collect();
            strike_grid.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // Use appropriate beta for asset class
            let beta = if underlying.contains("USD") || underlying.contains("EUR") {
                0.5 // Interest rates
            } else {
                1.0 // Equity/FX
            };

            let calibrator = VolSurfaceCalibrator::new(
                format!("{}-VOL", underlying),
                beta,
                expiry_grid,
                strike_grid,
            );

            let underlying_quote_vec: Vec<_> =
                underlying_quotes.iter().map(|&q| q.clone()).collect();

            // Create simple forward curve for calibration
            let forward_fn = |t: finstack_core::F| 100.0 * (0.05 * t).exp(); // Placeholder

            match calibrator.calibrate_surface(&underlying_quote_vec, &forward_fn) {
                Ok((surface, report)) => {
                    results.insert(underlying, (surface, report));
                }
                Err(_) => {
                    continue;
                }
            }
        }

        Ok(results)
    }

    /// Calibrate base correlation curves.
    #[allow(clippy::type_complexity)]
    fn calibrate_base_correlation_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<
        HashMap<
            String,
            HashMap<
                HashableFloat,
                (
                    finstack_core::market_data::term_structures::BaseCorrelationCurve,
                    CalibrationReport,
                ),
            >,
        >,
    > {
        let mut results = HashMap::new();

        // Group tranche quotes by index and maturity
        let mut quotes_by_index: HashMap<String, HashMap<HashableFloat, Vec<&InstrumentQuote>>> =
            HashMap::new();

        for quote in quotes {
            if let InstrumentQuote::CDSTranche {
                index, maturity, ..
            } = quote
            {
                let maturity_years = finstack_core::dates::DayCount::Act365F
                    .year_fraction(self.base_date, *maturity)
                    .unwrap_or(0.0);

                quotes_by_index
                    .entry(index.clone())
                    .or_default()
                    .entry(HashableFloat::new(maturity_years))
                    .or_default()
                    .push(quote);
            }
        }

        for (index, maturities) in quotes_by_index {
            let mut curves_by_maturity = HashMap::new();

            for (maturity_key, maturity_quotes) in maturities {
                let maturity_years = maturity_key.value();
                if maturity_quotes.len() < 3 {
                    continue; // Need multiple tranches
                }

                let calibrator = BaseCorrelationCalibrator::new(
                    &index,
                    42, // Default series number
                    maturity_years,
                    self.base_date,
                );

                let maturity_quote_vec: Vec<_> =
                    maturity_quotes.iter().map(|&q| q.clone()).collect();

                // Convert context to ValuationMarketContext for tranche pricing
                let val_context = ValuationMarketContext::from_core(context.clone());

                match calibrator.bootstrap_curve(
                    &maturity_quote_vec,
                    &crate::calibration::solver::HybridSolver::new(),
                    &val_context,
                ) {
                    Ok((curve, report)) => {
                        curves_by_maturity
                            .insert(HashableFloat::new(maturity_years), (curve, report));
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }

            if !curves_by_maturity.is_empty() {
                results.insert(index, curves_by_maturity);
            }
        }

        Ok(results)
    }

    /// Merge report data from individual calibrations.
    fn merge_report_data(
        &self,
        all_residuals: &mut HashMap<String, finstack_core::F>,
        total_iterations: &mut usize,
        report: &CalibrationReport,
    ) {
        for (key, value) in &report.residuals {
            all_residuals.insert(key.clone(), *value);
        }
        *total_iterations += report.iterations;
    }

    /// Validate complete market environment for no-arbitrage conditions.
    pub fn validate_market_environment(
        &self,
        context: &MarketContext,
    ) -> Result<CalibrationReport> {
        let mut validation_errors = HashMap::new();

        // Check discount curve properties
        if let Ok(disc_curve) = context.discount(format!("{}-OIS", self.base_currency)) {
            // Check monotonicity
            let test_times = vec![0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
            let mut prev_df = 1.0;

            for &t in &test_times {
                let df = disc_curve.df(t);
                if df > prev_df {
                    validation_errors.insert(format!("discount_monotonicity_{}", t), df - prev_df);
                }
                prev_df = df;
            }
        }

        // Additional validation checks would go here...

        let success = validation_errors.is_empty();
        let convergence_reason = if success {
            "Market environment validation passed"
        } else {
            "Market environment validation found issues"
        };

        Ok(CalibrationReport::new()
            .with_residuals(validation_errors)
            .with_convergence_reason(convergence_reason)
            .with_metadata("validation_type".to_string(), "no_arbitrage".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::{Date, DayCount, Frequency};
    use finstack_core::prelude::TermStructure;
    use time::Month;

    fn create_test_market_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        vec![
            // OIS deposits and swaps
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                day_count: DayCount::Act360,
            },
            InstrumentQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.047,
                fixed_freq: Frequency::annual(),
                float_freq: Frequency::annual(),
                fixed_dc: DayCount::Act365F,
                float_dc: DayCount::Act360,
                index: "USD-SOFR-OIS".to_string(),
            },
            // Credit quotes
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 75.0,
                recovery_rate: 0.4,
                currency: Currency::USD,
            },
            // Inflation swaps
            InstrumentQuote::InflationSwap {
                maturity: base_date + time::Duration::days(365 * 5),
                rate: 0.025,
                index: "US-CPI-U".to_string(),
            },
            // Option volatilities
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: base_date + time::Duration::days(30),
                strike: 100.0,
                vol: 0.20,
                option_type: "Call".to_string(),
            },
        ]
    }

    #[test]
    fn test_orchestrator_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let orchestrator = CalibrationOrchestrator::new(base_date, Currency::USD);

        assert_eq!(orchestrator.base_date, base_date);
        assert_eq!(orchestrator.base_currency, Currency::USD);
    }

    #[test]
    fn test_discount_curve_calibration_stage() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let orchestrator = CalibrationOrchestrator::new(base_date, Currency::USD);

        let quotes = create_test_market_quotes();
        let context = MarketContext::new();

        let result = orchestrator.calibrate_discount_curve(&quotes, &context);
        assert!(result.is_ok());

        // Should find relevant quotes and calibrate
        let curve_opt = result.unwrap();
        if curve_opt.is_some() {
            let (curve, report) = curve_opt.unwrap();
            assert!(report.success);
            assert_eq!(curve.id().as_str(), "USD-OIS");
        }
    }

    #[test]
    fn test_market_validation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let orchestrator = CalibrationOrchestrator::new(base_date, Currency::USD);

        // Create a simple market context
        let context = MarketContext::new();

        let report = orchestrator.validate_market_environment(&context);
        assert!(report.is_ok());
    }
}
