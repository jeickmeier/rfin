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
//! use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;
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
//! ```rust,ignore
//! use finstack_core::dates::{day_count_fraction, DayCount};
//! use time::macros::date;
//!
//! let start = date!(2024-01-15);
//! let end = date!(2025-01-15);
//!
//! // Apply day-count convention
//! let time = day_count_fraction(start, end, DayCount::Act365F);
//!
//! // Create time grid
//! let grid = TimeGrid::uniform(time, 252)?;
//! ```
//!
//! See [CONVENTIONS.md](CONVENTIONS.md) for detailed guidelines.

use finstack_core::Result;
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
    /// ```rust,ignore
    /// // 1 year with 252 trading days
    /// let grid = TimeGrid::uniform(1.0, 252)?;
    /// ```
    pub fn uniform(t_max: f64, num_steps: usize) -> Result<Self> {
        if t_max <= 0.0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        if num_steps == 0 {
            return Err(finstack_core::error::InputError::Invalid.into());
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
    /// ```rust,ignore
    /// // Custom grid with more steps near expiry
    /// let times = vec![0.0, 0.5, 0.75, 0.9, 1.0];
    /// let grid = TimeGrid::from_times(times)?;
    /// ```
    pub fn from_times(times: Vec<f64>) -> Result<Self> {
        if times.is_empty() {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        if times[0] != 0.0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        // Validate monotonicity
        for i in 1..times.len() {
            if times[i] <= times[i - 1] {
                return Err(finstack_core::error::InputError::NonMonotonicKnots.into());
            }
        }

        // Compute time steps
        let mut dts = Vec::with_capacity(times.len() - 1);
        for i in 0..times.len() - 1 {
            dts.push(times[i + 1] - times[i]);
        }

        Ok(Self { times, dts })
    }

    /// Number of time steps.
    pub fn num_steps(&self) -> usize {
        self.dts.len()
    }

    /// Total time span.
    pub fn t_max(&self) -> f64 {
        *self.times.last().unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_grid() {
        let grid = TimeGrid::uniform(1.0, 100).unwrap();
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
        let grid = TimeGrid::from_times(times).unwrap();
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

