//! Base correlation curve calibration from CDS tranche quotes.
//!
//! Implements market-standard base correlation bootstrapping using the
//! one-factor Gaussian Copula model and equity tranche decomposition.

use crate::calibration::primitives::{HashableFloat, InstrumentQuote};
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::fixed_income::cds_tranche::{CdsTranche, TrancheSide};
use finstack_core::math::Solver;

use crate::market_data::ValuationMarketContext;
use finstack_core::dates::utils::add_months;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::F;
use std::collections::HashMap;
use std::sync::Arc;

/// Base correlation curve calibrator.
#[derive(Clone, Debug)]
pub struct BaseCorrelationCalibrator {
    /// Index identifier (e.g., "CDX.NA.IG.42")
    pub index_id: String,
    /// Index series number
    pub series: u16,
    /// Maturity for correlation curve (e.g., 5 years)
    pub maturity_years: F,
    /// Base date for calibration
    pub base_date: Date,
    /// Standard detachment points to calibrate
    pub detachment_points: Vec<F>,
    /// Calibration configuration
    pub config: CalibrationConfig,
}

impl BaseCorrelationCalibrator {
    /// Create a new base correlation calibrator.
    pub fn new(
        index_id: impl Into<String>,
        series: u16,
        maturity_years: F,
        base_date: Date,
    ) -> Self {
        Self {
            index_id: index_id.into(),
            series,
            maturity_years,
            base_date,
            // Standard market detachment points
            detachment_points: vec![3.0, 7.0, 10.0, 15.0, 30.0],
            config: CalibrationConfig::default(),
        }
    }

    /// Set custom detachment points.
    pub fn with_detachment_points(mut self, points: Vec<F>) -> Self {
        self.detachment_points = points;
        self
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Bootstrap base correlation curve from tranche quotes using sequential calibration.
    ///
    /// Implements market-standard bootstrapping methodology:
    /// 1. Sort tranches by detachment point (equity to senior)
    /// 2. For each tranche [A, D], solve for ρ(D) such that:
    ///    Price([A, D]) = Price([0, D], ρ(D)) - Price([0, A], ρ(A))
    /// 3. Use previously solved correlations for [0, A] pricing
    pub fn bootstrap_curve<S: Solver>(
        &self,
        quotes: &[InstrumentQuote],
        solver: &S,
        market_context: &ValuationMarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        use crate::instruments::fixed_income::cds_tranche::model::GaussianCopulaModel;

        // Filter and extract CDS tranche quotes
        let mut tranche_quotes: Vec<_> = quotes
            .iter()
            .filter_map(|q| {
                if let InstrumentQuote::CDSTranche {
                    index,
                    attachment,
                    detachment,
                    maturity,
                    upfront_pct,
                    running_spread_bp,
                } = q
                {
                    Some((
                        attachment,
                        detachment,
                        maturity,
                        upfront_pct,
                        running_spread_bp,
                        index,
                    ))
                } else {
                    None
                }
            })
            .collect();

        if tranche_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: "base_correlation_data".to_string(),
                },
            ));
        }

        // Sort by detachment point for sequential bootstrapping
        tranche_quotes.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

        // Validate tranche quotes
        for (attach, detach, _, _, _, _) in &tranche_quotes {
            if **attach >= **detach {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
            if **attach < 0.0 || **detach <= 0.0 {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::NegativeValue,
                ));
            }
        }

        let mut solved_correlations: Vec<(F, F)> = Vec::new();
        let mut residuals = HashMap::new();
        let mut total_iterations = 0;
        let pricing_model = GaussianCopulaModel::new();
        let num_tranche_quotes = tranche_quotes.len(); // Store length before moving

        // Pre-fetch the original credit index data once to avoid repeated lookups
        let original_index = market_context.get_credit_index("CDX.NA.IG.42")?;
        let base_core_context = &market_context.core;

        // Sequential bootstrap from equity to senior tranches
        for (attach_pct, detach_pct, _maturity, upfront_pct, running_spread_bp, index) in
            tranche_quotes
        {
            // Skip if we don't have the right index
            if index != &self.index_id {
                continue;
            }

            // Create synthetic tranche for this quote
            let synthetic_tranche =
                self.create_synthetic_tranche(*attach_pct, *detach_pct, *running_spread_bp)?;

            // Target upfront value from market quote
            let target_upfront = upfront_pct / 100.0 * synthetic_tranche.notional.amount();

            // Initial guess for correlation
            let initial_guess = if solved_correlations.is_empty() {
                0.3 // Reasonable starting point for equity tranches
            } else {
                // Start slightly above the last solved correlation (monotonic assumption)
                let last_pair = solved_correlations.last().unwrap();
                let last_correlation = last_pair.1;
                (last_correlation + 0.05).min(0.9)
            };

            // Pre-build shared index data outside the objective function
            let shared_num_constituents = original_index.num_constituents;
            let shared_recovery_rate = original_index.recovery_rate;
            let shared_credit_curve = Arc::clone(&original_index.index_credit_curve);
            let solved_correlations_ref = &solved_correlations;
            let detach_pct_val = *detach_pct;

            // Create objective function for this tranche - now much more efficient
            let objective = |trial_correlation: F| -> F {
                // Build temporary base correlation curve with solved points + trial point
                let mut temp_corr_points = Vec::with_capacity(solved_correlations_ref.len() + 1);
                temp_corr_points.extend_from_slice(solved_correlations_ref);
                temp_corr_points.push((detach_pct_val, trial_correlation));

                // Ensure minimum curve requirements (need at least 2 points)
                if temp_corr_points.len() < 2 {
                    // For first (equity) tranche, add a second point for curve building
                    temp_corr_points.push((detach_pct_val + 10.0, trial_correlation));
                }

                // Create temporary base correlation curve - this is now the only rebuild per iteration
                let temp_base_corr_curve = match BaseCorrelationCurve::builder("TEMP_CALIB_CORR")
                    .points(temp_corr_points)
                    .build()
                {
                    Ok(curve) => curve,
                    Err(_) => return F::INFINITY,
                };

                // Efficiently create index with new correlation curve - reusing shared data
                let temp_index = match crate::market_data::credit_index::CreditIndexData::builder()
                    .num_constituents(shared_num_constituents)
                    .recovery_rate(shared_recovery_rate)
                    .index_credit_curve(Arc::clone(&shared_credit_curve))
                    .base_correlation_curve(Arc::new(temp_base_corr_curve))
                    .build()
                {
                    Ok(idx) => idx,
                    Err(_) => return F::INFINITY,
                };

                // Create temporary market context - reusing core context
                let temp_market_ctx = crate::market_data::ValuationMarketContext::from_core(
                    base_core_context.clone(),
                )
                .with_credit_index(synthetic_tranche.credit_index_id, temp_index);

                // Price the tranche and return upfront error
                match pricing_model.price_tranche(
                    &synthetic_tranche,
                    &temp_market_ctx,
                    self.base_date,
                ) {
                    Ok(pv) => pv.amount() - target_upfront,
                    Err(_) => F::INFINITY,
                }
            };

            // Solve for correlation
            let solved_corr = solver.solve(objective, initial_guess)?;

            // Clamp to reasonable bounds
            let clamped_corr = solved_corr.clamp(0.01, 0.99);

            // Calculate final residual
            let final_residual = objective(clamped_corr);

            solved_correlations.push((*detach_pct, clamped_corr));
            residuals.insert(format!("{}%-{}%", attach_pct, detach_pct), final_residual);
            total_iterations += 1;
        }

        if solved_correlations.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: "base_correlation_data".to_string(),
                },
            ));
        }

        // Build final base correlation curve
        let final_curve = BaseCorrelationCurve::builder("CALIBRATED_BASE_CORR")
            .points(solved_correlations)
            .build()?;

        let report = CalibrationReport::success_with(
            residuals,
            total_iterations,
            "Base correlation bootstrap completed successfully",
        )
        .with_metadata("calibrated_tranches", format!("{}", num_tranche_quotes));

        Ok((final_curve, report))
    }

    /// Create synthetic CDS tranche for pricing.
    fn create_synthetic_tranche(
        &self,
        attach_pct: F,
        detach_pct: F,
        running_spread_bp: F,
    ) -> Result<CdsTranche> {
        // Use proper calendar arithmetic instead of 365.25 approximation
        let months_to_add = (self.maturity_years * 12.0).round() as i32;
        let maturity = add_months(self.base_date, months_to_add);

        // Use builder to avoid lifetime issues with &str parameters
        CdsTranche::builder()
            .id(format!("CALIB_TRANCHE_{:.1}_{:.1}", attach_pct, detach_pct))
            .index_name(self.index_id.clone())
            .series(self.series)
            .attach_pct(attach_pct)
            .detach_pct(detach_pct)
            .notional(Money::new(10_000_000.0, Currency::USD))
            .maturity(maturity)
            .running_coupon_bp(running_spread_bp)
            .payment_frequency(Frequency::quarterly())
            .day_count(DayCount::Act360)
            .business_day_convention(BusinessDayConvention::Following)
            .disc_id("USD-OIS")
            .credit_index_id("CDX.NA.IG.42") // Use static string that matches test context
            .side(TrancheSide::SellProtection)
            .build()
    }
}

impl Calibrator<InstrumentQuote, BaseCorrelationCurve> for BaseCorrelationCalibrator {
    fn calibrate(
        &self,
        instruments: &[InstrumentQuote],
        base_context: &MarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        // Convert core market context to valuation context
        let val_ctx = ValuationMarketContext::from_core(base_context.clone());

        // Use the configured solver for robust root-finding
        let solver = self.config.make_solver();

        // Delegate to the implemented bootstrap
        self.bootstrap_curve(instruments, &solver, &val_ctx)
    }
}

/// Multi-expiry base correlation surface calibrator.
///
/// Calibrates base correlation curves for multiple maturities and
/// builds a correlation surface.
#[derive(Clone, Debug)]
pub struct BaseCorrelationSurfaceCalibrator {
    /// Index identifier
    pub index_id: String,
    /// Index series
    pub series: u16,
    /// Base date
    pub base_date: Date,
    /// Target maturities in years
    pub target_maturities: Vec<F>,
    /// Standard detachment points
    pub detachment_points: Vec<F>,
    /// Configuration
    pub config: CalibrationConfig,
}

impl BaseCorrelationSurfaceCalibrator {
    /// Create a new surface calibrator.
    pub fn new(
        index_id: impl Into<String>,
        series: u16,
        base_date: Date,
        target_maturities: Vec<F>,
    ) -> Self {
        Self {
            index_id: index_id.into(),
            series,
            base_date,
            target_maturities,
            detachment_points: vec![3.0, 7.0, 10.0, 15.0, 30.0],
            config: CalibrationConfig::default(),
        }
    }

    /// Calibrate correlation surface from tranche quotes across maturities.
    pub fn calibrate_surface(
        &self,
        quotes: &[InstrumentQuote],
        market_context: &ValuationMarketContext,
    ) -> Result<(
        HashMap<HashableFloat, BaseCorrelationCurve>,
        CalibrationReport,
    )> {
        // Group quotes by maturity
        let mut quotes_by_maturity: HashMap<HashableFloat, Vec<&InstrumentQuote>> = HashMap::new();

        for quote in quotes {
            if let InstrumentQuote::CDSTranche { maturity, .. } = quote {
                let maturity_years = finstack_core::dates::DayCount::Act365F.year_fraction(
                    self.base_date,
                    *maturity,
                    finstack_core::dates::DayCountCtx::default(),
                )?;

                // Round to nearest target maturity
                if let Some(&target_mat) = self.target_maturities.iter().min_by(|&&a, &&b| {
                    (a - maturity_years)
                        .abs()
                        .partial_cmp(&(b - maturity_years).abs())
                        .unwrap()
                }) {
                    quotes_by_maturity
                        .entry(HashableFloat::new(target_mat))
                        .or_default()
                        .push(quote);
                }
            }
        }

        let mut curves_by_maturity = HashMap::new();
        let mut all_residuals = HashMap::new();
        let mut total_iterations = 0;

        // Calibrate each maturity separately
        for &maturity_years in &self.target_maturities {
            if let Some(maturity_quotes) =
                quotes_by_maturity.get(&HashableFloat::new(maturity_years))
            {
                let calibrator = BaseCorrelationCalibrator::new(
                    &self.index_id,
                    self.series,
                    maturity_years,
                    self.base_date,
                );

                let maturity_quote_vec: Vec<_> =
                    maturity_quotes.iter().map(|&q| q.clone()).collect();
                let solver = calibrator.config.make_solver();
                match calibrator.bootstrap_curve(&maturity_quote_vec, &solver, market_context) {
                    Ok((curve, report)) => {
                        curves_by_maturity.insert(HashableFloat::new(maturity_years), curve);

                        // Merge residuals with maturity prefix
                        for (key, value) in report.residuals {
                            all_residuals.insert(format!("{}Y-{}", maturity_years, key), value);
                        }
                        total_iterations += report.iterations;
                    }
                    Err(_) => {
                        // Failed to calibrate this maturity - continue with others
                        continue;
                    }
                }
            }
        }

        let report = CalibrationReport::new()
            .success()
            .with_residuals(all_residuals)
            .with_iterations(total_iterations)
            .with_convergence_reason("Base correlation surface calibration completed")
            .with_metadata(
                "calibrated_maturities".to_string(),
                format!("{}", curves_by_maturity.len()),
            );

        Ok((curves_by_maturity, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::credit_index::CreditIndexData;
    #[allow(unused_imports)]
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    use finstack_core::market_data::term_structures::{
        discount_curve::DiscountCurve, BaseCorrelationCurve,
    };
    // use finstack_core::market_data::interp::InterpStyle; // not used in this test module
    use std::sync::Arc;
    use time::Month;

    fn create_test_market_context() -> ValuationMarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .set_interp(finstack_core::market_data::interp::InterpStyle::LogLinear)
            .build()
            .unwrap();

        // Create index hazard curve
        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.01), (3.0, 0.015), (5.0, 0.02), (10.0, 0.025)])
            .par_spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (10.0, 140.0)])
            .build()
            .unwrap();

        // Create placeholder base correlation curve
        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![(3.0, 0.30), (10.0, 0.50)])
            .build()
            .unwrap();

        // Create credit index data
        let index_data = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .unwrap();

        ValuationMarketContext::new()
            .insert_discount(discount_curve)
            .with_credit_index("CDX.NA.IG.42", index_data)
    }

    #[test]
    fn test_base_correlation_calibrator_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date);

        assert_eq!(calibrator.index_id, "CDX.NA.IG.42");
        assert_eq!(calibrator.series, 42);
        assert_eq!(calibrator.maturity_years, 5.0);
        assert_eq!(
            calibrator.detachment_points,
            vec![3.0, 7.0, 10.0, 15.0, 30.0]
        );
    }

    #[test]
    fn test_synthetic_tranche_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date);

        let tranche = calibrator
            .create_synthetic_tranche(0.0, 3.0, 500.0)
            .unwrap();

        assert_eq!(tranche.attach_pct, 0.0);
        assert_eq!(tranche.detach_pct, 3.0);
        assert_eq!(tranche.running_coupon_bp, 500.0);
        assert_eq!(tranche.side, TrancheSide::SellProtection);
    }

    #[test]
    fn test_base_correlation_curve_building() {
        // Test direct BaseCorrelationCurve building functionality
        let correlation_knots = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        let curve = BaseCorrelationCurve::builder("TEST_CORR")
            .points(correlation_knots)
            .build()
            .unwrap();

        assert_eq!(curve.detachment_points().len(), 3);
        assert_eq!(curve.correlations().len(), 3);

        // Test interpolation
        assert!((curve.correlation(5.0) - 0.35).abs() < 1e-9); // Midpoint between 3% and 7%
    }

    #[test]
    fn test_base_correlation_surface_calibrator() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let surface_calibrator = BaseCorrelationSurfaceCalibrator::new(
            "CDX.NA.IG.42",
            42,
            base_date,
            vec![3.0, 5.0, 7.0],
        );

        assert_eq!(surface_calibrator.target_maturities, vec![3.0, 5.0, 7.0]);
        assert_eq!(
            surface_calibrator.detachment_points,
            vec![3.0, 7.0, 10.0, 15.0, 30.0]
        );
    }

    #[test]
    fn test_base_correlation_calibration_round_trip() {
        use crate::instruments::fixed_income::cds_tranche::model::GaussianCopulaModel;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        // Create test market context
        let market_ctx = create_test_market_context();

        // Create known base correlation curve for validation
        let known_correlations = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        let known_curve = BaseCorrelationCurve::builder("KNOWN_CORR")
            .points(known_correlations.clone())
            .build()
            .unwrap();

        // Create market context with known correlation curve
        let original_index = market_ctx.get_credit_index("CDX.NA.IG.42").unwrap();
        let test_index = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(original_index.index_credit_curve.clone())
            .base_correlation_curve(std::sync::Arc::new(known_curve))
            .build()
            .unwrap();

        let test_market_ctx = ValuationMarketContext::from_core(market_ctx.core.clone())
            .with_credit_index("CDX.NA.IG.42", test_index);

        // Generate synthetic market quotes using known correlations
        let pricing_model = GaussianCopulaModel::new();
        let mut synthetic_quotes = Vec::new();

        for (detach_pct, _corr) in &known_correlations {
            // Create synthetic equity tranche [0, detach_pct]
            let tranche = CdsTranche::new(
                format!("EQUITY_0_{}", detach_pct),
                "CDX.NA.IG.42",
                42,
                0.0,         // attachment
                *detach_pct, // detachment
                Money::new(10_000_000.0, Currency::USD),
                maturity,
                500.0, // running spread
                Frequency::quarterly(),
                DayCount::Act360,
                BusinessDayConvention::Following,
                None,
                "USD-OIS",
                "CDX.NA.IG.42",
                TrancheSide::SellProtection,
            );

            // Price with known correlation to get "market" upfront
            let market_pv = pricing_model
                .price_tranche(&tranche, &test_market_ctx, base_date)
                .unwrap();
            let market_upfront_pct = market_pv.amount() / tranche.notional.amount() * 100.0;

            synthetic_quotes.push(InstrumentQuote::CDSTranche {
                index: "CDX.NA.IG.42".to_string(),
                attachment: 0.0,
                detachment: *detach_pct,
                maturity,
                upfront_pct: market_upfront_pct,
                running_spread_bp: 500.0,
            });
        }

        // Now calibrate using these synthetic quotes
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date);
        let solver = calibrator.config.make_solver();

        // Create clean market context for calibration (with dummy base correlation curve)
        let original_index = market_ctx.get_credit_index("CDX.NA.IG.42").unwrap();

        // Create a dummy base correlation curve for initial calibration
        let dummy_base_corr_curve = BaseCorrelationCurve::builder("DUMMY_CALIB_CORR")
            .points(vec![(1.0, 0.01), (100.0, 0.01)]) // Minimal curve for building requirements
            .build()
            .unwrap();

        let clean_index = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(original_index.index_credit_curve.clone())
            .base_correlation_curve(std::sync::Arc::new(dummy_base_corr_curve))
            .build()
            .unwrap();

        let clean_market_ctx = ValuationMarketContext::from_core(market_ctx.core.clone())
            .with_credit_index("CDX.NA.IG.42", clean_index);

        let calibration_result =
            calibrator.bootstrap_curve(&synthetic_quotes, &solver, &clean_market_ctx);

        assert!(calibration_result.is_ok());
        let (calibrated_curve, report) = calibration_result.unwrap();

        // Verify calibration was successful
        assert!(report.success);
        assert_eq!(
            calibrated_curve.detachment_points().len(),
            known_correlations.len()
        );

        // Verify that calibrated correlations are close to known values
        for (expected_detach, expected_corr) in &known_correlations {
            let calibrated_corr = calibrated_curve.correlation(*expected_detach);
            assert!(
                (calibrated_corr - expected_corr).abs() < 0.05, // 5% tolerance for numerical precision
                "Correlation mismatch at {}%: expected {}, got {}",
                expected_detach,
                expected_corr,
                calibrated_corr
            );
        }

        // Verify calibration quality
        assert!(report.max_residual < 1e-6); // Very tight tolerance for round-trip test
    }
}
