//! Content-addressed cache key for valuation results.
//!
//! Uses a 128-bit fingerprint of all key components (instrument ID plus
//! an [`InstrumentFingerprint`], market data version, per-curve versions,
//! model key, and metrics set) to enable O(1) cache lookup.
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
//! [`CacheKeyInput::instrument_fingerprint`] must encode every pricing
//! -relevant field of the instrument whenever the instrument can be
//! mutated in place. If a caller changes a coupon spec or a pricing
//! override without bumping either the instrument ID or the fingerprint,
//! the cache will serve a stale, incorrect PV.
//!
//! The cache therefore forces callers to pick one of two explicit
//! attestations via [`InstrumentFingerprint`]:
//!
//! - [`InstrumentFingerprint::ImmutableById`] — the instrument is frozen
//!   after construction, so the instrument ID alone is sufficient for
//!   cache correctness. Using this for a mutable instrument is a bug.
//! - [`InstrumentFingerprint::ContentHash`] — a non-zero hash derived
//!   from the instrument body (e.g. `std::hash::Hash` on the instrument
//!   or a canonical `serde_json` form hashed with a stable hasher). The
//!   cache does not compute this itself, which keeps the crate
//!   independent of the instrument trait boundary and lets callers
//!   batch/cache the hash alongside the instrument.

use crate::metrics::MetricId;
use crate::pricer::ModelKey;
use finstack_core::types::CurveId;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU64;

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

/// How an instrument's pricing-relevant state is represented in the
/// cache key.
///
/// Callers must pick a variant that matches the instrument's actual
/// mutability contract:
///
/// - [`ImmutableById`](Self::ImmutableById) — the instrument is frozen
///   after construction. The `instrument_id` carries all the identity
///   the cache needs; choose this only when the instrument type
///   genuinely rejects in-place mutation (including through
///   `pricing_overrides`).
/// - [`ContentHash`](Self::ContentHash) — a non-zero 64-bit hash of the
///   instrument's pricing-relevant fields. Using a [`NonZeroU64`]
///   eliminates the "I meant immutable but forgot to set anything"
///   footgun of an ambient `0` default.
///
/// Two fingerprints are encoded distinctly (via a discriminator byte) so
/// that `ImmutableById` and `ContentHash(0xFFFFFFFFFFFFFFFF)` can never
/// alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentFingerprint {
    /// Instrument cannot be mutated after construction; identify by
    /// `instrument_id` alone.
    ImmutableById,
    /// Non-zero 64-bit hash of the instrument's pricing-relevant fields.
    ContentHash(NonZeroU64),
}

impl InstrumentFingerprint {
    /// Construct a [`ContentHash`](Self::ContentHash) fingerprint,
    /// returning `None` when `hash == 0`.
    ///
    /// `0` is reserved to encourage callers to use
    /// [`ImmutableById`](Self::ImmutableById) explicitly rather than
    /// silently falling back to a zero value.
    #[must_use]
    pub fn from_hash(hash: u64) -> Option<Self> {
        NonZeroU64::new(hash).map(Self::ContentHash)
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
    /// Explicit statement of how the instrument's pricing-relevant
    /// state contributes to the key.
    ///
    /// The previous API accepted a bare `u64` with a documented
    /// "`0` means immutable" sentinel; that made silent stale reads
    /// trivially easy to hit when a caller forgot to compute a real
    /// content hash. The enum forces the caller to attest to one
    /// specific contract for every lookup.
    pub instrument_fingerprint: InstrumentFingerprint,
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

        // Discriminator byte distinguishes ImmutableById from ContentHash so
        // that a forgotten content hash can never alias an immutable-by-id
        // lookup (and vice versa).
        match input.instrument_fingerprint {
            InstrumentFingerprint::ImmutableById => {
                buf.push(0x00);
                // Zero-filled slot keeps the wire format aligned with the
                // ContentHash variant.
                buf.extend_from_slice(&0u64.to_le_bytes());
            }
            InstrumentFingerprint::ContentHash(hash) => {
                buf.push(0x01);
                buf.extend_from_slice(&hash.get().to_le_bytes());
            }
        }
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
            instrument_fingerprint: InstrumentFingerprint::ImmutableById,
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
        let fingerprint = InstrumentFingerprint::from_hash(instrument_content_hash)
            .expect("test inputs must supply non-zero content hashes");
        CacheKeyInput {
            instrument_id,
            instrument_fingerprint: fingerprint,
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
    fn from_hash_rejects_zero() {
        assert!(InstrumentFingerprint::from_hash(0).is_none());
        assert!(InstrumentFingerprint::from_hash(1).is_some());
    }

    #[test]
    fn immutable_vs_content_hash_never_alias() {
        // Choosing ImmutableById must not produce the same key as any
        // ContentHash value — the discriminator byte enforces this even
        // if a caller's content hash collides with the zero slot used
        // for the immutable encoding.
        let curves: Vec<(CurveId, u64)> = vec![];
        let metrics: Vec<MetricId> = vec![];
        let key_immutable = CacheKey::new(&CacheKeyInput {
            instrument_id: "BOND-001",
            instrument_fingerprint: InstrumentFingerprint::ImmutableById,
            market_version: 0,
            curve_versions: &curves,
            model_key: ModelKey::Discounting,
            metrics: &metrics,
        });
        let key_with_hash = CacheKey::new(&CacheKeyInput {
            instrument_id: "BOND-001",
            instrument_fingerprint: InstrumentFingerprint::from_hash(1).expect("non-zero"),
            market_version: 0,
            curve_versions: &curves,
            model_key: ModelKey::Discounting,
            metrics: &metrics,
        });
        assert_ne!(key_immutable, key_with_hash);
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
