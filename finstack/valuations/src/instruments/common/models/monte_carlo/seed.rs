//! Deterministic seed derivation for Monte Carlo pricing.
//!
//! Provides deterministic seed generation based on instrument ID and scenario
//! name. This ensures that MC-priced instruments produce identical results
//! across multiple runs when computing greeks via finite differences.
//!
//! # Usage
//!
//! ```rust,no_run
//! use finstack_core::types::InstrumentId;
//! use finstack_valuations::instruments::common::models::monte_carlo::seed;
//!
//! let instrument_id = InstrumentId::from("BOND-001");
//!
//! // Base pricing scenario
//! let base_seed = seed::derive_seed(&instrument_id, "base");
//!
//! // Greek calculation scenarios
//! let delta_up_seed = seed::derive_seed_for_metric(&instrument_id, "delta", "up");
//! let delta_down_seed = seed::derive_seed_for_metric(&instrument_id, "delta", "down");
//! # let _ = (base_seed, delta_up_seed, delta_down_seed);
//! ```

use finstack_core::types::InstrumentId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Derive a deterministic seed from an instrument ID and scenario name.
///
/// The seed is computed using a hash of the instrument ID string and scenario name.
/// This ensures that:
/// - Same instrument + same scenario → same seed
/// - Different instruments → different seeds (high probability)
/// - Different scenarios → different seeds
///
/// # Arguments
/// * `instrument_id` - The instrument identifier
/// * `scenario` - Scenario name (e.g., "base", "delta_up", "vega_down")
///
/// # Returns
/// A deterministic u64 seed
pub fn derive_seed(instrument_id: &InstrumentId, scenario: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    instrument_id.as_ref().hash(&mut hasher);
    scenario.hash(&mut hasher);
    hasher.finish()
}

/// Derive a seed for a specific metric calculation scenario.
///
/// Convenience function that constructs scenario name from metric and bump direction.
///
/// # Arguments
/// * `instrument_id` - The instrument identifier
/// * `metric_name` - Name of the metric (e.g., "delta", "vega", "rho")
/// * `bump_direction` - Bump direction ("up", "down", "base")
///
/// # Returns
/// A deterministic u64 seed
pub fn derive_seed_for_metric(
    instrument_id: &InstrumentId,
    metric_name: &str,
    bump_direction: &str,
) -> u64 {
    let scenario = format!("{}_{}", metric_name, bump_direction);
    derive_seed(instrument_id, &scenario)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::types::InstrumentId;

    #[test]
    fn test_seed_determinism() {
        let id = InstrumentId::from("TEST-001");

        // Same inputs should produce same seed
        let seed1 = derive_seed(&id, "base");
        let seed2 = derive_seed(&id, "base");
        assert_eq!(seed1, seed2);

        // Different scenarios should produce different seeds
        let seed_base = derive_seed(&id, "base");
        let seed_delta_up = derive_seed(&id, "delta_up");
        assert_ne!(seed_base, seed_delta_up);
    }

    #[test]
    fn test_seed_for_metric() {
        let id = InstrumentId::from("TEST-002");

        let seed_up = derive_seed_for_metric(&id, "delta", "up");
        let seed_down = derive_seed_for_metric(&id, "delta", "down");

        // Different directions should produce different seeds
        assert_ne!(seed_up, seed_down);

        // Should match explicit scenario construction
        let seed_up_explicit = derive_seed(&id, "delta_up");
        assert_eq!(seed_up, seed_up_explicit);
    }

    #[test]
    fn test_seed_different_instruments() {
        let id1 = InstrumentId::from("TEST-003");
        let id2 = InstrumentId::from("TEST-004");

        let seed1 = derive_seed(&id1, "base");
        let seed2 = derive_seed(&id2, "base");

        // Different instruments should produce different seeds (with high probability)
        assert_ne!(seed1, seed2);
    }
}
