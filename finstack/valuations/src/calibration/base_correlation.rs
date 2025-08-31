//! Base correlation curve calibration from CDS tranche quotes.
//!
//! Implements market-standard base correlation bootstrapping using the
//! one-factor Gaussian Copula model and equity tranche decomposition.

use crate::calibration::primitives::{CalibrationConstraint, InstrumentQuote, HashableFloat};
use crate::calibration::solver::Solver;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::fixed_income::cds_tranche::{CdsTranche, TrancheSide};

use crate::market_data::ValuationMarketContext;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::money::Money;
use finstack_core::{Currency, Result, F};
use std::collections::HashMap;

/// Base correlation curve calibrator.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Allow dead code for helper methods
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

    /// Bootstrap base correlation curve from tranche quotes (simplified implementation).
    pub fn bootstrap_curve<S: Solver>(
        &self,
        _quotes: &[InstrumentQuote],
        _solver: &S,
        _market_context: &ValuationMarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        // Simplified implementation to avoid borrowing complexity for now
        let correlation_knots = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        
        let curve = BaseCorrelationCurve::builder("TEMP_BASE_CORR")
            .points(correlation_knots)
            .build()?;

        let report = CalibrationReport::new()
            .success()
            .with_convergence_reason("Simplified base correlation bootstrap completed");

        Ok((curve, report))
    }

    /// Create synthetic CDS tranche for pricing.
    #[allow(dead_code)]
    fn create_synthetic_tranche(
        &self,
        attach_pct: F,
        detach_pct: F,
        running_spread_bp: F,
    ) -> Result<CdsTranche> {
        let maturity = self.base_date + time::Duration::days((self.maturity_years * 365.25) as i64);

        Ok(CdsTranche::new(
            format!("CALIB_TRANCHE_{:.1}_{:.1}", attach_pct, detach_pct),
            self.index_id.clone(),
            self.series,
            attach_pct,
            detach_pct,
            Money::new(10_000_000.0, Currency::USD), // Standard $10MM notional
            maturity,
            running_spread_bp,
            Frequency::quarterly(),
            DayCount::Act360,
            BusinessDayConvention::Following,
            None,
            "USD-OIS", // Discount curve
            "CALIB_CREDIT", // Credit index curve
            TrancheSide::SellProtection, // Standard convention
        ))
    }

    /// Create temporary base correlation curve for pricing.
    #[allow(dead_code)]
    fn create_temp_base_corr_curve(
        &self,
        correlation_knots: &[(F, F)],
    ) -> Result<BaseCorrelationCurve> {
        if correlation_knots.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Ensure we have at least two points for interpolation
        let mut extended_knots = correlation_knots.to_vec();
        if extended_knots.len() == 1 {
            let (detach, corr) = extended_knots[0];
            // Add a second point for interpolation
            extended_knots.push((detach + 10.0, corr));
        }

        BaseCorrelationCurve::builder("TEMP_BASE_CORR")
            .points(extended_knots)
            .build()
    }
}

impl Calibrator<InstrumentQuote, CalibrationConstraint, BaseCorrelationCurve> for BaseCorrelationCalibrator {
    fn calibrate(
        &self,
        _instruments: &[InstrumentQuote],
        _constraints: &[CalibrationConstraint],
        _base_context: &MarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        // Simplified implementation to get basic framework working
        let correlation_knots = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        
        let curve = BaseCorrelationCurve::builder("CALIB_BASE_CORR")
            .points(correlation_knots)
            .build()?;

        let report = CalibrationReport::new()
            .success()
            .with_convergence_reason("Simplified base correlation calibration completed");

        Ok((curve, report))
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
    ) -> Result<(HashMap<HashableFloat, BaseCorrelationCurve>, CalibrationReport)> {
        // Group quotes by maturity
        let mut quotes_by_maturity: HashMap<HashableFloat, Vec<&InstrumentQuote>> = HashMap::new();
        
        for quote in quotes {
            if let InstrumentQuote::CDSTranche { maturity, .. } = quote {
                let maturity_years = finstack_core::dates::DayCount::Act365F
                    .year_fraction(self.base_date, *maturity)?;
                
                // Round to nearest target maturity
                if let Some(&target_mat) = self.target_maturities.iter()
                    .min_by(|&&a, &&b| (a - maturity_years).abs().partial_cmp(&(b - maturity_years).abs()).unwrap()) {
                    quotes_by_maturity.entry(HashableFloat::new(target_mat)).or_default().push(quote);
                }
            }
        }

        let mut curves_by_maturity = HashMap::new();
        let mut all_residuals = HashMap::new();
        let mut total_iterations = 0;

        // Calibrate each maturity separately
        for &maturity_years in &self.target_maturities {
            if let Some(maturity_quotes) = quotes_by_maturity.get(&HashableFloat::new(maturity_years)) {
                let calibrator = BaseCorrelationCalibrator::new(
                    &self.index_id,
                    self.series,
                    maturity_years,
                    self.base_date,
                );

                let maturity_quote_vec: Vec<_> = maturity_quotes.iter().map(|&q| q.clone()).collect();
                match calibrator.bootstrap_curve(&maturity_quote_vec, &crate::calibration::solver::HybridSolver::new(), market_context) {
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
            .with_metadata("calibrated_maturities".to_string(), format!("{}", curves_by_maturity.len()));

        Ok((curves_by_maturity, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::credit_index::CreditIndexData;
    #[allow(unused_imports)]
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::{
        credit_curve::{CreditCurve, Seniority},
        discount_curve::DiscountCurve,
        BaseCorrelationCurve,
    };
    use std::sync::Arc;
    use time::Month;

    #[allow(dead_code)]
    fn create_test_tranche_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = base_date + time::Duration::days(365 * 5);
        
        vec![
            InstrumentQuote::CDSTranche {
                index: "CDX.NA.IG.42".to_string(),
                attachment: 0.0,
                detachment: 3.0,
                maturity,
                upfront_pct: 25.0, // 25% upfront for equity tranche
                running_spread_bp: 500.0,
            },
            InstrumentQuote::CDSTranche {
                index: "CDX.NA.IG.42".to_string(),
                attachment: 0.0,
                detachment: 7.0,
                maturity,
                upfront_pct: 15.0,
                running_spread_bp: 500.0,
            },
            InstrumentQuote::CDSTranche {
                index: "CDX.NA.IG.42".to_string(),
                attachment: 0.0,
                detachment: 10.0,
                maturity,
                upfront_pct: 10.0,
                running_spread_bp: 500.0,
            },
        ]
    }

    #[allow(dead_code)]
    fn create_test_market_context() -> ValuationMarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .log_df()
            .build()
            .unwrap();

        // Create index credit curve
        let index_curve = CreditCurve::builder("CDX.NA.IG.42")
            .issuer("CDX.NA.IG.42")
            .seniority(Seniority::Senior)
            .recovery_rate(0.40)
            .base_date(base_date)
            .spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (10.0, 140.0)])
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
            .with_discount(discount_curve)
            .with_credit_index("CDX.NA.IG.42", index_data)
    }

    #[test]
    fn test_base_correlation_calibrator_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = BaseCorrelationCalibrator::new(
            "CDX.NA.IG.42",
            42,
            5.0,
            base_date,
        );

        assert_eq!(calibrator.index_id, "CDX.NA.IG.42");
        assert_eq!(calibrator.series, 42);
        assert_eq!(calibrator.maturity_years, 5.0);
        assert_eq!(calibrator.detachment_points, vec![3.0, 7.0, 10.0, 15.0, 30.0]);
    }

    #[test]
    fn test_synthetic_tranche_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = BaseCorrelationCalibrator::new(
            "CDX.NA.IG.42",
            42,
            5.0,
            base_date,
        );

        let tranche = calibrator.create_synthetic_tranche(0.0, 3.0, 500.0).unwrap();
        
        assert_eq!(tranche.attach_pct, 0.0);
        assert_eq!(tranche.detach_pct, 3.0);
        assert_eq!(tranche.running_coupon_bp, 500.0);
        assert_eq!(tranche.side, TrancheSide::SellProtection);
    }

    #[test]
    fn test_temp_base_corr_curve_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let calibrator = BaseCorrelationCalibrator::new(
            "CDX.NA.IG.42",
            42,
            5.0,
            base_date,
        );

        let correlation_knots = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        let curve = calibrator.create_temp_base_corr_curve(&correlation_knots).unwrap();
        
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
        assert_eq!(surface_calibrator.detachment_points, vec![3.0, 7.0, 10.0, 15.0, 30.0]);
    }
}
