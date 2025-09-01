//! Hazard curve bootstrapping from CDS par spreads.
//!
//! Calibrates a piecewise-constant hazard curve by matching CDS par spreads
//! sequentially across maturities using an objective that drives the CDS NPV
//! to ~0 at the quoted spread.

use crate::calibration::primitives::{CalibrationConstraint, InstrumentQuote};
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::instruments::fixed_income::cds::{cds_pricer::CDSPricer, CDSConvention, CreditDefaultSwap, PayReceive};
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
    /// Calibration configuration
    pub config: CalibrationConfig,
}

impl HazardCurveCalibrator {
    /// Create a new hazard curve calibrator.
    pub fn new(
        entity: impl Into<String>,
        seniority: Seniority,
        recovery_rate: F,
        base_date: finstack_core::dates::Date,
        currency: Currency,
    ) -> Self {
        Self {
            entity: entity.into(),
            seniority,
            recovery_rate,
            base_date,
            currency,
            config: CalibrationConfig::default(),
        }
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
        let mut cds_quotes: Vec<(finstack_core::dates::Date, F)> = quotes
            .iter()
            .filter_map(|q| match q {
                InstrumentQuote::CDS { entity, maturity, spread_bp, .. } if entity == &self.entity => {
                    Some((*maturity, *spread_bp))
                }
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

        for (maturity, market_spread_bp) in &cds_quotes {
            // ISDA time axis
            let tenor_years = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
                self.base_date,
                *maturity,
                CDSConvention::IsdaNa.day_count(),
            );
            if tenor_years <= 0.0 { continue; }

            // Synthetic CDS at market spread
            let cds = CreditDefaultSwap::new_isda(
                format!("CALIB_CDS_{}", maturity),
                Money::new(10_000_000.0, self.currency),
                &self.entity,
                PayReceive::PayProtection,
                CDSConvention::IsdaNa,
                self.base_date,
                *maturity,
                *market_spread_bp,
                "CALIB_HAZARD",
                self.recovery_rate,
                "USD-OIS",
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

                let temp_curve = match temp_curve { Ok(c) => c, Err(_) => return F::INFINITY };
                let disc = match discount_curve_opt { Some(d) => d, None => return F::INFINITY };

                // Objective: PV per $ notional ≈ 0 using quoted spread
                match pricer.npv(&cds, disc, &temp_curve, self.base_date) {
                    Ok(pv) => pv.amount() / cds.notional.amount(),
                    Err(_) => F::INFINITY,
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
            residuals.insert(format!("CDS-{}", maturity), res);
            total_iterations += 1;
        }

        // Build final hazard curve with stable id
        let id_owned = format!("{}-{}", self.entity, self.seniority);
        let id_static: &'static str = Box::leak(id_owned.into_boxed_str());

        let curve = HazardCurve::builder(id_static)
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
            .with_metadata("recovery_rate".to_string(), format!("{:.3}", self.recovery_rate));

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
        let disc = base_context.discount("USD-OIS")?;
        let solver = crate::calibration::solver::HybridSolver::new();
        self.bootstrap_internal(instruments, &solver, Some(disc.as_ref()))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
    use finstack_core::dates::Date;
    use time::Month;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.95), (3.0, 0.90), (5.0, 0.85), (10.0, 0.75)])
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

        let calibrator = HazardCurveCalibrator::new("AAPL", Seniority::Senior, 0.40, base_date, Currency::USD);
        let solver = crate::calibration::solver::HybridSolver::new();
        let (hazard, report) = calibrator
            .bootstrap_curve(&quotes, &solver, &disc)
            .expect("hazard curve calibration failed");
        assert!(report.success);

        // Reprice each quoted CDS and assert PV per $1MM is within $1
        let pricer = CDSPricer::new();
        for q in quotes {
            if let InstrumentQuote::CDS { maturity, spread_bp, .. } = q {
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
                assert!(pv.amount().abs() <= 1.0, "repricing error too large: {}", pv.amount());
            }
        }
    }

    #[test]
    fn hazard_calibration_basic_properties_and_metadata() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let quotes = test_cds_quotes();
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new("AAPL", Seniority::Senior, 0.40, base_date, Currency::USD);
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
        let t1 = dc.year_fraction(base_date, base_date + time::Duration::days(365)).unwrap();
        let t3 = dc.year_fraction(base_date, base_date + time::Duration::days(365 * 3)).unwrap();
        let t5 = dc.year_fraction(base_date, base_date + time::Duration::days(365 * 5)).unwrap();
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
        let calibrator = HazardCurveCalibrator::new("AAPL", Seniority::Senior, 0.40, base_date, Currency::USD);
        let solver = crate::calibration::solver::HybridSolver::new();
        let empty: Vec<InstrumentQuote> = vec![];
        let res = calibrator.bootstrap_curve(&empty, &solver, &disc);
        assert!(res.is_err());
    }
}

