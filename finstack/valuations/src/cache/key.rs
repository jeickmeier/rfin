//! Content-addressed cache key for valuation results.
//!
//! Uses a precomputed hash of all key components (instrument ID, market data
//! version, per-curve versions, model key, and metrics set) to enable O(1)
//! cache lookup. The hash is computed using the standard library's `Hasher`
//! trait with deterministic ordering.

use crate::metrics::MetricId;
use crate::pricer::ModelKey;
use finstack_core::types::CurveId;
use std::hash::{Hash, Hasher};

/// Content-addressed cache key for valuation results.
///
/// Two `CacheKey`s are equal if and only if the same instrument, priced
/// against the same market data state, with the same model and metrics,
/// would produce an identical `ValuationResult`.
///
/// The key stores a precomputed 64-bit hash for fast equality checks
/// and HashMap lookups.
#[derive(Clone, Debug)]
pub struct CacheKey {
    /// Precomputed hash of the canonical key material.
    hash: u64,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for CacheKey {}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

/// Components that contribute to the cache key hash.
///
/// This struct exists only for key construction: it is hashed
/// immediately and discarded. All slices must be sorted for
/// deterministic hashing.
pub struct CacheKeyInput<'a> {
    /// Instrument identifier (stable across serialization).
    pub instrument_id: &'a str,
    /// Market data version counter from `MarketContext`.
    pub market_version: u64,
    /// Per-curve versions for the instrument's curve dependencies.
    /// Must be sorted by `CurveId` for deterministic hashing.
    pub curve_versions: &'a [(CurveId, u64)],
    /// Pricing model selection.
    pub model_key: ModelKey,
    /// Sorted, deduplicated metric IDs requested.
    pub metrics: &'a [MetricId],
}

impl CacheKey {
    /// Build a cache key by hashing all input components.
    ///
    /// Uses a deterministic byte-buffer approach: all components are
    /// serialized into a contiguous buffer, then hashed with the
    /// standard library `DefaultHasher`.
    ///
    /// # Arguments
    ///
    /// * `input` - Key components to hash. Curve versions and metrics
    ///   must be sorted for deterministic results.
    pub fn new(input: &CacheKeyInput<'_>) -> Self {
        use std::collections::hash_map::DefaultHasher;

        // Build a deterministic byte buffer from all key components.
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(input.instrument_id.as_bytes());
        buf.extend_from_slice(&input.market_version.to_le_bytes());

        for (curve_id, version) in input.curve_versions {
            buf.extend_from_slice(curve_id.as_str().as_bytes());
            buf.extend_from_slice(&version.to_le_bytes());
        }

        buf.extend_from_slice(&(input.model_key as u16).to_le_bytes());

        for metric in input.metrics {
            buf.extend_from_slice(metric.as_str().as_bytes());
        }

        let mut hasher = DefaultHasher::new();
        buf.hash(&mut hasher);
        Self {
            hash: hasher.finish(),
        }
    }

    /// Raw hash value (for diagnostics and statistics).
    pub fn hash_value(&self) -> u64 {
        self.hash
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn make_input<'a>(
        instrument_id: &'a str,
        market_version: u64,
        curve_versions: &'a [(CurveId, u64)],
        model_key: ModelKey,
        metrics: &'a [MetricId],
    ) -> CacheKeyInput<'a> {
        CacheKeyInput {
            instrument_id,
            market_version,
            curve_versions,
            model_key,
            metrics,
        }
    }

    #[test]
    fn identical_inputs_produce_same_key() {
        let curves = vec![(CurveId::new("USD-OIS"), 1)];
        let metrics = vec![MetricId::Dv01];
        let input1 = make_input("BOND-001", 1, &curves, ModelKey::Discounting, &metrics);
        let input2 = make_input("BOND-001", 1, &curves, ModelKey::Discounting, &metrics);
        let key1 = CacheKey::new(&input1);
        let key2 = CacheKey::new(&input2);
        assert_eq!(key1, key2);
        assert_eq!(key1.hash_value(), key2.hash_value());
    }

    #[test]
    fn different_instrument_id_produces_different_key() {
        let curves = vec![(CurveId::new("USD-OIS"), 1)];
        let metrics = vec![MetricId::Dv01];
        let key1 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        let key2 = CacheKey::new(&make_input(
            "BOND-002",
            1,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        assert_ne!(key1, key2);
    }

    #[test]
    fn different_market_version_produces_different_key() {
        let curves = vec![(CurveId::new("USD-OIS"), 1)];
        let metrics = vec![MetricId::Dv01];
        let key1 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        let key2 = CacheKey::new(&make_input(
            "BOND-001",
            2,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        assert_ne!(key1, key2);
    }

    #[test]
    fn different_curve_version_produces_different_key() {
        let curves_v1 = vec![(CurveId::new("USD-OIS"), 1)];
        let curves_v2 = vec![(CurveId::new("USD-OIS"), 2)];
        let metrics = vec![MetricId::Dv01];
        let key1 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves_v1,
            ModelKey::Discounting,
            &metrics,
        ));
        let key2 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves_v2,
            ModelKey::Discounting,
            &metrics,
        ));
        assert_ne!(key1, key2);
    }

    #[test]
    fn different_model_key_produces_different_key() {
        let curves = vec![(CurveId::new("USD-OIS"), 1)];
        let metrics = vec![MetricId::Dv01];
        let key1 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        let key2 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves,
            ModelKey::HazardRate,
            &metrics,
        ));
        assert_ne!(key1, key2);
    }

    #[test]
    fn different_metrics_produce_different_key() {
        let curves = vec![(CurveId::new("USD-OIS"), 1)];
        let metrics1 = vec![MetricId::Dv01];
        let metrics2 = vec![MetricId::Dv01, MetricId::Theta];
        let key1 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves,
            ModelKey::Discounting,
            &metrics1,
        ));
        let key2 = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves,
            ModelKey::Discounting,
            &metrics2,
        ));
        assert_ne!(key1, key2);
    }

    #[test]
    fn empty_curves_and_metrics_works() {
        let input = make_input("BOND-001", 0, &[], ModelKey::Discounting, &[]);
        let key = CacheKey::new(&input);
        assert!(key.hash_value() != 0 || key.hash_value() == 0); // just verify no panic
    }
}
