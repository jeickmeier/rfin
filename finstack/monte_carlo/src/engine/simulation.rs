//! Per-path simulation primitives used by the `McEngine` pricing loops.
//!
//! The three methods exposed here (`simulate_path`, `simulate_path_with_capture`,
//! and `simulate_antithetic_pair`) are pure per-path helpers. The outer loops
//! (serial / parallel, with or without capture) live in
//! [`super::pricing`] and call these primitives.

use super::pricing::McEngine;
use crate::paths::{PathPoint, SimulatedPath};
use crate::traits::{Discretization, PathState, Payoff, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;
use smallvec::SmallVec;

impl McEngine {
    /// Simulate a single Monte Carlo path.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn simulate_path<R, P, D, F>(
        &self,
        rng: &mut R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &mut F,
        state: &mut [f64],
        z: &mut [f64],
        work: &mut [f64],
        currency: Currency,
    ) -> Result<f64>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Initialize state
        state.copy_from_slice(initial_state);

        // Create initial path state
        let mut path_state = PathState::new(0, 0.0);
        process.populate_path_state(state, &mut path_state);
        path_state.set_uniform_random(rng.next_u01());
        payoff.on_event(&mut path_state);

        // Simulate path through time steps
        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            rng.fill_std_normals(z);
            disc.step(process, t, dt, state, z, work);

            path_state.set_step_time(step + 1, t + dt);
            process.populate_path_state(state, &mut path_state);
            path_state.set_uniform_random(rng.next_u01());

            // Process payoff event
            payoff.on_event(&mut path_state);
        }

        // Extract payoff value (currency will be added by caller)
        let payoff_money = payoff.value(currency);
        Ok(payoff_money.amount())
    }

    /// Simulate a single Monte Carlo path with full capture.
    ///
    /// Returns the payoff value and the captured path data.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn simulate_path_with_capture<R, P, D, F>(
        &self,
        rng: &mut R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &mut F,
        state: &mut [f64],
        z: &mut [f64],
        work: &mut [f64],
        path_id: usize,
        discount_factor: f64,
        currency: Currency,
    ) -> Result<(f64, SimulatedPath)>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Initialize state
        state.copy_from_slice(initial_state);

        // Create initial path state for payoff
        let mut path_state = PathState::new(0, 0.0);
        process.populate_path_state(state, &mut path_state);
        path_state.set_uniform_random(rng.next_u01());
        payoff.on_event(&mut path_state);

        // Initialize simulated path after the initial event so step-0 payoff and
        // cashflow state are captured consistently.
        let num_steps = self.config.time_grid.num_steps() + 1; // +1 for initial point
        let mut simulated_path = SimulatedPath::with_capacity(path_id, num_steps);
        let initial_state_vec = SmallVec::from_slice(state);
        let mut initial_point = PathPoint::with_state(0, 0.0, initial_state_vec);
        path_state.drain_cashflows(|time, amount, cf_type| {
            initial_point.add_typed_cashflow(time, amount, cf_type);
        });
        if self.config.path_capture.capture_payoffs {
            let payoff_money = payoff.value(currency);
            initial_point.set_payoff(payoff_money.amount());
        }
        simulated_path.add_point(initial_point);

        // Simulate path through time steps
        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            rng.fill_std_normals(z);
            disc.step(process, t, dt, state, z, work);

            path_state.set_step_time(step + 1, t + dt);
            process.populate_path_state(state, &mut path_state);
            path_state.set_uniform_random(rng.next_u01());

            // Process payoff event (payoff may add cashflows to path_state)
            payoff.on_event(&mut path_state);

            // Capture this point with state vector
            let state_vec = SmallVec::from_slice(state);
            let mut point = PathPoint::with_state(step + 1, t + dt, state_vec);

            // Transfer cashflows from PathState to PathPoint
            path_state.drain_cashflows(|time, amount, cf_type| {
                point.add_typed_cashflow(time, amount, cf_type);
            });

            if self.config.path_capture.capture_payoffs {
                // Capture intermediate payoff value (undiscounted)
                let payoff_money = payoff.value(currency);
                point.set_payoff(payoff_money.amount());
            }
            simulated_path.add_point(point);
        }

        // Extract final payoff value
        let payoff_money = payoff.value(currency);
        let payoff_value = payoff_money.amount();

        // Set final discounted value
        simulated_path.set_final_value(payoff_value * discount_factor);

        // Calculate IRR from cashflows (if available)
        let cashflow_amounts = simulated_path.extract_cashflow_amounts();
        if cashflow_amounts.len() >= 2 {
            // Use periodic IRR approximation (assumes roughly equal spacing)
            use finstack_core::cashflow::InternalRateOfReturn;
            if let Ok(irr) = cashflow_amounts.irr(None) {
                simulated_path.set_irr(irr);
            }
        }

        Ok((payoff_value, simulated_path))
    }

    /// Simulate one antithetic pair and return the average payoff (in amount).
    ///
    /// Uses separate work buffers for primary and antithetic paths to prevent
    /// state corruption in discretizations with stateful work buffers (e.g.,
    /// rough Heston, rBergomi, Cheyette rough-vol).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn simulate_antithetic_pair<R, P, D, F>(
        &self,
        rng: &mut R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &mut F,
        state_p: &mut [f64],
        state_a: &mut [f64],
        z: &mut [f64],
        z_anti: &mut [f64],
        work: &mut [f64],
        work_anti: &mut [f64],
        currency: Currency,
    ) -> Result<f64>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Primary path state and payoff
        state_p.copy_from_slice(initial_state);
        let mut payoff_p = payoff.clone();
        let mut path_state_p = PathState::new(0, 0.0);
        process.populate_path_state(state_p, &mut path_state_p);
        let u_init = rng.next_u01();
        path_state_p.set_uniform_random(u_init);
        payoff_p.on_event(&mut path_state_p);

        // Antithetic path state and payoff
        state_a.copy_from_slice(initial_state);
        let mut payoff_a = payoff.clone();
        let mut path_state_a = PathState::new(0, 0.0);
        process.populate_path_state(state_a, &mut path_state_a);
        path_state_a.set_uniform_random(1.0 - u_init);
        payoff_a.on_event(&mut path_state_a);

        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            rng.fill_std_normals(z);
            for i in 0..z.len() {
                z_anti[i] = -z[i];
            }

            disc.step(process, t, dt, state_p, z, work);
            disc.step(process, t, dt, state_a, z_anti, work_anti);

            let u_step = rng.next_u01();

            path_state_p.set_step_time(step + 1, t + dt);
            process.populate_path_state(state_p, &mut path_state_p);
            path_state_p.set_uniform_random(u_step);
            payoff_p.on_event(&mut path_state_p);

            path_state_a.set_step_time(step + 1, t + dt);
            process.populate_path_state(state_a, &mut path_state_a);
            path_state_a.set_uniform_random(1.0 - u_step);
            payoff_a.on_event(&mut path_state_a);
        }

        let v_p = payoff_p.value(currency).amount();
        let v_a = payoff_a.value(currency).amount();
        Ok(0.5 * (v_p + v_a))
    }
}
