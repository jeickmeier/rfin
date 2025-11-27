//! Three-factor Monte Carlo path generation for revolving credit facilities.
//!
//! Generates correlated paths for utilization, interest rates, and credit spreads
//! using the existing `RevolvingCreditProcess` infrastructure.
//!
//! # Variance Reduction
//!
//! Supports antithetic variance reduction when enabled via `StochasticUtilizationSpec.antithetic`.
//! This mirrors each path with negated random variates, typically reducing variance by ~50%
//! for smooth payoff functions.
//!
//! # CIR Process Stability
//!
//! The CIR credit spread process requires the Feller condition (2κθ > σ²) to guarantee
//! positive spreads. When violated, a warning is logged and the process may occasionally
//! touch zero. The QE discretization scheme handles boundary behavior gracefully.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::Result;

use crate::instruments::common::mc::process::ou::HullWhite1FParams;
use crate::instruments::common::mc::rng::philox::PhiloxRng;
use crate::instruments::common::mc::rng::sobol::SobolRng;
use crate::instruments::common::mc::time_grid::TimeGrid;
use crate::instruments::common::mc::traits::{Discretization, RandomStream, StochasticProcess};
use crate::instruments::common::models::monte_carlo::discretization::revolving_credit::RevolvingCreditDiscretization;
use crate::instruments::common::models::monte_carlo::process::revolving_credit::{
    CreditSpreadParams, InterestRateSpec, RevolvingCreditProcess, RevolvingCreditProcessParams,
    UtilizationParams,
};

use super::super::cashflow_engine::ThreeFactorPathData;
use super::super::types::{
    BaseRateSpec, CreditSpreadProcessSpec, InterestRateProcessSpec, McConfig, RevolvingCredit,
    StochasticUtilizationSpec, UtilizationProcess,
};

/// Generate 3-factor MC paths using the existing process infrastructure.
///
/// This function creates correlated paths for utilization, interest rates, and credit spreads
/// by simulating the `RevolvingCreditProcess` across the payment schedule.
///
/// # Arguments
///
/// * `stoch_spec` - Stochastic specification with utilization process and MC config
/// * `mc_config` - Monte Carlo configuration with correlation and process details
/// * `facility` - Revolving credit facility
/// * `market` - Market context for curves
/// * `payment_dates` - Payment schedule dates
///
/// # Variance Reduction
///
/// When `stoch_spec.antithetic` is true and Sobol QMC is not used, generates paths
/// in pairs using antithetic variates (z and -z), reducing variance for smooth payoffs.
///
/// # Returns
///
/// Vector of `ThreeFactorPathData`, one per simulated path
#[cfg(feature = "mc")]
pub fn generate_three_factor_paths(
    stoch_spec: &StochasticUtilizationSpec,
    mc_config: &McConfig,
    facility: &RevolvingCredit,
    market: &MarketContext,
    payment_dates: &[Date],
) -> Result<Vec<ThreeFactorPathData>> {
    // Use facility's day count for consistent time calculations
    let day_count = facility.day_count;
    // Build utilization parameters
    let (util_params, is_zero_vol) = match &stoch_spec.utilization_process {
        UtilizationProcess::MeanReverting {
            target_rate,
            speed,
            volatility,
        } => {
            // Handle zero volatility case (for deterministic/parity testing)
            let is_zero = volatility.abs() < 1e-8;
            let vol = if is_zero { 1e-10 } else { *volatility };
            (UtilizationParams::new(*speed, *target_rate, vol), is_zero)
        }
    };

    // Build interest rate specification
    let (interest_rate_spec, rate_curve_opt) = match &facility.base_rate_spec {
        BaseRateSpec::Fixed { rate } => (InterestRateSpec::Fixed { rate: *rate }, None),
        BaseRateSpec::Floating(spec) => {
            match &mc_config.interest_rate_process {
                Some(InterestRateProcessSpec::HullWhite1F {
                    kappa,
                    sigma,
                    initial,
                    theta,
                }) => (
                    InterestRateSpec::Floating {
                        params: HullWhite1FParams::new(*kappa, *sigma, *theta),
                        initial: *initial,
                    },
                    None,
                ),
                None => {
                    // Use deterministic forward curve
                    let fwd = market.get_forward_ref(spec.index_id.as_str())?;
                    let times = fwd.knots().to_vec();
                    let rates = fwd.forwards().to_vec();
                    (
                        InterestRateSpec::DeterministicForward {
                            times: times.clone(),
                            rates: rates.clone(),
                        },
                        Some((times, rates)),
                    )
                }
            }
        }
    };

    // Build credit spread parameters
    let credit_spread_params = build_credit_spread_params(mc_config, facility, market)?;

    // Create 3-factor process with correlation
    let mut process_params =
        RevolvingCreditProcessParams::new(util_params, interest_rate_spec, credit_spread_params);

    if let Some(corr_matrix) = &mc_config.correlation_matrix {
        process_params = process_params.with_correlation(*corr_matrix);
    }

    // Compute time offset for market curve alignment
    let disc_curve = market.get_discount_ref(facility.discount_curve_id.as_str())?;
    let disc_dc = disc_curve.day_count();
    let base_date = disc_curve.base_date();
    let t_start =
        disc_dc.year_fraction(base_date, facility.commitment_date, DayCountCtx::default())?;
    process_params = process_params.with_time_offset(t_start);

    let process = RevolvingCreditProcess::new(process_params);

    // Convert payment dates to time points using facility's day count
    let time_points = dates_to_times(payment_dates, facility.commitment_date, day_count)?;
    let time_grid = TimeGrid::from_times(time_points.clone())?;

    // Set up discretization scheme
    let disc = RevolvingCreditDiscretization::new(process.correlation())?;

    // Prepare buffers for simulation
    let num_paths = stoch_spec.num_paths;
    let num_steps = time_grid.num_steps();
    let num_factors = process.num_factors();
    let initial_state = process.params().initial_state(facility.utilization_rate());

    let mut paths = Vec::with_capacity(num_paths);
    let seed = stoch_spec.seed.unwrap_or(42);
    let use_sobol = stoch_spec.use_sobol_qmc;
    let use_antithetic = stoch_spec.antithetic && !use_sobol; // Antithetic not compatible with Sobol

    // Allocate reusable buffers
    let mut z = vec![0.0; num_factors];
    let mut z_neg = if use_antithetic {
        vec![0.0; num_factors]
    } else {
        Vec::new()
    };
    let mut work = vec![0.0; disc.work_size(&process)];

    if use_sobol {
        let mut rng = SobolRng::new(num_factors, seed);

        for _path_idx in 0..num_paths {
            let mut state = initial_state.to_vec();
            let mut utilization_path = Vec::with_capacity(num_steps + 1);
            let mut short_rate_path = Vec::with_capacity(num_steps + 1);
            let mut credit_spread_path = Vec::with_capacity(num_steps + 1);

            // For deterministic forward, set initial rate from curve
            if let Some((ref times, ref rates)) = rate_curve_opt {
                state[1] = interpolate_rate(time_grid.times()[0], times, rates);
            }

            // Record initial state
            utilization_path.push(state[0].clamp(0.0, 1.0));
            short_rate_path.push(state[1]);
            credit_spread_path.push(state[2].max(0.0));

            // Evolve through time
            for i in 0..num_steps {
                let t_next = time_grid.times()[i + 1];

                if !is_zero_vol {
                    let t = time_grid.times()[i];
                    let dt = t_next - t;

                    // Generate random normal variates
                    rng.fill_std_normals(&mut z);

                    // Apply discretization scheme to evolve state
                    disc.step(&process, t, dt, &mut state, &z, &mut work);
                }
                // else: keep state constant for zero volatility

                // For deterministic forward, manually update short rate from curve
                if let Some((ref times, ref rates)) = rate_curve_opt {
                    state[1] = interpolate_rate(t_next, times, rates);
                }

                // Record state (with bounds for utilization and spread)
                utilization_path.push(state[0].clamp(0.0, 1.0));
                short_rate_path.push(state[1]);
                credit_spread_path.push(state[2].max(0.0));
            }

            paths.push(ThreeFactorPathData {
                utilization_path,
                short_rate_path,
                credit_spread_path,
                time_points: time_grid.times().to_vec(),
                payment_dates: payment_dates.to_vec(),
            });
        }
    } else {
        let mut rng = PhiloxRng::new(seed);

        // For antithetic variance reduction, generate pairs of paths
        let paths_per_iteration = if use_antithetic { 2 } else { 1 };
        let num_iterations = if use_antithetic {
            num_paths.div_ceil(2)
        } else {
            num_paths
        };

        for _iter_idx in 0..num_iterations {
            // Generate random variates for this iteration
            let mut z_sequences: Vec<Vec<f64>> = Vec::with_capacity(num_steps);
            for _ in 0..num_steps {
                rng.fill_std_normals(&mut z);
                z_sequences.push(z.clone());
            }

            // Generate path(s) - one or two depending on antithetic mode
            for sign_idx in 0..paths_per_iteration {
                // Stop if we've reached the requested number of paths
                if paths.len() >= num_paths {
                    break;
                }

                let mut state = initial_state.to_vec();
                let mut utilization_path = Vec::with_capacity(num_steps + 1);
                let mut short_rate_path = Vec::with_capacity(num_steps + 1);
                let mut credit_spread_path = Vec::with_capacity(num_steps + 1);

                // For deterministic forward, set initial rate from curve
                if let Some((ref times, ref rates)) = rate_curve_opt {
                    state[1] = interpolate_rate(time_grid.times()[0], times, rates);
                }

                // Record initial state
                utilization_path.push(state[0].clamp(0.0, 1.0));
                short_rate_path.push(state[1]);
                credit_spread_path.push(state[2].max(0.0));

                // Evolve through time
                for (i, z_seq) in z_sequences.iter().enumerate().take(num_steps) {
                    let t_next = time_grid.times()[i + 1];

                    if !is_zero_vol {
                        let t = time_grid.times()[i];
                        let dt = t_next - t;

                        // Use +z for first path, -z for antithetic path
                        if sign_idx == 0 {
                            // Apply discretization scheme to evolve state with +z
                            disc.step(&process, t, dt, &mut state, z_seq, &mut work);
                        } else {
                            // Antithetic path: use -z
                            for (j, val) in z_seq.iter().enumerate() {
                                z_neg[j] = -val;
                            }
                            disc.step(&process, t, dt, &mut state, &z_neg, &mut work);
                        }
                    }

                    // For deterministic forward, manually update short rate from curve
                    if let Some((ref times, ref rates)) = rate_curve_opt {
                        state[1] = interpolate_rate(t_next, times, rates);
                    }

                    // Record state (with bounds for utilization and spread)
                    utilization_path.push(state[0].clamp(0.0, 1.0));
                    short_rate_path.push(state[1]);
                    credit_spread_path.push(state[2].max(0.0));
                }

                paths.push(ThreeFactorPathData {
                    utilization_path,
                    short_rate_path,
                    credit_spread_path,
                    time_points: time_grid.times().to_vec(),
                    payment_dates: payment_dates.to_vec(),
                });
            }
        }
    }

    Ok(paths)
}

// Use centralized constants from parent module
use super::super::MIN_CIR_SPREAD as CIR_MIN_SPREAD;

/// Build credit spread parameters from MC config.
///
/// # Feller Condition
///
/// For CIR processes, validates the Feller condition: 2κθ > σ². When violated,
/// the process can reach zero. A warning is logged but the process proceeds
/// since the QE discretization handles boundary behavior gracefully.
#[cfg(feature = "mc")]
fn build_credit_spread_params(
    mc_config: &McConfig,
    facility: &RevolvingCredit,
    market: &MarketContext,
) -> Result<CreditSpreadParams> {
    match &mc_config.credit_spread_process {
        CreditSpreadProcessSpec::Cir {
            kappa,
            theta,
            sigma,
            initial,
        } => {
            // Apply stability guards for CIR parameters
            let stable_initial = initial.max(CIR_MIN_SPREAD);
            let stable_theta = theta.max(CIR_MIN_SPREAD);
            let stable_kappa = kappa.max(CIR_MIN_SPREAD);

            // Check Feller condition: 2κθ > σ²
            // When satisfied, the process is guaranteed to stay positive
            let feller_lhs = 2.0 * stable_kappa * stable_theta;
            let feller_rhs = sigma * sigma;
            let feller_ratio = feller_lhs / feller_rhs.max(CIR_MIN_SPREAD);

            if feller_ratio < 1.0 {
                // Feller condition violated - log warning in debug builds
                // The QE discretization will handle this gracefully but spreads may touch zero
                #[cfg(debug_assertions)]
                eprintln!(
                    "[WARN] CIR Feller condition violated: 2κθ/σ² = {:.3} < 1 (kappa={}, theta={}, sigma={}). \
                     Credit spreads may touch zero.",
                    feller_ratio, stable_kappa, stable_theta, sigma
                );
                // Silence unused variable warning in release builds
                let _ = feller_ratio;
            }

            Ok(CreditSpreadParams::new(
                stable_kappa,
                stable_theta,
                *sigma,
                stable_initial,
            ))
        }
        CreditSpreadProcessSpec::Constant(spread) => {
            // Use constant spread with minimal dynamics
            let stable_spread = spread.max(0.0);
            Ok(CreditSpreadParams::new(
                0.01,
                stable_spread,
                0.001,
                stable_spread,
            ))
        }
        CreditSpreadProcessSpec::MarketAnchored {
            hazard_curve_id,
            kappa,
            implied_vol,
            tenor_years,
        } => {
            // Pull hazard curve and compute tenor
            let hazard = market.get_hazard_ref(hazard_curve_id.as_str())?;
            let dc = hazard.day_count();
            let base_date = hazard.base_date();

            let t_maturity =
                dc.year_fraction(base_date, facility.maturity_date, DayCountCtx::default())?;
            let t = tenor_years.unwrap_or_else(|| t_maturity.max(CIR_MIN_SPREAD));

            // Survival and average hazard over [0,T]
            let sp_t = hazard.sp(t);
            let avg_lambda = if t > 0.0 { (-sp_t.ln()) / t } else { 0.0 };

            // Initial hazard from first segment
            let mut first_lambda = None;
            if let Some((_, lambda)) = hazard.knot_points().next() {
                first_lambda = Some(lambda.max(0.0));
            }
            let lambda0 = first_lambda.unwrap_or(avg_lambda).max(0.0);

            // Map hazard ↔ spread using s ≈ (1 − R) · λ
            let one_minus_r = (1.0 - mc_config.recovery_rate).max(1e-6);
            let s0 = (one_minus_r * lambda0).max(CIR_MIN_SPREAD);
            let s_bar = (one_minus_r * avg_lambda).max(CIR_MIN_SPREAD);

            // Mean-anchored CIR params
            let k = kappa.max(CIR_MIN_SPREAD);
            let a = if (k * t).abs() < CIR_MIN_SPREAD {
                1.0 - 0.5 * k * t
            } else {
                (1.0 - (-k * t).exp()) / (k * t)
            };
            let theta = if (1.0 - a).abs() < 1e-12 {
                s_bar
            } else {
                ((s_bar - a * s0) / (1.0 - a)).max(CIR_MIN_SPREAD)
            };

            // Volatility scaled to match fractional vol
            let sigma = (*implied_vol) * s_bar.max(CIR_MIN_SPREAD).sqrt();

            // Check Feller condition: 2κθ > σ²
            let feller_lhs = 2.0 * k * theta;
            let feller_rhs = sigma * sigma;
            let feller_ratio = feller_lhs / feller_rhs.max(CIR_MIN_SPREAD);

            if feller_ratio < 1.0 {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[WARN] Market-anchored CIR Feller condition violated: 2κθ/σ² = {:.3} < 1",
                    feller_ratio
                );
                // Silence unused variable warning in release builds
                let _ = feller_ratio;
            }

            Ok(CreditSpreadParams::new(k, theta, sigma, s0))
        }
    }
}

/// Convert payment dates to time points (years from commitment date).
///
/// Uses the specified day count convention for consistent time fraction calculations
/// across the facility's cashflow engine and path generation.
fn dates_to_times(
    payment_dates: &[Date],
    commitment_date: Date,
    day_count: DayCount,
) -> Result<Vec<f64>> {
    payment_dates
        .iter()
        .map(|&date| day_count.year_fraction(commitment_date, date, DayCountCtx::default()))
        .collect()
}

/// Interpolate rate from knot points (linear interpolation).
#[cfg(feature = "mc")]
fn interpolate_rate(t: f64, times: &[f64], rates: &[f64]) -> f64 {
    if times.is_empty() {
        return 0.0;
    }
    if times.len() == 1 {
        return rates[0];
    }

    // Find bracketing interval
    if t <= times[0] {
        return rates[0];
    }
    if t >= times[times.len() - 1] {
        return rates[rates.len() - 1];
    }

    // Binary search for interval
    for i in 0..(times.len() - 1) {
        if t >= times[i] && t <= times[i + 1] {
            let alpha = (t - times[i]) / (times[i + 1] - times[i]);
            return rates[i] + alpha * (rates[i + 1] - rates[i]);
        }
    }

    rates[rates.len() - 1]
}

/// Stub for non-MC builds
#[cfg(not(feature = "mc"))]
pub fn generate_three_factor_paths(
    _stoch_spec: &StochasticUtilizationSpec,
    _mc_config: &McConfig,
    _facility: &RevolvingCredit,
    _market: &MarketContext,
    _payment_dates: &[Date],
) -> Result<Vec<ThreeFactorPathData>> {
    Err(finstack_core::Error::Validation(
        "MC feature required for path generation".to_string(),
    ))
}
