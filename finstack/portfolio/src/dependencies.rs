//! Portfolio-level dependency index for selective repricing.
//!
//! Provides a normalized key model ([`MarketFactorKey`]) and an inverted index
//! ([`DependencyIndex`]) that maps market factor keys to the set of portfolio
//! positions that depend on them. The index is built from each instrument's
//! [`finstack_valuations::instruments::MarketDependencies`] and enables
//! efficient lookup of affected positions
//! when a subset of market data changes.

use finstack_core::currency::Currency;
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use finstack_core::HashSet;
use finstack_valuations::instruments::common::traits::RatesCurveKind;
use finstack_valuations::instruments::MarketDependencies;

/// Normalized market factor key for portfolio-level dependency tracking.
///
/// Each variant captures enough information to uniquely identify one atomic
/// market data input.  The key space is intentionally broader than curves
/// alone so the index can route spot, vol, FX, and series changes without
/// a second abstraction layer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MarketFactorKey {
    /// A rate curve identified by curve ID and kind (discount / forward / credit).
    Curve {
        /// Curve identifier matching [`CurveId`] in market data.
        id: CurveId,
        /// Curve kind (discount, forward, credit).
        kind: RatesCurveKind,
    },
    /// Equity, commodity, or other spot price identifier.
    Spot(String),
    /// Volatility surface identifier.
    VolSurface(String),
    /// FX pair (base/quote currencies).
    Fx {
        /// Base currency.
        base: Currency,
        /// Quote currency.
        quote: Currency,
    },
    /// Scalar time series identifier (e.g., OHLC history for realized variance).
    Series(String),
}

impl MarketFactorKey {
    /// Create a curve key from a `CurveId` and [`RatesCurveKind`].
    ///
    /// # Returns
    ///
    /// Curve market-factor key.
    pub fn curve(id: CurveId, kind: RatesCurveKind) -> Self {
        Self::Curve { id, kind }
    }

    /// Create a spot key.
    ///
    /// # Returns
    ///
    /// Spot market-factor key.
    pub fn spot(id: impl Into<String>) -> Self {
        Self::Spot(id.into())
    }

    /// Create a vol-surface key.
    ///
    /// # Returns
    ///
    /// Volatility-surface market-factor key.
    pub fn vol_surface(id: impl Into<String>) -> Self {
        Self::VolSurface(id.into())
    }

    /// Create an FX-pair key.
    ///
    /// # Returns
    ///
    /// FX market-factor key.
    pub fn fx(base: Currency, quote: Currency) -> Self {
        Self::Fx { base, quote }
    }

    /// Create a time-series key.
    ///
    /// # Returns
    ///
    /// Scalar-series market-factor key.
    pub fn series(id: impl Into<String>) -> Self {
        Self::Series(id.into())
    }
}

/// Flatten a [`MarketDependencies`] into a deduplicated set of [`MarketFactorKey`]s.
///
/// # Arguments
///
/// * `deps` - Instrument dependency description to normalize.
///
/// # Returns
///
/// Deduplicated normalized key set.
pub fn flatten_dependencies(deps: &MarketDependencies) -> HashSet<MarketFactorKey> {
    let mut keys = HashSet::default();

    for (curve_id, kind) in deps.curves.all_with_kind() {
        keys.insert(MarketFactorKey::curve(curve_id, kind));
    }
    for spot_id in &deps.spot_ids {
        keys.insert(MarketFactorKey::Spot(spot_id.clone()));
    }
    for vol_id in &deps.vol_surface_ids {
        keys.insert(MarketFactorKey::VolSurface(vol_id.clone()));
    }
    for pair in &deps.fx_pairs {
        keys.insert(MarketFactorKey::Fx {
            base: pair.base,
            quote: pair.quote,
        });
    }
    for series_id in &deps.series_ids {
        keys.insert(MarketFactorKey::Series(series_id.clone()));
    }

    keys
}

/// Inverted index mapping market factor keys to affected position indices.
///
/// Stored alongside the `position_index` on [`Portfolio`](crate::portfolio::Portfolio)
/// as a derived, non-serialized cache.  The index maps each [`MarketFactorKey`]
/// to the position indices whose instruments depend on that key.
///
/// Positions whose `market_dependencies()` returned an error are tracked
/// separately in [`unresolved`](Self::unresolved) and are conservatively
/// included in every `affected_positions` query.
#[derive(Debug, Clone, Default)]
pub struct DependencyIndex {
    inner: HashMap<MarketFactorKey, Vec<usize>>,
    /// Position indices whose instruments failed to report dependencies.
    /// These are always included in any affected-position query as a
    /// conservative fallback.
    unresolved: Vec<usize>,
}

impl DependencyIndex {
    /// Build the dependency index from a slice of positions.
    ///
    /// Iterates all positions, calls `instrument.market_dependencies()`,
    /// flattens each into normalized keys, and records the position index.
    /// Instruments that return an error from `market_dependencies()` are
    /// tracked as unresolved and conservatively included in every query.
    ///
    /// # Returns
    ///
    /// Newly built dependency index.
    pub fn build(positions: &[crate::position::Position]) -> Self {
        let mut inner: HashMap<MarketFactorKey, Vec<usize>> = HashMap::default();
        let mut unresolved = Vec::new();

        for (idx, position) in positions.iter().enumerate() {
            let deps = match position.instrument.market_dependencies() {
                Ok(d) => d,
                Err(_) => {
                    unresolved.push(idx);
                    continue;
                }
            };

            for key in flatten_dependencies(&deps) {
                let entry = inner.entry(key).or_default();
                if !entry.contains(&idx) {
                    entry.push(idx);
                }
            }
        }

        Self { inner, unresolved }
    }

    /// Look up position indices affected by a single market factor key.
    ///
    /// # Returns
    ///
    /// Slice of matching position indices, or an empty slice when the key is absent.
    pub fn positions_for_key(&self, key: &MarketFactorKey) -> &[usize] {
        self.inner.get(key).map_or(&[], |v| v.as_slice())
    }

    /// Collect the deduplicated, sorted union of position indices affected by
    /// any of the supplied keys, plus all unresolved positions.
    ///
    /// # Returns
    ///
    /// Sorted affected-position indices.
    pub fn affected_positions(&self, keys: &[MarketFactorKey]) -> Vec<usize> {
        let mut seen = HashSet::default();
        let mut result = Vec::new();

        for &idx in &self.unresolved {
            if seen.insert(idx) {
                result.push(idx);
            }
        }

        for key in keys {
            for &idx in self.positions_for_key(key) {
                if seen.insert(idx) {
                    result.push(idx);
                }
            }
        }

        result.sort_unstable();
        result
    }

    /// Position indices whose instruments failed to report dependencies.
    ///
    /// # Returns
    ///
    /// Slice of unresolved position indices.
    pub fn unresolved(&self) -> &[usize] {
        &self.unresolved
    }

    /// Total number of distinct market factor keys tracked.
    ///
    /// # Returns
    ///
    /// Number of normalized factor keys stored in the index.
    pub fn factor_count(&self) -> usize {
        self.inner.len()
    }

    /// Check whether the index is empty (no resolved keys and no unresolved positions).
    ///
    /// # Returns
    ///
    /// `true` when the index contains no keys and no unresolved positions.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty() && self.unresolved.is_empty()
    }

    /// Iterate over all tracked market factor keys and their position indices.
    ///
    /// # Returns
    ///
    /// Iterator over normalized factor keys and matching position-index slices.
    pub fn iter(&self) -> impl Iterator<Item = (&MarketFactorKey, &[usize])> {
        self.inner.iter().map(|(k, v)| (k, v.as_slice()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_valuations::instruments::InstrumentCurves;

    #[test]
    fn flatten_empty_deps() {
        let deps = MarketDependencies::new();
        let keys = flatten_dependencies(&deps);
        assert!(keys.is_empty());
    }

    #[test]
    fn flatten_deduplicates() {
        let mut deps = MarketDependencies::new();
        let curves = InstrumentCurves::builder()
            .discount("USD".into())
            .forward("USD".into())
            .build()
            .expect("valid curves");
        deps.add_curves(curves.clone());
        deps.add_curves(curves);
        deps.add_spot_id("SPX");
        deps.add_spot_id("SPX");

        let keys = flatten_dependencies(&deps);
        let curve_count = keys
            .iter()
            .filter(|k| matches!(k, MarketFactorKey::Curve { .. }))
            .count();
        let spot_count = keys
            .iter()
            .filter(|k| matches!(k, MarketFactorKey::Spot(_)))
            .count();
        assert_eq!(curve_count, 2, "discount + forward for USD");
        assert_eq!(spot_count, 1, "SPX deduplicated");
        assert_eq!(keys.len(), 3, "2 curves + 1 spot");
    }

    #[test]
    fn dependency_index_empty_portfolio() {
        let index = DependencyIndex::build(&[]);
        assert!(index.is_empty());
        assert_eq!(index.factor_count(), 0);
        assert!(index.unresolved().is_empty());
    }
}
