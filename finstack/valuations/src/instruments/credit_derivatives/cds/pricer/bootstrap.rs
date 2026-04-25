use super::config::CDSPricerConfig;
use super::engine::CDSPricer;
use crate::calibration::api::schema::HazardCurveParams;
use crate::calibration::solver::SolverConfig;
use crate::calibration::targets::hazard::HazardCurveTarget;
use crate::calibration::{CalibrationConfig, CalibrationMethod};
use crate::constants::{credit, numerical, BASIS_POINTS_PER_UNIT};
use crate::instruments::credit_derivatives::cds::{CdsDocClause, CreditDefaultSwap, PayReceive};
use crate::market::conventions::ids::CdsConventionKey;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::ids::{Pillar, QuoteId};
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::currency::Currency;
use finstack_core::dates::{next_cds_date, Date, DateExt, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve, Seniority};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::{Error, Result};
use rust_decimal::Decimal;

/// Configuration for CDS bootstrapping.
///
/// Controls how synthetic CDS instruments are constructed during hazard curve
/// bootstrapping to match market quote conventions.
#[derive(Debug, Clone)]
pub(crate) struct BootstrapConvention {
    /// CDS convention (determines day count, frequency, etc.)
    pub(crate) convention: crate::instruments::credit_derivatives::cds::CDSConvention,
    /// Whether to use IMM dates for maturity (20th of Mar/Jun/Sep/Dec)
    pub(crate) use_imm_dates: bool,
}

impl Default for BootstrapConvention {
    fn default() -> Self {
        Self {
            convention: crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa,
            use_imm_dates: true, // Standard market practice
        }
    }
}

impl BootstrapConvention {
    pub(super) fn representative_convention_key(&self) -> CdsConventionKey {
        match self.convention {
            crate::instruments::credit_derivatives::cds::CDSConvention::IsdaNa => {
                CdsConventionKey {
                    currency: Currency::USD,
                    doc_clause: CdsDocClause::IsdaNa,
                }
            }
            crate::instruments::credit_derivatives::cds::CDSConvention::IsdaEu => {
                CdsConventionKey {
                    currency: Currency::EUR,
                    doc_clause: CdsDocClause::IsdaEu,
                }
            }
            crate::instruments::credit_derivatives::cds::CDSConvention::IsdaAs => {
                CdsConventionKey {
                    currency: Currency::JPY,
                    doc_clause: CdsDocClause::IsdaAs,
                }
            }
            crate::instruments::credit_derivatives::cds::CDSConvention::Custom => {
                CdsConventionKey {
                    currency: Currency::USD,
                    doc_clause: CdsDocClause::Custom,
                }
            }
        }
    }
}

/// Bootstrap hazard rates from CDS spreads to a simple hazard curve
pub(crate) struct CDSBootstrapper {
    pub(super) config: CDSPricerConfig,
    pub(super) convention: BootstrapConvention,
}

impl Default for CDSBootstrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSBootstrapper {
    /// Create new bootstrapper with default config
    pub(crate) fn new() -> Self {
        Self {
            config: CDSPricerConfig::default(),
            convention: BootstrapConvention::default(),
        }
    }

    /// Create bootstrapper with custom convention
    pub(crate) fn with_convention(convention: BootstrapConvention) -> Self {
        Self {
            config: CDSPricerConfig::default(),
            convention,
        }
    }

    /// Create bootstrapper with custom pricer config and convention
    pub(crate) fn with_config(config: CDSPricerConfig, convention: BootstrapConvention) -> Self {
        Self { config, convention }
    }

    /// Bootstrap hazard curve from CDS spreads (tenor years, spread bps)
    ///
    /// This method constructs synthetic CDS instruments for each input tenor/spread
    /// pair and solves for the hazard rate that reproduces the quoted spread.
    ///
    /// # Arguments
    ///
    /// * `cds_spreads` - Slice of (tenor_years, spread_bps) pairs
    /// * `recovery_rate` - Assumed recovery rate for the reference entity
    /// * `disc` - Discount curve for present value calculations
    /// * `base_date` - Valuation date and curve base date
    ///
    /// # IMM Date Handling
    ///
    /// When `use_imm_dates` is true (default), maturities are aligned to the
    /// standard CDS IMM dates (20th of Mar/Jun/Sep/Dec). For example:
    /// - A 5Y CDS quoted on 2024-01-15 would have maturity 2029-03-20
    /// - Premium start is the most recent IMM date (2023-12-20)
    pub(crate) fn bootstrap_hazard_curve(
        &self,
        cds_spreads: &[(f64, f64)],
        recovery_rate: f64,
        disc: &DiscountCurve,
        base_date: Date,
    ) -> Result<HazardCurve> {
        let mut sorted_spreads = cds_spreads.to_vec();
        sorted_spreads.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if sorted_spreads.is_empty() {
            return Err(Error::Input(finstack_core::InputError::TooFewPoints));
        }

        let convention_key = self.convention.representative_convention_key();

        let entity = "BOOTSTRAPPED".to_string();
        let quotes: Vec<MarketQuote> = sorted_spreads
            .iter()
            .map(|(tenor_years, spread_bp)| {
                MarketQuote::Cds(CdsQuote::CdsParSpread {
                    id: QuoteId::new(format!("BOOTSTRAPPED-{tenor_years:.6}")),
                    entity: entity.clone(),
                    convention: convention_key.clone(),
                    pillar: Pillar::Tenor(Tenor::from_years(
                        *tenor_years,
                        self.convention.convention.day_count(),
                    )),
                    spread_bp: *spread_bp,
                    recovery_rate,
                })
            })
            .collect();

        let mut config = CalibrationConfig {
            calibration_method: CalibrationMethod::Bootstrap,
            solver: SolverConfig::brent_default()
                .with_tolerance(self.config.bootstrap_tolerance)
                .with_max_iterations(self.config.bootstrap_max_iterations),
            ..Default::default()
        };
        if sorted_spreads
            .iter()
            .any(|(_, spread_bp)| *spread_bp >= 1_000.0)
        {
            config.hazard_curve.hazard_hard_max = config.hazard_curve.hazard_hard_max.max(100.0);
            config.hazard_curve.validation_tolerance =
                config.hazard_curve.validation_tolerance.max(1e-6);
            config.validation.max_hazard_rate = config.validation.max_hazard_rate.max(2.0);
        }

        let params = HazardCurveParams {
            curve_id: CurveId::from("BOOTSTRAPPED"),
            entity,
            seniority: Seniority::Senior,
            currency: convention_key.currency,
            base_date,
            discount_curve_id: disc.id().clone(),
            recovery_rate,
            notional: 1_000_000.0,
            method: CalibrationMethod::Bootstrap,
            interpolation: Default::default(),
            par_interp: finstack_core::market_data::term_structures::ParInterp::Linear,
            doc_clause: Some(format!("{:?}", convention_key.doc_clause)),
        };

        let base_context = MarketContext::new().insert(disc.clone());
        let (context, _) = HazardCurveTarget::solve(&params, &quotes, &base_context, &config)?;
        Ok(context
            .get_hazard(params.curve_id.as_str())?
            .as_ref()
            .clone())
    }

    fn create_synthetic_cds(
        &self,
        base_date: Date,
        tenor_years: f64,
        spread_bps: f64,
        recovery_rate: f64,
    ) -> Result<CreditDefaultSwap> {
        let spread_bp_decimal = Decimal::try_from(spread_bps).map_err(|e| {
            Error::Validation(format!(
                "spread_bps {} cannot be represented as Decimal: {}",
                spread_bps, e
            ))
        })?;

        // Determine premium start and end dates
        let (start_date, end_date) = if self.convention.use_imm_dates {
            // IMM-aligned dates: maturities on 20th of Mar/Jun/Sep/Dec
            // Premium start is the most recent IMM date on or before base_date
            let prev_imm = self.previous_imm_date(base_date);
            let months = (tenor_years * 12.0).round() as i32;
            // End date is the IMM date approximately `months` months after base_date
            let approx_end = base_date.add_months(months);
            let end_imm = next_cds_date(approx_end);
            (prev_imm, end_imm)
        } else {
            // Non-IMM: simple date arithmetic
            let months = (tenor_years * 12.0).round() as i32;
            let end_date = base_date.add_months(months);
            (base_date, end_date)
        };

        CreditDefaultSwap::new_isda(
            finstack_core::types::InstrumentId::new(format!("SYNTHETIC_{:.1}Y", tenor_years)),
            Money::new(1_000_000.0, Currency::USD),
            PayReceive::PayFixed,
            self.convention.convention,
            spread_bp_decimal,
            start_date,
            end_date,
            recovery_rate,
            finstack_core::types::CurveId::new("DISC"),
            finstack_core::types::CurveId::new("CREDIT"),
        )
    }

    /// Find the most recent IMM date on or before the given date.
    ///
    /// IMM dates are the 20th of Mar, Jun, Sep, Dec.
    fn previous_imm_date(&self, date: Date) -> Date {
        use time::Month;

        let year = date.year();
        let month = date.month();
        let day = date.day();

        // IMM months are Mar(3), Jun(6), Sep(9), Dec(12)
        let month_num: u8 = month.into();

        // Find the current or previous IMM month.
        // For dates within an IMM month but before the 20th, we must fall back
        // to the previous IMM month (e.g., Dec 5 → Sep 20, not Dec 20).
        let (imm_year, imm_month) = if month_num == 12 && day >= 20 {
            // Dec 20 or later -> Dec 20 of this year
            (year, Month::December)
        } else if month_num > 9 || (month_num == 9 && day >= 20) {
            // Sep 20 or later (through Dec 19) -> Sep 20 of this year
            (year, Month::September)
        } else if month_num > 6 || (month_num == 6 && day >= 20) {
            // Jun 20 or later (through Sep 19) -> Jun 20 of this year
            (year, Month::June)
        } else if month_num > 3 || (month_num == 3 && day >= 20) {
            // Mar 20 or later (through Jun 19) -> Mar 20 of this year
            (year, Month::March)
        } else {
            // Before Mar 20 -> Dec 20 of previous year
            (year - 1, Month::December)
        };

        // Return the IMM date (20th of the month)
        Date::from_calendar_date(imm_year, imm_month, 20).unwrap_or(date)
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_for_hazard_rate(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        target_spread_bps: f64,
        pricer: &CDSPricer,
        existing_knots: &[(f64, f64)],
        current_tenor: f64,
        base_date: Date,
    ) -> Result<f64> {
        // Capture the first underlying pricing/curve-construction error so that
        // a solver convergence failure can be reported with its root cause
        // rather than as an opaque "did not converge".
        let last_inner_error: std::cell::RefCell<Option<finstack_core::Error>> =
            std::cell::RefCell::new(None);

        // Objective function: ParSpread(h) - TargetSpread = 0
        // f64::NAN signals an invalid region to Brent's bracketing solver.
        let objective = |h: f64| -> f64 {
            let surv = match self.create_temp_hazard_curve(
                existing_knots,
                current_tenor,
                h,
                base_date,
                cds.protection.recovery_rate,
            ) {
                Ok(curve) => curve,
                Err(e) => {
                    if last_inner_error.borrow().is_none() {
                        *last_inner_error.borrow_mut() = Some(e);
                    }
                    return f64::NAN;
                }
            };
            match pricer.par_spread(cds, disc, &surv, base_date) {
                Ok(spread) => spread - target_spread_bps,
                Err(e) => {
                    if last_inner_error.borrow().is_none() {
                        *last_inner_error.borrow_mut() = Some(e);
                    }
                    f64::NAN
                }
            }
        };

        // Initial guess using credit triangle approximation: h ~ S / (1-R)
        // Or use the last bootstrapped hazard rate if available
        let lgd = (1.0 - cds.protection.recovery_rate).max(numerical::DIVISION_EPSILON);
        let implied_hazard = target_spread_bps / BASIS_POINTS_PER_UNIT / lgd;

        let initial_guess = if let Some(&(_, last_h)) = existing_knots.last() {
            last_h
        } else {
            implied_hazard
        };

        // Adaptive bracket: for distressed credits (high spreads), expand upper bound
        let bracket_min = credit::MIN_HAZARD_RATE;
        let bracket_max = (implied_hazard * credit::HAZARD_RATE_BRACKET_MULTIPLIER)
            .max(credit::DEFAULT_MAX_HAZARD_RATE);

        let solver = BrentSolver {
            tolerance: self.config.bootstrap_tolerance,
            max_iterations: self.config.bootstrap_max_iterations,
            ..Default::default()
        };

        match solver.solve(objective, initial_guess.clamp(bracket_min, bracket_max)) {
            Ok(h) => Ok(h),
            Err(solver_err) => {
                if let Some(inner) = last_inner_error.borrow_mut().take() {
                    Err(finstack_core::Error::Calibration {
                        message: format!(
                            "CDS hazard-rate bootstrap failed at tenor {current_tenor:.4}y \
                             (target spread {target_spread_bps} bp): solver did not converge \
                             ({solver_err}); first underlying error: {inner}"
                        ),
                        category: "cds_hazard_bootstrap".to_string(),
                    })
                } else {
                    Err(finstack_core::Error::Calibration {
                        message: format!(
                            "CDS hazard-rate bootstrap solver did not converge at tenor \
                             {current_tenor:.4}y (target spread {target_spread_bps} bp): \
                             {solver_err}"
                        ),
                        category: "cds_hazard_bootstrap".to_string(),
                    })
                }
            }
        }
    }

    fn create_temp_hazard_curve(
        &self,
        existing_knots: &[(f64, f64)],
        current_tenor: f64,
        current_rate: f64,
        base_date: Date,
        recovery_rate: f64,
    ) -> Result<HazardCurve> {
        let mut knots = existing_knots.to_vec();
        knots.push((current_tenor, current_rate));

        HazardCurve::builder("TEMP")
            .base_date(base_date)
            .recovery_rate(recovery_rate)
            .knots(knots)
            .build()
    }
}
