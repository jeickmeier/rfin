//! Content-addressed cache key for valuation results.
//!
//! Uses a 128-bit fingerprint of all key components (instrument ID and
//! body-content hash, market data version, per-curve versions, model key,
//! and metrics set) to enable O(1) cache lookup.
//!
//! # 128-bit fingerprinting
//!
//! A single 64-bit hash has a ~1-in-4-billion per-pair collision rate. For
//! an overnight portfolio of 10k-100k keys that can silently return a
//! wrong PV. This module computes two independent SipHashes (by salting
//! the material with distinct byte prefixes) and treats the `(h1, h2)`
//! pair as the key. Effective collision resistance is ~2^64 per pair.
//!
//! # Instrument-body content hash (IMPORTANT)
//!
//! [`CacheKeyInput::instrument_content_hash`] must encode every field of
//! the instrument that affects pricing. If a caller mutates an
//! instrument in-place (e.g. changes a coupon spec or a pricing override)
//! without bumping either the instrument ID or the content hash, the
//! cache will serve a stale, incorrect PV. Callers can compute this hash
//! from:
//!
//! - `std::hash::Hash` on the instrument, when it implements `Hash`; or
//! - a canonical serde_json form of the instrument, hashed with a stable
//!   hasher.
//!
//! The cache does not attempt to compute the content hash itself — that
//! keeps this crate independent of the instrument trait boundary and
//! lets callers batch/cache the hash alongside the instrument.

use crate::metrics::MetricId;
use crate::pricer::ModelKey;
use finstack_core::types::CurveId;
use std::hash::{Hash, Hasher};

/// Domain separator for the first fingerprint hash.
const FP_SALT_A: &[u8] = b"finstack.cache.v1.a:";

/// Domain separator for the second (independent) fingerprint hash.
const FP_SALT_B: &[u8] = b"finstack.cache.v1.b:";

/// Content-addressed cache key for valuation results.
///
/// Two `CacheKey`s are equal if and only if both 64-bit fingerprints
/// match — a 128-bit equality check that makes silent hash collisions
/// astronomically unlikely for practical portfolio sizes.
///
/// The key stores the fingerprint pair for fast equality checks and
/// HashMap lookups.
#[derive(Clone, Debug)]
pub struct CacheKey {
    /// First 64-bit fingerprint (SipHash of salted material).
    hash_a: u64,
    /// Second, independent 64-bit fingerprint.
    hash_b: u64,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.hash_a == other.hash_a && self.hash_b == other.hash_b
    }
}

impl Eq for CacheKey {}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Only mix the first fingerprint into the HashMap bucketing — the
        // second one is used for equality tie-breaking. Mixing both would
        // double the hashing cost without reducing collisions in the
        // HashMap itself (collisions there are resolved by `eq`).
        self.hash_a.hash(state);
    }
}

/// Components that contribute to the cache key fingerprint.
///
/// This struct exists only for key construction: it is hashed
/// immediately and discarded. All slices must be sorted for
/// deterministic hashing.
pub struct CacheKeyInput<'a> {
    /// Instrument identifier (stable across serialization).
    pub instrument_id: &'a str,
    /// Hash of the instrument's pricing-relevant fields.
    ///
    /// **Must** cover every mutable field that affects the valuation,
    /// including `pricing_overrides`. See the module-level docs for
    /// guidance on computing this. Pass `0` only for instruments that
    /// are genuinely immutable after construction — and be aware that
    /// `0` opens the cache to stale reads if that assumption breaks.
    pub instrument_content_hash: u64,
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
    /// Build a cache key by computing a 128-bit fingerprint of all input
    /// components.
    ///
    /// Uses two independent SipHash-1-3 computations with distinct
    /// domain-separation prefixes. Equal fingerprint pairs across two
    /// different inputs would require independent collisions in both
    /// hashes, giving ~2^64 effective collision resistance per pair.
    ///
    /// # Arguments
    ///
    /// * `input` - Key components to hash. Curve versions and metrics
    ///   must be sorted for deterministic results.
    pub fn new(input: &CacheKeyInput<'_>) -> Self {
        // Serialise all components into a deterministic byte buffer.
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(input.instrument_id.as_bytes());
        buf.push(0xFF); // separator to avoid instrument_id/content ambiguity
        buf.extend_from_slice(&input.instrument_content_hash.to_le_bytes());
        buf.extend_from_slice(&input.market_version.to_le_bytes());

        for (curve_id, version) in input.curve_versions {
            buf.extend_from_slice(curve_id.as_str().as_bytes());
            buf.push(0xFE);
            buf.extend_from_slice(&version.to_le_bytes());
        }

        buf.extend_from_slice(&(input.model_key as u16).to_le_bytes());

        for metric in input.metrics {
            buf.extend_from_slice(metric.as_str().as_bytes());
            buf.push(0xFD);
        }

        Self {
            hash_a: siphash_with_salt(FP_SALT_A, &buf),
            hash_b: siphash_with_salt(FP_SALT_B, &buf),
        }
    }

    /// Primary 64-bit fingerprint (HashMap bucketing key and stats).
    pub fn hash_value(&self) -> u64 {
        self.hash_a
    }

    /// Full 128-bit fingerprint as `(hash_a, hash_b)`.
    ///
    /// Useful for diagnostics, on-disk cache indexing, or detecting
    /// unexpected collisions in production (if two distinct inputs ever
    /// produce the same `hash_a` the `hash_b` should still differ).
    pub fn fingerprint(&self) -> (u64, u64) {
        (self.hash_a, self.hash_b)
    }
}

/// Hash `buf` with a domain-separation salt using the standard-library
/// SipHash-1-3. Two distinct salts give two statistically independent
/// 64-bit fingerprints.
fn siphash_with_salt(salt: &[u8], buf: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    salt.hash(&mut hasher);
    buf.hash(&mut hasher);
    hasher.finish()
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
            instrument_content_hash: 0,
            market_version,
            curve_versions,
            model_key,
            metrics,
        }
    }

    fn make_input_with_content<'a>(
        instrument_id: &'a str,
        instrument_content_hash: u64,
        market_version: u64,
        curve_versions: &'a [(CurveId, u64)],
        model_key: ModelKey,
        metrics: &'a [MetricId],
    ) -> CacheKeyInput<'a> {
        CacheKeyInput {
            instrument_id,
            instrument_content_hash,
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

    #[test]
    fn different_content_hash_produces_different_key() {
        // Same instrument_id but different content hash: must miss in cache.
        // This is the regression guard against "stale PV after in-place
        // instrument mutation".
        let curves = vec![(CurveId::new("USD-OIS"), 1)];
        let metrics = vec![MetricId::Dv01];
        let key_v1 = CacheKey::new(&make_input_with_content(
            "BOND-001",
            0x1111_1111_1111_1111,
            1,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        let key_v2 = CacheKey::new(&make_input_with_content(
            "BOND-001",
            0x2222_2222_2222_2222,
            1,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        assert_ne!(key_v1, key_v2);
        assert_ne!(key_v1.fingerprint(), key_v2.fingerprint());
    }

    #[test]
    fn fingerprint_halves_are_independent() {
        // The two 64-bit halves of the fingerprint must not be equal for
        // any realistic input — if they were, the 128-bit collision
        // resistance claim would be false.
        let curves = vec![(CurveId::new("USD-OIS"), 1)];
        let metrics = vec![MetricId::Dv01];
        let key = CacheKey::new(&make_input(
            "BOND-001",
            1,
            &curves,
            ModelKey::Discounting,
            &metrics,
        ));
        let (a, b) = key.fingerprint();
        assert_ne!(a, b);
    }

    #[test]
    fn instrument_id_vs_content_hash_separation() {
        // An instrument_id whose bytes happen to contain a u64-encoded
        // content hash value must not collide with a different split.
        // The 0xFF separator byte between id and content hash enforces
        // this.
        let key1 = CacheKey::new(&make_input_with_content(
            "A",
            0x4242_4242_4242_4242,
            0,
            &[],
            ModelKey::Discounting,
            &[],
        ));
        let key2 = CacheKey::new(&make_input_with_content(
            "AB",
            0x0042_4242_4242_4242,
            0,
            &[],
            ModelKey::Discounting,
            &[],
        ));
        assert_ne!(key1, key2);
    }
}
