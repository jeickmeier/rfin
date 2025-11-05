//! Pricing engine for revolving credit facilities.
//!
//! Provides both deterministic and Monte Carlo pricing implementations.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::cashflows::generate_deterministic_cashflows;
use super::types::RevolvingCredit;

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
        let schedule = generate_deterministic_cashflows(facility, as_of)?;

        // Get discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();

        // Convert cashflows to dated flows and filter future only
        let dated_flows: Vec<(finstack_core::dates::Date, Money)> = schedule
            .flows
            .into_iter()
            .filter(|cf| cf.date > as_of)
            .map(|cf| (cf.date, cf.amount))
            .collect();

        // Use core npv_static for efficient discounting
        use finstack_core::cashflow::discounting::npv_static;
        let pv_cashflows = if !dated_flows.is_empty() {
            npv_static(disc, as_of, disc_dc, &dated_flows)?
        } else {
            Money::new(0.0, facility.commitment_amount.currency())
        };

        // Handle upfront fee at pricer level (consistent with MC pricer)
        // Upfront fee is paid by lender at commitment, so it reduces facility value
        let upfront_fee_pv = if let Some(upfront_fee) = facility.fees.upfront_fee {
            if facility.commitment_date > as_of {
                // Discount upfront fee from commitment date to as_of
                let t_commitment = disc_dc.year_fraction(
                    disc.base_date(),
                    facility.commitment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let t_as_of = disc_dc.year_fraction(
                    disc.base_date(),
                    as_of,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                
                let df_commitment = disc.df(t_commitment);
                let df_as_of = disc.df(t_as_of);
                let df = if df_as_of > 0.0 {
                    df_commitment / df_as_of
                } else {
                    1.0
                };
                
                upfront_fee.amount() * df
            } else {
                // Commitment date in the past or today - no discounting needed
                upfront_fee.amount()
            }
        } else {
            0.0
        };

        // Lender perspective: upfront fee is an outflow, so subtract from PV
        let total_pv = pv_cashflows.amount() - upfront_fee_pv;
        
        Ok(Money::new(total_pv, facility.commitment_amount.currency()))
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
        let disc = market
            .get_discount_ref(facility.discount_curve_id.as_str())
            .map_err(|e| PricingError::model_failure(e.to_string()))?;
        let as_of = disc.base_date();

        // Price the facility
        let pv = Self::price_deterministic(facility, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

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
    /// If `mc_config` is present in the stochastic spec, uses multi-factor modeling
    /// with credit spread and interest rate dynamics. Otherwise, uses simple
    /// utilization-only simulation.
    pub fn price_stochastic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        use super::types::DrawRepaySpec;

        // Extract stochastic spec
        let stoch_spec = match &facility.draw_repay_spec {
            DrawRepaySpec::Stochastic(spec) => spec.as_ref(),
            _ => return Err(finstack_core::error::InputError::Invalid.into()),
        };

        // Check if advanced MC config is present
        #[cfg(feature = "mc")]
        if let Some(ref mc_config) = stoch_spec.mc_config {
            return Self::price_multi_factor(facility, market, as_of, stoch_spec, mc_config);
        }

        // Fall back to simple utilization-only simulation
        Self::price_simple_utilization(facility, market, as_of, stoch_spec)
    }

    /// Simple utilization-only Monte Carlo pricing (legacy implementation).
    #[cfg(feature = "mc")]
    fn price_simple_utilization(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        stoch_spec: &super::types::StochasticUtilizationSpec,
    ) -> finstack_core::Result<Money> {
        use super::types::UtilizationProcess;

        // Extract mean-reverting parameters
        let (target_rate, speed, volatility) = match &stoch_spec.utilization_process {
            UtilizationProcess::MeanReverting {
                target_rate,
                speed,
                volatility,
            } => (*target_rate, *speed, *volatility),
        };

        // Get discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();
        let base_date = disc.base_date();

        // Compute as_of discount factor
        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df_as_of = disc.df(t_as_of);

        // Time horizon in years
        let t_start = disc_dc.year_fraction(
            base_date,
            facility.commitment_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let t_end = disc_dc.year_fraction(
            base_date,
            facility.maturity_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let time_horizon = t_end - t_start;

        if time_horizon <= 0.0 {
            return Ok(Money::new(0.0, facility.commitment_amount.currency()));
        }

        // Setup MC simulation
        let num_paths = stoch_spec.num_paths;
        let seed = stoch_spec.seed.unwrap_or(42);

        // Use quarterly time steps for cashflow generation
        let dt = 0.25; // 3 months
        let num_steps = (time_horizon / dt).ceil() as usize;

        // Initialize RNG
        use crate::instruments::common::mc::rng::philox::PhiloxRng;
        use crate::instruments::common::mc::traits::RandomStream;

        let base_rng = PhiloxRng::new(seed);

        // Run MC simulation
        let mut sum_pv = 0.0;

        for path_idx in 0..num_paths {
            let mut rng = base_rng.split(path_idx as u64);

            // Simulate utilization path
            let mut utilization = facility.utilization_rate();
            let mut path_pv = 0.0;

            // Simple default model (optional)
            let mut default_threshold: Option<f64> = None;
            let mut cum_hazard = 0.0;
            let mut defaulted = false;
            let mut recovery_rate = 0.0;
            let mut hazard_rate = 0.0;
            if let Some(def) = &stoch_spec.default_model {
                recovery_rate = def.recovery_rate.clamp(0.0, 1.0);
                hazard_rate = def
                    .annual_hazard
                    .unwrap_or_else(|| def.annual_spread.unwrap_or(0.0) / (1.0 - recovery_rate).max(1e-6))
                    .max(0.0);
                // Draw Exp(1) threshold: E = -ln(U)
                let u = rng.next_u01().clamp(1e-12, 1.0 - 1e-12);
                default_threshold = Some(-u.ln());
            }

            // Prepare base rate sources for interest calculation
            let (fixed_rate_opt, fwd_opt, margin_bp_opt) = match &facility.base_rate_spec {
                super::types::BaseRateSpec::Fixed { rate } => (Some(*rate), None, None),
                super::types::BaseRateSpec::Floating { index_id, margin_bp, .. } => {
                    let fwd = market.get_forward_ref(index_id.as_str())?;
                    (None, Some(fwd), Some(*margin_bp))
                }
            };

            // Include upfront fee at commitment date (if applicable)
            if let Some(upfront) = facility.fees.upfront_fee {
                if facility.commitment_date >= as_of {
                    let df_commit = {
                        let df_abs = disc.df(t_start);
                        if df_as_of != 0.0 { df_abs / df_as_of } else { 1.0 }
                    };
                    path_pv += upfront.amount() * df_commit;
                }
            }

            for step in 0..num_steps {
                let t = t_start + (step as f64) * dt;
                let t_next = (t + dt).min(t_end);
                let actual_dt = t_next - t;

                if actual_dt <= 0.0 {
                    break;
                }

                // Default check
                if let Some(th) = default_threshold {
                    if !defaulted {
                        cum_hazard += hazard_rate * actual_dt;
                        if cum_hazard >= th {
                            defaulted = true;
                            // Recovery at default time
                            let commitment = facility.commitment_amount.amount();
                            let drawn_now = commitment * utilization;
                            let df_abs = disc.df(t_next);
                            let df = if df_as_of != 0.0 { df_abs / df_as_of } else { 1.0 };
                            path_pv += drawn_now * recovery_rate * df;
                        }
                    }
                }

                if defaulted {
                    break;
                }

                // Current drawn and undrawn amounts based on utilization
                let commitment = facility.commitment_amount.amount();
                let drawn = commitment * utilization;
                let undrawn = commitment - drawn;

                // Calculate cashflows for this period
                // Interest on drawn
                let period_rate = if let Some(r) = fixed_rate_opt {
                    r
                } else {
                    let fwd = fwd_opt.expect("forward curve available");
                    let m = margin_bp_opt.unwrap_or(0.0) * 1e-4;
                    fwd.rate(actual_dt).max(0.0) + m
                };
                let interest = drawn * period_rate * actual_dt;

                // Commitment fee on undrawn
                let commitment_fee = undrawn * (facility.fees.commitment_fee_bp * 1e-4) * actual_dt;

                // Usage fee on drawn
                let usage_fee = drawn * (facility.fees.usage_fee_bp * 1e-4) * actual_dt;

                // Facility fee on total commitment
                let facility_fee = commitment * (facility.fees.facility_fee_bp * 1e-4) * actual_dt;

                // Total cashflow for this period
                let total_cf = interest + commitment_fee + usage_fee + facility_fee;

                // Discount to valuation date
                let df_abs = disc.df(t_next);
                let df = if df_as_of != 0.0 {
                    df_abs / df_as_of
                } else {
                    1.0
                };

                path_pv += total_cf * df;

                // Add terminal repayment of outstanding principal at maturity
                if step == num_steps - 1 && !defaulted {
                    // Repay drawn balance at maturity
                    path_pv += drawn * df;
                }

                // Evolve utilization using Euler-Maruyama discretization
                // dU = speed * (target - U) * dt + volatility * sqrt(dt) * dW
                if step < num_steps - 1 {
                    let drift = speed * (target_rate - utilization) * actual_dt;
                    let diffusion = volatility * actual_dt.sqrt() * rng.next_std_normal();
                    utilization += drift + diffusion;

                    // Clamp utilization to [0, 1]
                    utilization = utilization.clamp(0.0, 1.0);
                }
            }

            sum_pv += path_pv;
        }

        // Average across paths
        let mean_pv = sum_pv / (num_paths as f64);

        Ok(Money::new(mean_pv, facility.commitment_amount.currency()))
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
        use crate::instruments::common::mc::traits::StochasticProcess;
        use crate::instruments::common::mc::time_grid::TimeGrid;
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
            let correlation = [
                [1.0, 0.0, rho],
                [0.0, 1.0, 0.0],
                [rho, 0.0, 1.0],
            ];
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
        discount_factors.push(if df_as_of > 0.0 { disc_curve.df(t_start) / df_as_of } else { 1.0 });
        for i in 0..num_steps {
            let t_abs = t_start + time_grid.time(i + 1);
            let df_abs = disc_curve.df(t_abs);
            discount_factors.push(if df_as_of > 0.0 { df_abs / df_as_of } else { 1.0 });
        }

        // Build payoff (NOTE: upfront fee handled separately below, not in payoff)
        let fees = FeeStructure::new(
            facility.fees.commitment_fee_bp,
            facility.fees.usage_fee_bp,
            facility.fees.facility_fee_bp,
        );

        let is_fixed_rate = matches!(facility.base_rate_spec, BaseRateSpec::Fixed { .. });
        let (fixed_rate, margin_bp) = match &facility.base_rate_spec {
            BaseRateSpec::Fixed { rate } => (*rate, 0.0),
            BaseRateSpec::Floating { margin_bp, .. } => (0.0, *margin_bp),
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
        // Upfront fee is paid by lender at commitment, so it reduces facility value
        let upfront_fee_pv = if let Some(upfront_fee) = facility.fees.upfront_fee {
            // Discount upfront fee from commitment date to as_of
            let t_as_of = disc_dc.year_fraction(
                base_date,
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let df_as_of = disc_curve.df(t_as_of);
            let df_commitment = disc_curve.df(t_start);
            let df = if df_as_of > 0.0 {
                df_commitment / df_as_of
            } else {
                1.0
            };
            upfront_fee.amount() * df
        } else {
            0.0
        };

        // Combine path-dependent PV with upfront fee
        // Lender perspective: upfront fee is an outflow, so subtract from PV
        let total_pv = estimate.mean.amount() - upfront_fee_pv;

        Ok(Money::new(
            total_pv,
            facility.commitment_amount.currency(),
        ))
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
        let disc = market
            .get_discount_ref(facility.discount_curve_id.as_str())
            .map_err(|e| PricingError::model_failure(e.to_string()))?;
        let as_of = disc.base_date();

        // Price the facility using MC
        let pv = Self::price_stochastic(facility, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
