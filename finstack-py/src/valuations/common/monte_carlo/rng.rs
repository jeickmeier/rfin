//! Python bindings for the Philox 4×32-10 counter-based RNG.
//!
//! Exposes the Philox RNG so Python users can pre-construct a generator with a
//! known seed or string-based deterministic seed.

use finstack_monte_carlo::rng::philox::PhiloxRng;
use finstack_monte_carlo::traits::RandomStream;
use pyo3::prelude::*;

/// FNV-1a hash matching the one inside `PhiloxRng::deterministic_from_str`.
fn fnv1a(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Philox 4×32-10 counter-based random number generator.
///
/// This is a parallel-friendly, reproducible PRNG used by the Monte Carlo
/// engine. Each seed produces a deterministic stream of random numbers.
///
/// Args:
///     seed: 64-bit integer seed.
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "PhiloxRng",
    frozen
)]
pub(crate) struct PyPhiloxRng {
    seed: u64,
}

#[pymethods]
impl PyPhiloxRng {
    /// Create a Philox RNG with the given numeric seed.
    #[new]
    fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Create a deterministic RNG from a string seed.
    ///
    /// Useful for human-readable reproducible scenarios
    /// (e.g. ``"risk-run-2024-01-15"``).
    ///
    /// Args:
    ///     seed_str: Arbitrary string hashed into a numeric seed.
    ///
    /// Returns:
    ///     A ``PhiloxRng`` instance.
    #[staticmethod]
    fn from_string(seed_str: &str) -> Self {
        Self {
            seed: fnv1a(seed_str),
        }
    }

    /// The numeric seed used by this RNG.
    #[getter]
    fn seed(&self) -> u64 {
        self.seed
    }

    /// Generate ``n`` uniform random numbers in ``[0, 1)``.
    fn uniform(&self, n: usize) -> Vec<f64> {
        let mut rng = PhiloxRng::new(self.seed);
        let mut out = vec![0.0; n];
        rng.fill_u01(&mut out);
        out
    }

    /// Generate ``n`` standard-normal random numbers.
    fn standard_normal(&self, n: usize) -> Vec<f64> {
        let mut rng = PhiloxRng::new(self.seed);
        let mut out = vec![0.0; n];
        rng.fill_std_normals(&mut out);
        out
    }

    fn __repr__(&self) -> String {
        format!("PhiloxRng(seed={})", self.seed)
    }
}

// PhiloxRng is reconstructed from the stored seed when needed by engine.rs.
