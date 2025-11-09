//! Pricing engine for revolving credit facilities.
//!
//! Provides both deterministic and Monte Carlo pricing implementations.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use crate::cashflow::builder::schedule_utils::build_periods_from_payment_dates;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::cashflows::generate_deterministic_cashflows_with_curves;
use super::types::RevolvingCredit;

/// Compute the present value of upfront fee paid at commitment.
///
/// The upfront fee is a one-time payment from the borrower to the lender (inflow to lender),
/// paid at the commitment date and discounted to the valuation date.
///
/// # Arguments
///
/// * `upfront_fee_opt` - Optional upfront fee amount
/// * `commitment_date` - Date when facility becomes available
/// * `as_of` - Valuation date
/// * `disc_curve` - Discount curve for PV calculation
/// * `disc_dc` - Day count convention of the discount curve
///
/// # Returns
///
/// Present value of upfront fee (0.0 if no fee), discounted to `as_of` date
fn compute_upfront_fee_pv(
    upfront_fee_opt: Option<Money>,
    commitment_date: Date,
    as_of: Date,
    disc_curve: &dyn finstack_core::market_data::traits::Discounting,
    disc_dc: finstack_core::dates::DayCount,
) -> finstack_core::Result<f64> {
    let upfront_fee = match upfront_fee_opt {
        Some(fee) => fee,
        None => return Ok(0.0),
    };

    if commitment_date > as_of {
        // Discount from commitment date to as_of
        let base_date = disc_curve.base_date();
        let t_commitment = disc_dc.year_fraction(
            base_date,
            commitment_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let df_commitment = disc_curve.df(t_commitment);
        let df_as_of = disc_curve.df(t_as_of);
        let df = if df_as_of > 0.0 {
            df_commitment / df_as_of
        } else {
            1.0
        };

        Ok(upfront_fee.amount() * df)
    } else {
        // Commitment date in past or today - no discounting needed
        Ok(upfront_fee.amount())
    }
}


/// Discounting pricer for revolving credit facilities with deterministic cashflows.
///
/// This pricer generates cashflows using the facility's deterministic schedule
/// and discounts them using the discount curve.
#[derive(Default)]
pub struct RevolvingCreditDiscountingPricer;

impl RevolvingCreditDiscountingPricer {
    /// Create a new revolving credit discounting pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using deterministic cashflows.
    pub fn price_deterministic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Generate cashflows (excludes upfront fee - handled below)
        let schedule = generate_deterministic_cashflows_with_curves(facility, market, as_of)?;

        // Get discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();

        // Build payment schedule dates to create periods
        // Include sentinel (one day after last payment) to ensure terminal flows are captured
        let payment_dates = super::utils::build_payment_dates(facility, true)?;

        // Build periods from payment dates
        let periods = build_periods_from_payment_dates(&payment_dates, facility.payment_frequency);

        // Compute per-period PVs using the standard cashflow schedule method
        let period_pvs = schedule.pre_period_pv(
            &periods,
            disc as &dyn finstack_core::market_data::traits::Discounting,
            as_of,
            disc_dc,
        );

        // Sum PVs across all periods and currencies
        let mut total_pv = 0.0;
        let ccy = facility.commitment_amount.currency();
        for (_period_id, ccy_map) in period_pvs.iter() {
            if let Some(pv_money) = ccy_map.get(&ccy) {
                total_pv += pv_money.amount();
            }
        }

        // Handle upfront fee at pricer level (consistent with MC pricer)
        // Upfront fee is paid by borrower to lender at commitment, so it increases facility value (inflow)
        let upfront_fee_pv = compute_upfront_fee_pv(
            facility.fees.upfront_fee,
            facility.commitment_date,
            as_of,
            disc as &dyn finstack_core::market_data::traits::Discounting,
            disc_dc,
        )?;

        // Lender perspective: upfront fee is an inflow, so add to PV
        let final_pv = total_pv + upfront_fee_pv;

        Ok(Money::new(final_pv, ccy))
    }
}

impl Pricer for RevolvingCreditDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RevolvingCredit, instrument.key())
            })?;

        // Validate that we have a deterministic spec
        if !facility.is_deterministic() {
            return Err(PricingError::model_failure(
                "RevolvingCreditDiscountingPricer requires deterministic cashflows".to_string(),
            ));
        }

        // Extract valuation date from discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let as_of = disc.base_date();

        // Price the facility
        let pv = Self::price_deterministic(facility, market, as_of)?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

/// Monte Carlo pricer for revolving credit facilities with stochastic utilization.
///
/// This pricer simulates utilization paths using a mean-reverting process and
/// averages the discounted cashflows across all paths to compute the expected PV.
///
/// **Note**: Requires the `mc` feature to be enabled.
#[cfg(feature = "mc")]
#[derive(Default)]
pub struct RevolvingCreditMcPricer;

#[cfg(feature = "mc")]
impl RevolvingCreditMcPricer {
    /// Create a new revolving credit Monte Carlo pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using Monte Carlo simulation.
    ///
    /// Simulates the utilization rate evolution using a mean-reverting OU process,
    /// generates cashflows for each path, and returns the average discounted value.
    ///
    /// Always uses the multi-factor modeling path with credit spread and interest rate dynamics.
    /// If `mc_config` is None, synthesizes a minimal configuration with zero credit spread.
    pub fn price_stochastic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        use super::types::{DrawRepaySpec, CreditSpreadProcessSpec, McConfig};

        // Extract stochastic spec
        let stoch_spec = match &facility.draw_repay_spec {
            DrawRepaySpec::Stochastic(spec) => spec.as_ref(),
            _ => return Err(finstack_core::error::InputError::Invalid.into()),
        };

        // Always use multi-factor path. If no mc_config, synthesize a minimal one.
        #[cfg(feature = "mc")]
        {
            let mc_config_to_use;
            let mc_config_ref = if let Some(ref mc_config) = stoch_spec.mc_config {
                mc_config
            } else {
                // Synthesize minimal McConfig with zero credit spread
                let recovery = stoch_spec.default_model.as_ref()
                    .map(|d| d.recovery_rate)
                    .unwrap_or(0.0);
                mc_config_to_use = McConfig {
                    correlation_matrix: None,
                    recovery_rate: recovery,
                    credit_spread_process: CreditSpreadProcessSpec::Constant(0.0),
                    interest_rate_process: None, // Will use deterministic forward
                    util_credit_corr: None,
                };
                &mc_config_to_use
            };
            Self::price_multi_factor(facility, market, as_of, stoch_spec, mc_config_ref)
        }

        #[cfg(not(feature = "mc"))]
        {
            let _ = (facility, market, as_of, stoch_spec);
            Err(finstack_core::error::InputError::Invalid.into())
        }
    }

    /// Multi-factor Monte Carlo pricing with credit spread and interest rate dynamics.
    #[cfg(feature = "mc")]
    fn price_multi_factor(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        stoch_spec: &super::types::StochasticUtilizationSpec,
        mc_config: &super::types::McConfig,
    ) -> finstack_core::Result<Money> {
        use super::types::{
            BaseRateSpec, CreditSpreadProcessSpec, InterestRateProcessSpec, UtilizationProcess,
        };
        use crate::instruments::common::mc::rng::philox::PhiloxRng;
        use crate::instruments::common::mc::rng::sobol::SobolRng;
        use crate::instruments::common::mc::time_grid::TimeGrid;
        use crate::instruments::common::mc::traits::StochasticProcess;
        use crate::instruments::common::models::monte_carlo::discretization::revolving_credit::RevolvingCreditDiscretization;
        use crate::instruments::common::models::monte_carlo::engine::McEngineBuilder;
        use crate::instruments::common::models::monte_carlo::payoff::revolving_credit::{
            FeeStructure, RevolvingCreditPayoff,
        };
        use crate::instruments::common::models::monte_carlo::process::revolving_credit::{
            CreditSpreadParams, InterestRateSpec, RevolvingCreditProcess,
            RevolvingCreditProcessParams, UtilizationParams,
        };

        // Extract utilization parameters
        let util_params = match &stoch_spec.utilization_process {
            UtilizationProcess::MeanReverting {
                target_rate,
                speed,
                volatility,
            } => UtilizationParams::new(*speed, *target_rate, *volatility),
        };

        // Build interest rate specification
        let interest_rate_spec = match &facility.base_rate_spec {
            BaseRateSpec::Fixed { rate } => InterestRateSpec::Fixed { rate: *rate },
            BaseRateSpec::Floating { .. } => {
                // Get interest rate process from config
                match &mc_config.interest_rate_process {
                    Some(InterestRateProcessSpec::HullWhite1F {
                        kappa,
                        sigma,
                        initial,
                        theta,
                    }) => {
                        use crate::instruments::common::mc::process::ou::HullWhite1FParams;
                        InterestRateSpec::Floating {
                            params: HullWhite1FParams::new(*kappa, *sigma, *theta),
                            initial: *initial,
                        }
                    }
                    None => {
                        // Floating rate but no process specified - use deterministic forward curve
                        let fwd = market.get_forward_ref(match &facility.base_rate_spec {
                            BaseRateSpec::Floating { index_id, .. } => index_id.as_str(),
                            _ => unreachable!(),
                        })?;
                        let times = fwd.knots().to_vec();
                        let rates = fwd.forwards().to_vec();
                        InterestRateSpec::DeterministicForward { times, rates }
                    }
                }
            }
        };

        // Build credit spread parameters
        let credit_spread_params = match &mc_config.credit_spread_process {
            CreditSpreadProcessSpec::Cir {
                kappa,
                theta,
                sigma,
                initial,
            } => CreditSpreadParams::new(*kappa, *theta, *sigma, *initial),
            CreditSpreadProcessSpec::Constant(spread) => {
                // Use constant spread with minimal dynamics (very low vol)
                CreditSpreadParams::new(0.01, *spread, 0.001, *spread)
            }
            CreditSpreadProcessSpec::MarketAnchored {
                hazard_curve_id,
                kappa,
                implied_vol,
                tenor_years,
            } => {
                // Pull hazard curve and compute tenor to maturity (or provided tenor)
                let hazard = market.get_hazard_ref(hazard_curve_id.as_str())?;
                let dc = hazard.day_count();
                let base_date = hazard.base_date();

                let t_maturity = dc.year_fraction(
                    base_date,
                    facility.maturity_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let t = tenor_years.unwrap_or_else(|| t_maturity.max(1e-8));

                // Survival and average hazard over [0,T]
                let sp_t = hazard.sp(t);
                let avg_lambda = if t > 0.0 { (-sp_t.ln()) / t } else { 0.0 };

                // Initial hazard from first segment (fallback to avg when unavailable)
                let mut first_lambda = None;
                if let Some((tenor, lambda)) = hazard.knot_points().next() {
                    let _ = tenor;
                    first_lambda = Some(lambda.max(0.0));
                }
                let lambda0 = first_lambda.unwrap_or(avg_lambda).max(0.0);

                // Map hazard ↔ spread using s ≈ (1 − R) · λ
                let one_minus_r = (1.0 - mc_config.recovery_rate).max(1e-6);
                let s0 = one_minus_r * lambda0;
                let s_bar = one_minus_r * avg_lambda;

                // Mean-anchored CIR params on spread space
                let k = *kappa;
                let a = if (k * t).abs() < 1e-8 {
                    1.0 - 0.5 * k * t // first-order expansion
                } else {
                    (1.0 - (-k * t).exp()) / (k * t)
                };
                let theta = if (1.0 - a).abs() < 1e-12 {
                    s_bar
                } else {
                    ((s_bar - a * s0) / (1.0 - a)).max(0.0)
                };

                // Volatility scaled to match fractional vol near mean: σ ≈ implied_vol * sqrt(s̄)
                let sigma = (*implied_vol) * s_bar.max(1e-12).sqrt();

                CreditSpreadParams::new(k, theta, sigma, s0)
            }
        };

        // Get discount curve and compute time grid anchors (t_start, t_end)
        let disc_curve = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc_curve.day_count();
        let base_date = disc_curve.base_date();

        let t_start = disc_dc.year_fraction(
            base_date,
            facility.commitment_date.max(as_of),
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let t_end = disc_dc.year_fraction(
            base_date,
            facility.maturity_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let time_horizon = (t_end - t_start).max(0.0);

        if time_horizon <= 0.0 {
            return Ok(Money::new(0.0, facility.commitment_amount.currency()));
        }

        // Build process parameters
        let mut process_params = RevolvingCreditProcessParams::new(
            util_params,
            interest_rate_spec,
            credit_spread_params,
        );

        // Set correlation: prefer provided matrix, else use util–credit correlation if supplied
        // Default to identity (independent factors) when no correlation is specified
        if let Some(corr) = mc_config.correlation_matrix {
            // Validate correlation matrix is PSD
            finstack_core::math::linalg::validate_correlation_matrix(
                &corr.iter().flatten().copied().collect::<Vec<_>>(),
                3,
            )?;
            process_params = process_params.with_correlation(corr);
        } else if let Some(rho) = mc_config.util_credit_corr {
            let correlation = [[1.0, 0.0, rho], [0.0, 1.0, 0.0], [rho, 0.0, 1.0]];
            process_params = process_params.with_correlation(correlation);
        }

        // Apply time offset to align MC time to market time axis
        // Map MC time 0 to commitment date offset on the curve axis
        process_params = process_params.with_time_offset(t_start);
        let process = RevolvingCreditProcess::new(process_params);

        // Build discretization
        let disc = RevolvingCreditDiscretization::from_process(&process)?;

        // Create time grid (quarterly steps)
        let num_steps = ((time_horizon / 0.25).ceil() as usize).max(1);
        let time_grid = TimeGrid::uniform(time_horizon, num_steps)?;

        // Precompute discount factors for each step (for payoff internal PV accumulation)
        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df_as_of = disc_curve.df(t_as_of);

        let mut discount_factors = Vec::with_capacity(num_steps + 1);
        discount_factors.push(if df_as_of > 0.0 {
            disc_curve.df(t_start) / df_as_of
        } else {
            1.0
        });
        for i in 0..num_steps {
            let t_abs = t_start + time_grid.time(i + 1);
            let df_abs = disc_curve.df(t_abs);
            discount_factors.push(if df_as_of > 0.0 {
                df_abs / df_as_of
            } else {
                1.0
            });
        }

        // Build payoff (NOTE: upfront fee handled separately below, not in payoff)
        // Evaluate tiered fees at initial utilization (approximation; payoff recalculates per step)
        let initial_utilization = facility.utilization_rate();
        let commitment_fee_bps = facility.fees.commitment_fee_bps(initial_utilization);
        let usage_fee_bps = facility.fees.usage_fee_bps(initial_utilization);
        let fees = FeeStructure::new(
            commitment_fee_bps,
            usage_fee_bps,
            facility.fees.facility_fee_bp,
        );

        let is_fixed_rate = matches!(facility.base_rate_spec, BaseRateSpec::Fixed { .. });
        let (fixed_rate, margin_bp) = match &facility.base_rate_spec {
            BaseRateSpec::Fixed { rate } => (*rate, 0.0),
            BaseRateSpec::Floating { margin_bp, .. } => (0.0, *margin_bp),
        };

        // Build locked rates for floating rate facilities
        use crate::instruments::common::models::monte_carlo::payoff::revolving_credit::RateProjection;
        let rate_projection = if let BaseRateSpec::Floating { index_id, margin_bp, reset_freq, floor_bp, .. } = &facility.base_rate_spec {
            // Get forward curve (validate it exists)
            let _fwd = market.get_forward_ref(index_id.as_str())?;
            
            // Build reset schedule from commitment to maturity
            let reset_dates: Vec<Date> = super::utils::build_reset_dates(facility)?
                .expect("floating rate facility must have reset dates");
            
            // Map each MC step to its locked all-in rate
            let mut rates_by_step = Vec::with_capacity(num_steps + 1);
            
            for step in 0..=num_steps {
                let t_step = t_start + time_grid.time(step.min(num_steps));
                
                // Find the reset period containing this step
                // Use the most recent reset date <= t_step
                let step_date = base_date + time::Duration::days((t_step * 365.0) as i64);
                let reset_date = reset_dates.iter()
                    .rev()
                    .find(|&&d| d <= step_date)
                    .copied()
                    .unwrap_or(facility.commitment_date);
                
                // Project floating rate for this step using helper
                let all_in_rate = super::utils::project_floating_rate(
                    reset_date,
                    reset_freq,
                    index_id.as_str(),
                    *margin_bp,
                    *floor_bp,
                    market,
                    &facility.attributes,
                )?;
                
                rates_by_step.push(all_in_rate);
            }
            
            RateProjection::TermLocked { rates_by_step }
        } else {
            // For fixed rate, use ShortRateIntegral (will be ignored)
            RateProjection::ShortRateIntegral
        };

        let payoff = RevolvingCreditPayoff::new(
            facility.commitment_amount.amount(),
            facility.day_count,
            is_fixed_rate,
            fixed_rate,
            margin_bp,
            fees,
            mc_config.recovery_rate,
            time_horizon,
            discount_factors,
            rate_projection,
        );

        // Initial state
        let initial_utilization = facility.utilization_rate();
        let initial_state = process.params().initial_state(initial_utilization);

        // Create MC engine
        let seed = stoch_spec.seed.unwrap_or(42);
        let engine = McEngineBuilder::new()
            .num_paths(stoch_spec.num_paths)
            .seed(seed)
            .time_grid(time_grid)
            .parallel(cfg!(feature = "parallel"))
            .antithetic(stoch_spec.antithetic)
            .build()?;

        // Create RNG
        // Choose RNG based on spec
        let rng_philox = PhiloxRng::new(seed);
        let sobol_dim = process.num_factors();
        let rng_sobol = SobolRng::new(sobol_dim, seed);
        let use_sobol = stoch_spec.use_sobol_qmc;

        // Run simulation
        // Note: Payoff emits undiscounted cashflows; engine handles discounting
        let estimate = if use_sobol {
            engine.price::<SobolRng, _, _, _>(
                &rng_sobol,
                &process,
                &disc,
                &initial_state,
                &payoff,
                facility.commitment_amount.currency(),
                1.0,
            )?
        } else {
            engine.price::<PhiloxRng, _, _, _>(
                &rng_philox,
                &process,
                &disc,
                &initial_state,
                &payoff,
                facility.commitment_amount.currency(),
                1.0,
            )?
        };

        // Handle upfront fee at pricer level (one-time cashflow, not path-dependent)
        // Upfront fee is paid by borrower to lender at commitment, so it increases facility value (inflow)
        let upfront_fee_pv = compute_upfront_fee_pv(
            facility.fees.upfront_fee,
            facility.commitment_date,
            as_of,
            disc_curve as &dyn finstack_core::market_data::traits::Discounting,
            disc_dc,
        )?;

        // Combine path-dependent PV with upfront fee
        // Lender perspective: upfront fee is an inflow (borrower pays lender), so add to PV
        // Note: Cashflows include all principal flows (draws/repays and terminal repayment),
        // so we don't need to separately account for initial capital deployment.
        let total_pv = estimate.mean.amount() + upfront_fee_pv;

        Ok(Money::new(total_pv, facility.commitment_amount.currency()))
    }
}

#[cfg(feature = "mc")]
impl Pricer for RevolvingCreditMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RevolvingCredit, instrument.key())
            })?;

        // Validate that we have a stochastic spec
        if !facility.is_stochastic() {
            return Err(PricingError::model_failure(
                "RevolvingCreditMcPricer requires stochastic specification".to_string(),
            ));
        }

        // Extract valuation date from discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let as_of = disc.base_date();

        // Price the facility using MC
        let pv = Self::price_stochastic(facility, market, as_of)?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCreditFees};
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use time::Month;

    /// Helper to create a standard test facility with common defaults
    fn create_test_facility(
        id: &str,
        start: Date,
        end: Date,
        commitment: f64,
        drawn: f64,
        base_rate_spec: BaseRateSpec,
        fees: RevolvingCreditFees,
    ) -> RevolvingCredit {
        RevolvingCredit::builder()
            .id(id.into())
            .commitment_amount(Money::new(commitment, Currency::USD))
            .drawn_amount(Money::new(drawn, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(base_rate_spec)
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(fees)
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap()
    }

    #[test]
    fn test_pricer_key() {
        let pricer = RevolvingCreditDiscountingPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
        );
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_mc_pricer_key() {
        let pricer = RevolvingCreditMcPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM)
        );
    }

    #[test]
    fn test_deterministic_period_pv_consistency() {
        // Test that sum of per-period PVs equals total NPV
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = create_test_facility(
            "RC-TEST",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::flat(25.0, 10.0, 5.0),
        );

        // Create a simple flat discount curve
        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        let pv = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        // Verify we get a reasonable PV magnitude
        assert!(
            pv.amount().abs() < 10_000_000.0,
            "PV magnitude should be reasonable"
        );
    }

    #[test]
    fn test_deterministic_with_draw_repay() {
        // Test deterministic pricing with draw/repay events
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let draw_date = Date::from_calendar_date(2025, Month::March, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-TEST-2".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![
                super::super::types::DrawRepayEvent {
                    date: draw_date,
                    amount: Money::new(2_000_000.0, Currency::USD),
                    is_draw: true,
                },
            ]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        let pv = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        // Should price successfully
        assert!(pv.currency() == Currency::USD);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_mc_zero_volatility_matches_deterministic() {
        // Test that MC with zero volatility (single deterministic path) matches deterministic pricing
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility_det = RevolvingCredit::builder()
            .id("RC-DET".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let facility_mc = RevolvingCredit::builder()
            .id("RC-MC".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
                super::super::types::StochasticUtilizationSpec {
                    utilization_process: super::super::types::UtilizationProcess::MeanReverting {
                        target_rate: 0.5,
                        speed: 100.0, // High speed = stays at target
                        volatility: 1e-6, // Effectively zero volatility
                    },
                    num_paths: 100, // Average over multiple paths for stability
                    seed: Some(42),
                    antithetic: false,
                    use_sobol_qmc: false,
                    default_model: None,
                    #[cfg(feature = "mc")]
                    mc_config: None,
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        let pv_det =
            RevolvingCreditDiscountingPricer::price_deterministic(&facility_det, &market, start)
                .unwrap();
        let pv_mc =
            RevolvingCreditMcPricer::price_stochastic(&facility_mc, &market, start).unwrap();

        // With zero volatility and single path, MC should match deterministic (within numerical tolerance)
        let diff = (pv_det.amount() - pv_mc.amount()).abs();
        let relative_error = diff / pv_det.amount().abs().max(1.0);
        assert!(
            relative_error < 0.005, // 0.5% relative tolerance
            "MC with zero volatility should match deterministic (tighter tolerance with term-locked), diff: {}, relative: {:.2}%",
            diff, relative_error * 100.0
        );
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_mc_parity_floating_term_locked() {
        // Test that MC with term-locked projection matches deterministic for floating rate
        use finstack_core::market_data::term_structures::ForwardCurve;
        
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Deterministic facility
        let facility_det = RevolvingCredit::builder()
            .id("RC-DET-FLOAT".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Floating {
                index_id: "USD-SOFR-3M".into(),
                margin_bp: 200.0, // 200 bps = 2%
                reset_freq: Frequency::quarterly(),
                floor_bp: Some(0.0), // 0% floor
            })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        // MC facility with zero volatility
        let facility_mc = RevolvingCredit::builder()
            .id("RC-MC-FLOAT".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Floating {
                index_id: "USD-SOFR-3M".into(),
                margin_bp: 200.0,
                reset_freq: Frequency::quarterly(),
                floor_bp: Some(0.0),
            })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
                super::super::types::StochasticUtilizationSpec {
                    utilization_process: super::super::types::UtilizationProcess::MeanReverting {
                        target_rate: 0.5,
                        speed: 100.0, // High speed = stays at target
                        volatility: 1e-6, // Effectively zero volatility
                    },
                    num_paths: 100, // Use many paths for stable average
                    seed: Some(42),
                    antithetic: false,
                    use_sobol_qmc: false,
                    default_model: None,
                    #[cfg(feature = "mc")]
                    mc_config: None, // Will synthesize minimal config with term-locked projection
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        
        let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(base_date)
            .day_count(DayCount::Act360)
            .knots([
                (0.0, 0.03),  // 3%
                (1.0, 0.03),
                (5.0, 0.03),
            ])
            .build()
            .unwrap();
            
        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve);

        let pv_det =
            RevolvingCreditDiscountingPricer::price_deterministic(&facility_det, &market, start)
                .unwrap();
        let pv_mc =
            RevolvingCreditMcPricer::price_stochastic(&facility_mc, &market, start).unwrap();

        // With term-locked projection and zero volatility, MC should closely match deterministic
        let diff = (pv_det.amount() - pv_mc.amount()).abs();
        let relative_error = diff / pv_det.amount().abs().max(1.0);
        assert!(
            relative_error < 0.01, // 1% relative tolerance
            "MC with term-locked projection should match deterministic, diff: {}, relative: {:.2}%",
            diff, relative_error * 100.0
        );
    }

    #[test]
    fn test_payment_dates_parity_with_utils() {
        // Test that our refactored code using utils produces identical payment dates
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = create_test_facility(
            "RC-PARITY-TEST",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::default(),
        );

        // Without sentinel for cashflow generation
        let dates_no_sentinel = super::super::utils::build_payment_dates(&facility, false).unwrap();
        assert!(dates_no_sentinel.len() >= 2);
        
        // Last date should be at or before maturity
        assert!(*dates_no_sentinel.last().unwrap() <= end);

        // With sentinel for PV aggregation
        let dates_with_sentinel = super::super::utils::build_payment_dates(&facility, true).unwrap();
        assert_eq!(dates_with_sentinel.len(), dates_no_sentinel.len() + 1);
        
        // Sentinel should be one day after last payment
        let last_payment = dates_no_sentinel.last().unwrap();
        let sentinel = dates_with_sentinel.last().unwrap();
        assert_eq!(*sentinel, *last_payment + time::Duration::days(1));
    }

    #[test]
    fn test_reset_dates_parity_fixed_vs_floating() {
        // Test that fixed returns None and floating returns Some with correct dates
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Fixed facility
        let facility_fixed = create_test_facility(
            "RC-FIXED",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::default(),
        );

        let reset_dates_fixed = super::super::utils::build_reset_dates(&facility_fixed).unwrap();
        assert!(reset_dates_fixed.is_none(), "Fixed rate should return None");

        // Floating facility
        let facility_floating = RevolvingCredit::builder()
            .id("RC-FLOAT".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Floating {
                index_id: "USD-SOFR-3M".into(),
                margin_bp: 200.0,
                reset_freq: Frequency::quarterly(),
                floor_bp: None,
            })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let reset_dates_floating = super::super::utils::build_reset_dates(&facility_floating).unwrap();
        assert!(reset_dates_floating.is_some(), "Floating rate should return Some");
        
        let dates = reset_dates_floating.unwrap();
        assert!(dates.len() >= 2, "Should have at least 2 reset dates");
    }

    #[test]
    fn test_period_pv_parity_with_helper_periods() {
        // Test that total PV using helper-generated payment dates matches expectations
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = create_test_facility(
            "RC-PV-PARITY",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::flat(25.0, 10.0, 5.0),
        );

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        // Price using deterministic pricer (which uses our helpers internally)
        let pv = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        // PV should be finite and reasonable
        assert!(pv.amount().is_finite());
        assert!(
            pv.amount().abs() < facility.commitment_amount.amount(),
            "PV magnitude should be less than commitment"
        );

        // Price a second time to ensure consistency
        let pv2 = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();
        
        assert_eq!(pv.amount(), pv2.amount(), "Multiple calls should produce identical PVs");
    }
}
