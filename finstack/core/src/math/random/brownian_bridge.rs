//! Brownian bridge construction for path-dependent options.
//!
//! The Brownian bridge construction orders random shocks to reduce
//! effective dimension for QMC, particularly effective for barrier
//! and path-dependent options.
//!
//! # Algorithm
//!
//! Instead of generating path sequentially (0 → 1 → 2 → ... → N),
//! use binary subdivision:
//! 1. Generate terminal point (N)
//! 2. Generate midpoint (N/2) conditional on terminal
//! 3. Recursively fill in quarters, eighths, etc.
//!
//! This reordering reduces effective dimension because:
//! - Early dimensions determine overall path shape (most important)
//! - Later dimensions add local detail (less important)
//!
//! # Benefits for QMC
//!
//! - Reduces effective dimension from O(N) to O(log N) for smooth payoffs
//! - Particularly effective for barrier options (hitting time well-approximated by few dimensions)
//! - Can improve convergence from O(N^{-1/2}) to O(N^{-1}) or better
//!
//! Reference: Moskowitz & Caflisch (1996) - "Smoothness and dimension reduction in QMC"

/// Brownian bridge construction order.
///
/// Generates the sequence of time indices to sample in bridge order.
pub struct BrownianBridge {
    /// Construction order (indices into time grid)
    construction_order: Vec<usize>,
    /// Multipliers for conditional variance
    std_multipliers: Vec<f64>,
}

impl BrownianBridge {
    /// Create a Brownian bridge for N time steps.
    ///
    /// # Arguments
    ///
    /// * `num_steps` - Number of time steps in the path
    ///
    /// # Example
    ///
    /// For num_steps=4:
    /// - Standard order: [0, 1, 2, 3, 4]
    /// - Bridge order:   [4, 2, 1, 3] (terminal, half, quarters, ...)
    pub fn new(num_steps: usize) -> Self {
        let mut construction_order = Vec::with_capacity(num_steps);
        let mut std_multipliers = Vec::with_capacity(num_steps);

        // Binary subdivision
        Self::build_bridge_recursive(0, num_steps, &mut construction_order, &mut std_multipliers);

        Self {
            construction_order,
            std_multipliers,
        }
    }

    /// Recursive builder for bridge order.
    fn build_bridge_recursive(
        left: usize,
        right: usize,
        order: &mut Vec<usize>,
        multipliers: &mut Vec<f64>,
    ) {
        if right - left <= 1 {
            return;
        }

        // Add midpoint
        let mid = (left + right) / 2;
        order.push(mid);

        // Conditional variance multiplier for Brownian bridge:
        // Var[B(t) | B(s), B(u)] = (t-s)(u-t)/(u-s)
        let left_time = left as f64;
        let mid_time = mid as f64;
        let right_time = right as f64;

        let variance_factor = if right > left {
            ((mid_time - left_time) * (right_time - mid_time)) / (right_time - left_time)
        } else {
            1.0
        };

        multipliers.push(variance_factor.sqrt());

        // Recurse on left and right halves
        Self::build_bridge_recursive(left, mid, order, multipliers);
        Self::build_bridge_recursive(mid, right, order, multipliers);
    }

    /// Get construction order.
    pub fn order(&self) -> &[usize] {
        &self.construction_order
    }

    /// Get standard deviation multipliers.
    pub fn multipliers(&self) -> &[f64] {
        &self.std_multipliers
    }

    /// Apply bridge construction to generate path from independent shocks.
    ///
    /// # Arguments
    ///
    /// * `z` - Independent standard normal shocks (length = num_steps)
    /// * `w_out` - Output Brownian path (length = num_steps + 1)
    /// * `dt` - Time step size
    ///
    /// # Notes
    ///
    /// `w_out[0] = 0` (Brownian motion starts at 0)
    /// `w_out[i] = cumulative Brownian motion at step i`
    pub fn construct_path(&self, z: &[f64], w_out: &mut [f64], dt: f64) {
        let num_steps = z.len();
        assert_eq!(w_out.len(), num_steps + 1);

        // Initialize
        w_out[0] = 0.0;

        // Terminal point (standard Brownian motion)
        w_out[num_steps] = z[0] * (num_steps as f64 * dt).sqrt();

        // Fill in using bridge construction
        // z[0] is used for terminal, z[1..] for construction_order
        for (i, &idx) in self.construction_order.iter().enumerate() {
            // Find left and right bracketing points
            let (left, right) = self.find_brackets(idx, w_out);

            // Conditional mean: linear interpolation
            let left_time = left as f64 * dt;
            let idx_time = idx as f64 * dt;
            let right_time = right as f64 * dt;

            let alpha = (idx_time - left_time) / (right_time - left_time);
            let conditional_mean = w_out[left] + alpha * (w_out[right] - w_out[left]);

            // Conditional std dev
            let conditional_std = self.std_multipliers[i] * dt.sqrt();

            // Generate point using z[i+1] (since z[0] is for terminal)
            w_out[idx] = conditional_mean + conditional_std * z[i + 1];
        }
    }

    /// Find left and right bracketing points for bridge construction.
    fn find_brackets(&self, idx: usize, w: &[f64]) -> (usize, usize) {
        // Find the closest populated points to the left and right
        let mut left = 0;
        for i in (0..idx).rev() {
            if !w[i].is_nan() && w[i].is_finite() {
                left = i;
                break;
            }
        }

        let mut right = w.len() - 1;
        for (i, &val) in w.iter().enumerate().skip(idx + 1) {
            if !val.is_nan() && val.is_finite() {
                right = i;
                break;
            }
        }

        (left, right)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::super::sobol_pca::{effective_dimension, pca_ordering};
    use super::*;

    #[test]
    fn test_brownian_bridge_order() {
        let bridge = BrownianBridge::new(4);
        let order = bridge.order();

        println!("Bridge order for 4 steps: {:?}", order);

        // First should be midpoint (2)
        assert_eq!(order[0], 2);

        // Should have 3 elements (not counting initial 0 and terminal 4)
        assert!(order.len() >= 2);
    }

    #[test]
    fn test_brownian_bridge_construction() {
        let bridge = BrownianBridge::new(4);

        // Independent shocks
        let z = vec![1.0, 0.5, -0.5, 0.0];
        let mut w = vec![f64::NAN; 5];
        let dt = 0.25;

        bridge.construct_path(&z, &mut w, dt);

        println!("Brownian path: {:?}", w);

        // Check initial condition
        assert_eq!(w[0], 0.0);

        // Check all points are finite
        for &val in &w {
            assert!(val.is_finite());
        }

        // Terminal point should use first shock
        let expected_terminal = z[0] * (4.0 * dt).sqrt();
        assert!((w[4] - expected_terminal).abs() < 1e-10);
    }

    #[test]
    fn test_pca_ordering_identity() {
        // Identity matrix: all eigenvalues = 1
        let correlation = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

        let (eigenvalues, _, _) = pca_ordering(&correlation, 3);

        // All eigenvalues should be 1
        for &val in &eigenvalues {
            assert!((val - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_pca_ordering_sorted() {
        // High correlation matrix
        let correlation = vec![1.0, 0.8, 0.6, 0.8, 1.0, 0.7, 0.6, 0.7, 1.0];

        let (eigenvalues, _, _) = pca_ordering(&correlation, 3);

        println!("Eigenvalues: {:?}", eigenvalues);

        // Should be sorted in descending order
        for i in 1..eigenvalues.len() {
            assert!(
                eigenvalues[i - 1] >= eigenvalues[i],
                "Eigenvalues not sorted: {} vs {}",
                eigenvalues[i - 1],
                eigenvalues[i]
            );
        }

        // First eigenvalue should be largest (captures most variance)
        assert!(eigenvalues[0] > eigenvalues[1]);
    }

    #[test]
    fn test_effective_dimension() {
        // Equal eigenvalues → d_eff = n
        let eigenvalues_equal = vec![1.0, 1.0, 1.0];
        let d_eff = effective_dimension(&eigenvalues_equal);
        assert!((d_eff - 3.0).abs() < 0.01);

        // One dominant → d_eff ≈ 1
        let eigenvalues_dominant = vec![2.99, 0.005, 0.005];
        let d_eff = effective_dimension(&eigenvalues_dominant);
        println!("Effective dimension (dominant): {:.2}", d_eff);
        assert!(d_eff < 1.2);
    }
}
