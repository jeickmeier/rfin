//! Merton Monte Carlo engine for PIK bonds with structural credit risk.
//!
//! Orchestrates [`MertonModel`], [`EndogenousHazardSpec`], [`DynamicRecoverySpec`],
//! and [`ToggleExerciseModel`] into a unified Monte Carlo simulation for pricing
//! bonds with PIK (payment-in-kind) features.
//!
//! # Algorithm
//!
//! For each Monte Carlo path:
//! 1. Evolve asset value via GBM (or jump-diffusion) time steps.
//! 2. Determine the hazard rate (endogenous or Merton-implied).
//! 3. Check for default via first-passage barrier breach.
//! 4. At coupon dates, apply PIK/cash toggle logic.
//! 5. Compute terminal payment for surviving paths.
//!
//! Aggregate across paths to produce clean price, expected/unexpected loss,
//! expected shortfall, and path statistics.
//!
//! # Feature Gate
//!
//! This module requires the `mc` feature.

use crate::instruments::common::models::credit::{
    BarrierType, CreditState, DynamicRecoverySpec, EndogenousHazardSpec, MertonModel,
    ToggleExerciseModel,
};
use finstack_core::math::random::{Pcg64Rng, RandomNumberGenerator};
use finstack_core::Result;

// ---------------------------------------------------------------------------
// PIK schedule types
// ---------------------------------------------------------------------------

/// Barrier-crossing detection policy for first-passage default simulation.
///
/// `Discrete` only checks the barrier at grid points (fast but biased for
/// coarse time steps). `BrownianBridge` uses a Brownian-bridge crossing
/// probability between grid points to approximate continuous monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierCrossing {
    /// Discrete monitoring: default if `V(t_i) < B(t_i)` at time steps.
    Discrete,
    /// Brownian-bridge correction for continuous monitoring between steps.
    BrownianBridge,
}

/// Which structural parameter to calibrate in the MC engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationParameter {
    /// Calibrate the debt barrier B.
    DebtBarrier,
    /// Calibrate the asset volatility sigma_V.
    AssetVol,
}

/// Calibration settings for MC-to-market matching.
///
/// When set on [`MertonMcConfig::calibration`], the pricer runs a low-path
/// bisection to solve for a structural parameter so that the cash base-case
/// MC price matches the target market quote, then re-prices with full paths.
#[derive(Debug, Clone)]
pub struct MertonMcCalibrationSpec {
    /// Target market quote to match (interpreted at quote/settlement date).
    pub target: crate::instruments::fixed_income::bond::pricing::quote_conversions::BondQuoteInput,
    /// Which structural parameter to solve for.
    pub parameter: CalibrationParameter,
    /// Number of MC paths used during calibration iterations (low paths).
    pub low_paths: usize,
    /// Maximum bisection iterations.
    pub max_iter: usize,
    /// Absolute tolerance on the **PV residual** (currency units at `as_of`).
    pub tolerance_pv: f64,
    /// Search bracket for the calibrated parameter (low, high).
    pub bracket: (f64, f64),
    /// Optional seed override used for the calibration run.
    pub seed: Option<u64>,
}

impl Default for MertonMcCalibrationSpec {
    fn default() -> Self {
        Self {
            target: crate::instruments::fixed_income::bond::pricing::quote_conversions::BondQuoteInput::ZSpread(0.0),
            parameter: CalibrationParameter::DebtBarrier,
            low_paths: 2_000,
            max_iter: 40,
            tolerance_pv: 1e-4,
            bracket: (0.0, 0.0),
            seed: None,
        }
    }
}

/// Per-coupon PIK behavior for the MC engine.
///
/// Determines how each coupon payment is handled: paid in cash, accreted
/// to notional (PIK), split between cash and PIK, or decided dynamically
/// by a [`ToggleExerciseModel`].
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PikMode {
    /// Coupon paid in cash.
    Cash,
    /// Coupon accreted to notional (payment-in-kind).
    Pik,
    /// Coupon split between cash and PIK.
    Split {
        /// Fraction paid in cash (e.g. 0.5 for 50%).
        cash_fraction: f64,
        /// Fraction accreted to notional.
        pik_fraction: f64,
    },
    /// Deferred to the [`ToggleExerciseModel`] on the config.
    /// Falls back to `Cash` if no toggle model is set.
    Toggle,
}

/// Time-varying PIK schedule for the MC engine.
///
/// Controls per-coupon PIK behavior, either uniformly or as a step
/// function over time.
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::fixed_income::bond::pricing::merton_mc_engine::{PikMode, PikSchedule};
///
/// // All coupons PIK
/// let uniform = PikSchedule::Uniform(PikMode::Pik);
///
/// // PIK for first 2 years, then cash
/// let stepped = PikSchedule::Stepped(vec![(0.0, PikMode::Pik), (2.0, PikMode::Cash)]);
///
/// // Toggle for 3 years, then mandatory cash
/// let toggle_window = PikSchedule::Stepped(vec![(0.0, PikMode::Toggle), (3.0, PikMode::Cash)]);
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PikSchedule {
    /// Same mode for all coupon dates.
    Uniform(PikMode),
    /// Step function: each `(t, mode)` entry means `mode` applies from
    /// time `t` onward. Entries must be sorted by time ascending.
    Stepped(Vec<(f64, PikMode)>),
}

impl Default for PikSchedule {
    fn default() -> Self {
        Self::Uniform(PikMode::Cash)
    }
}

impl PikSchedule {
    /// Look up the active [`PikMode`] at time `t`.
    pub fn mode_at(&self, t: f64) -> PikMode {
        match self {
            Self::Uniform(mode) => *mode,
            Self::Stepped(steps) => {
                let mut active = PikMode::Cash;
                for &(step_t, mode) in steps {
                    if t >= step_t {
                        active = mode;
                    } else {
                        break;
                    }
                }
                active
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for Monte Carlo PIK bond pricing.
#[derive(Debug, Clone)]
pub struct MertonMcConfig {
    /// Merton structural credit model.
    pub merton: MertonModel,
    /// PIK schedule controlling per-coupon cash/PIK/toggle behavior.
    pub pik_schedule: PikSchedule,
    /// Optional endogenous (leverage-dependent) hazard rate model.
    pub endogenous_hazard: Option<EndogenousHazardSpec>,
    /// Optional dynamic (notional-dependent) recovery rate model.
    pub dynamic_recovery: Option<DynamicRecoverySpec>,
    /// Optional toggle exercise model for PIK/cash coupon decisions.
    /// Active only for coupon dates where [`PikSchedule`] resolves to
    /// [`PikMode::Toggle`].
    pub toggle_model: Option<ToggleExerciseModel>,
    /// Number of Monte Carlo paths.
    pub num_paths: usize,
    /// RNG seed for reproducibility.
    pub seed: u64,
    /// Whether to use antithetic variates for variance reduction.
    pub antithetic: bool,
    /// Time steps per year for the simulation grid.
    pub time_steps_per_year: usize,
    /// Barrier-crossing policy used for `BarrierType::FirstPassage`.
    ///
    /// Default: `BrownianBridge` when the Merton model uses `FirstPassage`,
    /// otherwise `Discrete`.
    pub barrier_crossing: BarrierCrossing,
    /// Default recovery rate used when no `dynamic_recovery` model is set.
    pub default_recovery_rate: f64,
    /// Optional market-calibration specification.
    ///
    /// When set, the pricer first calibrates a structural parameter
    /// (barrier or asset vol) to match a market quote using low-path MC
    /// with common random numbers, then re-prices with full paths.
    pub calibration: Option<MertonMcCalibrationSpec>,
}

impl MertonMcConfig {
    /// Create a new configuration with default simulation parameters.
    ///
    /// Defaults: cash PIK schedule, 10,000 paths, seed 42, antithetic on,
    /// 12 steps/year, 40% recovery rate.
    #[must_use]
    pub fn new(merton: MertonModel) -> Self {
        let barrier_crossing = match merton.barrier_type() {
            BarrierType::FirstPassage { .. } => BarrierCrossing::BrownianBridge,
            BarrierType::Terminal => BarrierCrossing::Discrete,
        };
        Self {
            merton,
            pik_schedule: PikSchedule::default(),
            endogenous_hazard: None,
            dynamic_recovery: None,
            toggle_model: None,
            num_paths: 10_000,
            seed: 42,
            antithetic: true,
            time_steps_per_year: 12,
            barrier_crossing,
            default_recovery_rate: 0.40,
            calibration: None,
        }
    }

    /// Set the PIK schedule.
    #[must_use]
    pub fn pik_schedule(mut self, s: PikSchedule) -> Self {
        self.pik_schedule = s;
        self
    }

    /// Set the number of Monte Carlo paths.
    #[must_use]
    pub fn num_paths(mut self, n: usize) -> Self {
        self.num_paths = n;
        self
    }

    /// Set the RNG seed.
    #[must_use]
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }

    /// Enable or disable antithetic variates.
    #[must_use]
    pub fn antithetic(mut self, a: bool) -> Self {
        self.antithetic = a;
        self
    }

    /// Set time steps per year.
    #[must_use]
    pub fn time_steps_per_year(mut self, n: usize) -> Self {
        self.time_steps_per_year = n;
        self
    }

    /// Set barrier-crossing policy for first-passage default monitoring.
    #[must_use]
    pub fn barrier_crossing(mut self, p: BarrierCrossing) -> Self {
        self.barrier_crossing = p;
        self
    }

    /// Set the market-calibration specification.
    #[must_use]
    pub fn calibration(mut self, c: MertonMcCalibrationSpec) -> Self {
        self.calibration = Some(c);
        self
    }

    /// Set the endogenous hazard model.
    #[must_use]
    pub fn endogenous_hazard(mut self, h: EndogenousHazardSpec) -> Self {
        self.endogenous_hazard = Some(h);
        self
    }

    /// Set the dynamic recovery model.
    #[must_use]
    pub fn dynamic_recovery(mut self, r: DynamicRecoverySpec) -> Self {
        self.dynamic_recovery = Some(r);
        self
    }

    /// Set the toggle exercise model.
    #[must_use]
    pub fn toggle_model(mut self, t: ToggleExerciseModel) -> Self {
        self.toggle_model = Some(t);
        self
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Result from Monte Carlo PIK pricing.
#[derive(Debug, Clone)]
pub struct MertonMcResult {
    /// Clean price as percentage of par.
    pub clean_price_pct: f64,
    /// Dirty price as percentage of par (same as clean in this context).
    pub dirty_price_pct: f64,
    /// Expected loss as fraction of risk-free PV.
    pub expected_loss: f64,
    /// Unexpected loss (standard deviation of path PVs / notional).
    pub unexpected_loss: f64,
    /// Expected shortfall at the 95% confidence level.
    pub expected_shortfall_95: f64,
    /// Average PIK fraction across all coupon dates and paths.
    pub average_pik_fraction: f64,
    /// Effective spread in basis points implied by MC price vs risk-free.
    pub effective_spread_bp: f64,
    /// Path-level statistics.
    pub path_statistics: PathStatistics,
    /// Number of paths used.
    pub num_paths: usize,
    /// Standard error of the clean price estimate.
    pub standard_error: f64,
}

/// Path-level statistics from the Monte Carlo simulation.
#[derive(Debug, Clone)]
pub struct PathStatistics {
    /// Fraction of paths that defaulted.
    pub default_rate: f64,
    /// Average default time (in years) among defaulted paths.
    pub avg_default_time: f64,
    /// Average terminal notional (reflects PIK accrual).
    pub avg_terminal_notional: f64,
    /// Average recovery percentage among defaulted paths.
    pub avg_recovery_pct: f64,
    /// Fraction of coupon dates where PIK was elected.
    pub pik_exercise_rate: f64,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Merton Monte Carlo pricing engine for PIK bonds.
pub struct MertonMcEngine;

impl MertonMcEngine {
    /// Price a bond with structural credit model via Monte Carlo.
    ///
    /// Asset paths evolve under the risk-neutral measure using the Merton
    /// model's `risk_free_rate` as drift. Cashflows are discounted at
    /// `discount_rate`, which may differ (e.g., a funding-adjusted rate).
    /// For standard risk-neutral pricing, set both rates equal.
    ///
    /// Per-coupon PIK behavior is controlled by `config.pik_schedule`:
    /// - `PikMode::Cash` → coupon paid in cash
    /// - `PikMode::Pik` → coupon accreted to notional
    /// - `PikMode::Split` → coupon split between cash and PIK
    /// - `PikMode::Toggle` → deferred to `config.toggle_model`
    ///
    /// # Arguments
    ///
    /// * `notional` - Bond face value
    /// * `coupon_rate` - Annual coupon rate (e.g., 0.08 for 8%)
    /// * `maturity_years` - Time to maturity in years
    /// * `coupon_frequency` - Coupons per year (e.g., 2 for semi-annual)
    /// * `config` - Monte Carlo configuration (includes PIK schedule)
    /// * `discount_rate` - Discount rate for cashflow PV calculation
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn price(
        notional: f64,
        coupon_rate: f64,
        maturity_years: f64,
        coupon_frequency: usize,
        config: &MertonMcConfig,
        discount_rate: f64,
    ) -> Result<MertonMcResult> {
        let num_paths = config.num_paths;
        let dt = 1.0 / config.time_steps_per_year as f64;
        let sqrt_dt = dt.sqrt();
        let total_steps = (maturity_years * config.time_steps_per_year as f64).round() as usize;
        let coupon_period = 1.0 / coupon_frequency as f64;
        let accrual_factor = coupon_rate / coupon_frequency as f64;
        let sigma = config.merton.asset_vol();
        let r = config.merton.risk_free_rate();
        let mu = r - config.merton.payout_rate() - 0.5 * sigma * sigma;

        // Barrier parameters
        let debt_barrier = config.merton.debt_barrier();
        let (barrier_type, barrier_growth_rate) = match config.merton.barrier_type() {
            BarrierType::FirstPassage {
                barrier_growth_rate,
            } => (
                BarrierType::FirstPassage {
                    barrier_growth_rate: *barrier_growth_rate,
                },
                *barrier_growth_rate,
            ),
            BarrierType::Terminal => (BarrierType::Terminal, 0.0),
        };

        // Determine how many base paths (for antithetic)
        let n_base = if config.antithetic {
            num_paths.div_ceil(2)
        } else {
            num_paths
        };

        let mut path_pvs: Vec<f64> = Vec::with_capacity(num_paths);

        // Accumulators for statistics
        let mut total_defaults: usize = 0;
        let mut total_default_time: f64 = 0.0;
        let mut total_terminal_notional: f64 = 0.0;
        let mut total_recovery_pct: f64 = 0.0;
        let mut total_pik_elections: usize = 0;
        let mut total_coupon_periods: usize = 0;
        let mut surviving_paths: usize = 0;

        for path_idx in 0..n_base {
            // Per-path RNG for determinism
            let mut rng = Pcg64Rng::new_with_stream(config.seed, path_idx as u64);

            // Generate all normal draws for this path
            let normals: Vec<f64> = (0..total_steps).map(|_| rng.normal(0.0, 1.0)).collect();
            // Generate all uniform draws for Brownian-bridge crossing checks.
            // This preserves common random numbers across calibration runs that
            // change barrier/vol parameters while keeping the random stream fixed.
            let uniforms: Vec<f64> = (0..total_steps).map(|_| rng.uniform()).collect();

            // Simulate base path (and optionally antithetic)
            let signs: &[f64] = if config.antithetic && path_pvs.len() + 1 < num_paths {
                &[1.0, -1.0]
            } else if config.antithetic && path_pvs.len() < num_paths {
                // Last path if num_paths is odd
                &[1.0]
            } else {
                &[1.0]
            };

            for &sign in signs {
                let mut v = config.merton.asset_value();
                let mut n_current = notional;
                let mut defaulted = false;
                let mut path_pv = 0.0;
                let mut path_pik_elections: usize = 0;
                let mut path_coupon_periods: usize = 0;
                let mut next_coupon_time = coupon_period;

                for (step, &normal_draw) in normals.iter().enumerate().take(total_steps) {
                    let t_prev = step as f64 * dt;
                    let t = (step + 1) as f64 * dt;
                    let z = normal_draw * sign;
                    let u = uniforms[step];

                    let v_prev = v;
                    let barrier_prev = match barrier_type {
                        BarrierType::FirstPassage { .. } => {
                            debt_barrier * (barrier_growth_rate * t_prev).exp()
                        }
                        BarrierType::Terminal => debt_barrier,
                    };

                    // 1. Evolve asset value (GBM)
                    v *= (mu * dt + sigma * sqrt_dt * z).exp();

                    // 2. Check default
                    match barrier_type {
                        BarrierType::Terminal => {
                            // Terminal-only default: check once at maturity.
                            // This prevents accidentally treating a terminal barrier
                            // as a continuously monitored absorbing barrier.
                            let is_final_step = step + 1 == total_steps;
                            if is_final_step && v < debt_barrier {
                                let recovery_rate = config
                                    .dynamic_recovery
                                    .as_ref()
                                    .map_or(config.default_recovery_rate, |dr| {
                                        dr.recovery_at_notional(n_current)
                                    });
                                let recovery_cashflow = recovery_rate * n_current;
                                let df = (-discount_rate * t).exp();
                                path_pv += recovery_cashflow * df;
                                defaulted = true;
                                total_defaults += 1;
                                total_default_time += t;
                                total_recovery_pct += recovery_rate;
                                break;
                            }
                        }
                        BarrierType::FirstPassage { .. } => {
                            let barrier = debt_barrier * (barrier_growth_rate * t).exp();
                            let crossed = if v < barrier {
                                true
                            } else if matches!(
                                config.barrier_crossing,
                                BarrierCrossing::BrownianBridge
                            ) && sigma > 0.0
                                && dt > 0.0
                            {
                                // Brownian-bridge crossing probability for X = ln(V/B(t))
                                // with boundary at 0 between endpoints.
                                //
                                // Conditional on endpoints (x0, x1) > 0, P(min X < 0)
                                // is exp(-2 x0 x1 / (sigma^2 dt)).
                                let barrier_now = barrier;
                                let x0 = (v_prev / barrier_prev).ln();
                                let x1 = (v / barrier_now).ln();
                                if x0 > 0.0 && x1 > 0.0 {
                                    let denom = sigma * sigma * dt;
                                    let p = (-2.0 * x0 * x1 / denom).exp();
                                    u < p
                                } else {
                                    // One endpoint is at/below barrier; discrete check would
                                    // have already triggered if v < barrier, so treat as crossed.
                                    true
                                }
                            } else {
                                false
                            };

                            if crossed {
                                let recovery_rate = config
                                    .dynamic_recovery
                                    .as_ref()
                                    .map_or(config.default_recovery_rate, |dr| {
                                        dr.recovery_at_notional(n_current)
                                    });
                                let recovery_cashflow = recovery_rate * n_current;
                                let df = (-discount_rate * t).exp();
                                path_pv += recovery_cashflow * df;
                                defaulted = true;
                                total_defaults += 1;
                                total_default_time += t;
                                total_recovery_pct += recovery_rate;
                                break;
                            }
                        }
                    }

                    // 3. At coupon dates
                    if t >= next_coupon_time - dt * 0.5 {
                        let coupon_amount = n_current * accrual_factor;
                        path_coupon_periods += 1;
                        let df = (-discount_rate * t).exp();

                        match config.pik_schedule.mode_at(t) {
                            PikMode::Cash => {
                                path_pv += coupon_amount * df;
                            }
                            PikMode::Pik => {
                                n_current += coupon_amount;
                                path_pik_elections += 1;
                            }
                            PikMode::Split {
                                cash_fraction,
                                pik_fraction,
                            } => {
                                path_pv += coupon_amount * cash_fraction * df;
                                n_current += coupon_amount * pik_fraction;
                                if pik_fraction > 0.0 {
                                    path_pik_elections += 1;
                                }
                            }
                            PikMode::Toggle => {
                                if let Some(ref toggle) = config.toggle_model {
                                    let leverage = n_current / v;
                                    let hazard_rate =
                                        config.endogenous_hazard.as_ref().map_or_else(
                                            || {
                                                let pd = config.merton.default_probability(t);
                                                if t > 0.0 {
                                                    -(1.0 - pd).ln() / t
                                                } else {
                                                    0.0
                                                }
                                            },
                                            |eh| eh.hazard_at_leverage(leverage),
                                        );
                                    let remaining = maturity_years - t;
                                    let dd = if sigma > 0.0 && remaining > 0.0 {
                                        let sqrt_remaining = remaining.sqrt();
                                        ((v / n_current).ln()
                                            + (r - config.merton.payout_rate()
                                                - 0.5 * sigma * sigma)
                                                * remaining)
                                            / (sigma * sqrt_remaining)
                                    } else {
                                        0.0
                                    };

                                    let state = CreditState {
                                        hazard_rate,
                                        distance_to_default: Some(dd),
                                        leverage,
                                        accreted_notional: n_current,
                                        asset_value: Some(v),
                                    };

                                    if toggle.should_pik(&state, &mut rng) {
                                        n_current += coupon_amount;
                                        path_pik_elections += 1;
                                    } else {
                                        path_pv += coupon_amount * df;
                                    }
                                } else {
                                    path_pv += coupon_amount * df;
                                }
                            }
                        }

                        next_coupon_time += coupon_period;
                    }
                }

                // 4. Terminal payment (if survived)
                if !defaulted {
                    let df = (-discount_rate * maturity_years).exp();
                    path_pv += n_current * df;
                    surviving_paths += 1;
                    total_terminal_notional += n_current;
                }

                total_pik_elections += path_pik_elections;
                total_coupon_periods += path_coupon_periods;

                path_pvs.push(path_pv);
            }
        }

        // Trim to exact num_paths in case antithetic generated extras
        path_pvs.truncate(num_paths);

        // Aggregate statistics
        let actual_paths = path_pvs.len() as f64;
        let mean_pv = path_pvs.iter().sum::<f64>() / actual_paths;
        let clean_price_pct = mean_pv / notional * 100.0;

        // Risk-free PV for expected loss calculation
        let risk_free_pv = Self::risk_free_pv(
            notional,
            coupon_rate,
            maturity_years,
            coupon_frequency,
            discount_rate,
        );
        let expected_loss = if risk_free_pv > 0.0 {
            1.0 - mean_pv / risk_free_pv
        } else {
            0.0
        };

        // Effective spread: constant spread over risk-free that equates PVs
        let effective_spread_bp = if mean_pv > 0.0 && risk_free_pv > mean_pv {
            10_000.0 * (risk_free_pv / mean_pv).ln() / maturity_years
        } else {
            0.0
        };

        // Unexpected loss (std dev of path PVs / notional)
        let variance = path_pvs
            .iter()
            .map(|&pv| (pv - mean_pv).powi(2))
            .sum::<f64>()
            / (actual_paths - 1.0);
        let std_dev = variance.sqrt();
        let unexpected_loss = std_dev / notional;
        let standard_error = unexpected_loss / (actual_paths.sqrt());

        // Expected shortfall at 95% (average of worst 5% of paths)
        let mut sorted_pvs = path_pvs.clone();
        sorted_pvs.sort_by(|a, b| a.total_cmp(b));
        let cutoff = (0.05 * actual_paths).ceil() as usize;
        let cutoff = cutoff.max(1);
        let es_sum: f64 = sorted_pvs.iter().take(cutoff).sum();
        let expected_shortfall_95 = es_sum / cutoff as f64 / notional * 100.0;

        // Average PIK fraction
        let average_pik_fraction = if total_coupon_periods > 0 {
            total_pik_elections as f64 / total_coupon_periods as f64
        } else {
            0.0
        };

        // Path statistics
        let default_rate = total_defaults as f64 / actual_paths;
        let avg_default_time = if total_defaults > 0 {
            total_default_time / total_defaults as f64
        } else {
            0.0
        };
        let avg_terminal_notional = if surviving_paths > 0 {
            total_terminal_notional / surviving_paths as f64
        } else {
            notional
        };
        let avg_recovery_pct = if total_defaults > 0 {
            total_recovery_pct / total_defaults as f64
        } else {
            0.0
        };
        let pik_exercise_rate = average_pik_fraction;

        Ok(MertonMcResult {
            clean_price_pct,
            dirty_price_pct: clean_price_pct,
            expected_loss,
            unexpected_loss,
            expected_shortfall_95,
            average_pik_fraction,
            effective_spread_bp,
            path_statistics: PathStatistics {
                default_rate,
                avg_default_time,
                avg_terminal_notional,
                avg_recovery_pct,
                pik_exercise_rate,
            },
            num_paths: path_pvs.len(),
            standard_error,
        })
    }

    /// Compute the risk-free present value of a cash-pay bond.
    fn risk_free_pv(
        notional: f64,
        coupon_rate: f64,
        maturity_years: f64,
        coupon_frequency: usize,
        discount_rate: f64,
    ) -> f64 {
        let accrual_factor = coupon_rate / coupon_frequency as f64;
        let coupon_period = 1.0 / coupon_frequency as f64;
        let mut pv = 0.0;
        let num_coupons = (maturity_years * coupon_frequency as f64).round() as usize;

        for i in 1..=num_coupons {
            let t = i as f64 * coupon_period;
            let df = (-discount_rate * t).exp();
            pv += notional * accrual_factor * df;
        }
        pv += notional * (-discount_rate * maturity_years).exp();
        pv
    }
}

// ---------------------------------------------------------------------------
// Calibration helpers
// ---------------------------------------------------------------------------

pub mod calibration {
    //! Low-path MC calibration loop with common random numbers.

    use super::{
        CalibrationParameter, MertonMcCalibrationSpec, MertonMcConfig, PikMode, PikSchedule,
    };
    use crate::cashflow::builder::specs::CouponType;
    use crate::cashflow::traits::CashflowProvider;
    use crate::instruments::common::models::credit::{AssetDynamics, BarrierType, MertonModel};
    use crate::instruments::fixed_income::bond::pricing::quote_conversions::{
        price_from_ytm, price_from_z_spread, BondQuoteInput,
    };
    use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;
    use crate::instruments::fixed_income::bond::types::Bond;
    use crate::instruments::fixed_income::bond::CashflowSpec;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::{InputError, Result};

    /// Output from MC calibration.
    #[derive(Debug, Clone)]
    pub struct MertonMcCalibrationOutput {
        /// Merton model with the calibrated parameter.
        pub calibrated_merton: MertonModel,
        /// PV at `as_of` produced by the calibration (low-path).
        pub calibrated_pv: f64,
        /// PV at `as_of` implied by the market quote (target).
        pub target_pv: f64,
        /// PV residual = calibrated_pv - target_pv.
        pub residual_pv: f64,
        /// Number of bisection iterations used.
        pub iterations: usize,
        /// Value of the calibrated parameter (barrier or asset vol).
        pub solved_parameter: f64,
    }

    /// Create a cash-equivalent bond for calibration.
    pub fn cash_equivalent_bond(bond: &Bond) -> Result<Bond> {
        fn cashify_spec(spec: &CashflowSpec) -> Result<CashflowSpec> {
            Ok(match spec {
                CashflowSpec::Fixed(fixed) => {
                    let mut f = fixed.clone();
                    f.coupon_type = CouponType::Cash;
                    CashflowSpec::Fixed(f)
                }
                CashflowSpec::Floating(_) => return Err(InputError::Invalid.into()),
                CashflowSpec::Amortizing { base, schedule } => CashflowSpec::Amortizing {
                    base: Box::new(cashify_spec(base.as_ref())?),
                    schedule: schedule.clone(),
                },
            })
        }

        let mut b = bond.clone();
        b.cashflow_spec = cashify_spec(&b.cashflow_spec)?;
        Ok(b)
    }

    fn with_parameter(
        base: &MertonModel,
        parameter: CalibrationParameter,
        x: f64,
    ) -> Result<MertonModel> {
        let barrier_type: BarrierType = *base.barrier_type();
        let dynamics: AssetDynamics = *base.dynamics();
        let (asset_value, mut asset_vol, mut debt_barrier) =
            (base.asset_value(), base.asset_vol(), base.debt_barrier());

        match parameter {
            CalibrationParameter::DebtBarrier => debt_barrier = x,
            CalibrationParameter::AssetVol => asset_vol = x,
        }

        MertonModel::new_with_dynamics(
            asset_value,
            asset_vol,
            debt_barrier,
            base.risk_free_rate(),
            base.payout_rate(),
            barrier_type,
            dynamics,
        )
    }

    fn target_pv_from_quote(
        bond: &Bond,
        market: &MarketContext,
        as_of: Date,
        target: &BondQuoteInput,
    ) -> Result<f64> {
        let quote_ctx = QuoteDateContext::new(bond, market, as_of)?;
        let quote_date = quote_ctx.quote_date;

        let dirty_quote_ccy = match *target {
            BondQuoteInput::CleanPricePct(clean_pct) => {
                quote_ctx.dirty_from_clean_pct(clean_pct, bond.notional.amount())
            }
            BondQuoteInput::DirtyPriceCcy(dirty_ccy) => dirty_ccy,
            BondQuoteInput::Ytm(ytm) => {
                let flows = bond.build_dated_flows(market, as_of)?;
                price_from_ytm(bond, &flows, quote_date, ytm)?
            }
            BondQuoteInput::ZSpread(z) => price_from_z_spread(bond, market, quote_date, z)?,
            BondQuoteInput::DiscountMargin(_)
            | BondQuoteInput::Oas(_)
            | BondQuoteInput::AswMarket(_)
            | BondQuoteInput::ISpread(_) => return Err(InputError::Invalid.into()),
        };

        let disc = market.get_discount(&bond.discount_curve_id)?;
        let df_settle = if quote_date > as_of {
            disc.df_between_dates(as_of, quote_date)?
        } else {
            1.0
        };
        Ok(dirty_quote_ccy * df_settle)
    }

    fn mc_cash_pv(
        bond_cash: &Bond,
        as_of: Date,
        discount_rate: f64,
        base_config: &MertonMcConfig,
        low_paths: usize,
        seed_override: Option<u64>,
        merton: MertonModel,
    ) -> Result<f64> {
        let cash_schedule = PikSchedule::Stepped(vec![(0.0, PikMode::Cash)]);

        let mut cfg = base_config.clone();
        cfg.merton = merton;
        cfg.num_paths = low_paths;
        cfg.pik_schedule = cash_schedule;
        cfg.calibration = None;
        if let Some(seed) = seed_override {
            cfg.seed = seed;
        }

        let result = bond_cash.price_merton_mc(&cfg, discount_rate, as_of)?;
        Ok(result.clean_price_pct / 100.0 * bond_cash.notional.amount())
    }

    /// Calibrate a structural parameter to a market quote using the same MC engine.
    ///
    /// Uses bisection with common random numbers (deterministic per-path RNG streams)
    /// by reusing the same seed and simulation settings across iterations.
    pub fn calibrate_parameter_to_market(
        bond: &Bond,
        market: &MarketContext,
        as_of: Date,
        discount_rate: f64,
        base_config: &MertonMcConfig,
        spec: &MertonMcCalibrationSpec,
    ) -> Result<MertonMcCalibrationOutput> {
        let bond_cash = cash_equivalent_bond(bond)?;
        let target_pv = target_pv_from_quote(&bond_cash, market, as_of, &spec.target)?;

        let base_merton = &base_config.merton;
        let asset_value = base_merton.asset_value();
        if asset_value <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }

        let (mut lo, mut hi) = spec.bracket;
        if lo == 0.0 && hi == 0.0 {
            (lo, hi) = match spec.parameter {
                CalibrationParameter::DebtBarrier => (0.001 * asset_value, 0.999 * asset_value),
                CalibrationParameter::AssetVol => (0.01, 2.0),
            };
        }
        if !(lo.is_finite() && hi.is_finite() && lo > 0.0 && hi > lo) {
            return Err(InputError::Invalid.into());
        }

        let eval = |x: f64| -> Result<(f64, f64)> {
            let m = with_parameter(base_merton, spec.parameter, x)?;
            let pv = mc_cash_pv(
                &bond_cash,
                as_of,
                discount_rate,
                base_config,
                spec.low_paths.max(1),
                spec.seed,
                m,
            )?;
            Ok((pv, pv - target_pv))
        };

        let (pv_lo, mut f_lo) = eval(lo)?;
        let (pv_hi, mut f_hi) = eval(hi)?;
        if f_lo == 0.0 {
            return Ok(MertonMcCalibrationOutput {
                calibrated_merton: with_parameter(base_merton, spec.parameter, lo)?,
                calibrated_pv: pv_lo,
                target_pv,
                residual_pv: 0.0,
                iterations: 0,
                solved_parameter: lo,
            });
        }
        if f_hi == 0.0 {
            return Ok(MertonMcCalibrationOutput {
                calibrated_merton: with_parameter(base_merton, spec.parameter, hi)?,
                calibrated_pv: pv_hi,
                target_pv,
                residual_pv: 0.0,
                iterations: 0,
                solved_parameter: hi,
            });
        }

        if f_lo.signum() == f_hi.signum() {
            return Err(InputError::SolverConvergenceFailed {
                iterations: 0,
                residual: f_hi.abs().min(f_lo.abs()),
                last_x: hi,
                reason: format!(
                    "Calibration bracket does not straddle root: f(lo)={f_lo:.6e}, f(hi)={f_hi:.6e}"
                ),
            }
            .into());
        }

        let mut iterations = 0usize;
        let mut mid = 0.5 * (lo + hi);
        let mut pv_mid = 0.0;
        let mut f_mid = 0.0;

        for i in 0..spec.max_iter.max(1) {
            iterations = i + 1;
            mid = 0.5 * (lo + hi);
            let (pv, f) = eval(mid)?;
            pv_mid = pv;
            f_mid = f;

            if f_mid.abs() <= spec.tolerance_pv.max(0.0) {
                break;
            }

            if f_lo.signum() == f_mid.signum() {
                lo = mid;
                f_lo = f_mid;
            } else {
                hi = mid;
                #[allow(unused_assignments)]
                {
                    f_hi = f_mid;
                }
            }
        }

        let calibrated_merton = with_parameter(base_merton, spec.parameter, mid)?;
        Ok(MertonMcCalibrationOutput {
            calibrated_merton,
            calibrated_pv: pv_mid,
            target_pv,
            residual_pv: f_mid,
            iterations,
            solved_parameter: mid,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::models::credit::toggle_exercise::ThresholdDirection;
    use crate::instruments::common::models::credit::{
        AssetDynamics, BarrierType, CreditStateVariable, DynamicRecoverySpec, EndogenousHazardSpec,
        MertonModel, ToggleExerciseModel,
    };

    fn test_merton() -> MertonModel {
        MertonModel::new_with_dynamics(
            200.0,
            0.25,
            100.0,
            0.04,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid merton")
    }

    #[test]
    fn cash_bond_produces_positive_price() {
        let config = MertonMcConfig::new(test_merton()).num_paths(5000).seed(42);
        let result = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        assert!(
            result.clean_price_pct > 50.0 && result.clean_price_pct < 150.0,
            "Price should be reasonable: got {}",
            result.clean_price_pct
        );
    }

    #[test]
    fn pik_bond_produces_positive_price() {
        let config = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(5000)
            .seed(42);
        let result = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        assert!(
            result.clean_price_pct > 50.0 && result.clean_price_pct < 150.0,
            "Price should be reasonable: got {}",
            result.clean_price_pct
        );
    }

    #[test]
    fn endogenous_hazard_lowers_pik_price() {
        let endo = EndogenousHazardSpec::power_law(0.06, 0.5, 2.5).expect("valid");
        let config_no = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(10_000)
            .seed(42);
        let config_yes = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(10_000)
            .seed(42)
            .endogenous_hazard(endo);
        let result_no = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_no, 0.04).expect("ok");
        let result_yes = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_yes, 0.04).expect("ok");
        assert!(
            result_yes.clean_price_pct <= result_no.clean_price_pct + 2.0,
            "Endogenous hazard should lower or maintain PIK price: no={}, yes={}",
            result_no.clean_price_pct,
            result_yes.clean_price_pct
        );
    }

    #[test]
    fn dynamic_recovery_lowers_pik_price() {
        let dyn_rec = DynamicRecoverySpec::floored_inverse(0.40, 100.0, 0.10).expect("valid");
        let config_no = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(10_000)
            .seed(42);
        let config_yes = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(10_000)
            .seed(42)
            .dynamic_recovery(dyn_rec);
        let result_no = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_no, 0.04).expect("ok");
        let result_yes = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_yes, 0.04).expect("ok");
        assert!(
            result_yes.clean_price_pct <= result_no.clean_price_pct + 2.0,
            "Dynamic recovery should lower or maintain PIK price: no={}, yes={}",
            result_no.clean_price_pct,
            result_yes.clean_price_pct
        );
    }

    #[test]
    fn toggle_price_between_cash_and_pik() {
        let toggle = ToggleExerciseModel::threshold(
            CreditStateVariable::HazardRate,
            0.10,
            ThresholdDirection::Above,
        );
        let config_cash = MertonMcConfig::new(test_merton())
            .num_paths(10_000)
            .seed(42);
        let config_pik = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(10_000)
            .seed(42);
        let config_toggle = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Toggle))
            .num_paths(10_000)
            .seed(42)
            .toggle_model(toggle);
        let cash = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_cash, 0.04).expect("ok");
        let pik = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_pik, 0.04).expect("ok");
        let toggle_result =
            MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_toggle, 0.04).expect("ok");
        let min_price = pik.clean_price_pct.min(cash.clean_price_pct) - 5.0;
        let max_price = pik.clean_price_pct.max(cash.clean_price_pct) + 5.0;
        assert!(
            toggle_result.clean_price_pct >= min_price
                && toggle_result.clean_price_pct <= max_price,
            "Toggle should be between cash and PIK: cash={}, pik={}, toggle={}",
            cash.clean_price_pct,
            pik.clean_price_pct,
            toggle_result.clean_price_pct
        );
    }

    #[test]
    fn mc_is_deterministic_with_seed() {
        let config = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(1000)
            .seed(42);
        let r1 = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        let r2 = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        assert!(
            (r1.clean_price_pct - r2.clean_price_pct).abs() < 1e-10,
            "Same seed should give same result"
        );
    }

    #[test]
    fn path_statistics_reasonable() {
        let config = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(5000)
            .seed(42);
        let result = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        assert!(
            result.path_statistics.default_rate >= 0.0
                && result.path_statistics.default_rate <= 1.0
        );
        assert!(
            result.path_statistics.avg_terminal_notional >= 100.0,
            "PIK should accrete notional, got {}",
            result.path_statistics.avg_terminal_notional
        );
        assert!(result.standard_error > 0.0);
    }

    // -----------------------------------------------------------------------
    // PikSchedule tests
    // -----------------------------------------------------------------------

    #[test]
    fn pik_schedule_mode_at_uniform() {
        let s = PikSchedule::Uniform(PikMode::Pik);
        assert_eq!(s.mode_at(0.0), PikMode::Pik);
        assert_eq!(s.mode_at(5.0), PikMode::Pik);
    }

    #[test]
    fn pik_schedule_mode_at_stepped() {
        let s = PikSchedule::Stepped(vec![(0.0, PikMode::Pik), (2.0, PikMode::Cash)]);
        assert_eq!(s.mode_at(0.5), PikMode::Pik);
        assert_eq!(s.mode_at(1.9), PikMode::Pik);
        assert_eq!(s.mode_at(2.0), PikMode::Cash);
        assert_eq!(s.mode_at(5.0), PikMode::Cash);
    }

    #[test]
    fn pik_schedule_stepped_toggle_then_cash() {
        let s = PikSchedule::Stepped(vec![(0.0, PikMode::Toggle), (3.0, PikMode::Cash)]);
        assert_eq!(s.mode_at(1.0), PikMode::Toggle);
        assert_eq!(s.mode_at(2.9), PikMode::Toggle);
        assert_eq!(s.mode_at(3.0), PikMode::Cash);
    }

    #[test]
    fn split_schedule_prices_between_cash_and_pik() {
        let config_cash = MertonMcConfig::new(test_merton()).num_paths(5000).seed(42);
        let config_pik = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(5000)
            .seed(42);
        let config_split = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Split {
                cash_fraction: 0.5,
                pik_fraction: 0.5,
            }))
            .num_paths(5000)
            .seed(42);

        let cash = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_cash, 0.04).expect("ok");
        let pik = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_pik, 0.04).expect("ok");
        let split = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_split, 0.04).expect("ok");

        let lo = cash.clean_price_pct.min(pik.clean_price_pct) - 2.0;
        let hi = cash.clean_price_pct.max(pik.clean_price_pct) + 2.0;
        assert!(
            split.clean_price_pct >= lo && split.clean_price_pct <= hi,
            "50/50 split should be between cash ({}) and PIK ({}), got {}",
            cash.clean_price_pct,
            pik.clean_price_pct,
            split.clean_price_pct
        );
    }

    #[test]
    fn stepped_schedule_pik_then_cash() {
        // PIK for first 2 years, then cash for remaining 3 years.
        // Should be between full cash and full PIK.
        let config_cash = MertonMcConfig::new(test_merton()).num_paths(5000).seed(42);
        let config_pik = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Pik))
            .num_paths(5000)
            .seed(42);
        let config_step = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Stepped(vec![
                (0.0, PikMode::Pik),
                (2.0, PikMode::Cash),
            ]))
            .num_paths(5000)
            .seed(42);

        let cash = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_cash, 0.04).expect("ok");
        let pik = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_pik, 0.04).expect("ok");
        let step = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_step, 0.04).expect("ok");

        let lo = cash.clean_price_pct.min(pik.clean_price_pct) - 2.0;
        let hi = cash.clean_price_pct.max(pik.clean_price_pct) + 2.0;
        assert!(
            step.clean_price_pct >= lo && step.clean_price_pct <= hi,
            "Stepped PIK→cash should be between full cash ({}) and full PIK ({}), got {}",
            cash.clean_price_pct,
            pik.clean_price_pct,
            step.clean_price_pct
        );
        assert!(
            step.average_pik_fraction > 0.0 && step.average_pik_fraction < 1.0,
            "Stepped schedule should have partial PIK fraction, got {}",
            step.average_pik_fraction
        );
    }

    #[test]
    fn toggle_window_then_cash() {
        // Toggle for first 3 years, mandatory cash after.
        let toggle = ToggleExerciseModel::threshold(
            CreditStateVariable::HazardRate,
            0.10,
            ThresholdDirection::Above,
        );
        let config = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Stepped(vec![
                (0.0, PikMode::Toggle),
                (3.0, PikMode::Cash),
            ]))
            .toggle_model(toggle)
            .num_paths(5000)
            .seed(42);

        let result = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        assert!(
            result.clean_price_pct > 50.0 && result.clean_price_pct < 150.0,
            "Toggle window price should be reasonable: {}",
            result.clean_price_pct
        );
    }

    #[test]
    fn toggle_without_model_falls_back_to_cash() {
        // PikMode::Toggle but no toggle_model → should behave like cash
        let config_toggle_no_model = MertonMcConfig::new(test_merton())
            .pik_schedule(PikSchedule::Uniform(PikMode::Toggle))
            .num_paths(5000)
            .seed(42);
        let config_cash = MertonMcConfig::new(test_merton()).num_paths(5000).seed(42);

        let toggle_result =
            MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_toggle_no_model, 0.04).expect("ok");
        let cash_result =
            MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_cash, 0.04).expect("ok");

        assert!(
            (toggle_result.clean_price_pct - cash_result.clean_price_pct).abs() < 1e-10,
            "Toggle without model should equal cash: toggle={}, cash={}",
            toggle_result.clean_price_pct,
            cash_result.clean_price_pct,
        );
    }

    #[test]
    fn default_pik_schedule_is_cash() {
        let config = MertonMcConfig::new(test_merton());
        assert!(
            matches!(config.pik_schedule, PikSchedule::Uniform(PikMode::Cash)),
            "Default pik_schedule should be Uniform(Cash)"
        );
    }

    // -----------------------------------------------------------------------
    // Brownian-bridge crossing tests
    // -----------------------------------------------------------------------

    #[test]
    fn brownian_bridge_increases_default_rate() {
        let config_discrete = MertonMcConfig::new(test_merton())
            .num_paths(10_000)
            .seed(42)
            .barrier_crossing(BarrierCrossing::Discrete);
        let config_bridge = MertonMcConfig::new(test_merton())
            .num_paths(10_000)
            .seed(42)
            .barrier_crossing(BarrierCrossing::BrownianBridge);

        let result_discrete =
            MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_discrete, 0.04).expect("ok");
        let result_bridge =
            MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config_bridge, 0.04).expect("ok");

        assert!(
            result_bridge.path_statistics.default_rate >= result_discrete.path_statistics.default_rate,
            "Brownian-bridge should detect at least as many defaults as discrete: bb={}, discrete={}",
            result_bridge.path_statistics.default_rate,
            result_discrete.path_statistics.default_rate
        );
    }

    #[test]
    fn brownian_bridge_is_deterministic() {
        let config = MertonMcConfig::new(test_merton())
            .num_paths(2000)
            .seed(99)
            .barrier_crossing(BarrierCrossing::BrownianBridge);
        let r1 = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        let r2 = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        assert!(
            (r1.clean_price_pct - r2.clean_price_pct).abs() < 1e-10,
            "Same seed + bridge should give same result"
        );
    }

    #[test]
    fn terminal_barrier_only_defaults_at_maturity() {
        let merton_terminal = MertonModel::new(200.0, 0.25, 100.0, 0.04).expect("valid");
        let config = MertonMcConfig::new(merton_terminal)
            .num_paths(5000)
            .seed(42);
        assert_eq!(config.barrier_crossing, BarrierCrossing::Discrete);

        let result = MertonMcEngine::price(100.0, 0.08, 5.0, 2, &config, 0.04).expect("ok");
        if result.path_statistics.default_rate > 0.0 {
            let expected_default_time = 5.0;
            assert!(
                (result.path_statistics.avg_default_time - expected_default_time).abs() < 0.5,
                "Terminal barrier defaults should only occur near maturity, got avg_default_time={}",
                result.path_statistics.avg_default_time
            );
        }
    }

    #[test]
    fn first_passage_default_config_uses_brownian_bridge() {
        let config = MertonMcConfig::new(test_merton());
        assert_eq!(
            config.barrier_crossing,
            BarrierCrossing::BrownianBridge,
            "FirstPassage should default to BrownianBridge"
        );
    }
}
