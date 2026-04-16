//! PD term structure: cumulative and marginal default probability curves.
//!
//! Stores (tenor, cumulative_pd) pairs with log-linear interpolation on
//! survival probability (equivalent to piecewise-constant hazard rates).

use serde::{Deserialize, Serialize};

use crate::credit::migration::TransitionMatrix;

use super::error::PdCalibrationError;

/// A term structure of cumulative default probabilities.
///
/// Stores (tenor, cumulative_pd) pairs where cumulative PD is monotonically
/// non-decreasing and bounded in [0, 1]. Interpolation is log-linear on
/// survival probability (equivalent to piecewise-constant hazard rates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdTermStructure {
    /// Sorted tenor grid in years.
    tenors: Vec<f64>,
    /// Cumulative default probabilities at each tenor.
    cumulative_pds: Vec<f64>,
}

impl PdTermStructure {
    /// Cumulative default probability at an arbitrary horizon via
    /// log-linear interpolation on survival probability.
    ///
    /// - For t <= first tenor: flat extrapolation (constant hazard rate).
    /// - For t >= last tenor: flat extrapolation of final hazard rate.
    #[must_use]
    pub fn cumulative_pd(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        if self.tenors.is_empty() {
            return 0.0;
        }

        let n = self.tenors.len();

        if t <= self.tenors[0] {
            // Flat extrapolation: constant hazard from time 0 to first tenor
            let h = self.hazard_rate_segment(0);
            1.0 - (-h * t).exp()
        } else if t >= self.tenors[n - 1] {
            // Flat extrapolation of final hazard rate
            let h = self.hazard_rate_segment(n - 1);
            let s_last = 1.0 - self.cumulative_pds[n - 1];
            let dt = t - self.tenors[n - 1];
            1.0 - s_last * (-h * dt).exp()
        } else {
            // Binary search for the enclosing interval
            let idx = match self.tenors.binary_search_by(|probe| {
                probe.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                Ok(i) => return self.cumulative_pds[i],
                Err(i) => i - 1,
            };

            // Log-linear interpolation on survival probability
            let t0 = self.tenors[idx];
            let t1 = self.tenors[idx + 1];
            let s0 = 1.0 - self.cumulative_pds[idx];
            let s1 = 1.0 - self.cumulative_pds[idx + 1];

            if s0 <= 0.0 || s1 <= 0.0 {
                // Degenerate: survival is zero, PD = 1
                return 1.0;
            }

            let frac = (t - t0) / (t1 - t0);
            let ln_s = s0.ln() * (1.0 - frac) + s1.ln() * frac;
            1.0 - ln_s.exp()
        }
    }

    /// Marginal (forward) default probability between t1 and t2,
    /// conditional on survival to t1.
    ///
    /// marginal = (S(t1) - S(t2)) / S(t1)
    ///
    /// Returns 0.0 if t2 <= t1 or survival at t1 is zero.
    #[must_use]
    pub fn marginal_pd(&self, t1: f64, t2: f64) -> f64 {
        if t2 <= t1 {
            return 0.0;
        }
        let s1 = 1.0 - self.cumulative_pd(t1);
        if s1 <= 0.0 {
            return 0.0;
        }
        let s2 = 1.0 - self.cumulative_pd(t2);
        (s1 - s2) / s1
    }

    /// Annualised hazard rate at time t (piecewise constant).
    ///
    /// For t in [t_i, t_{i+1}]: h = -ln(S(t_{i+1})/S(t_i)) / (t_{i+1} - t_i)
    #[must_use]
    pub fn hazard_rate(&self, t: f64) -> f64 {
        if self.tenors.is_empty() {
            return 0.0;
        }

        let n = self.tenors.len();

        if t <= self.tenors[0] {
            self.hazard_rate_segment(0)
        } else if t >= self.tenors[n - 1] {
            self.hazard_rate_segment(n - 1)
        } else {
            let idx = match self.tenors.binary_search_by(|probe| {
                probe.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                Ok(i) => i,
                Err(i) => i - 1,
            };
            self.hazard_rate_between(idx, idx + 1)
        }
    }

    /// Tenor grid.
    #[must_use]
    pub fn tenors(&self) -> &[f64] {
        &self.tenors
    }

    /// Cumulative PDs at the tenor grid points.
    #[must_use]
    pub fn cumulative_pds(&self) -> &[f64] {
        &self.cumulative_pds
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Hazard rate for the first segment (from time 0 to first tenor).
    fn hazard_rate_segment(&self, idx: usize) -> f64 {
        if idx == 0 {
            let s = 1.0 - self.cumulative_pds[0];
            if s <= 0.0 || self.tenors[0] <= 0.0 {
                return 0.0;
            }
            -s.ln() / self.tenors[0]
        } else if idx < self.tenors.len() {
            self.hazard_rate_between(idx.saturating_sub(1), idx)
        } else {
            // Beyond last tenor: use last segment's rate
            let n = self.tenors.len();
            if n < 2 {
                self.hazard_rate_segment(0)
            } else {
                self.hazard_rate_between(n - 2, n - 1)
            }
        }
    }

    /// Hazard rate between two tenor grid points.
    fn hazard_rate_between(&self, i: usize, j: usize) -> f64 {
        let s_i = 1.0 - self.cumulative_pds[i];
        let s_j = 1.0 - self.cumulative_pds[j];
        let dt = self.tenors[j] - self.tenors[i];
        if s_i <= 0.0 || s_j <= 0.0 || dt <= 0.0 {
            return 0.0;
        }
        -(s_j / s_i).ln() / dt
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`PdTermStructure`] from multiple sources.
///
/// Accepts PD data from transition matrices, explicit (tenor, pd) pairs,
/// or other sources. When multiple sources provide data for overlapping
/// tenors, the builder averages them.
///
/// # Examples
///
/// ```
/// use finstack_core::credit::pd::{PdTermStructureBuilder, PdTermStructure};
///
/// let ts = PdTermStructureBuilder::new()
///     .with_cumulative_pds(&[(1.0, 0.002), (3.0, 0.008), (5.0, 0.018)])
///     .build()
///     .expect("valid term structure");
///
/// let pd_2y = ts.cumulative_pd(2.0);
/// assert!(pd_2y > 0.002 && pd_2y < 0.008);
/// ```
pub struct PdTermStructureBuilder {
    points: Vec<(f64, f64)>,
}

impl PdTermStructureBuilder {
    /// Create a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Add explicit (tenor, cumulative_pd) pairs.
    #[must_use]
    pub fn with_cumulative_pds(mut self, pairs: &[(f64, f64)]) -> Self {
        self.points.extend_from_slice(pairs);
        self
    }

    /// Extract cumulative PDs from a [`TransitionMatrix`] for a given
    /// initial rating, at specified tenors (integer years).
    ///
    /// Uses matrix powers: P(n) = P^n, PD(n) = P(n)[rating, default].
    /// Requires the transition matrix to have a defined default state.
    ///
    /// # Errors
    ///
    /// - [`PdCalibrationError::NoDefaultState`] if the matrix has no default state.
    /// - [`PdCalibrationError::UnknownRating`] if `initial_rating` is not in the scale.
    pub fn from_transition_matrix(
        mut self,
        tm: &TransitionMatrix,
        initial_rating: &str,
        tenors: &[f64],
    ) -> Result<Self, PdCalibrationError> {
        let scale = tm.scale();
        let default_idx = scale
            .default_state()
            .ok_or(PdCalibrationError::NoDefaultState)?;
        let rating_idx = scale
            .index_of(initial_rating)
            .ok_or_else(|| PdCalibrationError::UnknownRating {
                rating: initial_rating.to_owned(),
            })?;

        // Compute matrix powers for integer tenors
        let base = tm.as_matrix().clone();
        let n = base.nrows();

        for &tenor in tenors {
            let power = tenor.round() as u32;
            if power == 0 {
                self.points.push((tenor, 0.0));
                continue;
            }

            // Compute base^power by repeated squaring
            let mut result = nalgebra::DMatrix::identity(n, n);
            let mut current_base = base.clone();
            let mut exp = power;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = &result * &current_base;
                }
                current_base = &current_base * &current_base;
                exp /= 2;
            }

            let pd = result[(rating_idx, default_idx)];
            self.points.push((tenor, pd.clamp(0.0, 1.0)));
        }

        Ok(self)
    }

    /// Build the term structure, enforcing monotonicity.
    ///
    /// If cumulative PDs are not monotonically non-decreasing after sorting
    /// by tenor, applies isotonic regression to enforce monotonicity.
    ///
    /// # Errors
    ///
    /// - [`PdCalibrationError::EmptyTermStructure`] if no points were added.
    /// - [`PdCalibrationError::InvalidTenor`] if any tenor is <= 0.
    pub fn build(self) -> Result<PdTermStructure, PdCalibrationError> {
        if self.points.is_empty() {
            return Err(PdCalibrationError::EmptyTermStructure);
        }

        // Validate tenors
        for &(t, _) in &self.points {
            if t <= 0.0 || !t.is_finite() {
                return Err(PdCalibrationError::InvalidTenor { value: t });
            }
        }

        // Sort by tenor, average duplicate tenors
        let mut sorted: Vec<(f64, f64)> = self.points;
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Merge duplicate tenors by averaging
        let mut merged: Vec<(f64, f64)> = Vec::new();
        let mut i = 0;
        while i < sorted.len() {
            let mut j = i + 1;
            let mut sum_pd = sorted[i].1;
            let mut count = 1.0;
            while j < sorted.len() && (sorted[j].0 - sorted[i].0).abs() < 1e-12 {
                sum_pd += sorted[j].1;
                count += 1.0;
                j += 1;
            }
            merged.push((sorted[i].0, sum_pd / count));
            i = j;
        }

        // Clamp PDs to [0, 1]
        for point in &mut merged {
            point.1 = point.1.clamp(0.0, 1.0);
        }

        // Enforce monotonicity via pool-adjacent-violators (isotonic regression)
        let mut pds: Vec<f64> = merged.iter().map(|p| p.1).collect();
        isotonic_regression(&mut pds);

        let tenors: Vec<f64> = merged.iter().map(|p| p.0).collect();

        Ok(PdTermStructure {
            tenors,
            cumulative_pds: pds,
        })
    }
}

impl Default for PdTermStructureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Pool-adjacent-violators algorithm for isotonic (non-decreasing) regression.
fn isotonic_regression(values: &mut [f64]) {
    let n = values.len();
    if n <= 1 {
        return;
    }

    // Forward pass: enforce non-decreasing
    let mut i = 1;
    while i < n {
        if values[i] < values[i - 1] {
            // Pool: average with previous
            let avg = (values[i - 1] + values[i]) / 2.0;
            values[i - 1] = avg;
            values[i] = avg;
            // Walk back to fix earlier violations
            let mut j = i - 1;
            while j > 0 && values[j] < values[j - 1] {
                let pool_avg = (values[j - 1] + values[j]) / 2.0;
                values[j - 1] = pool_avg;
                values[j] = pool_avg;
                j -= 1;
            }
        }
        i += 1;
    }

    // Ensure all values are still in [0, 1]
    for v in values.iter_mut() {
        *v = v.clamp(0.0, 1.0);
    }
}
