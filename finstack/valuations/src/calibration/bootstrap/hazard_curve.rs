//! Hazard curve bootstrapping from CDS par spreads.
//!
//! Calibrates a piecewise-constant hazard curve by matching CDS par spreads
//! sequentially across maturities using an objective that drives the CDS NPV
//! to ~0 at the quoted spread.

use crate::calibration::primitives::{CalibrationConstraint, InstrumentQuote};
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::fixed_income::cds::{
    cds_pricer::CDSPricer, CDSConvention, CreditDefaultSwap, PayReceive,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::hazard_curve::{HazardCurve, Seniority};
use finstack_core::market_data::traits::Discount;
use finstack_core::money::Money;
use finstack_core::{Currency, Result, F};
use std::collections::HashMap;

/// Hazard curve bootstrapper using CDS par spreads.
#[derive(Clone, Debug)]
pub struct HazardCurveCalibrator {
    /// Reference entity name
    pub entity: String,
    /// Seniority level (metadata)
    pub seniority: Seniority,
    /// Recovery rate assumption
    pub recovery_rate: F,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Currency (metadata)
    pub currency: Currency,
    /// Discount curve identifier for collateral discounting
    pub discount_curve_id: String,
    /// Calibration configuration
    pub config: CalibrationConfig,
}

impl HazardCurveCalibrator {
    /// Helper to determine default discount curve ID from currency.
    /// Uses common market conventions for collateral.
    pub fn default_discount_curve_id(currency: Currency) -> String {
        match currency {
            Currency::USD => "USD-OIS".to_string(),
            Currency::EUR => "EUR-OIS".to_string(), 
            Currency::GBP => "GBP-OIS".to_string(),
            Currency::JPY => "JPY-OIS".to_string(),
            Currency::CHF => "CHF-OIS".to_string(),
            Currency::CAD => "CAD-OIS".to_string(),
            Currency::AUD => "AUD-OIS".to_string(),
            Currency::SEK => "SEK-OIS".to_string(),
            Currency::NOK => "NOK-OIS".to_string(),
            Currency::DKK => "DKK-OIS".to_string(),
            _ => format!("{}-OIS", currency),
        }
    }

    /// Create a new hazard curve calibrator.
    pub fn new(
        entity: impl Into<String>,
        seniority: Seniority,
        recovery_rate: F,
        base_date: finstack_core::dates::Date,
        currency: Currency,
        discount_curve_id: impl Into<String>,
    ) -> Self {
        Self {
            entity: entity.into(),
            seniority,
            recovery_rate,
            base_date,
            currency,
            discount_curve_id: discount_curve_id.into(),
            config: CalibrationConfig::default(),
        }
    }

    /// Create a new hazard curve calibrator using default discount curve ID.
    /// This is a convenience method that uses standard OIS curves based on currency.
    pub fn new_with_default_discount(
        entity: impl Into<String>,
        seniority: Seniority,
        recovery_rate: F,
        base_date: finstack_core::dates::Date,
        currency: Currency,
    ) -> Self {
        let discount_curve_id = Self::default_discount_curve_id(currency);
        Self::new(entity, seniority, recovery_rate, base_date, currency, discount_curve_id)
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    fn bootstrap_internal<S: crate::calibration::solver::Solver>(
        &self,
        quotes: &[InstrumentQuote],
        solver: &S,
        discount_curve_opt: Option<&dyn Discount>,
    ) -> Result<(HazardCurve, CalibrationReport)> {
        // Extract CDS quotes for this entity and sort by maturity
        let mut cds_quotes: Vec<(finstack_core::dates::Date, F, Option<F>)> = quotes
            .iter()
            .filter_map(|q| match q {
                InstrumentQuote::CDS {
                    entity,
                    maturity,
                    spread_bp,
                    ..
                } if entity == &self.entity => Some((*maturity, *spread_bp, None)),
                InstrumentQuote::CDSUpfront {
                    entity,
                    maturity,
                    upfront_pct,
                    running_spread_bp,
                    ..
                } if entity == &self.entity => Some((*maturity, *running_spread_bp, Some(*upfront_pct))),
                _ => None,
            })
            .collect();

        if cds_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        cds_quotes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Sequentially solve hazards per tenor to match market PV≈0
        let mut hazard_knots: Vec<(F, F)> = Vec::new();
        let mut par_knots: Vec<(F, F)> = Vec::new();
        let mut residuals: HashMap<String, F> = HashMap::new();
        let mut total_iterations: usize = 0;

        for (maturity, market_spread_bp, upfront_pct_opt) in &cds_quotes {
            // ISDA time axis
            let tenor_years = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
                self.base_date,
                *maturity,
                CDSConvention::IsdaNa.day_count(),
            );
            if tenor_years <= 0.0 {
                continue;
            }

            // Synthetic CDS at market spread
            // Create CDS constants for static lifetime requirements
            const CALIB_HAZARD_ID: &str = "CALIB_HAZARD";
            const CALIB_DISC_ID: &str = "CALIB_DISC";
            
            let cds = CreditDefaultSwap::new_isda(
                format!("CALIB_CDS_{}", maturity),
                Money::new(10_000_000.0, self.currency),
                &self.entity,
                PayReceive::PayProtection,
                CDSConvention::IsdaNa,
                self.base_date,
                *maturity,
                *market_spread_bp,
                CALIB_HAZARD_ID,
                self.recovery_rate,
                CALIB_DISC_ID,
            );

            let pricer = CDSPricer::new();
            let hazard_so_far = hazard_knots.clone();

            let objective = |trial_lambda: F| -> F {
                // Build temporary hazard curve with prior segments + trial point
                let mut temp_knots = hazard_so_far.clone();
                temp_knots.push((tenor_years, trial_lambda.max(0.0)));

                let temp_curve = HazardCurve::builder("TEMP_CALIB")
                    .base_date(self.base_date)
                    .day_count(CDSConvention::IsdaNa.day_count())
                    .recovery_rate(self.recovery_rate)
                    .knots(temp_knots)
                    .build();

                let temp_curve = match temp_curve {
                    Ok(c) => c,
                    Err(_) => return F::INFINITY,
                };
                let disc = match discount_curve_opt {
                    Some(d) => d,
                    None => return F::INFINITY,
                };

                // Calculate CDS NPV
                let npv_result = pricer.npv(&cds, disc, &temp_curve, self.base_date);
                let npv = match npv_result {
                    Ok(pv) => pv.amount(),
                    Err(_) => return F::INFINITY,
                };

                // Objective depends on quote type
                match upfront_pct_opt {
                    None => {
                        // Par spread quote: PV per $ notional ≈ 0 using quoted spread
                        npv / cds.notional.amount()
                    }
                    Some(upfront_pct) => {
                        // Upfront quote: PV should equal upfront payment
                        let expected_upfront = cds.notional.amount() * upfront_pct / 100.0;
                        (npv - expected_upfront) / cds.notional.amount()
                    }
                }
            };

            // Initial guess: last solved λ or s/(1-R)
            let initial_guess = hazard_knots
                .last()
                .map(|&(_, l)| l)
                .unwrap_or(*market_spread_bp / 10000.0 / (1.0 - self.recovery_rate));

            let solved = solver.solve(objective, initial_guess)?;
            hazard_knots.push((tenor_years, solved.max(0.0)));
            par_knots.push((tenor_years, *market_spread_bp));

            let res = objective(solved).abs();
            let key = match upfront_pct_opt {
                None => format!("CDS-PAR-{}", maturity),
                Some(_) => format!("CDS-UPFRONT-{}", maturity),
            };
            residuals.insert(key, res);
            total_iterations += 1;
        }

        // Build final hazard curve with stable id
        let id_owned = format!("{}-{}", self.entity, self.seniority);

        let curve = HazardCurve::builder(id_owned)
            .issuer(&self.entity)
            .seniority(self.seniority)
            .currency(self.currency)
            .recovery_rate(self.recovery_rate)
            .day_count(CDSConvention::IsdaNa.day_count())
            .base_date(self.base_date)
            .knots(hazard_knots)
            .par_spreads(par_knots)
            .build()?;

        let report = CalibrationReport::new()
            .success()
            .with_residuals(residuals)
            .with_iterations(total_iterations)
            .with_convergence_reason("Hazard curve bootstrap completed")
            .with_metadata("entity".to_string(), self.entity.clone())
            .with_metadata(
                "recovery_rate".to_string(),
                format!("{:.3}", self.recovery_rate),
            );

        Ok((curve, report))
    }
}

impl HazardCurveCalibrator {
    /// Bootstrap API used in tests and examples (explicit solver + discount curve).
    pub fn bootstrap_curve<S: crate::calibration::solver::Solver>(
        &self,
        quotes: &[InstrumentQuote],
        solver: &S,
        discount_curve: &dyn finstack_core::market_data::traits::Discount,
    ) -> Result<(HazardCurve, CalibrationReport)> {
        self.bootstrap_internal(quotes, solver, Some(discount_curve))
    }
}

impl Calibrator<InstrumentQuote, CalibrationConstraint, HazardCurve> for HazardCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[InstrumentQuote],
        _constraints: &[CalibrationConstraint],
        base_context: &MarketContext,
    ) -> Result<(HazardCurve, CalibrationReport)> {
        let disc = base_context.discount(&self.discount_curve_id)?;
        let solver = crate::calibration::solver::HybridSolver::new();
        self.bootstrap_internal(instruments, &solver, Some(disc.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
    use time::Month;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .knots([
                (0.0, 1.0),
                (1.0, 0.95),
                (3.0, 0.90),
                (5.0, 0.85),
                (10.0, 0.75),
            ])
            .build()
            .unwrap()
    }

    fn test_cds_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        vec![
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365),
                spread_bp: 50.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 3),
                spread_bp: 75.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 100.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
        ]
    }

    #[test]
    fn hazard_calibration_reprices_cds_within_one_dollar_per_million() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let quotes = test_cds_quotes();
        let disc = test_discount_curve();

        let calibrator =
            HazardCurveCalibrator::new("AAPL", Seniority::Senior, 0.40, base_date, Currency::USD, "USD-OIS");
        let solver = crate::calibration::solver::HybridSolver::new();
        let (hazard, report) = calibrator
            .bootstrap_curve(&quotes, &solver, &disc)
            .expect("hazard curve calibration failed");
        assert!(report.success);

        // Reprice each quoted CDS and assert PV per $1MM is within $1
        let pricer = CDSPricer::new();
        for q in quotes {
            if let InstrumentQuote::CDS {
                maturity,
                spread_bp,
                ..
            } = q
            {
                let cds = CreditDefaultSwap::new_isda(
                    format!("CDS-{}", maturity),
                    Money::new(1_000_000.0, Currency::USD),
                    "AAPL",
                    PayReceive::PayProtection,
                    CDSConvention::IsdaNa,
                    base_date,
                    maturity,
                    spread_bp,
                    "AAPL-Senior",
                    0.40,
                    "USD-OIS",
                );

                let pv = pricer
                    .npv(&cds, &disc, &hazard, base_date)
                    .expect("cds npv failed");
                assert!(
                    pv.amount().abs() <= 1.0,
                    "repricing error too large: {}",
                    pv.amount()
                );
            }
        }
    }

    #[test]
    fn hazard_calibration_basic_properties_and_metadata() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let quotes = test_cds_quotes();
        let disc = test_discount_curve();

        let calibrator =
            HazardCurveCalibrator::new("AAPL", Seniority::Senior, 0.40, base_date, Currency::USD, "USD-OIS");
        let solver = crate::calibration::solver::HybridSolver::new();
        let (hazard, report) = calibrator
            .bootstrap_curve(&quotes, &solver, &disc)
            .expect("hazard curve calibration failed");
        assert!(report.success);

        // Day count and recovery metadata
        assert_eq!(hazard.day_count(), CDSConvention::IsdaNa.day_count());
        assert!((hazard.recovery_rate() - 0.40).abs() < 1e-12);
        assert_eq!(hazard.base_date(), base_date);
        assert_eq!(hazard.seniority, Some(Seniority::Senior));

        // Tenors strictly increasing and lambdas non-negative (check survival monotonicity)
        let times = [1.0, 3.0, 5.0];
        let mut prev_sp = 1.0;
        for &t in &times {
            let sp = hazard.sp(t);
            assert!(sp <= prev_sp + 1e-12);
            prev_sp = sp;
        }

        // Par spread retrieval at pillar times (use same day-count mapping as bootstrap)
        let dc = hazard.day_count();
        let t1 = dc
            .year_fraction(base_date, base_date + time::Duration::days(365))
            .unwrap();
        let t3 = dc
            .year_fraction(base_date, base_date + time::Duration::days(365 * 3))
            .unwrap();
        let t5 = dc
            .year_fraction(base_date, base_date + time::Duration::days(365 * 5))
            .unwrap();
        assert!((hazard.quoted_spread_bp(t1, ParInterp::Linear) - 50.0).abs() < 1e-6);
        assert!((hazard.quoted_spread_bp(t3, ParInterp::Linear) - 75.0).abs() < 1e-6);
        assert!((hazard.quoted_spread_bp(t5, ParInterp::Linear) - 100.0).abs() < 1e-6);
        // Interpolated in-between (between 1Y and 3Y)
        let mid = hazard.quoted_spread_bp((t1 + t3) * 0.5, ParInterp::Linear);
        assert!(mid > 50.0 && mid < 75.0);

        // Residuals are small
        for v in report.residuals.values() {
            assert!(v.abs() <= 1e-6);
        }
    }

    #[test]
    fn hazard_calibration_errors_on_empty_quotes() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let disc = test_discount_curve();
        let calibrator =
            HazardCurveCalibrator::new("AAPL", Seniority::Senior, 0.40, base_date, Currency::USD, "USD-OIS");
        let solver = crate::calibration::solver::HybridSolver::new();
        let empty: Vec<InstrumentQuote> = vec![];
        let res = calibrator.bootstrap_curve(&empty, &solver, &disc);
        assert!(res.is_err());
    }

    #[test]
    fn test_upfront_cds_quote_support() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let disc = test_discount_curve();

        // Test with upfront quote
        let upfront_quote = vec![
            InstrumentQuote::CDSUpfront {
                entity: "DISTRESSED".to_string(),
                maturity: base_date + time::Duration::days(365),
                upfront_pct: 5.0, // 5% upfront
                running_spread_bp: 300.0, // 300bp running
                recovery_rate: 0.25, // Lower recovery for distressed
                currency: Currency::USD,
            }
        ];

        let calibrator = HazardCurveCalibrator::new(
            "DISTRESSED", 
            Seniority::Senior, 
            0.25, 
            base_date, 
            Currency::USD, 
            "USD-OIS"
        );
        let solver = crate::calibration::solver::HybridSolver::new();
        let result = calibrator.bootstrap_curve(&upfront_quote, &solver, &disc);
        
        // Should succeed and handle upfront quote properly
        assert!(result.is_ok());
        let (_curve, report) = result.unwrap();
        assert!(report.success);
        
        // Check that residual key indicates upfront quote
        let upfront_residual_key = format!("CDS-UPFRONT-{}", base_date + time::Duration::days(365));
        assert!(report.residuals.contains_key(&upfront_residual_key));
    }

    #[test]
    fn test_default_discount_curve_id_helpers() {
        // Test currency-based discount curve ID generation
        assert_eq!(HazardCurveCalibrator::default_discount_curve_id(Currency::USD), "USD-OIS");
        assert_eq!(HazardCurveCalibrator::default_discount_curve_id(Currency::EUR), "EUR-OIS");
        assert_eq!(HazardCurveCalibrator::default_discount_curve_id(Currency::GBP), "GBP-OIS");
        
        // Test convenience constructor
        let calibrator = HazardCurveCalibrator::new_with_default_discount(
            "TEST", 
            Seniority::Senior, 
            0.40, 
            Date::from_calendar_date(2025, Month::January, 1).unwrap(), 
            Currency::JPY
        );
        assert_eq!(calibrator.discount_curve_id, "JPY-OIS");
    }
}
