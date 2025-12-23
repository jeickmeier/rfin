//! Time grids for Monte Carlo simulation.
//!
//! Provides uniform and custom time grids with validation.
//!
//! # Time Convention
//!
//! Time grids operate on **year fractions** (f64), not calendar dates.
//! The MC engine is agnostic to day-count conventions.
//!
//! ## Design Philosophy
//!
//! - **MC Layer**: Pure mathematical time (this module)
//! - **Instrument Layer**: Converts dates → year fractions using `finstack_core::dates`
//!
//! ## Usage
//!
//! ```rust
//! # use finstack_core::Result;
//! # fn main() -> Result<()> {
//! use finstack_core::math::time_grid::TimeGrid;
//!
//! // Uniform grid: 1 year with 252 trading days
//! let grid = TimeGrid::uniform(1.0, 252)?;
//!
//! // Custom grid with irregular periods
//! let times = vec![0.0, 0.25, 0.5, 0.75, 1.0]; // Quarterly
//! let grid = TimeGrid::from_times(times)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Converting from Dates
//!
//! Use `finstack_core::dates` to convert calendar dates to year fractions:
//!
//! ```rust,no_run
//! use finstack_core::dates::{DayCount, DayCountCtx};
//! use finstack_core::math::time_grid::TimeGrid;
//! use time::macros::date;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let start = date!(2024-01-15);
//! let end = date!(2025-01-15);
//!
//! // Apply day-count convention
//! let time = DayCount::Act365F.year_fraction(start, end, DayCountCtx::default())?;
//!
//! // Create time grid
//! let grid = TimeGrid::uniform(time, 252)?;
//! # let _ = grid;
//! # Ok(())
//! # }
//! ```
//!
//! See the Monte Carlo conventions doc in `finstack-valuations` for detailed guidelines.

use crate::dates::{Date, DayCount, DayCountCtx};
use crate::Result;
use thiserror::Error;

/// Time grid for Monte Carlo simulation.
///
/// Defines the discretization points in time from t=0 to t=T.
#[derive(Clone, Debug)]
pub struct TimeGrid {
    /// Time points in years (monotonically increasing)
    times: Vec<f64>,
    /// Time steps (dt[i] = times[i+1] - times[i])
    dts: Vec<f64>,
}

/// Error type for time grid construction and validation
#[derive(Debug, Error)]
#[error("Invalid time grid: {0}")]
pub struct TimeGridError(String);

impl TimeGrid {
    /// Create a uniform time grid from 0 to T with N steps.
    ///
    /// # Arguments
    ///
    /// * `t_max` - Final time in years (must be > 0)
    /// * `num_steps` - Number of time steps (must be > 0)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// // 1 year with 252 trading days
    /// use finstack_core::math::time_grid::TimeGrid;
    /// # fn main() -> finstack_core::Result<()> {
    /// let grid = TimeGrid::uniform(1.0, 252)?;
    /// # let _ = grid;
    /// # Ok(())
    /// # }
    /// ```
    pub fn uniform(t_max: f64, num_steps: usize) -> Result<Self> {
        if t_max <= 0.0 {
            return Err(crate::error::InputError::Invalid.into());
        }
        if num_steps == 0 {
            return Err(crate::error::InputError::Invalid.into());
        }

        let dt = t_max / num_steps as f64;
        let mut times = Vec::with_capacity(num_steps + 1);
        let mut dts = Vec::with_capacity(num_steps);

        times.push(0.0);
        for i in 1..=num_steps {
            times.push(i as f64 * dt);
            dts.push(dt);
        }

        Ok(Self { times, dts })
    }

    /// Create a custom time grid from explicit time points.
    ///
    /// # Arguments
    ///
    /// * `times` - Monotonically increasing time points (must start at 0)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// // Custom grid with more steps near expiry
    /// use finstack_core::math::time_grid::TimeGrid;
    /// # fn main() -> finstack_core::Result<()> {
    /// let times = vec![0.0, 0.5, 0.75, 0.9, 1.0];
    /// let grid = TimeGrid::from_times(times)?;
    /// # let _ = grid;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_times(times: Vec<f64>) -> Result<Self> {
        if times.is_empty() {
            return Err(crate::error::InputError::Invalid.into());
        }
        if times[0] != 0.0 {
            return Err(crate::error::InputError::Invalid.into());
        }

        // Validate monotonicity and check for duplicate/near-duplicate times
        const MIN_DT_THRESHOLD: f64 = 1e-12;
        for i in 1..times.len() {
            if times[i] <= times[i - 1] {
                return Err(crate::error::InputError::NonMonotonicKnots.into());
            }
            // Check for duplicate or near-duplicate time points
            if (times[i] - times[i - 1]).abs() < MIN_DT_THRESHOLD {
                return Err(crate::error::InputError::Invalid.into());
            }
        }

        // Compute time steps
        let mut dts = Vec::with_capacity(times.len() - 1);
        for i in 0..times.len() - 1 {
            dts.push(times[i + 1] - times[i]);
        }

        // Check for minimum dt to prevent numerical issues
        const MIN_DT: f64 = 1e-10;
        if let Some(&min_dt) = dts
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        {
            if min_dt < MIN_DT {
                return Err(crate::error::InputError::Invalid.into());
            }
        }

        Ok(Self { times, dts })
    }

    /// Number of time steps.
    pub fn num_steps(&self) -> usize {
        self.dts.len()
    }

    /// Total time span.
    pub fn t_max(&self) -> f64 {
        *self
            .times
            .last()
            .expect("TimeGrid should have at least one time point")
    }

    /// Get time at step i.
    pub fn time(&self, step: usize) -> f64 {
        self.times[step]
    }

    /// Get time step size at step i (dt[i] = t[i+1] - t[i]).
    pub fn dt(&self, step: usize) -> f64 {
        self.dts[step]
    }

    /// Get all time points.
    pub fn times(&self) -> &[f64] {
        &self.times
    }

    /// Get all time steps.
    pub fn dts(&self) -> &[f64] {
        &self.dts
    }

    /// Check if grid is uniform (all dts equal within tolerance).
    pub fn is_uniform(&self) -> bool {
        if self.dts.is_empty() {
            return true;
        }
        let first_dt = self.dts[0];
        let tol = 1e-10;
        self.dts.iter().all(|&dt| (dt - first_dt).abs() < tol)
    }
}

/// Map Bermudan exercise dates (as year fractions relative to maturity) to step indices.
pub fn map_exercise_dates_to_steps(
    exercise_dates: &[f64],
    total_time: f64,
    steps: usize,
) -> Vec<usize> {
    let mut out = Vec::new();
    if total_time <= 0.0 || steps == 0 {
        return out;
    }
    for &ex_time in exercise_dates {
        let ratio = if total_time != 0.0 {
            ex_time / total_time
        } else {
            0.0
        };
        let step = (ratio * steps as f64).round() as usize;
        if step <= steps {
            out.push(step);
        }
    }
    out
}

/// Map a calendar date to a step index using a day-count convention.
pub fn map_date_to_step(
    base_date: Date,
    event_date: Date,
    maturity_date: Date,
    steps: usize,
    dc: DayCount,
) -> usize {
    let ttm = dc
        .year_fraction(base_date, maturity_date, DayCountCtx::default())
        .unwrap_or(0.0);
    if ttm <= 0.0 || steps == 0 {
        return 0;
    }
    let t_event = dc
        .year_fraction(base_date, event_date, DayCountCtx::default())
        .unwrap_or(0.0)
        .clamp(0.0, ttm);
    let step_index = ((t_event / ttm) * steps as f64).round() as usize;
    step_index.min(steps)
}

/// Map multiple calendar dates to step indices.
pub fn map_dates_to_steps(
    base_date: Date,
    dates: &[Date],
    maturity_date: Date,
    steps: usize,
    dc: DayCount,
) -> Vec<usize> {
    dates
        .iter()
        .map(|&d| map_date_to_step(base_date, d, maturity_date, steps, dc))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_grid() {
        let grid =
            TimeGrid::uniform(1.0, 100).expect("Uniform grid creation should succeed in test");
        assert_eq!(grid.num_steps(), 100);
        assert_eq!(grid.t_max(), 1.0);
        assert!(grid.is_uniform());
        assert_eq!(grid.dt(0), 0.01);
        assert_eq!(grid.time(0), 0.0);
        assert_eq!(grid.time(100), 1.0);
    }

    #[test]
    fn test_custom_grid() {
        let times = vec![0.0, 0.1, 0.5, 1.0];
        let grid = TimeGrid::from_times(times).expect("TimeGrid creation should succeed in test");
        assert_eq!(grid.num_steps(), 3);
        assert_eq!(grid.t_max(), 1.0);
        assert!(!grid.is_uniform());
        assert_eq!(grid.dt(0), 0.1);
        assert_eq!(grid.dt(1), 0.4);
        assert_eq!(grid.dt(2), 0.5);
    }

    #[test]
    fn test_invalid_grids() {
        // Zero t_max
        assert!(TimeGrid::uniform(0.0, 100).is_err());
        // Zero steps
        assert!(TimeGrid::uniform(1.0, 0).is_err());
        // Empty times
        assert!(TimeGrid::from_times(vec![]).is_err());
        // Doesn't start at 0
        assert!(TimeGrid::from_times(vec![0.1, 0.5, 1.0]).is_err());
        // Non-monotonic
        assert!(TimeGrid::from_times(vec![0.0, 0.5, 0.3, 1.0]).is_err());
    }
}
