//! Base correlation curve calibration from CDS tranche quotes.
//!
//! Implements market-standard base correlation bootstrapping using the
//! one-factor Gaussian Copula model and equity tranche decomposition.

use crate::calibration::quote::CreditQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use finstack_core::math::Solver;
use ordered_float::OrderedFloat;

use finstack_core::dates::utils::add_months;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::market_data::MarketContext;
// use finstack_core::market_data::context::MarketContext; // use re-export above
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::money::Money;
use finstack_core::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Interpolation method for base correlation curves (currently linear only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrelationInterp {
    /// Linear interpolation between detachment points
    Linear,
}

/// Minimum correlation bound (1% to avoid numerical issues)
const MIN_CORRELATION: f64 = 0.01;

/// Maximum correlation bound (99% to avoid numerical issues)
const MAX_CORRELATION: f64 = 0.99;

/// Default initial correlation guess for equity tranches
const INITIAL_CORRELATION_GUESS: f64 = 0.3;

/// Correlation step size for monotonic assumption
const CORRELATION_STEP: f64 = 0.05;

/// Maximum correlation for monotonic extrapolation
const MAX_MONOTONIC_CORRELATION: f64 = 0.9;

/// Base correlation curve calibrator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseCorrelationCalibrator {
    /// Index identifier (e.g., "CDX.NA.IG.42")
    pub index_id: String,
    /// Index series number
    pub series: u16,
    /// Maturity for correlation curve (e.g., 5 years)
    pub maturity_years: f64,
    /// Base date for calibration
    pub base_date: Date,
    /// Discount curve identifier used for tranche PVs
    pub discount_curve_id: finstack_core::types::CurveId,
    /// Standard detachment points to calibrate
    pub detachment_points: Vec<f64>,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Interpolation used for base correlation between detachment points
    pub corr_interp: CorrelationInterp,
}

impl BaseCorrelationCalibrator {
    /// Create a new base correlation calibrator.
    pub fn new(
        index_id: impl Into<String>,
        series: u16,
        maturity_years: f64,
        base_date: Date,
    ) -> Self {
        Self {
            index_id: index_id.into(),
            series,
            maturity_years,
            base_date,
            // Default to common OIS discounting for USD; configurable via with_discount_curve_id
            discount_curve_id: finstack_core::types::CurveId::from("USD-OIS"),
            // Standard market detachment points
            detachment_points: vec![3.0, 7.0, 10.0, 15.0, 30.0],
            config: CalibrationConfig::default(),
            corr_interp: CorrelationInterp::Linear,
        }
    }

    /// Set custom detachment points.
    pub fn with_detachment_points(mut self, points: Vec<f64>) -> Self {
        self.detachment_points = points;
        self
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the discount curve identifier used when pricing synthetic tranches.
    pub fn with_discount_curve_id(
        mut self,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
    ) -> Self {
        self.discount_curve_id = discount_curve_id.into();
        self
    }

    /// Set interpolation method for base correlation between detachment points.
    pub fn with_corr_interp(mut self, interp: CorrelationInterp) -> Self {
        self.corr_interp = interp;
        self
    }

    /// Bootstrap base correlation curve from tranche quotes using sequential calibration.
    ///
    /// Implements market-standard bootstrapping methodology:
    /// 1. Sort tranches by detachment point (equity to senior)
    /// 2. For each tranche [A, D], solve for ρ(D) such that:
    ///    Price([A, D]) = Price([0, D], ρ(D)) - Price([0, A], ρ(A))
    /// 3. Use previously solved correlations for [0, A] pricing
    fn bootstrap_curve<S: Solver>(
        &self,
        quotes: &[CreditQuote],
        solver: &S,
        market_context: &MarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        use crate::instruments::cds_tranche::pricer::CDSTranchePricer;

        // Filter and extract CDS tranche quotes, keeping only the requested index
        let mut tranche_quotes: Vec<_> = quotes
            .iter()
            .filter_map(|q| {
                if let CreditQuote::CDSTranche {
                    index,
                    attachment,
                    detachment,
                    maturity,
                    upfront_pct,
                    running_spread_bp,
                } = q
                {
                    if index == &self.index_id {
                        Some((
                            attachment,
                            detachment,
                            maturity,
                            upfront_pct,
                            running_spread_bp,
                        ))
                    } else {
                        None
                    }
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

        // Validate no NaN values in detachment points before sorting
        for (_, detach, _, _, _) in &tranche_quotes {
            if !detach.is_finite() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        // Sort by detachment point for sequential bootstrapping
        tranche_quotes.sort_by(|a, b| OrderedFloat(*a.1).cmp(&OrderedFloat(*b.1)));

        // Validate tranche quotes
        for (attach, detach, _, _, _) in &tranche_quotes {
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

        let mut solved_correlations: Vec<(f64, f64)> = Vec::new();
        let mut residuals = BTreeMap::new();
        let mut total_iterations = 0;
        let pricing_model = CDSTranchePricer::new();
        let num_tranche_quotes = tranche_quotes.len(); // Store length before moving

        // Sequential bootstrap from equity to senior tranches
        for (index, (attach_pct, detach_pct, _maturity, upfront_pct, running_spread_bp)) in
            tranche_quotes.into_iter().enumerate()
        {
            // Create synthetic tranche for this quote
            let synthetic_tranche =
                self.create_synthetic_tranche(*attach_pct, *detach_pct, *running_spread_bp)?;

            // Target upfront value from market quote
            let target_upfront = upfront_pct / 100.0 * synthetic_tranche.notional.amount();

            // Initial guess for correlation
            let initial_guess = if solved_correlations.is_empty() {
                INITIAL_CORRELATION_GUESS // Reasonable starting point for equity tranches
            } else {
                // Start slightly above the last solved correlation (monotonic assumption)
                let last_pair = solved_correlations
                    .last()
                    .expect("solved_correlations should not be empty in else branch");
                let last_correlation = last_pair.1;
                (last_correlation + CORRELATION_STEP).min(MAX_MONOTONIC_CORRELATION)
            };

            let market_ctx_ref = market_context;

            // Pre-allocate correlation points buffer to reduce allocations in objective
            // Note: We still need to clone per evaluation due to solver API constraints,
            // but pre-allocating capacity reduces reallocation overhead
            let mut base_corr_points = Vec::with_capacity(solved_correlations.len() + 2);
            base_corr_points.extend_from_slice(&solved_correlations);

            let objective = |trial_correlation: f64| -> f64 {
                // Reuse pre-allocated buffer and only update last point
                let mut temp_corr_points = base_corr_points.clone();
                temp_corr_points.push((*detach_pct, trial_correlation));

                // Ensure minimum curve requirements (need at least 2 points)
                if temp_corr_points.len() < 2 {
                    temp_corr_points.push((*detach_pct + 10.0, trial_correlation));
                }

                let temp_base_corr_curve = match BaseCorrelationCurve::builder("TEMP_CALIB_CORR")
                    .knots(temp_corr_points)
                    .build()
                {
                    Ok(curve) => Arc::new(curve),
                    Err(_) => return crate::calibration::PENALTY,
                };

                let mut temp_market_ctx = market_ctx_ref.clone();
                if !temp_market_ctx.update_base_correlation_curve(
                    &synthetic_tranche.credit_index_id,
                    temp_base_corr_curve,
                ) {
                    return crate::calibration::PENALTY;
                }

                match pricing_model.price_tranche(
                    &synthetic_tranche,
                    &temp_market_ctx,
                    self.base_date,
                ) {
                    Ok(pv) => pv.amount() - target_upfront,
                    Err(_) => crate::calibration::PENALTY,
                }
            };

            // Solve for correlation
            let solved_corr = solver.solve(objective, initial_guess)?;

            // Clamp to reasonable bounds
            let clamped_corr = solved_corr.clamp(MIN_CORRELATION, MAX_CORRELATION);

            // Calculate final residual
            let final_residual = objective(clamped_corr);

            solved_correlations.push((*detach_pct, clamped_corr));
            let key = index.to_string();
            residuals.insert(key, final_residual);
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
            .knots(solved_correlations)
            .build()?;

        // Validate the calibrated base correlation curve
        use crate::calibration::validation::CurveValidator;
        final_curve
            .validate()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("Calibrated base correlation curve failed validation: {}", e),
                category: "base_correlation_validation".to_string(),
            })?;

        let report = CalibrationReport::for_type("base_correlation", residuals, total_iterations)
            .with_metadata("calibrated_tranches", num_tranche_quotes.to_string())
            .with_metadata("corr_interp", format!("{:?}", self.corr_interp))
            .with_metadata("validation", "passed");

        Ok((final_curve, report))
    }

    /// Create synthetic CDS tranche for pricing.
    fn create_synthetic_tranche(
        &self,
        attach_pct: f64,
        detach_pct: f64,
        running_spread_bp: f64,
    ) -> Result<CdsTranche> {
        // Use proper calendar arithmetic instead of 365.25 approximation
        let months_to_add = (self.maturity_years * 12.0).round() as i32;
        let maturity = add_months(self.base_date, months_to_add);

        let id = finstack_core::types::InstrumentId::new(format!(
            "CALIB_TRANCHE_{:.1}_{:.1}",
            attach_pct, detach_pct
        ));
        CdsTranche::builder()
            .id(id)
            .index_name(self.index_id.to_owned())
            .series(self.series)
            .attach_pct(attach_pct)
            .detach_pct(detach_pct)
            .notional(Money::new(10_000_000.0, Currency::USD))
            .maturity(maturity)
            .running_coupon_bp(running_spread_bp)
            .payment_frequency(Frequency::quarterly())
            .day_count(DayCount::Act360)
            .business_day_convention(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .discount_curve_id(self.discount_curve_id.to_owned())
            .credit_index_id(finstack_core::types::CurveId::new(self.index_id.to_owned()))
            .side(TrancheSide::SellProtection)
            .effective_date_opt(None)
            .build()
    }
}

impl Calibrator<CreditQuote, BaseCorrelationCurve> for BaseCorrelationCalibrator {
    fn calibrate(
        &self,
        instruments: &[CreditQuote],
        base_context: &MarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        let solver = crate::calibration::create_simple_solver(&self.config);
        self.bootstrap_curve(instruments, &solver, base_context)
    }
}

/// Multi-expiry base correlation surface calibrator.
///
/// Calibrates base correlation curves for multiple maturities and
/// builds a correlation surface.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseCorrelationSurfaceCalibrator {
    /// Index identifier
    pub index_id: String,
    /// Index series
    pub series: u16,
    /// Base date
    pub base_date: Date,
    /// Target maturities in years
    pub target_maturities: Vec<f64>,
    /// Standard detachment points
    pub detachment_points: Vec<f64>,
    /// Configuration
    pub config: CalibrationConfig,
    /// Day count used to map tranche maturities to years for grouping
    pub time_dc: DayCount,
}

impl BaseCorrelationSurfaceCalibrator {
    /// Create a new surface calibrator.
    pub fn new(
        index_id: impl Into<String>,
        series: u16,
        base_date: Date,
        target_maturities: Vec<f64>,
    ) -> Self {
        Self {
            index_id: index_id.into(),
            series,
            base_date,
            target_maturities,
            detachment_points: vec![3.0, 7.0, 10.0, 15.0, 30.0],
            config: CalibrationConfig::default(),
            time_dc: DayCount::Act365F,
        }
    }

    /// Calibrate correlation surface from tranche quotes across maturities.
    pub fn calibrate_surface(
        &self,
        quotes: &[CreditQuote],
        market_context: &MarketContext,
    ) -> Result<(
        BTreeMap<OrderedFloat<f64>, BaseCorrelationCurve>,
        CalibrationReport,
    )> {
        // Group quotes by maturity
        let mut quotes_by_maturity: BTreeMap<OrderedFloat<f64>, Vec<&CreditQuote>> =
            BTreeMap::new();

        for quote in quotes {
            if let CreditQuote::CDSTranche { maturity, .. } = quote {
                let maturity_years = self.time_dc.year_fraction(
                    self.base_date,
                    *maturity,
                    finstack_core::dates::DayCountCtx::default(),
                )?;

                // Round to nearest target maturity
                if let Some(&target_mat) = self.target_maturities.iter().min_by(|&&a, &&b| {
                    (a - maturity_years)
                        .abs()
                        .partial_cmp(&(b - maturity_years).abs())
                        .expect("f64 comparison should always be comparable")
                }) {
                    quotes_by_maturity
                        .entry(target_mat.into())
                        .or_default()
                        .push(quote);
                }
            }
        }

        let mut curves_by_maturity = BTreeMap::new();
        let mut all_residuals = BTreeMap::new();
        let mut residual_key_counter: usize = 0;
        let mut total_iterations = 0;

        // Calibrate each maturity separately
        for &maturity_years in &self.target_maturities {
            if let Some(maturity_quotes) = quotes_by_maturity.get(&maturity_years.into()) {
                let calibrator = BaseCorrelationCalibrator::new(
                    &self.index_id,
                    self.series,
                    maturity_years,
                    self.base_date,
                );

                let maturity_quote_vec: Vec<_> =
                    maturity_quotes.iter().map(|&q| q.clone()).collect();
                let result = calibrator.calibrate(&maturity_quote_vec, market_context);
                match result {
                    Ok((curve, report)) => {
                        curves_by_maturity.insert(maturity_years.into(), curve);

                        // Merge residuals with compact numeric keys
                        for (_key, value) in report.residuals {
                            let k = format!("{:06}", residual_key_counter);
                            residual_key_counter += 1;
                            all_residuals.insert(k, value);
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

        let report = CalibrationReport::for_type(
            "base_correlation_surface",
            all_residuals,
            total_iterations,
        )
        .with_metadata(
            "calibrated_maturities",
            curves_by_maturity.len().to_string(),
        )
        .with_metadata("time_dc", format!("{:?}", self.time_dc));

        Ok((curves_by_maturity, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::CreditIndexData;
    use finstack_core::market_data::term_structures::{
        discount_curve::DiscountCurve, BaseCorrelationCurve,
    };
    // use finstack_core::math::interp::InterpStyle; // not used in this test module
    use std::sync::Arc;
    use time::Month;

    fn create_test_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // Create index hazard curve
        use finstack_core::market_data::term_structures::HazardCurve;
        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.01), (3.0, 0.015), (5.0, 0.02), (10.0, 0.025)])
            .par_spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (10.0, 140.0)])
            .build()
            .expect("HazardCurve builder should succeed with valid test data");

        // Create placeholder base correlation curve
        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![(3.0, 0.30), (10.0, 0.50)])
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        // Create credit index data
        let index_data = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .expect("CreditIndexData builder should succeed with valid test data");

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_credit_index("CDX.NA.IG.42", index_data)
    }

    #[test]
    fn test_base_correlation_calibrator_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
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
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date);

        let tranche = calibrator
            .create_synthetic_tranche(0.0, 3.0, 500.0)
            .expect("Synthetic tranche creation should succeed with valid test data");

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
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        assert_eq!(curve.detachment_points().len(), 3);
        assert_eq!(curve.correlations().len(), 3);

        // Test interpolation
        assert!((curve.correlation(5.0) - 0.35).abs() < 1e-9); // Midpoint between 3% and 7%
    }

    #[test]
    fn test_base_correlation_surface_calibrator() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
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
        use crate::instruments::cds_tranche::pricer::CDSTranchePricer;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        // Create test market context
        let market_ctx = create_test_market_context();

        // Create known base correlation curve for validation
        let known_correlations = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        let known_curve = BaseCorrelationCurve::builder("KNOWN_CORR")
            .points(known_correlations.clone())
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        // Create market context with known correlation curve
        let original_index = market_ctx
            .credit_index("CDX.NA.IG.42")
            .expect("Credit index should exist in test market context");
        let test_index = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(std::sync::Arc::clone(&original_index.index_credit_curve))
            .base_correlation_curve(std::sync::Arc::new(known_curve))
            .build()
            .expect("CreditIndexData builder should succeed with valid test data");

        let test_market_ctx = market_ctx
            .clone()
            .insert_credit_index("CDX.NA.IG.42", test_index);

        // Generate synthetic market quotes using known correlations
        let pricing_model = CDSTranchePricer::new();
        let mut synthetic_quotes = Vec::new();

        for (detach_pct, _corr) in &known_correlations {
            // Create synthetic equity tranche [0, detach_pct]
            let tranche_params = crate::instruments::cds_tranche::parameters::CDSTrancheParams::new(
                "CDX.NA.IG.42",
                42,
                0.0,         // attachment
                *detach_pct, // detachment
                Money::new(10_000_000.0, Currency::USD),
                maturity,
                500.0, // running spread
            );
            let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
            let tranche = CdsTranche::new(
                format!("EQUITY_0_{}", detach_pct),
                &tranche_params,
                &schedule_params,
                finstack_core::types::CurveId::from("USD-OIS"),
                finstack_core::types::CurveId::from("CDX.NA.IG.42"),
                TrancheSide::SellProtection,
            );

            // Price with known correlation to get "market" upfront
            let market_pv = pricing_model
                .price_tranche(&tranche, &test_market_ctx, base_date)
                .expect("Tranche pricing should succeed in test");
            let market_upfront_pct = market_pv.amount() / tranche.notional.amount() * 100.0;

            synthetic_quotes.push(CreditQuote::CDSTranche {
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

        // Create clean market context for calibration (with dummy base correlation curve)
        let original_index = market_ctx
            .credit_index("CDX.NA.IG.42")
            .expect("Credit index should exist in test market context");

        // Create a dummy base correlation curve for initial calibration
        let dummy_base_corr_curve = BaseCorrelationCurve::builder("DUMMY_CALIB_CORR")
            .points(vec![(1.0, 0.01), (100.0, 0.01)]) // Minimal curve for building requirements
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        let clean_index = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(std::sync::Arc::clone(&original_index.index_credit_curve))
            .base_correlation_curve(std::sync::Arc::new(dummy_base_corr_curve))
            .build()
            .expect("CreditIndexData builder should succeed with valid test data");

        let clean_market_ctx = market_ctx
            .clone()
            .insert_credit_index("CDX.NA.IG.42", clean_index);

        let calibration_result = calibrator.calibrate(&synthetic_quotes, &clean_market_ctx);

        assert!(calibration_result.is_ok());
        let (calibrated_curve, report) =
            calibration_result.expect("Calibration should succeed with synthetic quotes");

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
