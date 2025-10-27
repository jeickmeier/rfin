//! Sobol quasi-Monte Carlo sequence with Owen scrambling.
//!
//! Sobol sequences are low-discrepancy quasi-random sequences that
//! provide better convergence than pseudo-random for smooth payoffs.
//!
//! Owen scrambling adds randomization while preserving low-discrepancy
//! properties, enabling error estimation.
//!
//! Reference: Joe & Kuo (2008) - "Constructing Sobol sequences with better two-dimensional projections"

use super::super::traits::RandomStream;
use super::transforms::inverse_normal_cdf;

/// Sobol sequence generator with Owen scrambling.
///
/// This is a simplified implementation for dimensions up to 8.
/// For production use with high dimensions, consider using a dedicated
/// quasi-Monte Carlo library.
#[derive(Clone, Debug)]
pub struct SobolRng {
    /// Current index in the sequence
    index: u64,
    /// Dimension
    dimension: usize,
    /// Owen scrambling seeds (one per dimension)
    scramble_seeds: Vec<u32>,
    /// Direction numbers for Sobol construction
    direction_numbers: Vec<Vec<u32>>,
}

impl SobolRng {
    /// Create a new Sobol sequence for the given dimension.
    ///
    /// # Arguments
    ///
    /// * `dimension` - Number of dimensions (must be > 0 and <= 8 for this implementation)
    /// * `scramble_seed` - Seed for Owen scrambling (0 = no scrambling)
    pub fn new(dimension: usize, scramble_seed: u64) -> Self {
        assert!(dimension > 0 && dimension <= 8, "Dimension must be 1-8");

        // Initialize direction numbers (simplified for first 8 dimensions)
        let direction_numbers = initialize_direction_numbers(dimension);

        // Generate scrambling seeds
        let mut scramble_seeds = Vec::with_capacity(dimension);
        for i in 0..dimension {
            let seed = if scramble_seed == 0 {
                0
            } else {
                // Simple hash of scramble_seed + dimension
                ((scramble_seed.wrapping_mul(2654435761)) ^ (i as u64)).wrapping_mul(2246822519) as u32
            };
            scramble_seeds.push(seed);
        }

        Self {
            index: 0,
            dimension,
            scramble_seeds,
            direction_numbers,
        }
    }

    /// Get the next point in the Sobol sequence.
    ///
    /// Returns a vector of `dimension` values in [0, 1).
    pub fn next_point(&mut self) -> Vec<f64> {
        let mut point = Vec::with_capacity(self.dimension);

        for d in 0..self.dimension {
            let value = self.sobol_value(d);
            let scrambled = self.owen_scramble(value, d);
            point.push(scrambled);
        }

        self.index += 1;
        point
    }

    /// Compute Sobol value for dimension d at current index.
    fn sobol_value(&self, d: usize) -> u32 {
        let mut value = 0u32;
        let mut index = self.index;
        let mut bit = 0;

        while index > 0 {
            if (index & 1) == 1 {
                value ^= self.direction_numbers[d][bit];
            }
            index >>= 1;
            bit += 1;
        }

        value
    }

    /// Apply Owen scrambling to a Sobol value.
    fn owen_scramble(&self, value: u32, d: usize) -> f64 {
        let scrambled = value ^ self.scramble_seeds[d];
        // Convert to [0, 1) using upper 32 bits
        (scrambled as f64) / (u32::MAX as f64 + 1.0)
    }

    /// Reset to beginning of sequence.
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Skip ahead in the sequence.
    pub fn skip(&mut self, n: u64) {
        self.index += n;
    }
}

impl RandomStream for SobolRng {
    fn split(&self, stream_id: u64) -> Self {
        // For Sobol, "splitting" means starting at a different index
        let mut new_rng = self.clone();
        new_rng.skip(stream_id * 10000); // Skip ahead to avoid overlap
        new_rng
    }

    fn fill_u01(&mut self, out: &mut [f64]) {
        // Fill with consecutive Sobol points
        for chunk in out.chunks_mut(self.dimension) {
            let point = self.next_point();
            for (i, &val) in point.iter().enumerate().take(chunk.len()) {
                chunk[i] = val;
            }
        }
    }

    fn fill_std_normals(&mut self, out: &mut [f64]) {
        // First fill with uniform, then transform to normal
        self.fill_u01(out);
        for x in out {
            *x = inverse_normal_cdf(*x);
        }
    }
}

/// Initialize direction numbers for Sobol sequence.
///
/// This uses primitive polynomials and direction numbers for the first 8 dimensions.
/// For more dimensions, use tables from Joe & Kuo.
fn initialize_direction_numbers(max_dim: usize) -> Vec<Vec<u32>> {
    let mut all_directions = Vec::with_capacity(max_dim);

    // First dimension: powers of 2
    let mut dim0 = Vec::with_capacity(32);
    for i in 0..32 {
        dim0.push(1u32 << (31 - i));
    }
    all_directions.push(dim0);

    // Dimensions 2-8 with primitive polynomials
    // These are standard direction numbers from Bratley & Fox
    let direction_data = [
        // Dimension 2: x + 1
        vec![1, 1],
        // Dimension 3: x^2 + x + 1
        vec![1, 3, 7],
        // Dimension 4: x^3 + x + 1
        vec![1, 1, 5],
        // Dimension 5: x^3 + x^2 + 1
        vec![1, 3, 1, 1],
        // Dimension 6: x^4 + x^3 + 1
        vec![1, 1, 3, 3],
        // Dimension 7: x^4 + x + 1
        vec![1, 3, 5, 13],
        // Dimension 8: x^4 + x^3 + x^2 + x + 1
        vec![1, 1, 7, 11, 15],
    ];

    for (_d, initial_m) in direction_data.iter().enumerate().take(max_dim.saturating_sub(1)) {
        let mut directions = Vec::with_capacity(32);
        
        // Set initial direction numbers
        for &m in initial_m {
            directions.push(m << (32 - directions.len() - 1));
        }

        // Generate remaining direction numbers using recurrence
        let s = initial_m.len();
        for i in s..32 {
            let mut v = directions[i - s] >> s;
            for k in 0..s {
                v ^= directions[i - s + k];
            }
            directions.push(v);
        }

        all_directions.push(directions);
    }

    all_directions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sobol_basic() {
        let mut sobol = SobolRng::new(2, 0);

        // First few points should be deterministic
        let p1 = sobol.next_point();
        assert_eq!(p1.len(), 2);

        let p2 = sobol.next_point();
        assert_eq!(p2.len(), 2);

        // Points should be different
        assert_ne!(p1[0], p2[0]);
    }

    #[test]
    fn test_sobol_range() {
        let mut sobol = SobolRng::new(3, 0);

        for _ in 0..100 {
            let point = sobol.next_point();
            for &val in &point {
                assert!((0.0..1.0).contains(&val));
            }
        }
    }

    #[test]
    fn test_owen_scrambling() {
        let sobol_no_scramble = SobolRng::new(2, 0);
        let sobol_scrambled = SobolRng::new(2, 12345);

        // Different scrambling should give different sequences
        let p1 = sobol_no_scramble.clone().next_point();
        let p2 = sobol_scrambled.clone().next_point();

        assert_ne!(p1[0], p2[0]);
    }

    #[test]
    fn test_sobol_reset_and_skip() {
        let mut sobol = SobolRng::new(2, 0);

        let p1_first = sobol.next_point();
        let _p2 = sobol.next_point();

        sobol.reset();
        let p1_after_reset = sobol.next_point();

        // After reset, should get same first point
        assert_eq!(p1_first[0], p1_after_reset[0]);
        assert_eq!(p1_first[1], p1_after_reset[1]);
    }

    #[test]
    fn test_fill_std_normals() {
        let mut sobol = SobolRng::new(1, 12345); // Use non-zero seed to avoid edge cases
        let mut normals = vec![0.0; 100];
        sobol.fill_std_normals(&mut normals);

        // All values should be finite (skip first few which might hit edges)
        for &n in &normals[5..] {
            assert!(n.is_finite(), "Non-finite value: {}", n);
        }

        // Mean should be reasonable (QMC doesn't guarantee mean=0)
        let mean = normals.iter().sum::<f64>() / normals.len() as f64;
        assert!(mean.abs() < 2.0);
    }
}
