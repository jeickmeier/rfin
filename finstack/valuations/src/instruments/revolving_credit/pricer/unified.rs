//! Unified pricing engine for revolving credit facilities.
//!
//! Provides a single pricer that handles both deterministic and stochastic modes:
//! - **Deterministic**: Prices using pre-defined draw/repay events
//! - **Stochastic**: Generates 3-factor MC paths and prices each path deterministically
//!
//! # Architecture
//!
//! Stochastic pricing is implemented as averaging many deterministic path pricings,
//! ensuring consistency between modes and enabling full path capture for distribution analysis.

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::builder::CashFlowSchedule;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::results::{MoneyEstimate, MonteCarloResult};
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;

use super::super::cashflow_engine::{
    CashflowEngine, PathAwareCashflowSchedule, ThreeFactorPathData,
};
use super::super::types::{DrawRepaySpec, RevolvingCredit};
use super::components::compute_upfront_fee_pv;

#[cfg(feature = "mc")]
use super::path_generator::generate_three_factor_paths;

/// Result for a single path valuation.
///
/// Contains the present value, optional 3-factor path data, and the detailed cashflow schedule.
#[derive(Clone, Debug)]
pub struct PathResult {
    /// Present value for this path
    pub pv: Money,
    /// 3-factor path data (if from MC)
    pub path_data: Option<ThreeFactorPathData>,
    /// Cashflow schedule for this path
    pub cashflows: CashFlowSchedule,
}

/// Enhanced Monte Carlo results with full path details.
///
/// Extends the standard `MonteCarloResult` with individual path results
/// for distribution analysis and visualization.
#[cfg(feature = "mc")]
#[derive(Debug)]
pub struct EnhancedMonteCarloResult {
    /// Standard MC statistics (mean, std error, CI)
    pub mc_result: MonteCarloResult,
    /// Individual path results for distribution analysis
    pub path_results: Vec<PathResult>,
}

/// Unified pricer for revolving credit facilities.
///
/// Handles both deterministic and stochastic pricing using a single implementation.
/// Stochastic pricing generates paths and applies deterministic pricing to each path.
pub struct RevolvingCreditPricer {
    model: ModelKey,
}

impl Default for RevolvingCreditPricer {
    fn default() -> Self {
        Self {
            model: ModelKey::Discounting,
        }
    }
}

impl RevolvingCreditPricer {
    /// Create a new pricer instance with specified model.
    pub fn new(model: ModelKey) -> Self {
        Self { model }
    }
    /// Price a single path (deterministic or from MC).
    ///
    /// This is the core pricing logic used for both modes:
    /// - Discounts all cashflows
    /// - Applies survival weighting (static from hazard curve or dynamic from path)
    /// - Adds upfront fee PV
    ///
    /// # Arguments
    ///
    /// * `facility` - The revolving credit facility
    /// * `market` - Market context with curves
    /// * `as_of` - Valuation date
    /// * `path_schedule` - Cashflow schedule with optional path data
    ///
    /// # Returns
    ///
    /// A `PathResult` with PV, cashflows, and path data
    pub fn price_single_path(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: Date,
        path_schedule: &PathAwareCashflowSchedule,
    ) -> Result<PathResult> {
        let disc_curve = market.get_discount_ref(&facility.discount_curve_id)?;
        let disc_dc = disc_curve.day_count();

        // Compute survival probabilities
        let survival_probs = if let Some(ref path_data) = path_schedule.path_data {
            // Dynamic survival from credit spread path
            // Need to compute survival at each cashflow date, not just time points
            let cashflow_dates: Vec<Date> = path_schedule
                .schedule
                .flows
                .iter()
                .map(|cf| cf.date)
                .collect();
            Self::compute_dynamic_survival_at_dates(
                &path_data.credit_spread_path,
                &path_data.time_points,
                &path_data.payment_dates,
                &cashflow_dates,
                facility.recovery_rate,
                facility.commitment_date,
                facility.day_count,
            )?
        } else if let Some(ref hazard_id) = facility.hazard_curve_id {
            // Static survival from hazard curve
            let hazard = market.get_hazard_ref(hazard_id.as_str())?;
            hazard.survival_at_dates(
                &path_schedule
                    .schedule
                    .flows
                    .iter()
                    .map(|cf| cf.date)
                    .collect::<Vec<_>>(),
            )?
        } else {
            // No credit risk
            vec![1.0; path_schedule.schedule.flows.len()]
        };

        // Discount cashflows with survival weighting
        let mut total_pv = 0.0;
        let base_date = disc_curve.base_date();
        for (i, cf) in path_schedule.schedule.flows.iter().enumerate() {
            let t = disc_dc.year_fraction(
                base_date,
                cf.date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let df = disc_curve.df(t);
            let survival = survival_probs.get(i).copied().unwrap_or(1.0);
            total_pv += cf.amount.amount() * df * survival;
        }

        // Add Recovery Leg PV (Recovery Value upon default)
        // PV_rec = Sum [ Exposure(t) * RecoveryRate * DF(t) * ProbDefault(t-1, t) ]
        if facility.recovery_rate > 0.0 {
            // Determine grid points (payment dates) for integration
            let grid_dates = if let Some(ref path_data) = path_schedule.path_data {
                path_data.payment_dates.clone()
            } else {
                super::super::utils::build_payment_dates(facility, false)?
            };

            // Filter grid dates to future only
            let future_grid: Vec<Date> = grid_dates.into_iter().filter(|&d| d > as_of).collect();

            if !future_grid.is_empty() {
                // Compute survival at grid dates
                let survival_at_grid = if let Some(ref path_data) = path_schedule.path_data {
                    Self::compute_dynamic_survival_at_dates(
                        &path_data.credit_spread_path,
                        &path_data.time_points,
                        &path_data.payment_dates,
                        &future_grid,
                        facility.recovery_rate,
                        facility.commitment_date,
                        facility.day_count,
                    )?
                } else if let Some(ref hazard_id) = facility.hazard_curve_id {
                    let hazard = market.get_hazard_ref(hazard_id.as_str())?;
                    hazard.survival_at_dates(&future_grid)?
                } else {
                    vec![1.0; future_grid.len()]
                };

                // Compute Exposure at grid dates
                let exposure_at_grid = if let Some(ref path_data) = path_schedule.path_data {
                    // Optimized lookup: grid_dates came from path_data.payment_dates
                    // so we just need to align them.
                    // Since future_grid is just path_data.payment_dates filtered by > as_of,
                    // we can zip and filter.
                    path_data
                        .payment_dates
                        .iter()
                        .zip(path_data.utilization_path.iter())
                        .filter_map(|(date, util)| {
                            if *date > as_of {
                                Some(util * facility.commitment_amount.amount())
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    // For deterministic, simulate balance evolution
                    let mut exposures = Vec::with_capacity(future_grid.len());
                    for &date in &future_grid {
                        let bal = super::super::cashflow_engine::calculate_drawn_balance_at_date(
                            facility, date,
                        )?;
                        exposures.push(bal.amount());
                    }
                    exposures
                };

                // Get initial state at as_of
                let mut prev_sp = if let Some(ref path_data) = path_schedule.path_data {
                    Self::compute_dynamic_survival_at_dates(
                        &path_data.credit_spread_path,
                        &path_data.time_points,
                        &path_data.payment_dates,
                        &[as_of],
                        facility.recovery_rate,
                        facility.commitment_date,
                        facility.day_count,
                    )?[0]
                } else if let Some(ref hazard_id) = facility.hazard_curve_id {
                    let hazard = market.get_hazard_ref(hazard_id.as_str())?;
                    let t = hazard.day_count().year_fraction(
                        hazard.base_date(),
                        as_of,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    hazard.sp(t)
                } else {
                    1.0
                };

                let mut prev_exposure = if let Some(ref _path_data) = path_schedule.path_data {
                    // For stochastic start, assume as_of matches path start or use initial drawn
                    if as_of <= facility.commitment_date {
                        facility.drawn_amount.amount()
                    } else {
                        // If pricing mid-path, technically need path state at as_of.
                        // For simplicity/performance in this fix, use initial drawn if before path start,
                        // or interpolate if possible.
                        // Given path_data covers the whole life, we can look up.
                        // Simplification: Use first grid point's exposure or current drawn
                        facility.drawn_amount.amount()
                    }
                } else {
                    super::super::cashflow_engine::calculate_drawn_balance_at_date(facility, as_of)?
                        .amount()
                };

                // Integrate over intervals using ISDA-style trapezoidal integration
                let mut prev_date = as_of;
                for i in 0..future_grid.len() {
                    let curr_date = future_grid[i];
                    let curr_sp = survival_at_grid[i];
                    let curr_exposure = exposure_at_grid[i];

                    let prob_default = (prev_sp - curr_sp).max(0.0);

                    // Compute discount factors at interval endpoints
                    let t_prev = disc_dc
                        .year_fraction(
                            disc_curve.base_date(),
                            prev_date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);

                    let t_curr = disc_dc
                        .year_fraction(
                            disc_curve.base_date(),
                            curr_date,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(0.0);

                    // Use average of endpoint discount factors (ISDA CDS standard)
                    // This is more accurate than using the midpoint time's DF,
                    // especially for steep discount curves and longer intervals.
                    let df_prev = disc_curve.df(t_prev);
                    let df_curr = disc_curve.df(t_curr);
                    let df_avg = (df_prev + df_curr) / 2.0;

                    // Exposure Average (trapezoidal)
                    let exposure_avg = (prev_exposure + curr_exposure) / 2.0;

                    let recovery_flow =
                        exposure_avg * facility.recovery_rate * df_avg * prob_default;
                    total_pv += recovery_flow;

                    prev_sp = curr_sp;
                    prev_exposure = curr_exposure;
                    prev_date = curr_date;
                }
            }
        }

        // Add upfront fee if applicable
        if let Some(upfront) = facility.fees.upfront_fee {
            total_pv += compute_upfront_fee_pv(
                Some(upfront),
                facility.commitment_date,
                as_of,
                disc_curve as &dyn Discounting,
                disc_dc,
            )?;
        }

        Ok(PathResult {
            pv: Money::new(total_pv, facility.commitment_amount.currency()),
            path_data: path_schedule.path_data.clone(),
            cashflows: path_schedule.schedule.clone(),
        })
    }

    /// Main pricing entry point.
    ///
    /// Automatically dispatches to deterministic or stochastic pricing based on
    /// the facility's `draw_repay_spec`.
    ///
    /// # Arguments
    ///
    /// * `facility` - The revolving credit facility
    /// * `market` - Market context with curves
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value as `Money`
    pub fn price(facility: &RevolvingCredit, market: &MarketContext, as_of: Date) -> Result<Money> {
        match &facility.draw_repay_spec {
            DrawRepaySpec::Deterministic(_) => {
                // Single deterministic path
                let engine = CashflowEngine::new(facility, Some(market), as_of)?;
                let schedule = engine.generate_deterministic()?;
                let result = Self::price_single_path(facility, market, as_of, &schedule)?;
                Ok(result.pv)
            }
            DrawRepaySpec::Stochastic(_) => {
                #[cfg(feature = "mc")]
                {
                    let enhanced = Self::price_monte_carlo(facility, market, as_of)?;
                    Ok(enhanced.mc_result.estimate.mean)
                }
                #[cfg(not(feature = "mc"))]
                {
                    Err(finstack_core::Error::Validation(
                        "MC feature required for stochastic pricing".to_string(),
                    ))
                }
            }
        }
    }

    /// Price deterministically (explicit method for API clarity).
    ///
    /// This is the same as calling `price()` with a deterministic facility,
    /// but provides an explicit API for callers who know they have a deterministic spec.
    pub fn price_deterministic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let engine = CashflowEngine::new(facility, Some(market), as_of)?;
        let schedule = engine.generate_deterministic()?;
        let result = Self::price_single_path(facility, market, as_of, &schedule)?;
        Ok(result.pv)
    }

    /// Price with full MC path capture for analysis.
    ///
    /// Returns detailed results including all individual path PVs, cashflows,
    /// and trajectories for distribution analysis.
    ///
    /// # Arguments
    ///
    /// * `facility` - The revolving credit facility (must have stochastic spec)
    /// * `market` - Market context with curves
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Enhanced Monte Carlo result with full path details
    #[cfg(feature = "mc")]
    pub fn price_with_paths(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<EnhancedMonteCarloResult> {
        match &facility.draw_repay_spec {
            DrawRepaySpec::Stochastic(_) => Self::price_monte_carlo(facility, market, as_of),
            _ => Err(finstack_core::Error::Validation(
                "Path capture requires stochastic spec".into(),
            )),
        }
    }

    /// Internal MC pricing with 3-factor path generation and aggregation.
    ///
    /// This method:
    /// 1. Generates 3-factor MC paths (utilization, rate, spread)
    /// 2. Generates cashflows for each path
    /// 3. Prices each path deterministically
    /// 4. Computes MC statistics across all paths
    #[cfg(feature = "mc")]
    fn price_monte_carlo(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<EnhancedMonteCarloResult> {
        // Extract stochastic spec
        let stoch_spec = match &facility.draw_repay_spec {
            DrawRepaySpec::Stochastic(spec) => spec.as_ref(),
            _ => {
                return Err(finstack_core::Error::Validation(
                    "Stochastic spec required for MC pricing".to_string(),
                ))
            }
        };

        // Get or synthesize MC config
        use super::super::types::{CreditSpreadProcessSpec, McConfig};
        let mc_config_to_use;
        let mc_config = if let Some(ref mc_config) = stoch_spec.mc_config {
            mc_config.validate()?;
            mc_config
        } else {
            // Synthesize minimal McConfig
            // If facility has hazard curve, use market-anchored process; otherwise constant zero
            let credit_process = if let Some(ref hazard_id) = facility.hazard_curve_id {
                CreditSpreadProcessSpec::MarketAnchored {
                    hazard_curve_id: hazard_id.clone(),
                    kappa: 0.1,
                    implied_vol: 1e-10, // Minimal volatility for deterministic behavior
                    tenor_years: None,
                }
            } else {
                CreditSpreadProcessSpec::Constant(0.0)
            };

            mc_config_to_use = McConfig {
                correlation_matrix: None,
                recovery_rate: facility.recovery_rate,
                credit_spread_process: credit_process,
                interest_rate_process: None,
                util_credit_corr: None,
            };
            mc_config_to_use.validate()?;
            &mc_config_to_use
        };

        // Generate cashflow engine
        let engine = CashflowEngine::new(facility, Some(market), as_of)?;
        let payment_dates = super::super::utils::build_payment_dates(facility, false)?;

        // Generate 3-factor paths
        let paths =
            generate_three_factor_paths(stoch_spec, mc_config, facility, market, &payment_dates)?;

        // Price each path
        let mut path_results = Vec::with_capacity(paths.len());
        for path_data in paths {
            let schedule = engine.generate_stochastic_path(path_data)?;
            let result = Self::price_single_path(facility, market, as_of, &schedule)?;
            path_results.push(result);
        }

        // Compute MC statistics using Bessel-corrected variance (N-1 denominator)
        // for unbiased standard error estimation
        let pvs: Vec<f64> = path_results.iter().map(|r| r.pv.amount()).collect();
        let n = pvs.len() as f64;
        let mean = pvs.iter().sum::<f64>() / n;

        // Use N-1 for unbiased variance estimation (Bessel's correction)
        let variance = if pvs.len() > 1 {
            pvs.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)
        } else {
            0.0 // Single path case
        };
        let stderr = (variance / n).sqrt();

        // Compute 95% confidence interval (assuming asymptotic normality via CLT)
        let z_95 = 1.96;
        let ci_low = mean - z_95 * stderr;
        let ci_high = mean + z_95 * stderr;

        let estimate = MoneyEstimate {
            mean: Money::new(mean, facility.commitment_amount.currency()),
            stderr,
            ci_95: (
                Money::new(ci_low, facility.commitment_amount.currency()),
                Money::new(ci_high, facility.commitment_amount.currency()),
            ),
            num_paths: pvs.len(),
        };

        Ok(EnhancedMonteCarloResult {
            mc_result: MonteCarloResult {
                estimate,
                paths: None,
            },
            path_results,
        })
    }

    /// Compute dynamic survival probabilities at arbitrary cashflow dates.
    ///
    /// Interpolates survival from the credit spread path to match cashflow dates.
    ///
    /// Uses the relation: hazard_rate = credit_spread / (1 - recovery_rate)
    ///
    /// # Arguments
    ///
    /// * `credit_spreads` - Credit spread values at each time point
    /// * `time_points` - Time grid in years from commitment date
    /// * `_payment_dates` - Payment dates (unused but kept for API compatibility)
    /// * `cashflow_dates` - Dates at which to compute survival probabilities
    /// * `recovery_rate` - Recovery rate for hazard-to-spread mapping
    /// * `commitment_date` - Facility commitment date
    /// * `day_count` - Optional day count convention (defaults to Act365F if None)
    fn compute_dynamic_survival_at_dates(
        credit_spreads: &[f64],
        time_points: &[f64],
        _payment_dates: &[Date],
        cashflow_dates: &[Date],
        recovery_rate: f64,
        commitment_date: Date,
        day_count: DayCount,
    ) -> Result<Vec<f64>> {
        use finstack_core::dates::DayCountCtx;
        // Use facility day count for consistency with path generation
        let dc = day_count;

        // First, compute cumulative hazard at each payment date
        let mut cumulative_hazards = Vec::with_capacity(time_points.len());
        let mut cumulative_hazard = 0.0;
        cumulative_hazards.push(0.0); // At commitment date

        for i in 0..(credit_spreads.len() - 1) {
            let dt = time_points[i + 1] - time_points[i];
            let hazard_rate = credit_spreads[i] / (1.0 - recovery_rate).max(1e-6);
            cumulative_hazard += hazard_rate * dt;
            cumulative_hazards.push(cumulative_hazard);
        }

        // Now interpolate survival for each cashflow date
        let mut survival_probs = Vec::with_capacity(cashflow_dates.len());
        for &cf_date in cashflow_dates {
            // Find the interval containing cf_date
            let t_cf = dc.year_fraction(commitment_date, cf_date, DayCountCtx::default())?;

            // Find the bracketing payment dates
            let hazard_at_cf = if let Some(idx) = time_points.iter().position(|&t| t >= t_cf) {
                if idx == 0
                    || (time_points[idx] - t_cf).abs() < super::super::INTERPOLATION_TOLERANCE
                {
                    // At or before first point
                    cumulative_hazards[idx.min(cumulative_hazards.len() - 1)]
                } else {
                    // Interpolate between idx-1 and idx
                    let t0 = time_points[idx - 1];
                    let t1 = time_points[idx];
                    let h0 = cumulative_hazards[idx - 1];
                    let h1 = cumulative_hazards[idx];

                    let alpha = (t_cf - t0) / (t1 - t0).max(super::super::INTERPOLATION_TOLERANCE);
                    h0 + alpha * (h1 - h0)
                }
            } else {
                // After last point - use last cumulative hazard
                cumulative_hazards.last().copied().unwrap_or(0.0)
            };

            survival_probs.push((-hazard_at_cf).exp());
        }

        Ok(survival_probs)
    }
}

impl Pricer for RevolvingCreditPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        use crate::pricer::expect_inst;

        let facility: &RevolvingCredit = expect_inst(instrument, InstrumentType::RevolvingCredit)?;

        // Route to appropriate pricing method based on model
        let result_pv = match self.model {
            ModelKey::Discounting => {
                // For discounting, we use the unified price method which handles
                // deterministic specs (and errs on stochastic if MC not enabled/used)
                Self::price(facility, market, as_of)?
            }
            #[cfg(feature = "mc")]
            ModelKey::MonteCarloGBM => {
                // For MC, we ensure we're using the MC path
                let enhanced = Self::price_with_paths(facility, market, as_of)?;
                enhanced.mc_result.estimate.mean
            }
            _ => {
                return Err(PricingError::ModelFailure(format!(
                    "Unsupported model for RevolvingCredit: {}",
                    self.model
                )));
            }
        };

        // Wrap in ValuationResult
        let mut result = ValuationResult::stamped(facility.id.as_str(), as_of, result_pv);
        result.measures.insert(
            "model".to_string(),
            self.model.to_string().parse().unwrap_or(0.0),
        ); // Just tagging
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::types::CurveId;
    use time::Month;

    use crate::instruments::revolving_credit::types::BaseRateSpec;

    #[allow(dead_code)]
    fn create_test_facility() -> RevolvingCredit {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");

        RevolvingCredit::builder()
            .id("RC-TEST".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(Default::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .hazard_curve_id(CurveId::from("TEST-HZD"))
            .recovery_rate(0.4)
            .build()
            .expect("should succeed")
    }

    #[test]
    fn test_compute_dynamic_survival() {
        let spreads = vec![0.01, 0.02, 0.015, 0.018];
        let times = vec![0.0, 0.25, 0.5, 0.75];
        let recovery = 0.4;
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let payment_dates = vec![
            start,
            Date::from_calendar_date(2025, Month::April, 1).expect("valid date"),
            Date::from_calendar_date(2025, Month::July, 1).expect("valid date"),
            Date::from_calendar_date(2025, Month::October, 1).expect("valid date"),
        ];
        let cashflow_dates = payment_dates.clone();

        let survivals = RevolvingCreditPricer::compute_dynamic_survival_at_dates(
            &spreads,
            &times,
            &payment_dates,
            &cashflow_dates,
            recovery,
            start,
            DayCount::Act365F,
        )
        .expect("should succeed");

        assert_eq!(survivals.len(), 4);
        // Survival at t=0 should be 1.0
        assert!((survivals[0] - 1.0).abs() < 1e-10);
        // Survival should generally decrease over time (with positive spreads)
        // All survivals should be in (0, 1]
        for &s in &survivals {
            assert!(s > 0.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_day_count_consistency() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
        let dc_act360 = DayCount::Act360;

        // Create time points using Act360 (approx 1.0139 for 1 year)
        let t_end_act360 = dc_act360
            .year_fraction(start, end, Default::default())
            .expect("valid date range for year fraction");
        let time_points = vec![0.0, t_end_act360];

        // Spread path: 100bps constant
        let spreads = vec![0.01, 0.01];
        let recovery = 0.0; // Simple hazard = spread

        // We want to look up survival at 'end' date
        let cashflow_dates = vec![end];
        let payment_dates = vec![start, end]; // Dummy

        // 1. Correct: Pass Act360
        let survivals_correct = RevolvingCreditPricer::compute_dynamic_survival_at_dates(
            &spreads,
            &time_points,
            &payment_dates,
            &cashflow_dates,
            recovery,
            start,
            dc_act360,
        )
        .expect("should succeed");

        // Should match exact calculation: exp(-hazard * t)
        // hazard = 0.01
        // t = t_end_act360
        let expected = (-0.01 * t_end_act360).exp();
        assert!(
            (survivals_correct[0] - expected).abs() < 1e-10,
            "Correct day count should yield exact match. Got {}, expected {}",
            survivals_correct[0],
            expected
        );

        // 2. Incorrect: Pass Act365F (simulating the bug)
        let survivals_mismatch = RevolvingCreditPricer::compute_dynamic_survival_at_dates(
            &spreads,
            &time_points,
            &payment_dates,
            &cashflow_dates,
            recovery,
            start,
            DayCount::Act365F,
        )
        .expect("should succeed");

        assert!(
            (survivals_mismatch[0] - survivals_correct[0]).abs() > 1e-5,
            "Mismatching day counts should yield different results"
        );
    }
}
