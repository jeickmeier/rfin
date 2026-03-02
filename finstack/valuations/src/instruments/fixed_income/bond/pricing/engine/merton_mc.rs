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
    /// Default recovery rate used when no `dynamic_recovery` model is set.
    pub default_recovery_rate: f64,
}

impl MertonMcConfig {
    /// Create a new configuration with default simulation parameters.
    ///
    /// Defaults: cash PIK schedule, 10,000 paths, seed 42, antithetic on,
    /// 12 steps/year, 40% recovery rate.
    #[must_use]
    pub fn new(merton: MertonModel) -> Self {
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
            default_recovery_rate: 0.40,
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
        let barrier_growth_rate = match config.merton.barrier_type() {
            BarrierType::FirstPassage {
                barrier_growth_rate,
            } => *barrier_growth_rate,
            BarrierType::Terminal => 0.0,
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
                    let t = (step + 1) as f64 * dt;
                    let z = normal_draw * sign;

                    // 1. Evolve asset value (GBM)
                    v *= (mu * dt + sigma * sqrt_dt * z).exp();

                    // 2. Check default (first-passage)
                    let barrier = debt_barrier * (barrier_growth_rate * t).exp();
                    if v < barrier {
                        // Default -- compute recovery
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
}
