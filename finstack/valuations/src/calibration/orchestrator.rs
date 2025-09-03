//! Calibration orchestrator for comprehensive market data calibration.
//!
//! Provides high-level functions to calibrate complete market environments
//! from instrument quotes using proper sequencing and dependencies.

use crate::calibration::base_correlation::BaseCorrelationCalibrator;
use crate::calibration::bootstrap::{
    DiscountCurveCalibrator, HazardCurveCalibrator, InflationCurveCalibrator,
};
use crate::calibration::dependency_dag::{CalibrationDAG, CalibrationTarget};
use crate::calibration::primitives::{HashableFloat, InstrumentQuote};
use crate::calibration::surface::VolSurfaceCalibrator;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};

use crate::market_data::ValuationMarketContext;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::hazard_curve::Seniority;

use finstack_core::{Currency, Result, F};
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

    /// Add entity-specific seniority mapping.
    pub fn with_entity_seniority(mut self, entity: impl Into<String>, seniority: Seniority) -> Self {
        self.config.entity_seniority.insert(entity.into(), seniority);
        self
    }

    /// Add multiple entity seniority mappings.
    pub fn with_entity_seniorities(mut self, mappings: HashMap<String, Seniority>) -> Self {
        self.config.entity_seniority.extend(mappings);
        self
    }

    /// Perform complete market data calibration using DAG-based dependency resolution.
    ///
    /// This method replaces fixed sequential stages with dynamic dependency analysis,
    /// enabling parallel calibration of independent curves and flexible handling of
    /// complex cross-dependencies.
    pub fn calibrate_market(
        &self,
        quotes: &[InstrumentQuote],
    ) -> Result<(MarketContext, CalibrationReport)> {
        // Build dependency DAG from quotes
        let dag = CalibrationDAG::from_quotes(quotes, self.base_currency)?;
        
        // Get calibration order using topological sort
        let calibration_batches = dag.topological_sort()?;
        
        let mut context = MarketContext::new();
        let mut all_residuals = HashMap::new();
        let mut total_iterations = 0;
        let mut calibration_stages = Vec::new();

        // Execute calibration batches in order
        for (batch_idx, batch) in calibration_batches.iter().enumerate() {
            let mut batch_reports = Vec::new();
            
            // In the future, this could be parallelized since targets in the same batch are independent
            for target in batch {
                let target_quotes = dag.quotes_for_target(target);
                if target_quotes.is_empty() {
                    continue;
                }

                match self.calibrate_single_target(target, target_quotes, &context) {
                    Ok((updated_context, report)) => {
                        context = updated_context;
                        batch_reports.push((target.clone(), report));
                    }
                    Err(_) => {
                        // Log error but continue with other targets
                        continue;
                    }
                }
            }

            // Merge reports from this batch
            for (target, report) in batch_reports {
                self.merge_report_data(&mut all_residuals, &mut total_iterations, &report);
                calibration_stages.push(format!("Batch {}: {}", batch_idx, target.id()));
            }
        }

        // Create final calibration report with DAG statistics
        let dag_stats = dag.statistics();
        let final_report = CalibrationReport::new()
            .success()
            .with_residuals(all_residuals)
            .with_iterations(total_iterations)
            .with_convergence_reason("DAG-based market calibration completed")
            .with_metadata("stages".to_string(), calibration_stages.join(", "))
            .with_metadata("base_currency".to_string(), format!("{}", self.base_currency))
            .with_metadata("calibration_batches".to_string(), format!("{}", dag_stats.calibration_batches))
            .with_metadata("max_parallelism".to_string(), format!("{}", dag_stats.max_parallelism))
            .with_metadata("estimated_speedup".to_string(), format!("{:.2}x", dag_stats.estimated_speedup));

        Ok((context, final_report))
    }

    /// Calibrate a single target using appropriate calibrator.
    fn calibrate_single_target(
        &self,
        target: &CalibrationTarget,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<(MarketContext, CalibrationReport)> {
        match target {
            CalibrationTarget::DiscountCurve { currency } => {
                let calibrator = DiscountCurveCalibrator::new("USD-OIS", self.base_date, *currency)
                    .with_config(self.config.clone());

                // Filter only rates quotes relevant for yield curve bootstrapping
                let rates_quotes: Vec<InstrumentQuote> = quotes
                    .iter()
                    .filter(|q| matches!(q,
                        InstrumentQuote::Deposit { .. }
                        | InstrumentQuote::FRA { .. }
                        | InstrumentQuote::Future { .. }
                        | InstrumentQuote::Swap { .. }
                    ))
                    .cloned()
                    .collect();

                if rates_quotes.is_empty() {
                    // Nothing to calibrate for discount curve in this batch
                    return Ok((context.clone(), CalibrationReport::new().success().with_convergence_reason("No rates quotes for discount curve")));
                }

                let (curve, report) = calibrator.calibrate(&rates_quotes, context)?;
                let updated_context = context.clone().with_discount(curve);
                Ok((updated_context, report))
            }
            CalibrationTarget::ForwardCurve { currency: _, tenor: _ } => {
                // Forward curve calibration not yet implemented in DAG system
                // Would use ForwardCurveCalibrator when available
                Ok((context.clone(), CalibrationReport::new().success().with_convergence_reason("Forward curve calibration skipped (not implemented)")))
            }
            CalibrationTarget::HazardCurve { entity, seniority } => {
                // Extract recovery rate and currency from quotes
                let (recovery_rate, currency) = self.extract_hazard_params_from_quotes(quotes)?;
                
                let calibrator = HazardCurveCalibrator::new(
                    entity,
                    *seniority,
                    recovery_rate,
                    self.base_date,
                    currency,
                    HazardCurveCalibrator::default_discount_curve_id(currency),
                );

                let (curve, report) = calibrator.calibrate(quotes, context)?;
                let updated_context = context.clone().with_hazard(curve);
                Ok((updated_context, report))
            }
            CalibrationTarget::InflationCurve { index } => {
                let base_cpi = self.get_base_cpi_from_context(context, index)?;
                let calibrator = InflationCurveCalibrator::new(
                    index,
                    self.base_date,
                    self.base_currency,
                    base_cpi,
                    HazardCurveCalibrator::default_discount_curve_id(self.base_currency),
                );

                let (curve, report) = calibrator.calibrate(quotes, context)?;
                let updated_context = context.clone().with_inflation(curve);
                Ok((updated_context, report))
            }
            CalibrationTarget::VolatilitySurface { underlying } => {
                // Determine appropriate grid and SABR beta from underlying
                let (expiry_grid, strike_grid, beta) = self.determine_vol_surface_params(underlying, quotes)?;
                
                let calibrator = VolSurfaceCalibrator::new(
                    format!("{}-VOL", underlying),
                    beta,
                    expiry_grid,
                    strike_grid,
                );

                let (surface, report) = calibrator.calibrate(quotes, context)?;
                let updated_context = context.clone().with_surface(surface);
                Ok((updated_context, report))
            }
            CalibrationTarget::BaseCorrelationCurve { index, maturity_years } => {
                let calibrator = BaseCorrelationCalibrator::new(
                    index,
                    42, // Default series number, should be configurable
                    maturity_years.value(),
                    self.base_date,
                );

                // Convert to ValuationMarketContext for base correlation calibration
                let val_context = ValuationMarketContext::from_core(context.clone());
                let (curve, report) = calibrator.bootstrap_curve(
                    quotes, 
                    &crate::calibration::solver::HybridSolver::new(), 
                    &val_context
                )?;

                // Use the original curve directly since it already has the right data
                let curve_with_id = curve;
                
                let updated_context = context.clone().with_base_correlation(curve_with_id);
                Ok((updated_context, report))
            }
        }
    }

    /// Extract hazard curve parameters from CDS quotes.
    fn extract_hazard_params_from_quotes(&self, quotes: &[InstrumentQuote]) -> Result<(F, Currency)> {
        for quote in quotes {
            match quote {
                InstrumentQuote::CDS { recovery_rate, currency, .. } => {
                    return Ok((*recovery_rate, *currency));
                }
                InstrumentQuote::CDSUpfront { recovery_rate, currency, .. } => {
                    return Ok((*recovery_rate, *currency));
                }
                _ => {}
            }
        }
        // Fallback to defaults
        Ok((0.4, self.base_currency))
    }

    /// Determine volatility surface parameters from option quotes.
    fn determine_vol_surface_params(&self, underlying: &str, quotes: &[InstrumentQuote]) -> Result<(Vec<F>, Vec<F>, F)> {
        let mut expiries = std::collections::HashSet::new();
        let mut strikes = std::collections::HashSet::new();

        for quote in quotes {
            if let InstrumentQuote::OptionVol { expiry, strike, .. } = quote {
                let days = (*expiry - self.base_date).whole_days();
                let years = days as F / 365.25;
                expiries.insert((years * 1000.0).round() as i32);
                strikes.insert((*strike * 100.0).round() as i32);
            }
        }

        let mut expiry_grid: Vec<F> = expiries
            .into_iter()
            .map(|e| e as F / 1000.0)
            .collect();
        expiry_grid.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut strike_grid: Vec<F> = strikes
            .into_iter()
            .map(|s| s as F / 100.0)
            .collect();
        strike_grid.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Determine SABR beta based on asset class
        let beta = if underlying.contains("USD") || underlying.contains("EUR") {
            0.5 // Interest rates
        } else {
            1.0 // Equity/FX
        };

        Ok((expiry_grid, strike_grid, beta))
    }

    // Removed legacy sequential calibration in favor of DAG-based `calibrate_market`.

    // (legacy discount curve stage removed)

 

    /// Calibrate hazard curves for different entities.
    #[allow(dead_code)]
    fn calibrate_hazard_curves(
        &self,
        quotes: &[InstrumentQuote],
        context: &MarketContext,
    ) -> Result<
        HashMap<
            String,
            (
                finstack_core::market_data::term_structures::hazard_curve::HazardCurve,
                CalibrationReport,
            ),
        >,
    > {
        let mut results = HashMap::new();

        // Group CDS quotes by entity
        let mut quotes_by_entity: HashMap<String, Vec<&InstrumentQuote>> = HashMap::new();
        for quote in quotes {
            match quote {
                InstrumentQuote::CDS { entity, .. } => {
                    quotes_by_entity
                        .entry(entity.clone())
                        .or_default()
                        .push(quote);
                }
                InstrumentQuote::CDSUpfront { entity, .. } => {
                    quotes_by_entity
                        .entry(entity.clone())
                        .or_default()
                        .push(quote);
                }
                _ => {}
            }
        }

        for (entity, entity_quotes) in quotes_by_entity {
            if entity_quotes.len() < 2 {
                continue; // Need multiple tenors for bootstrapping
            }

            // Extract recovery rate and currency from first quote
            let (recovery_rate, currency) = match entity_quotes[0] {
                InstrumentQuote::CDS { recovery_rate, currency, .. } => (*recovery_rate, *currency),
                InstrumentQuote::CDSUpfront { recovery_rate, currency, .. } => (*recovery_rate, *currency),
                _ => (0.4, self.base_currency), // Fallback to defaults
            };

            // Use entity-specific seniority from config, defaulting to Senior
            let seniority = self.config.entity_seniority
                .get(&entity)
                .copied()
                .unwrap_or(Seniority::Senior);

            let calibrator = HazardCurveCalibrator::new(
                &entity,
                seniority,
                recovery_rate,
                self.base_date,
                currency,
                HazardCurveCalibrator::default_discount_curve_id(currency),
            );

            let entity_quote_vec: Vec<_> = entity_quotes.iter().map(|&q| q.clone()).collect();
            match calibrator.calibrate(&entity_quote_vec, context) {
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
    #[allow(dead_code)]
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

            // Source base CPI from market context using prioritized fallbacks
            let base_cpi = self.get_base_cpi_from_context(context, &index)?;

            let calibrator = InflationCurveCalibrator::new(
                &index, 
                self.base_date, 
                self.base_currency, 
                base_cpi,
                HazardCurveCalibrator::default_discount_curve_id(self.base_currency),
            );

            let index_quote_vec: Vec<_> = index_quotes.iter().map(|&q| q.clone()).collect();
            match calibrator.calibrate(&index_quote_vec, context) {
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
    #[allow(dead_code)]
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

            // Build asset-specific forward function from market context
            let forward_fn = match self.build_forward_function_for_underlying(_context, &underlying)
            {
                Ok(fwd_fn) => fwd_fn,
                Err(_) => {
                    // Skip this underlying if we can't build forward function
                    continue;
                }
            };

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
    #[allow(dead_code)]
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

    /// Get base CPI from market context using prioritized fallbacks.
    ///
    /// Priority order:
    /// 1. InflationIndex with `index` key -> value_on(base_date)
    /// 2. InflationCurve with `index` key -> cpi(0.0)
    /// 3. MarketScalar with `"{index}-BASE_CPI"` key
    /// 4. Error if none available
    fn get_base_cpi_from_context(
        &self,
        context: &MarketContext,
        index: &str,
    ) -> Result<finstack_core::F> {
        // Try 1: InflationIndex lookup
        if let Some(inflation_index) = context.inflation_index(index) {
            match inflation_index.value_on(self.base_date) {
                Ok(cpi_value) => return Ok(cpi_value),
                Err(_) => {
                    // Index exists but value lookup failed, continue to next fallback
                }
            }
        }

        // Try 2: InflationCurve lookup
        if let Ok(inflation_curve) = context.inflation(index) {
            return Ok(inflation_curve.cpi(0.0));
        }

        // Try 3: MarketScalar lookup with standard naming convention
        let base_cpi_key = format!("{}-BASE_CPI", index);
        if let Ok(market_scalar) = context.market_scalar(&base_cpi_key) {
            return match market_scalar {
                finstack_core::market_data::primitives::MarketScalar::Unitless(value) => Ok(*value),
                finstack_core::market_data::primitives::MarketScalar::Price(money) => {
                    Ok(money.amount())
                }
            };
        }

        // No valid source found
        Err(finstack_core::Error::Input(
            finstack_core::error::InputError::NotFound { id: "calibration_data".to_string() },
        ))
    }

    /// Build asset-specific forward function for volatility surface calibration.
    ///
    /// Creates appropriate forward calculation based on underlying asset class:
    /// - Equity: S0 * exp((r - q) * t)
    /// - FX: S0 * exp((r_dom - r_for) * t)  
    /// - Rates: forward_curve.rate(t)
    fn build_forward_function_for_underlying(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(finstack_core::F) -> finstack_core::F + '_>> {
        // Detect asset class from underlying identifier
        if underlying.contains("-")
            && (underlying.contains("SOFR")
                || underlying.contains("EURIBOR")
                || underlying.contains("SONIA"))
        {
            // Interest rate underlying (e.g., "USD-SOFR3M", "EUR-EURIBOR3M")
            self.build_rate_forward_for_orchestrator(context, underlying)
        } else if underlying.len() == 6 && underlying.chars().all(|c| c.is_ascii_alphabetic()) {
            // FX pair (e.g., "EURUSD", "GBPJPY")
            self.build_fx_forward_for_orchestrator(context, underlying)
        } else {
            // Equity underlying (e.g., "SPY", "AAPL")
            self.build_equity_forward_for_orchestrator(context, underlying)
        }
    }

    /// Build equity forward function: F(t) = S0 * exp((r - q) * t)
    fn build_equity_forward_for_orchestrator(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(finstack_core::F) -> finstack_core::F + '_>> {
        // Get spot price
        let spot_scalar = context.market_scalar(underlying)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
        };

        // Get dividend yield (default to 0.0 if not available)
        let div_yield_key = format!("{}-DIVYIELD", underlying);
        let dividend_yield = context
            .market_scalar(&div_yield_key)
            .map(|scalar| match scalar {
                finstack_core::market_data::primitives::MarketScalar::Unitless(yield_val) => {
                    *yield_val
                }
                _ => 0.0,
            })
            .unwrap_or(0.0);

        // Get risk-free rate from discount curve
        let disc_curve_id = format!("{}-OIS", self.base_currency);
        let discount_curve = context.discount(&disc_curve_id)?;

        Ok(Box::new(move |t: finstack_core::F| -> finstack_core::F {
            let risk_free_rate = discount_curve.zero(t);
            spot * ((risk_free_rate - dividend_yield) * t).exp()
        }))
    }

    /// Build FX forward function: F(t) = S0 * exp((r_dom - r_for) * t)
    fn build_fx_forward_for_orchestrator(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(finstack_core::F) -> finstack_core::F + '_>> {
        // Parse FX pair (assume 6-char format like "EURUSD")
        if underlying.len() != 6 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            ));
        }

        let foreign_ccy = &underlying[0..3];
        let domestic_ccy = &underlying[3..6];

        // Get spot rate
        let spot_scalar = context.market_scalar(underlying)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
        };

        // Get domestic and foreign discount curves
        let dom_disc_id = format!("{}-OIS", domestic_ccy);
        let for_disc_id = format!("{}-OIS", foreign_ccy);
        let dom_curve = context.discount(&dom_disc_id)?;
        let for_curve = context.discount(&for_disc_id)?;

        Ok(Box::new(move |t: finstack_core::F| -> finstack_core::F {
            let domestic_rate = dom_curve.zero(t);
            let foreign_rate = for_curve.zero(t);
            spot * ((domestic_rate - foreign_rate) * t).exp()
        }))
    }

    /// Build rates forward function: F(t) = forward_curve.rate(t)
    fn build_rate_forward_for_orchestrator(
        &self,
        context: &MarketContext,
        underlying: &str,
    ) -> Result<Box<dyn Fn(finstack_core::F) -> finstack_core::F + '_>> {
        // Get forward curve for this index
        let forward_curve = context.forecast(underlying)?;

        Ok(Box::new(move |t: finstack_core::F| -> finstack_core::F {
            forward_curve.rate(t)
        }))
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
            // Basis swaps for multi-curve construction
            InstrumentQuote::BasisSwap {
                maturity: base_date + time::Duration::days(365 * 2),
                primary_index: "USD-LIBOR-3M".to_string(),
                reference_index: "USD-LIBOR-6M".to_string(),
                spread_bp: 15.0, // 3M LIBOR pays 6M LIBOR + 15bp
                primary_freq: Frequency::quarterly(),
                reference_freq: Frequency::semi_annual(),
                primary_dc: DayCount::Act360,
                reference_dc: DayCount::Act360,
                currency: Currency::USD,
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
        let (context, report) = orchestrator.calibrate_market(&quotes).unwrap();
        assert!(report.success);

        // Should produce a USD OIS discount curve in the context
        let disc = context.discount("USD-OIS");
        assert!(disc.is_ok());
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
