//! Portfolio dependency index bindings for WASM.
//!
//! Wraps `finstack_portfolio::dependencies` types for selective repricing:
//! normalized market factor keys and the inverted dependency index.

use crate::utils::json::{from_js_value, to_js_value};
use finstack_portfolio::dependencies::{DependencyIndex, MarketFactorKey};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// MarketFactorKey
// ---------------------------------------------------------------------------

/// Normalized market factor key for portfolio-level dependency tracking.
///
/// Each variant captures enough information to uniquely identify one atomic
/// market data input: curves, spots, vol surfaces, FX pairs, or time series.
///
/// @example
/// ```javascript
/// const key = MarketFactorKey.spot("SPX");
/// console.log(key.kind);  // "Spot"
/// ```
#[wasm_bindgen(js_name = MarketFactorKey)]
#[derive(Clone)]
pub struct JsMarketFactorKey {
    inner: MarketFactorKey,
}

impl JsMarketFactorKey {
    pub(crate) fn inner(&self) -> &MarketFactorKey {
        &self.inner
    }
}

#[wasm_bindgen(js_class = MarketFactorKey)]
impl JsMarketFactorKey {
    /// Get the key kind (Curve, Spot, VolSurface, Fx, Series).
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        match &self.inner {
            MarketFactorKey::Curve { .. } => "Curve".to_string(),
            MarketFactorKey::Spot(_) => "Spot".to_string(),
            MarketFactorKey::VolSurface(_) => "VolSurface".to_string(),
            MarketFactorKey::Fx { .. } => "Fx".to_string(),
            MarketFactorKey::Series(_) => "Series".to_string(),
        }
    }

    /// Create a spot factor key.
    pub fn spot(id: &str) -> JsMarketFactorKey {
        Self {
            inner: MarketFactorKey::spot(id),
        }
    }

    /// Create a vol-surface factor key.
    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(id: &str) -> JsMarketFactorKey {
        Self {
            inner: MarketFactorKey::vol_surface(id),
        }
    }

    /// Create a time-series factor key.
    pub fn series(id: &str) -> JsMarketFactorKey {
        Self {
            inner: MarketFactorKey::series(id),
        }
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMarketFactorKey, JsValue> {
        from_js_value(value).map(|inner| Self { inner })
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// ---------------------------------------------------------------------------
// DependencyIndex
// ---------------------------------------------------------------------------

/// Inverted index mapping market factor keys to affected position indices.
///
/// Built from each instrument's market dependencies, enabling efficient
/// lookup of affected positions when a subset of market data changes.
#[wasm_bindgen(js_name = DependencyIndex)]
#[derive(Clone)]
pub struct JsDependencyIndex {
    inner: DependencyIndex,
}

#[wasm_bindgen(js_class = DependencyIndex)]
impl JsDependencyIndex {
    /// Total number of distinct market factor keys tracked.
    #[wasm_bindgen(getter, js_name = factorCount)]
    pub fn factor_count(&self) -> usize {
        self.inner.factor_count()
    }

    /// Whether the index is empty (no resolved keys and no unresolved positions).
    #[wasm_bindgen(getter, js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Position indices whose instruments failed to report dependencies.
    #[wasm_bindgen(getter)]
    pub fn unresolved(&self) -> Vec<usize> {
        self.inner.unresolved().to_vec()
    }

    /// Look up position indices affected by a single market factor key.
    #[wasm_bindgen(js_name = positionsForKey)]
    pub fn positions_for_key(&self, key: &JsMarketFactorKey) -> Vec<usize> {
        self.inner.positions_for_key(key.inner()).to_vec()
    }
}
