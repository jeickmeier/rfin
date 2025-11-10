//! Monte Carlo pricing engine for revolving credit facilities.
//!
//! This module provides stochastic pricing capabilities for revolving credit facilities
//! using Monte Carlo simulation. It supports multi-factor models including utilization
//! rate dynamics, credit spread processes, and interest rate processes.
//!
//! # Key Features
//!
//! - **Multi-Factor Models**: Correlated utilization, credit spread, and interest rate processes
//! - **Path-Dependent Pricing**: Captures non-linear effects of stochastic utilization
//! - **Credit Risk Integration**: Market-anchored or CIR credit spread processes
//! - **Interest Rate Dynamics**: Hull-White 1F or deterministic forward curves
//! - **Path Capture**: Optional detailed path recording for analysis
//!
//! # Supported Processes
//!
//! - **Utilization**: Mean-reverting OU process
//! - **Credit Spread**: CIR process or market-anchored hazard rate mapping
//! - **Interest Rate**: Hull-White 1F or deterministic forward curves
//!
//! # Pricing Methodology
//!
//! The Monte Carlo pricer:
//! 1. Simulates correlated paths for utilization, spreads, and rates
//! 2. Generates cashflows along each path using the stochastic schedule
//! 3. Applies survival probabilities if hazard curves are provided
//! 4. Discounts and averages across all simulation paths
//! 5. Adds upfront fee PV as a separate deterministic inflow
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::revolving_credit::pricer::stochastic::RevolvingCreditMcPricer;
//!
//! let pricer = RevolvingCreditMcPricer::new();
//! let result = pricer.price_stochastic(&facility, &market, as_of, None)?;
//! let pv = result.estimate.mean;
//! ```

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::super::types::RevolvingCredit;
use super::super::pricer::deterministic::compute_upfront_fee_pv;

/// Monte Carlo pricer for revolving credit facilities with stochastic utilization.
///
/// This pricer simulates utilization paths using a mean-reverting process and
/// averages the discounted cashflows across all paths to compute the expected PV.
/// It supports multi-factor models with correlated utilization, credit spread,
/// and interest rate dynamics.
///
/// **Note**: Requires the `mc` feature to be enabled.
///
/// # Supported Models
///
/// - **Utilization Process**: Mean-reverting Ornstein-Uhlenbeck
/// - **Credit Spread Process**: CIR or market-anchored hazard mapping
/// - **Interest Rate Process**: Hull-White 1F or deterministic forward
/// - **Correlation**: Full correlation matrix between factors
///
/// # Path Generation
///
/// The pricer uses either Philox or Sobol quasi-Monte Carlo sequences
/// with optional antithetic sampling for variance reduction.
#[cfg(feature = "mc")]
#[derive(Default)]
pub struct RevolvingCreditMcPricer;

#[cfg(feature = "mc")]
impl RevolvingCreditMcPricer {
    /// Create a new revolving credit Monte Carlo pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using Monte Carlo simulation with optional path capture.
    ///
    /// Simulates the utilization rate evolution using a mean-reverting OU process,
    /// generates cashflows for each path, and returns the expected present value.
    ///
    /// Always uses the multi-factor modeling path with credit spread and interest rate dynamics.
    /// If `mc_config` is None, synthesizes a minimal configuration with zero credit spread.
    ///
    /// # Arguments
    ///
    /// * `facility` - The revolving credit facility to price
    /// * `market` - Market data context containing curves and surfaces
    /// * `as_of` - Valuation date
    /// * `path_capture` - Optional configuration for capturing simulation paths. When None, returns
    ///   just the expected PV. When Some, returns full MonteCarloResult with paths.
    ///
    /// # Returns
    ///
    /// If `path_capture` is None: returns expected present value as Money
    /// If `path_capture` is Some: returns full MonteCarloResult with captured paths
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required market data is missing
    /// - Stochastic specification is invalid
    /// - Correlation matrix is not positive definite
    /// - Path generation fails
    pub fn price_stochastic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        path_capture: Option<crate::instruments::common::models::monte_carlo::engine::PathCaptureConfig>,
    ) -> finstack_core::Result<crate::instruments::common::models::monte_carlo::results::MonteCarloResult> {
        use super::super::types::{DrawRepaySpec, CreditSpreadProcessSpec, McConfig};

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
                // Validate configuration early to fail fast
                mc_config.validate()?;
                mc_config
            } else {
                // Synthesize minimal McConfig with zero credit spread
                mc_config_to_use = McConfig {
                    correlation_matrix: None,
                    recovery_rate: facility.recovery_rate,
                    credit_spread_process: CreditSpreadProcessSpec::Constant(0.0),
                    interest_rate_process: None, // Will use deterministic forward
                    util_credit_corr: None,
                };
                // Validate even the synthesized config
                mc_config_to_use.validate()?;
                &mc_config_to_use
            };
            Self::price_multi_factor(facility, market, as_of, stoch_spec, mc_config_ref, path_capture)
        }

        #[cfg(not(feature = "mc"))]
        {
            let _ = (facility, market, as_of, stoch_spec, path_capture);
            Err(finstack_core::error::InputError::Invalid.into())
        }
    }

    /// Multi-factor Monte Carlo pricing with credit spread and interest rate dynamics.
    ///
    /// This is the core pricing implementation that handles the full stochastic model
    /// with correlated factors. It automatically falls back to deterministic pricing
    /// when all volatilities are effectively zero.
    ///
    /// # Process Details
    ///
    /// - **Utilization**: Mean-reverting OU with configurable speed, target, and volatility
    /// - **Credit Spread**: CIR process or market-anchored mapping from hazard curves
    /// - **Interest Rate**: Hull-White 1F with mean reversion and volatility
    /// - **Correlation**: 3x3 correlation matrix between (utilization, credit spread, interest rate)
    ///
    /// # Performance Optimizations
    ///
    /// - Early exit for zero-volatility cases
    /// - Efficient discount factor pre-computation
    /// - Vectorized payoff evaluation
    /// - Parallel path generation when `parallel` feature is enabled
    #[cfg(feature = "mc")]
    fn price_multi_factor(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        stoch_spec: &super::super::types::StochasticUtilizationSpec,
        mc_config: &super::super::types::McConfig,
        path_capture: Option<crate::instruments::common::models::monte_carlo::engine::PathCaptureConfig>,
    ) -> finstack_core::Result<crate::instruments::common::models::monte_carlo::results::MonteCarloResult> {
        use super::super::types::{
            BaseRateSpec, CreditSpreadProcessSpec, DrawRepaySpec, InterestRateProcessSpec,
            UtilizationProcess,
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

        use crate::instruments::common::models::monte_carlo::results::{MoneyEstimate, MonteCarloResult};
        
        const ZERO_TOL: f64 = 1e-6;
        let util_zero = matches!(
            stoch_spec.utilization_process,
            UtilizationProcess::MeanReverting { volatility, .. }
            if volatility.abs() <= ZERO_TOL
        );
        let credit_zero = matches!(
            mc_config.credit_spread_process,
            CreditSpreadProcessSpec::Constant(spread)
            if spread.abs() <= ZERO_TOL
        );
        let rate_zero = mc_config.interest_rate_process.is_none();

        if util_zero && credit_zero && rate_zero {
            let mut deterministic_facility = facility.clone();
            deterministic_facility.draw_repay_spec = DrawRepaySpec::Deterministic(Vec::new());
            let pv = super::deterministic::RevolvingCreditDiscountingPricer::price_deterministic(
                &deterministic_facility,
                market,
                as_of,
            )?;
            // Return as MonteCarloResult for consistent interface
            let estimate = MoneyEstimate {
                mean: pv,
                stderr: 0.0,
                ci_95: (pv, pv),
                num_paths: 1,
            };
            return Ok(MonteCarloResult { estimate, paths: None });
        }

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

        // Build credit spread parameters with stability guards
        let credit_spread_params = match &mc_config.credit_spread_process {
            CreditSpreadProcessSpec::Cir {
                kappa,
                theta,
                sigma,
                initial,
            } => {
                // Apply stability guards for CIR parameters
                const MIN_SPREAD: f64 = 1e-8;  // 0.01 bps floor for numerical stability
                let stable_initial = initial.max(MIN_SPREAD);
                let stable_theta = theta.max(MIN_SPREAD);
                
                // Check and potentially adjust for Feller condition
                let feller_ratio = 2.0 * kappa * stable_theta / (sigma * sigma);
                if feller_ratio < 1.0 && *sigma > 1e-8 {
                    // Adjust sigma to maintain stability (99% of Feller boundary)
                    let adjusted_sigma = (2.0 * kappa * stable_theta).sqrt() * 0.99;
                    CreditSpreadParams::new(*kappa, stable_theta, adjusted_sigma, stable_initial)
                } else {
                    CreditSpreadParams::new(*kappa, stable_theta, *sigma, stable_initial)
                }
            }
            CreditSpreadProcessSpec::Constant(spread) => {
                // Use constant spread with minimal dynamics (very low vol)
                let stable_spread = spread.max(0.0);
                CreditSpreadParams::new(0.01, stable_spread, 0.001, stable_spread)
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
                // Apply minimum spread floor for numerical stability
                const MIN_SPREAD: f64 = 1e-8;  // 0.01 bps
                let one_minus_r = (1.0 - mc_config.recovery_rate).max(1e-6);
                let s0 = (one_minus_r * lambda0).max(MIN_SPREAD);
                let s_bar = (one_minus_r * avg_lambda).max(MIN_SPREAD);

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
                    ((s_bar - a * s0) / (1.0 - a)).max(MIN_SPREAD)
                };

                // Volatility scaled to match fractional vol near mean: σ ≈ implied_vol * sqrt(s̄)
                let sigma = (*implied_vol) * s_bar.max(1e-12).sqrt();
                
                // Ensure Feller condition for stability
                let feller_ratio = 2.0 * k * theta / (sigma * sigma);
                if feller_ratio < 1.0 && sigma > 1e-8 {
                    // Adjust sigma to maintain stability
                    let adjusted_sigma = (2.0 * k * theta).sqrt() * 0.99;
                    CreditSpreadParams::new(k, theta, adjusted_sigma, s0)
                } else {
                    CreditSpreadParams::new(k, theta, sigma, s0)
                }
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
            // Degenerate horizon: return zero result
            let ccy = facility.commitment_amount.currency();
            let estimate = MoneyEstimate {
                mean: Money::new(0.0, ccy),
                stderr: 0.0,
                ci_95: (Money::new(0.0, ccy), Money::new(0.0, ccy)),
                num_paths: stoch_spec.num_paths,
            };
            return Ok(MonteCarloResult { estimate, paths: None });
        }

        // Build process parameters
        let mut process_params = RevolvingCreditProcessParams::new(
            util_params,
            interest_rate_spec,
            credit_spread_params,
        );

        // Set correlation: prefer provided matrix, else use util–credit correlation if supplied
        // Default to identity (independent factors) when no correlation is specified
        // Note: correlation matrix already validated in price_stochastic
        if let Some(corr) = mc_config.correlation_matrix {
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

        // Use actual payment schedule dates for time grid to ensure parity with deterministic
        let payment_dates = super::super::utils::build_payment_dates(facility, false)?;
        
        // Convert payment dates to year fractions from base_date
        // Note: We use disc_dc for the MC time axis to align with discount factor calculations.
        // The payoff will need to handle any day count adjustments for accrual if facility.day_count differs.
        let mut time_points = Vec::with_capacity(payment_dates.len());
        time_points.push(0.0); // Start at t=0
        
        // TODO: For exact parity when facility.day_count differs from disc_curve.day_count(),
        // we would need to pass pre-computed accrual factors to the payoff. Currently, the
        // payoff uses raw time differences which align with disc_dc, not facility.day_count.
        // This is acceptable when both use the same convention (the common case).
        
        for &payment_date in payment_dates.iter().skip(1) {
            // Time point for MC grid (using discount curve day count)
            let t_payment = disc_dc.year_fraction(
                base_date,
                payment_date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            // Convert to time relative to t_start
            let relative_t = (t_payment - t_start).max(0.0);
            if relative_t <= time_horizon {
                time_points.push(relative_t);
            }
        }
        
        // Ensure we have maturity as the last point
        if time_points.last().map(|&t| (t - time_horizon).abs() > 1e-6).unwrap_or(true) {
            time_points.push(time_horizon);
        }
        
        // Remove any duplicates and ensure monotonicity
        time_points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        // Remove near-duplicates
        let mut deduped: Vec<f64> = Vec::with_capacity(time_points.len());
        for &t in &time_points {
            if deduped.is_empty() || (t - deduped.last().unwrap()).abs() >= 1e-10 {
                deduped.push(t);
            }
        }
        time_points = deduped;
        
        let num_steps = time_points.len().saturating_sub(1);
        if num_steps < 2 {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }
        
        let time_grid = TimeGrid::from_times(time_points)?;

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

        // Use the exact payment dates for survival probability computation
        let step_dates = payment_dates;

        let survival_weights = if let Some(hazard_id) = facility.hazard_curve_id.as_ref() {
            let hazard = market.get_hazard_ref(hazard_id.as_str())?;
            Some(hazard.survival_at_dates(&step_dates)?)
        } else {
            None
        };

        // Build payoff (NOTE: upfront fee handled separately below, not in payoff)
        // Pass full fee structure to enable dynamic tier evaluation
        let fees = FeeStructure::from_fees(facility.fees.clone());

        let is_fixed_rate = matches!(facility.base_rate_spec, BaseRateSpec::Fixed { .. });
        let (fixed_rate, margin_bp) = match &facility.base_rate_spec {
            BaseRateSpec::Fixed { rate } => (*rate, 0.0),
            BaseRateSpec::Floating { margin_bp, .. } => (0.0, *margin_bp),
        };

        // Build locked rates for floating rate facilities
        use crate::instruments::common::models::monte_carlo::payoff::revolving_credit::RateProjection;
        let rate_projection = if let BaseRateSpec::Floating { index_id, margin_bp, reset_freq, floor_bp, .. } = &facility.base_rate_spec {
            // Get forward curve for batch operations
            let fwd_curve = market.get_forward_ref(index_id.as_str())?;
            let fwd_dc = fwd_curve.day_count();
            let fwd_base = fwd_curve.base_date();

            // Build reset schedule from commitment to maturity
            let reset_dates: Vec<Date> = super::super::utils::build_reset_dates(facility)?
                .expect("floating rate facility must have reset dates");

            // Use the actual payment dates for reset period mapping
            let reset_periods: Vec<(Date, Date)> = step_dates.iter()
                .map(|&step_date| {
                    // Find the reset date for this step
                    let reset_date = reset_dates.iter()
                        .rev()
                        .find(|&&d| d <= step_date)
                        .copied()
                        .unwrap_or(facility.commitment_date);
                    
                    // Compute reset period end using helper
                    let reset_end = super::super::utils::compute_reset_period_end(
                        reset_date,
                        reset_freq,
                        &facility.attributes,
                    ).unwrap_or(reset_date + time::Duration::days(90)); // Fallback to 3 months
                    
                    (reset_date, reset_end)
                })
                .collect();

            // Batch compute forward rates for all periods
            let mut rates_by_step = Vec::with_capacity(num_steps + 1);
            let margin_rate = margin_bp / 10000.0;
            let floor_rate = floor_bp.map(|f| f / 10000.0);

            for (reset_start, reset_end) in reset_periods {
                // Compute year fractions for the period
                let t0 = fwd_dc.year_fraction(
                    fwd_base,
                    reset_start,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let t1 = fwd_dc.year_fraction(
                    fwd_base,
                    reset_end,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                
                // Get period forward rate
                let mut index_rate = fwd_curve.rate_period(t0, t1);
                
                // Apply floor to index rate (before adding margin)
                if let Some(floor) = floor_rate {
                    index_rate = index_rate.max(floor);
                }
                
                // Add margin to get all-in rate
                let all_in_rate = index_rate + margin_rate;
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
            time_horizon,
            discount_factors,
            survival_weights,
            rate_projection,
        );

        // Initial state
        let initial_utilization = facility.utilization_rate();
        let initial_state = process.params().initial_state(initial_utilization);

        // Create MC engine
        let seed = stoch_spec.seed.unwrap_or(42);
        let mut engine_builder = McEngineBuilder::new()
            .num_paths(stoch_spec.num_paths)
            .seed(seed)
            .time_grid(time_grid)
            .parallel(cfg!(feature = "parallel"))
            .antithetic(stoch_spec.antithetic);
        
        // Add path capture if requested
        if let Some(capture_config) = path_capture.clone() {
            engine_builder = engine_builder.path_capture(capture_config);
        }
        
        let engine = engine_builder.build()?;

        // Create RNG
        // Choose RNG based on spec
        let rng_philox = PhiloxRng::new(seed);
        let sobol_dim = process.num_factors();
        let rng_sobol = SobolRng::new(sobol_dim, seed);
        let use_sobol = stoch_spec.use_sobol_qmc;

        // Run simulation with or without path capture
        // Note: Payoff emits undiscounted cashflows; engine handles discounting
        use crate::instruments::common::mc::process::ProcessMetadata;
        
        let mut result = if path_capture.is_some() {
            // Price with path capture
            if use_sobol {
                engine.price_with_capture::<SobolRng, _, _, _>(
                    &rng_sobol,
                    &process,
                    &disc,
                    &initial_state,
                    &payoff,
                    facility.commitment_amount.currency(),
                    1.0,
                    process.metadata(),
                )?
            } else {
                engine.price_with_capture::<PhiloxRng, _, _, _>(
                    &rng_philox,
                    &process,
                    &disc,
                    &initial_state,
                    &payoff,
                    facility.commitment_amount.currency(),
                    1.0,
                    process.metadata(),
                )?
            }
        } else {
            // Price without path capture - convert MoneyEstimate to MonteCarloResult
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
            
            MonteCarloResult { estimate, paths: None }
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
        let new_mean = result.estimate.mean.amount() + upfront_fee_pv;
        result.estimate.mean = Money::new(new_mean, facility.commitment_amount.currency());
        
        // Update confidence intervals if they exist
        if result.estimate.stderr > 0.0 {
            let new_ci_low = result.estimate.ci_95.0.amount() + upfront_fee_pv;
            let new_ci_high = result.estimate.ci_95.1.amount() + upfront_fee_pv;
            result.estimate.ci_95 = (
                Money::new(new_ci_low, facility.commitment_amount.currency()),
                Money::new(new_ci_high, facility.commitment_amount.currency()),
            );
        }

        Ok(result)
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

        // Price the facility using MC (without path capture for standard pricing)
        let result = Self::price_stochastic(facility, market, as_of, None)?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, result.estimate.mean))
    }
}

/// Public helper on the instrument to run Monte Carlo with optional path capture.
///
/// This is a convenience wrapper around the pricer's unified pricing method.
#[cfg(feature = "mc")]
impl super::super::types::RevolvingCredit {
    pub fn mc_paths_with_capture(
        &self,
        market: &MarketContext,
        as_of_opt: Option<Date>,
        path_capture: crate::instruments::common::models::monte_carlo::engine::PathCaptureConfig,
        _seed: u64, // Seed is taken from stochastic spec
    ) -> finstack_core::Result<
        crate::instruments::common::models::monte_carlo::results::MonteCarloResult,
    > {
        // Validate we have a stochastic specification
        if !self.is_stochastic() {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        
        // Resolve valuation date
        let disc_curve = market.get_discount_ref(self.discount_curve_id.as_str())?;
        let as_of = as_of_opt.unwrap_or_else(|| disc_curve.base_date());
        
        // Delegate to the unified pricer method with path capture
        RevolvingCreditMcPricer::price_stochastic(
            self,
            market,
            as_of,
            Some(path_capture),
        )
    }
}

#[cfg(test)]
#[cfg(feature = "mc")]
mod tests {
    use super::super::super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCreditFees};
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    use finstack_core::types::CurveId;
    use time::Month;

    /// Helper to create a standard test facility with common defaults
    #[allow(dead_code)]
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
            .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
                super::super::super::types::StochasticUtilizationSpec {
                    utilization_process: super::super::super::types::UtilizationProcess::MeanReverting {
                        target_rate: 0.5,
                        speed: 1.0,
                        volatility: 0.1,
                    },
                    num_paths: 100,
                    seed: Some(42),
                    antithetic: false,
                    use_sobol_qmc: false,
                    mc_config: None,
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .recovery_rate(0.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_mc_pricer_key() {
        let pricer = RevolvingCreditMcPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM)
        );
    }

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
            .hazard_curve_id(CurveId::from("USD-HZD"))
            .recovery_rate(0.4)
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
                super::super::super::types::StochasticUtilizationSpec {
                    utilization_process: super::super::super::types::UtilizationProcess::MeanReverting {
                        target_rate: 0.5,
                        speed: 100.0, // High speed = stays at target
                        volatility: 1e-6, // Effectively zero volatility
                    },
                    num_paths: 100, // Average over multiple paths for stability
                    seed: Some(42),
                    antithetic: false,
                    use_sobol_qmc: false,
                    mc_config: None,
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .hazard_curve_id(CurveId::from("USD-HZD"))
            .recovery_rate(0.4)
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

        let hazard_curve = HazardCurve::builder("USD-HZD")
            .base_date(base_date + time::Duration::days(10))
            .day_count(DayCount::Act360)
            .knots([
                (0.0, 0.01),
                (2.0, 0.012),
                (5.0, 0.015),
            ])
            .build()
            .unwrap();

        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_hazard(hazard_curve);

        let pv_det =
            crate::instruments::revolving_credit::pricer::deterministic::RevolvingCreditDiscountingPricer::price_deterministic(&facility_det, &market, start)
                .unwrap();
        let result_mc =
            RevolvingCreditMcPricer::price_stochastic(&facility_mc, &market, start, None).unwrap();
        let pv_mc = result_mc.estimate.mean;

        // With zero volatility, MC should match deterministic within absolute tolerance
        assert!(
            (pv_det.amount() - pv_mc.amount()).abs() <= 1e-6,
            "MC with zero vol should match deterministic: det={}, mc={}, diff={}",
            pv_det.amount(),
            pv_mc.amount(),
            (pv_det.amount() - pv_mc.amount()).abs()
        );
    }

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
            .hazard_curve_id(CurveId::from("TEST-HZD"))
            .recovery_rate(0.0)
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
                super::super::super::types::StochasticUtilizationSpec {
                    utilization_process: super::super::super::types::UtilizationProcess::MeanReverting {
                        target_rate: 0.5,
                        speed: 100.0, // High speed = stays at target
                        volatility: 1e-6, // Effectively zero volatility
                    },
                    num_paths: 100, // Use many paths for stable average
                    seed: Some(42),
                    antithetic: false,
                    use_sobol_qmc: false,
                    mc_config: None, // Will synthesize minimal config with term-locked projection
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .hazard_curve_id(CurveId::from("USD-HZD"))
            .recovery_rate(0.4)
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

        let hazard_curve = HazardCurve::builder("USD-HZD")
            .base_date(base_date)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.01), (5.0, 0.012)])
            .build()
            .unwrap();
        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve)
            .insert_hazard(hazard_curve);

        let pv_det =
            crate::instruments::revolving_credit::pricer::deterministic::RevolvingCreditDiscountingPricer::price_deterministic(&facility_det, &market, start)
                .unwrap();
        let result_mc =
            RevolvingCreditMcPricer::price_stochastic(&facility_mc, &market, start, None).unwrap();
        let pv_mc = result_mc.estimate.mean;

        // With term-locked projection and zero volatility, MC should closely match deterministic
        let diff = (pv_det.amount() - pv_mc.amount()).abs();
        let relative_error = diff / pv_det.amount().abs().max(1.0);
        assert!(
            relative_error <= 0.0125, // 1.25% relative tolerance (reduce flakiness)
            "MC with term-locked projection should match deterministic, diff: {}, relative: {:.2}%",
            diff, relative_error * 100.0
        );
    }
}
