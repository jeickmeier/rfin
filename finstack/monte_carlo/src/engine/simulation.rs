use super::path_capture::PathCaptureMode;
use super::pricing::McEngine;
use crate::captured_path_stats::apply_captured_path_statistics;
use crate::estimate::Estimate;
use crate::online_stats::OnlineStats;
use crate::paths::{PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath};
use crate::traits::{Discretization, PathState, Payoff, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;
use smallvec::SmallVec;

#[cfg(feature = "parallel")]
use super::pricing::{adaptive_chunk_size, parallel_path_chunks};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::sync::Mutex;

impl McEngine {
    /// Serial pricing with path capture.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn price_serial_with_capture<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
        process_params: ProcessParams,
    ) -> Result<(Estimate, Option<PathDataset>)>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        let mut stats = OnlineStats::new();
        let dim = process.dim();
        let num_factors = process.num_factors();
        let work_size = disc.work_size(process);

        // Pre-allocate buffers (reused across paths)
        let mut state = vec![0.0; dim];
        let mut z = vec![0.0; num_factors];
        let mut work = vec![0.0; work_size];
        let mut state_a = vec![0.0; dim];
        let mut z_anti = vec![0.0; num_factors];
        let mut work_anti = vec![0.0; work_size];

        // Path capture setup
        let capture_enabled = self.config.path_capture.enabled;
        let sampling_method = match &self.config.path_capture.capture_mode {
            PathCaptureMode::All => PathSamplingMethod::All,
            PathCaptureMode::Sample { count, seed } => PathSamplingMethod::RandomSample {
                count: *count,
                seed: *seed,
            },
        };

        let mut captured_paths = if capture_enabled {
            let estimated_capacity = match self.config.path_capture.capture_mode {
                PathCaptureMode::All => self.config.num_paths,
                PathCaptureMode::Sample { count, .. } => count,
            };
            Vec::with_capacity(estimated_capacity)
        } else {
            Vec::new()
        };

        let mut payoff_local = payoff.clone();
        let mut num_skipped: usize = 0;

        for path_id in 0..self.config.num_paths {
            let mut path_rng = rng.split(path_id as u64);

            payoff_local.reset();
            payoff_local.on_path_start(&mut path_rng);

            let should_capture = capture_enabled
                && self
                    .config
                    .path_capture
                    .should_capture(path_id, self.config.num_paths);

            let (payoff_value, captured_path) = if should_capture {
                self.simulate_path_with_capture(
                    &mut path_rng,
                    process,
                    disc,
                    initial_state,
                    &mut payoff_local,
                    &mut state,
                    &mut z,
                    &mut work,
                    path_id,
                    discount_factor,
                    currency,
                )?
            } else {
                let val = if self.config.antithetic {
                    self.simulate_antithetic_pair(
                        &mut path_rng,
                        process,
                        disc,
                        initial_state,
                        &mut payoff_local,
                        &mut state,
                        &mut state_a,
                        &mut z,
                        &mut z_anti,
                        &mut work,
                        &mut work_anti,
                        currency,
                    )?
                } else {
                    self.simulate_path(
                        &mut path_rng,
                        process,
                        disc,
                        initial_state,
                        &mut payoff_local,
                        &mut state,
                        &mut z,
                        &mut work,
                        currency,
                    )?
                };
                (val, None)
            };

            // Accumulate statistics (skip non-finite values to prevent NaN poisoning)
            let discounted_value = payoff_value * discount_factor;
            if discounted_value.is_finite() {
                stats.update(discounted_value);
            } else {
                num_skipped += 1;
                tracing::warn!(
                    path_id,
                    payoff_value,
                    discount_factor,
                    "Skipping non-finite payoff value in MC statistics"
                );
            }

            // Store captured path
            if let Some(path) = captured_path {
                captured_paths.push(path);
            }

            // Check auto-stop condition
            if let Some(target) = self.config.target_ci_half_width {
                if stats.count() > 1000 && stats.ci_half_width() < target {
                    break;
                }
            }
        }

        // Compute median and percentiles if paths were captured
        let mut estimate = Estimate::new(
            stats.mean(),
            stats.stderr(),
            stats.confidence_interval(0.05),
            stats.count(),
        )
        .with_std_dev(stats.std_dev())
        .with_num_skipped(num_skipped);

        let paths = if capture_enabled {
            let mut dataset = PathDataset::new(stats.count(), sampling_method, process_params);
            for path in captured_paths {
                dataset.add_path(path);
            }
            estimate = apply_captured_path_statistics(estimate, &dataset.paths);
            Some(dataset)
        } else {
            None
        };

        Ok((estimate, paths))
    }

    /// Parallel pricing with path capture.
    #[cfg(feature = "parallel")]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn price_parallel_with_capture<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
        process_params: ProcessParams,
    ) -> Result<(Estimate, Option<PathDataset>)>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // For parallel execution with path capture, we need thread-safe collection
        let capture_enabled = self.config.path_capture.enabled;
        let sampling_method = match &self.config.path_capture.capture_mode {
            PathCaptureMode::All => PathSamplingMethod::All,
            PathCaptureMode::Sample { count, seed } => PathSamplingMethod::RandomSample {
                count: *count,
                seed: *seed,
            },
        };

        let captured_paths: Mutex<Vec<SimulatedPath>> = Mutex::new(Vec::new());

        // Split paths into chunks for parallel processing
        // Use adaptive chunk size if default (1000), otherwise use configured size
        let effective_chunk_size = if self.config.chunk_size == 1000 {
            adaptive_chunk_size(self.config.num_paths)
        } else {
            self.config.chunk_size
        };

        let chunks = parallel_path_chunks(self.config.num_paths, effective_chunk_size);

        // Process chunks in parallel
        let chunk_results: Vec<Result<(OnlineStats, usize)>> = chunks
            .par_iter()
            .map(|range| {
                let mut stats = OnlineStats::new();
                let mut chunk_skipped: usize = 0;
                let dim = process.dim();
                let num_factors = process.num_factors();
                let work_size = disc.work_size(process);

                let mut state = vec![0.0; dim];
                let mut z = vec![0.0; num_factors];
                let mut work = vec![0.0; work_size];
                let mut state_a = vec![0.0; dim];
                let mut z_anti = vec![0.0; num_factors];
                let mut work_anti = vec![0.0; work_size];
                let mut chunk_paths = if capture_enabled {
                    Vec::with_capacity(range.len() / 10 + 1)
                } else {
                    Vec::new()
                };
                let mut payoff_clone = payoff.clone();

                for path_id in range.clone() {
                    let mut path_rng = rng.split(path_id as u64);
                    payoff_clone.reset();
                    payoff_clone.on_path_start(&mut path_rng);

                    let should_capture = capture_enabled
                        && self
                            .config
                            .path_capture
                            .should_capture(path_id, self.config.num_paths);

                    let (payoff_value, captured_path) = if should_capture {
                        self.simulate_path_with_capture(
                            &mut path_rng,
                            process,
                            disc,
                            initial_state,
                            &mut payoff_clone,
                            &mut state,
                            &mut z,
                            &mut work,
                            path_id,
                            discount_factor,
                            currency,
                        )?
                    } else {
                        let val = if self.config.antithetic {
                            self.simulate_antithetic_pair(
                                &mut path_rng,
                                process,
                                disc,
                                initial_state,
                                &mut payoff_clone,
                                &mut state,
                                &mut state_a,
                                &mut z,
                                &mut z_anti,
                                &mut work,
                                &mut work_anti,
                                currency,
                            )?
                        } else {
                            self.simulate_path(
                                &mut path_rng,
                                process,
                                disc,
                                initial_state,
                                &mut payoff_clone,
                                &mut state,
                                &mut z,
                                &mut work,
                                currency,
                            )?
                        };
                        (val, None)
                    };

                    let discounted_value = payoff_value * discount_factor;
                    if discounted_value.is_finite() {
                        stats.update(discounted_value);
                    } else {
                        chunk_skipped += 1;
                        tracing::warn!(
                            path_id,
                            payoff_value,
                            discount_factor,
                            "Skipping non-finite payoff value in MC statistics"
                        );
                    }

                    if let Some(path) = captured_path {
                        chunk_paths.push(path);
                    }
                }

                // Store paths from this chunk
                if !chunk_paths.is_empty() {
                    // SAFETY: A poisoned mutex indicates a prior panic in another thread.
                    // Re-panicking here propagates that failure rather than silently continuing
                    // with potentially corrupted state.
                    #[allow(clippy::expect_used)]
                    captured_paths
                        .lock()
                        .expect("Mutex should not be poisoned")
                        .extend(chunk_paths);
                }

                Ok((stats, chunk_skipped))
            })
            .collect();

        // Collect and handle errors
        let chunk_stats: Vec<(OnlineStats, usize)> =
            chunk_results.into_iter().collect::<Result<Vec<_>>>()?;

        // Deterministically reduce chunk statistics
        let mut combined = OnlineStats::new();
        let mut num_skipped: usize = 0;
        for (chunk_stat, chunk_skipped) in chunk_stats {
            combined.merge(&chunk_stat);
            num_skipped += chunk_skipped;
        }

        let mut estimate = Estimate::new(
            combined.mean(),
            combined.stderr(),
            combined.confidence_interval(0.05),
            combined.count(),
        )
        .with_std_dev(combined.std_dev())
        .with_num_skipped(num_skipped);

        let paths = if capture_enabled {
            let mut dataset = PathDataset::new(combined.count(), sampling_method, process_params);
            // SAFETY: A poisoned mutex indicates a prior panic in another thread.
            // Re-panicking here propagates that failure rather than silently continuing
            // with potentially corrupted state.
            #[allow(clippy::expect_used)]
            let mut collected_paths = captured_paths
                .into_inner()
                .expect("Mutex should not be poisoned");
            // Sort by path_id for deterministic ordering across parallel runs
            collected_paths.sort_by_key(|p| p.path_id);
            for path in collected_paths {
                dataset.add_path(path);
            }
            estimate = apply_captured_path_statistics(estimate, &dataset.paths);

            Some(dataset)
        } else {
            None
        };

        Ok((estimate, paths))
    }

    /// Parallel pricing with path capture (fallback).
    #[cfg(not(feature = "parallel"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn price_parallel_with_capture<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
        process_params: ProcessParams,
    ) -> Result<(Estimate, Option<PathDataset>)>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Fall back to serial
        self.price_serial_with_capture(
            rng,
            process,
            disc,
            initial_state,
            payoff,
            currency,
            discount_factor,
            process_params,
        )
    }

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
    /// Returns the payoff value and optionally the captured path data.
    #[allow(clippy::too_many_arguments)]
    fn simulate_path_with_capture<R, P, D, F>(
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
    ) -> Result<(f64, Option<SimulatedPath>)>
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
            // Use finstack_core IRR calculation
            use finstack_core::cashflow::InternalRateOfReturn;
            if let Ok(irr) = cashflow_amounts.irr(None) {
                simulated_path.set_irr(irr);
            }
        }

        Ok((payoff_value, Some(simulated_path)))
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
