//! Simple market data calibration.
//!
//! Provides a straightforward approach to calibrate complete market environments
//! from instrument quotes without over-engineering.

use crate::calibration::bootstrap::{
    BaseCorrelationCalibrator, DiscountCurveCalibrator, HazardCurveCalibrator,
    InflationCurveCalibrator, VolSurfaceCalibrator,
};
use crate::calibration::primitives::{HashableFloat, InstrumentQuote};
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::hazard_curve::Seniority;
use finstack_core::prelude::*;
use finstack_core::F;
use std::collections::HashMap;

/// Simple market calibration builder.
///
/// Calibrates market data in a straightforward sequential order:
/// 1. Discount curves (required for everything else)
/// 2. Hazard curves and inflation curves (depend on discount)
/// 3. Volatility surfaces (depend on underlying curves)
/// 4. Base correlation curves (depend on hazard curves)
pub struct SimpleCalibration {
    base_date: Date,
    base_currency: Currency,
    config: CalibrationConfig,
    entity_seniority: HashMap<String, Seniority>,
}

impl SimpleCalibration {
    /// Create a new simple calibration.
    pub fn new(base_date: Date, base_currency: Currency) -> Self {
        Self {
            base_date,
            base_currency,
            config: CalibrationConfig::default(),
            entity_seniority: HashMap::new(),
        }
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Add entity seniority mapping.
    pub fn with_entity_seniority(mut self, entity: impl Into<String>, seniority: Seniority) -> Self {
        self.entity_seniority.insert(entity.into(), seniority);
        self
    }

    /// Calibrate complete market from quotes.
    ///
    /// Returns a MarketContext with all calibrated curves and a summary report.
    pub fn calibrate(&self, quotes: &[InstrumentQuote]) -> Result<(MarketContext, CalibrationReport)> {
        let mut context = MarketContext::new();
        let mut all_residuals = HashMap::new();
        let mut total_iterations = 0;

        // Step 1: Discount curves
        if let Ok((updated_context, report)) = self.calibrate_discount_curves(quotes, &context) {
            context = updated_context;
            self.merge_report(&mut all_residuals, &mut total_iterations, &report);
        }

        // Step 2: Hazard curves (need discount curves)
        if let Ok((updated_context, report)) = self.calibrate_hazard_curves(quotes, &context) {
            context = updated_context;
            self.merge_report(&mut all_residuals, &mut total_iterations, &report);
        }

        // Step 3: Inflation curves (need discount curves)
        if let Ok((updated_context, report)) = self.calibrate_inflation_curves(quotes, &context) {
            context = updated_context;
            self.merge_report(&mut all_residuals, &mut total_iterations, &report);
        }

        // Step 4: Volatility surfaces (need underlying curves)
        if let Ok((updated_context, report)) = self.calibrate_vol_surfaces(quotes, &context) {
            context = updated_context;
            self.merge_report(&mut all_residuals, &mut total_iterations, &report);
        }

        // Step 5: Base correlation curves (need hazard curves)
        if let Ok((updated_context, report)) = self.calibrate_base_correlation(quotes, &context) {
            context = updated_context;
            self.merge_report(&mut all_residuals, &mut total_iterations, &report);
        }

        let final_report = CalibrationReport::new()
            .success()
            .with_residuals(all_residuals)
            .with_iterations(total_iterations)
            .with_convergence_reason("Market calibration completed");

        Ok((context, final_report))
    }

    /// Calibrate discount curves.
    fn calibrate_discount_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<(MarketContext, CalibrationReport)> {
        // Filter rates quotes
        let rates_quotes: Vec<_> = quotes
            .iter()
            .filter(|q| matches!(
                q,
                InstrumentQuote::Deposit { .. }
                    | InstrumentQuote::FRA { .. }
                    | InstrumentQuote::Future { .. }
                    | InstrumentQuote::Swap { .. }
            ))
            .cloned()
            .collect();

        if rates_quotes.is_empty() {
            return Ok((context.clone(), CalibrationReport::new().success()));
        }

        let calibrator = DiscountCurveCalibrator::new("USD-OIS", self.base_date, self.base_currency)
            .with_config(self.config.clone());

        let (curve, report) = calibrator.calibrate(&rates_quotes, context)?;
        Ok((context.clone().insert_discount(curve), report))
    }

    /// Calibrate hazard curves.
    fn calibrate_hazard_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let mut updated_context = context.clone();
        let mut combined_report = CalibrationReport::new().success();

        // Group CDS quotes by entity
        let mut quotes_by_entity: HashMap<String, Vec<InstrumentQuote>> = HashMap::new();
        for quote in quotes {
            match quote {
                InstrumentQuote::CDS { entity, .. } | InstrumentQuote::CDSUpfront { entity, .. } => {
                    quotes_by_entity.entry(entity.clone()).or_default().push(quote.clone());
                }
                _ => {}
            }
        }

        for (entity, entity_quotes) in quotes_by_entity {
            if entity_quotes.len() < 2 {
                continue;
            }

            // Extract recovery rate and currency from first quote
            let (recovery_rate, currency) = match &entity_quotes[0] {
                InstrumentQuote::CDS { recovery_rate, currency, .. } => (*recovery_rate, *currency),
                InstrumentQuote::CDSUpfront { recovery_rate, currency, .. } => (*recovery_rate, *currency),
                _ => continue,
            };

            let seniority = self.entity_seniority.get(&entity).copied().unwrap_or(Seniority::Senior);

            let calibrator = HazardCurveCalibrator::new(
                &entity,
                seniority,
                recovery_rate,
                self.base_date,
                currency,
                HazardCurveCalibrator::default_discount_curve_id(currency),
            );

            if let Ok((curve, report)) = calibrator.calibrate(&entity_quotes, &updated_context) {
                updated_context = updated_context.insert_hazard(curve);
                self.merge_report(
                    &mut combined_report.residuals,
                    &mut combined_report.iterations,
                    &report,
                );
            }
        }

        Ok((updated_context, combined_report))
    }

    /// Calibrate inflation curves.
    fn calibrate_inflation_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let mut updated_context = context.clone();
        let mut combined_report = CalibrationReport::new().success();

        // Group inflation quotes by index
        let mut quotes_by_index: HashMap<String, Vec<InstrumentQuote>> = HashMap::new();
        for quote in quotes {
            if let InstrumentQuote::InflationSwap { index, .. } = quote {
                quotes_by_index.entry(index.clone()).or_default().push(quote.clone());
            }
        }

        for (index, index_quotes) in quotes_by_index {
            if index_quotes.len() < 2 {
                continue;
            }

            // Try to get base CPI from context, or use default
            let base_cpi = self.get_base_cpi(&updated_context, &index).unwrap_or(100.0);

            let calibrator = InflationCurveCalibrator::new(
                &index,
                self.base_date,
                self.base_currency,
                base_cpi,
                HazardCurveCalibrator::default_discount_curve_id(self.base_currency),
            );

            if let Ok((curve, report)) = calibrator.calibrate(&index_quotes, &updated_context) {
                updated_context = updated_context.insert_inflation(curve);
                self.merge_report(
                    &mut combined_report.residuals,
                    &mut combined_report.iterations,
                    &report,
                );
            }
        }

        Ok((updated_context, combined_report))
    }

    /// Calibrate volatility surfaces.
    fn calibrate_vol_surfaces(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let mut updated_context = context.clone();
        let mut combined_report = CalibrationReport::new().success();

        // Group option quotes by underlying
        let mut quotes_by_underlying: HashMap<String, Vec<InstrumentQuote>> = HashMap::new();
        for quote in quotes {
            if let InstrumentQuote::OptionVol { underlying, .. } = quote {
                quotes_by_underlying.entry(underlying.clone()).or_default().push(quote.clone());
            }
        }

        for (underlying, underlying_quotes) in quotes_by_underlying {
            if underlying_quotes.len() < 6 {
                continue;
            }

            // Extract expiry and strike grids
            let (expiry_grid, strike_grid) = self.extract_vol_grid(&underlying_quotes);

            // Determine SABR beta based on asset class
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
            )
            .with_base_date(self.base_date);

            if let Ok((surface, report)) = calibrator.calibrate(&underlying_quotes, &updated_context) {
                updated_context = updated_context.insert_surface(surface);
                self.merge_report(
                    &mut combined_report.residuals,
                    &mut combined_report.iterations,
                    &report,
                );
            }
        }

        Ok((updated_context, combined_report))
    }

    /// Calibrate base correlation curves.
    fn calibrate_base_correlation(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let mut updated_context = context.clone();
        let mut combined_report = CalibrationReport::new().success();

        // Group tranche quotes by index and maturity
        let mut quotes_by_index: HashMap<String, HashMap<HashableFloat, Vec<InstrumentQuote>>> = HashMap::new();

        for quote in quotes {
            if let InstrumentQuote::CDSTranche { index, maturity, .. } = quote {
                let maturity_years = DayCount::Act365F
                    .year_fraction(self.base_date, *maturity, DayCountCtx::default())
                    .unwrap_or(0.0);

                quotes_by_index
                    .entry(index.clone())
                    .or_default()
                    .entry(maturity_years.into())
                    .or_default()
                    .push(quote.clone());
            }
        }

        for (index, maturities) in quotes_by_index {
            for (maturity_key, maturity_quotes) in maturities {
                if maturity_quotes.len() < 3 {
                    continue;
                }

                let calibrator = BaseCorrelationCalibrator::new(
                    &index,
                    42, // Default series number
                    maturity_key.into_inner(),
                    self.base_date,
                );

                let result = calibrator.calibrate(&maturity_quotes, &updated_context);
                if let Ok((curve, report)) = result {
                    updated_context = updated_context.insert_base_correlation(curve);
                    self.merge_report(
                        &mut combined_report.residuals,
                        &mut combined_report.iterations,
                        &report,
                    );
                }
            }
        }

        Ok((updated_context, combined_report))
    }

    /// Extract volatility grid from option quotes.
    fn extract_vol_grid(&self, quotes: &[InstrumentQuote]) -> (Vec<F>, Vec<F>) {
        let mut expiries = std::collections::HashSet::new();
        let mut strikes = std::collections::HashSet::new();

        for quote in quotes {
            if let InstrumentQuote::OptionVol { expiry, strike, .. } = quote {
                let years = DayCount::Act365F
                    .year_fraction(self.base_date, *expiry, DayCountCtx::default())
                    .unwrap_or(0.0);
                expiries.insert((years * 1000.0).round() as i32);
                strikes.insert((*strike * 100.0).round() as i32);
            }
        }

        let mut expiry_grid: Vec<F> = expiries.into_iter().map(|e| e as F / 1000.0).collect();
        expiry_grid.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut strike_grid: Vec<F> = strikes.into_iter().map(|s| s as F / 100.0).collect();
        strike_grid.sort_by(|a, b| a.partial_cmp(b).unwrap());

        (expiry_grid, strike_grid)
    }

    /// Get base CPI from context.
    fn get_base_cpi(&self, context: &MarketContext, index: &str) -> Option<F> {
        // Try inflation index
        if let Some(inflation_index) = context.inflation_index(index) {
            if let Ok(value) = inflation_index.value_on(self.base_date) {
                return Some(value);
            }
        }

        // Try inflation curve
        if let Ok(curve) = context.infl(index) {
            return Some(curve.cpi(0.0));
        }

        // Try market scalar
        let key = format!("{}-BASE_CPI", index);
        if let Ok(scalar) = context.price(&key) {
            return match scalar {
                finstack_core::market_data::primitives::MarketScalar::Unitless(v) => Some(*v),
                finstack_core::market_data::primitives::MarketScalar::Price(m) => Some(m.amount()),
            };
        }

        None
    }

    /// Merge report data.
    fn merge_report(
        &self,
        all_residuals: &mut HashMap<String, F>,
        total_iterations: &mut usize,
        report: &CalibrationReport,
    ) {
        for (key, value) in &report.residuals {
            all_residuals.insert(key.clone(), *value);
        }
        *total_iterations += report.iterations;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::{DayCount, Frequency};
    use time::Month;

    fn create_test_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        vec![
            InstrumentQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
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
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 2),
                spread_bp: 50.0,
                recovery_rate: 0.4,
                currency: Currency::USD,
            },
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 75.0,
                recovery_rate: 0.4,
                currency: Currency::USD,
            },
        ]
    }

    #[test]
    fn test_simple_calibration() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibration = SimpleCalibration::new(base_date, Currency::USD);

        let quotes = create_test_quotes();
        let (context, report) = calibration.calibrate(&quotes).unwrap();

        assert!(report.success);
        
        // Should have discount curve
        assert!(context.disc("USD-OIS").is_ok());
        
        // Should have hazard curve (now we have 2 CDS quotes for AAPL)
        assert!(context.hazard("AAPL-Senior").is_ok());
    }
}
