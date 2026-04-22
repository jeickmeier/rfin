//! Generic HW1F Longstaff-Schwartz MC pricer for callable rate exotics.
//!
//! The pricer drives a user-supplied [`ExerciseBoundaryPayoff`] along
//! simulated short-rate paths produced by an exact HW1F discretization.
//! A two-phase algorithm is used:
//!
//! 1. **Forward pass.** For each path the harness runs the full simulation
//!    and records per-path the deterministic PV reported by
//!    [`Payoff::value`], as well as the short-rate and the inactive flag
//!    at each exercise date.
//! 2. **Backward pass.** Starting from maturity, the harness regresses
//!    continuation values via [`solve_least_squares`] against the
//!    [`standard_basis`] (ITM + active paths only) and rolls the per-path
//!    cashflow vector back, overwriting `cashflow[p]` with the call value
//!    whenever exercise is optimal.
//! 3. **Aggregation.** The average of the per-path cashflows is reported
//!    as the LSMC PV estimate together with a 95% confidence interval.
//!
//! Product payoffs implement [`ExerciseBoundaryPayoff`] (a supertrait of
//! [`Payoff`]); the harness is entirely agnostic to the product-specific
//! cashflow logic.
//!
//! # In-sample upward bias
//!
//! Regression and pricing share the same path set, which biases the reported
//! PV *upward* relative to the true callable value. The bias is typically
//! modest for standard swaption/Bermudan setups with `num_paths ≳ 10⁴` and
//! the default basis, but grows with richer bases and fewer paths. Consumers
//! seeking an unbiased estimate should run training and pricing on disjoint
//! path sets or complement this estimator with a dual upper bound.

use crate::instruments::rates::shared::exercise::{standard_basis, ExerciseBoundaryPayoff};
use crate::instruments::rates::shared::mc_config::RateExoticMcConfig;
use crate::instruments::rates::swaption::pricer::HullWhiteParams;
use finstack_core::currency::Currency;
use finstack_core::Result;
use finstack_monte_carlo::discretization::exact_hw1f::ExactHullWhite1F;
use finstack_monte_carlo::online_stats::OnlineStats;
use finstack_monte_carlo::pricer::lsq::solve_least_squares;
use finstack_monte_carlo::process::ou::HullWhite1FProcess;
use finstack_monte_carlo::results::MoneyEstimate;
use finstack_monte_carlo::rng::philox::PhiloxRng;
use finstack_monte_carlo::time_grid::TimeGrid;
use finstack_monte_carlo::traits::{Discretization, PathState, RandomStream, StateKey};

/// Generic HW1F LSMC pricer for callable rate exotics.
pub struct RateExoticHw1fLsmcPricer {
    /// HW1F short-rate parameters (κ, σ).
    pub hw_params: HullWhiteParams,
    /// Initial short rate r(0).
    pub r0: f64,
    /// Constant mean-reversion level θ for the HW1F short-rate process.
    ///
    /// The simulated short rate follows `dr_t = κ·(θ - r_t)·dt + σ·dW_t`.
    /// Use `0.0` for a pure Ornstein-Uhlenbeck process (mean-reverts to
    /// zero); use `r0` for a classic Vasicek process (mean-reverts to
    /// the initial rate). Curve-calibrated products should replace this
    /// single-θ constant with a time-dependent θ(t) schedule; the
    /// flat-θ harness is a deliberate stepping stone.
    pub theta: f64,
    /// Event (coupon/observation) times driving the payoff, strictly increasing.
    pub event_times: Vec<f64>,
    /// Exercise times — must be a subset of `event_times`.
    pub exercise_times: Vec<f64>,
    /// Call-price multiplier at each exercise date (typically 1.0 = par).
    pub call_prices: Vec<f64>,
    /// Notional for scaling the call payoff.
    pub notional: f64,
    /// Runtime Monte Carlo configuration.
    pub config: RateExoticMcConfig,
    /// Currency for the returned PV estimate.
    pub currency: Currency,
}

impl RateExoticHw1fLsmcPricer {
    /// Run the LSMC pricing: forward pass records path state, backward pass
    /// fits continuation values and applies optimal exercise.
    ///
    /// # Errors
    ///
    /// Returns a validation error if `event_times` is empty or not strictly
    /// increasing, if `exercise_times` are not a subset of `event_times`,
    /// if `call_prices` length does not match `exercise_times`, or if the
    /// time-grid construction fails. Propagates errors from
    /// [`solve_least_squares`].
    pub fn price<F, P>(&self, payoff_factory: F) -> Result<MoneyEstimate>
    where
        F: Fn() -> P + Sync,
        P: ExerciseBoundaryPayoff + 'static,
    {
        self.validate_inputs()?;

        let Some(&maturity) = self.event_times.last() else {
            return Err(finstack_core::Error::Validation(
                "RateExoticHw1fLsmcPricer requires at least one event time".into(),
            ));
        };

        let (grid, event_step_indices, exercise_event_indices) = build_grid_with_exercise_map(
            &self.event_times,
            &self.exercise_times,
            maturity,
            self.config.min_steps_between_events,
        )?;

        let process =
            HullWhite1FProcess::vasicek(self.hw_params.kappa, self.theta, self.hw_params.sigma);
        let disc = ExactHullWhite1F;
        let num_steps = grid.num_steps();
        let work_size = disc.work_size(&process);
        let raw_paths = self.config.raw_stream_count();
        let multiplicity = if self.config.antithetic { 2 } else { 1 };
        let n_paths = self.config.effective_path_count();
        let n_ex = self.exercise_times.len();
        let base_rng = PhiloxRng::new(self.config.seed);

        // Map exercise-date index -> position within event_step_indices.
        let exercise_event_pos = exercise_event_indices;

        let mut deterministic_pv = vec![0.0_f64; n_paths];
        let mut exercise_short_rates = vec![0.0_f64; n_paths * n_ex];
        let mut exercise_inactive = vec![false; n_paths * n_ex];

        let mut path_cursor: usize = 0;
        for path_id in 0..raw_paths {
            for anti in 0..multiplicity {
                let mut rng = base_rng.split(path_id as u64);
                let mut r = self.r0;
                let mut work = vec![0.0; work_size];
                let mut z = [0.0_f64; 1];
                let mut payoff = payoff_factory();
                payoff.reset();
                let mut state = PathState::new(0, 0.0);
                state.set_key(StateKey::ShortRate, r);

                let mut next_event = 0usize;
                let mut next_exercise = 0usize;
                for step in 0..num_steps {
                    let t = grid.time(step);
                    let dt = grid.dt(step);
                    rng.fill_std_normals(&mut z);
                    if anti == 1 {
                        z[0] = -z[0];
                    }
                    disc.step(
                        &process,
                        t,
                        dt,
                        core::slice::from_mut(&mut r),
                        &z,
                        &mut work,
                    );

                    let t_next = grid.time(step + 1);
                    state.set_step_time(step + 1, t_next);
                    state.set_key(StateKey::ShortRate, r);

                    while next_event < event_step_indices.len()
                        && event_step_indices[next_event] == step + 1
                    {
                        payoff.on_event(&mut state);

                        // Record exercise-date state if this event is an exercise date.
                        if next_exercise < exercise_event_pos.len()
                            && exercise_event_pos[next_exercise] == next_event
                        {
                            let flat = path_cursor * n_ex + next_exercise;
                            exercise_short_rates[flat] = r;
                            exercise_inactive[flat] = payoff.is_path_inactive();
                            next_exercise += 1;
                        }
                        next_event += 1;
                    }
                }

                deterministic_pv[path_cursor] = payoff.value(self.currency).amount();
                path_cursor += 1;
            }
        }

        // -- Phase 2: backward LSMC induction ----------------------------------
        let mut cashflow = deterministic_pv.clone();

        for ex_idx in (0..n_ex).rev() {
            let t_ex = self.exercise_times[ex_idx];
            let call_value = self.call_prices[ex_idx] * self.notional;

            // Collect ITM + active paths for regression.
            let mut itm_paths: Vec<usize> = Vec::new();
            let mut itm_basis: Vec<f64> = Vec::new();
            let mut itm_continuation: Vec<f64> = Vec::new();
            let mut num_basis: usize = 0;

            for (p, &cf) in cashflow.iter().enumerate() {
                let flat = p * n_ex + ex_idx;
                if exercise_inactive[flat] {
                    continue;
                }
                let exercise_value = call_value;
                if exercise_value <= 0.0 {
                    continue;
                }
                let r = exercise_short_rates[flat];
                let basis = standard_basis(t_ex, r);
                if num_basis == 0 {
                    num_basis = basis.len();
                }
                itm_paths.push(p);
                itm_basis.extend(basis);
                itm_continuation.push(cf);
            }

            if num_basis == 0 {
                num_basis = standard_basis(t_ex, 0.0).len();
            }

            if itm_paths.len() > num_basis + 2 {
                let coeffs =
                    solve_least_squares(&itm_basis, &itm_continuation, itm_paths.len(), num_basis)?;
                for (k, &p) in itm_paths.iter().enumerate() {
                    let offset = k * num_basis;
                    let basis_row = &itm_basis[offset..offset + num_basis];
                    let mut cont_hat = 0.0_f64;
                    for (b, c) in basis_row.iter().zip(coeffs.iter()) {
                        cont_hat += b * c;
                    }
                    if call_value > cont_hat {
                        cashflow[p] = call_value;
                    }
                }
            } else {
                // Fallback: pathwise optimal exercise against realized cashflow.
                for (p, cf) in cashflow.iter_mut().enumerate() {
                    let flat = p * n_ex + ex_idx;
                    if exercise_inactive[flat] {
                        continue;
                    }
                    if call_value > *cf {
                        *cf = call_value;
                    }
                }
            }
        }

        // -- Phase 3: aggregate ------------------------------------------------
        let mut stats = OnlineStats::new();
        for &v in &cashflow {
            stats.update(v);
        }

        let n = stats.count().max(1) as f64;
        let mean = stats.mean();
        let stderr = stats.std_dev() / n.sqrt();
        let lo = mean - 1.96 * stderr;
        let hi = mean + 1.96 * stderr;
        Ok(MoneyEstimate {
            mean: finstack_core::money::Money::new(mean, self.currency),
            stderr,
            ci_95: (
                finstack_core::money::Money::new(lo, self.currency),
                finstack_core::money::Money::new(hi, self.currency),
            ),
            num_paths: stats.count(),
            num_simulated_paths: stats.count(),
            std_dev: Some(stats.std_dev()),
            median: None,
            percentile_25: None,
            percentile_75: None,
            min: None,
            max: None,
            num_skipped: 0,
        })
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.event_times.is_empty() {
            return Err(finstack_core::Error::Validation(
                "RateExoticHw1fLsmcPricer requires at least one event time".into(),
            ));
        }
        if self.exercise_times.is_empty() {
            return Err(finstack_core::Error::Validation(
                "RateExoticHw1fLsmcPricer requires at least one exercise time".into(),
            ));
        }
        if self.exercise_times.len() != self.call_prices.len() {
            return Err(finstack_core::Error::Validation(format!(
                "exercise_times ({}) and call_prices ({}) length mismatch",
                self.exercise_times.len(),
                self.call_prices.len(),
            )));
        }
        for pair in self.event_times.windows(2) {
            if pair[1] <= pair[0] {
                return Err(finstack_core::Error::Validation(
                    "event_times must be strictly increasing".into(),
                ));
            }
        }
        for &t in &self.exercise_times {
            if !self.event_times.iter().any(|&e| (e - t).abs() < 1e-12) {
                return Err(finstack_core::Error::Validation(format!(
                    "exercise time {t} is not in event_times",
                )));
            }
        }
        Ok(())
    }
}

/// Build grid + map each exercise time to its position within
/// `event_step_indices` (i.e., the index of the event that coincides with
/// the exercise date).
#[allow(clippy::type_complexity)]
fn build_grid_with_exercise_map(
    event_times: &[f64],
    exercise_times: &[f64],
    maturity: f64,
    min_steps: usize,
) -> Result<(TimeGrid, Vec<usize>, Vec<usize>)> {
    let (grid, event_step_indices) =
        super::hw1f_mc::__test_only_build_event_aligned_grid(event_times, maturity, min_steps)?;
    let mut exercise_event_indices = Vec::with_capacity(exercise_times.len());
    for &t in exercise_times {
        let pos = event_times
            .iter()
            .position(|&e| (e - t).abs() < 1e-12)
            .ok_or_else(|| {
                finstack_core::Error::Validation(format!("exercise time {t} not in event_times"))
            })?;
        exercise_event_indices.push(pos);
    }
    Ok((grid, event_step_indices, exercise_event_indices))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::money::Money;
    use finstack_monte_carlo::traits::Payoff;

    /// Inert payoff that reports PV = notional (no coupons, no exercise benefit).
    #[derive(Debug, Clone)]
    struct ParPayoff {
        notional: f64,
    }
    impl Payoff for ParPayoff {
        fn on_event(&mut self, _s: &mut PathState) {}
        fn value(&self, ccy: Currency) -> Money {
            Money::new(self.notional, ccy)
        }
        fn reset(&mut self) {}
    }
    impl ExerciseBoundaryPayoff for ParPayoff {
        fn intrinsic_at(&self, _i: usize, _r: f64, ccy: Currency) -> Money {
            Money::new(self.notional, ccy)
        }
        fn continuation_basis(&self, _i: usize, t: f64, r: f64) -> Vec<f64> {
            standard_basis(t, r)
        }
    }

    #[test]
    fn noexercise_equals_par() {
        let pricer = RateExoticHw1fLsmcPricer {
            hw_params: HullWhiteParams::new(0.05, 0.001),
            r0: 0.03,
            theta: 0.0,
            event_times: vec![1.0, 2.0],
            exercise_times: vec![1.0, 2.0],
            call_prices: vec![1.0, 1.0],
            notional: 1_000_000.0,
            config: RateExoticMcConfig {
                num_paths: 200,
                ..Default::default()
            },
            currency: Currency::USD,
        };
        let est = pricer
            .price(|| ParPayoff {
                notional: 1_000_000.0,
            })
            .expect("ok");
        // With call_price=1.0, call_value == notional, and deterministic PV is
        // also notional. Issuer is indifferent; cashflow[p] stays at notional.
        assert!((est.mean.amount() - 1_000_000.0).abs() < 1e-6);
    }
}
