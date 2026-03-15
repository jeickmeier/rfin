//! Scenario specification bindings for WASM.

use crate::core::currency::JsCurrency;
use crate::scenarios::enums::{
    JsCompounding, JsCurveKind, JsTenorMatchMode, JsTimeRollMode, JsVolSurfaceKind,
};
use finstack_core::HashMap;
use finstack_scenarios::spec::RateBindingSpec;
use finstack_scenarios::{InstrumentType, OperationSpec, ScenarioSpec};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = RateBindingSpec)]
#[derive(Clone, Debug)]
pub struct JsRateBindingSpec {
    pub(crate) inner: RateBindingSpec,
}

#[wasm_bindgen(js_class = RateBindingSpec)]
impl JsRateBindingSpec {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_id: String,
        curve_id: String,
        tenor: String,
        compounding: Option<JsCompounding>,
        day_count: Option<String>,
    ) -> JsRateBindingSpec {
        JsRateBindingSpec {
            inner: RateBindingSpec {
                node_id: node_id.into(),
                curve_id,
                tenor,
                compounding: compounding.map(|c| c.inner).unwrap_or_default(),
                day_count,
            },
        }
    }

    /// Build from legacy `(node_id, curve_id)` mapping (tenor defaults to 1Y continuous).
    #[wasm_bindgen(js_name = fromLegacy)]
    pub fn from_legacy(node_id: String, curve_id: String) -> JsRateBindingSpec {
        JsRateBindingSpec {
            inner: RateBindingSpec::from_legacy(node_id, curve_id),
        }
    }

    /// Convert to JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize RateBindingSpec: {}", e)))
    }

    /// Build from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: &JsValue) -> Result<JsRateBindingSpec, JsValue> {
        let inner: RateBindingSpec = serde_wasm_bindgen::from_value(value.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse RateBindingSpec: {}", e)))?;
        Ok(JsRateBindingSpec { inner })
    }

    #[wasm_bindgen(getter, js_name = nodeId)]
    pub fn node_id(&self) -> String {
        self.inner.node_id.to_string()
    }

    #[wasm_bindgen(getter, js_name = curveId)]
    pub fn curve_id(&self) -> String {
        self.inner.curve_id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn tenor(&self) -> String {
        self.inner.tenor.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn compounding(&self) -> JsCompounding {
        JsCompounding::from(self.inner.compounding)
    }

    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> Option<String> {
        self.inner.day_count.clone()
    }
}

/// Individual operation within a scenario.
///
/// Use static methods to construct specific operation types.
#[wasm_bindgen]
pub struct JsOperationSpec {
    pub(crate) inner: OperationSpec,
}

#[wasm_bindgen]
impl JsOperationSpec {
    /// Create from JSON object.
    ///
    /// # Arguments
    /// * `value` - JavaScript object representing the operation spec
    ///
    /// # Returns
    /// Operation specification instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: &JsValue) -> Result<JsOperationSpec, JsValue> {
        let inner: OperationSpec = serde_wasm_bindgen::from_value(value.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse OperationSpec: {}", e)))?;
        Ok(JsOperationSpec { inner })
    }

    /// Convert to JSON object.
    ///
    /// # Returns
    /// JavaScript object representation
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize OperationSpec: {}", e)))
    }

    /// FX rate percent shift.
    ///
    /// # Arguments
    /// * `base` - Base currency
    /// * `quote` - Quote currency
    /// * `pct` - Percentage change (positive strengthens base)
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = marketFxPct)]
    pub fn market_fx_pct(base: &JsCurrency, quote: &JsCurrency, pct: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::MarketFxPct {
                base: base.inner(),
                quote: quote.inner(),
                pct,
            },
        }
    }

    /// Equity price percent shock.
    ///
    /// # Arguments
    /// * `ids` - Array of equity identifiers
    /// * `pct` - Percentage change to apply
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = equityPricePct)]
    pub fn equity_price_pct(ids: Vec<String>, pct: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::EquityPricePct { ids, pct },
        }
    }

    /// Instrument price shock by exact attribute match.
    ///
    /// # Arguments
    /// * `attrs` - JavaScript object with attribute filters (e.g., {sector: "Energy", rating: "BBB"})
    /// * `pct` - Percentage change to apply
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = instrumentPricePctByAttr)]
    pub fn instrument_price_pct_by_attr(
        attrs: &JsValue,
        pct: f64,
    ) -> Result<JsOperationSpec, JsValue> {
        let attrs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(attrs.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse attributes: {}", e)))?;
        let index_attrs: indexmap::IndexMap<String, String> = attrs_map.into_iter().collect();
        Ok(JsOperationSpec {
            inner: OperationSpec::InstrumentPricePctByAttr {
                attrs: index_attrs,
                pct,
            },
        })
    }

    /// Parallel shift to a curve (additive in basis points).
    ///
    /// # Arguments
    /// * `curve_kind` - Type of curve to shock
    /// * `curve_id` - Curve identifier
    /// * `bp` - Basis points to add
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = curveParallelBp)]
    pub fn curve_parallel_bp(
        curve_kind: &JsCurveKind,
        curve_id: String,
        bp: f64,
    ) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::CurveParallelBp {
                curve_kind: curve_kind.inner,
                curve_id,
                bp,
            },
        }
    }

    /// Node-specific basis point shifts for curve shaping.
    ///
    /// # Arguments
    /// * `curve_kind` - Type of curve to shock
    /// * `curve_id` - Curve identifier
    /// * `nodes` - Array of [tenor, bp] pairs
    /// * `match_mode` - Optional tenor matching mode (defaults to Interpolate)
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = curveNodeBp)]
    pub fn curve_node_bp(
        curve_kind: &JsCurveKind,
        curve_id: String,
        nodes: &JsValue,
        match_mode: Option<JsTenorMatchMode>,
    ) -> Result<JsOperationSpec, JsValue> {
        let nodes_vec: Vec<(String, f64)> = serde_wasm_bindgen::from_value(nodes.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse nodes: {}", e)))?;
        Ok(JsOperationSpec {
            inner: OperationSpec::CurveNodeBp {
                curve_kind: curve_kind.inner,
                curve_id,
                nodes: nodes_vec,
                match_mode: match_mode.map(|m| m.inner).unwrap_or_default(),
            },
        })
    }

    /// Parallel shift to base correlation surface (absolute points).
    ///
    /// # Arguments
    /// * `surface_id` - Identifier of the base correlation surface
    /// * `points` - Absolute correlation points to add
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = baseCorrParallelPts)]
    pub fn base_corr_parallel_pts(surface_id: String, points: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::BaseCorrParallelPts { surface_id, points },
        }
    }

    /// Bucket-specific base correlation shifts.
    ///
    /// # Arguments
    /// * `surface_id` - Identifier of the base correlation surface
    /// * `detachment_bps` - Optional array of detachment points in basis points
    /// * `maturities` - Optional array of maturity strings
    /// * `points` - Absolute correlation points to add to matching buckets
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = baseCorrBucketPts)]
    pub fn base_corr_bucket_pts(
        surface_id: String,
        detachment_bps: Option<Vec<i32>>,
        maturities: Option<Vec<String>>,
        points: f64,
    ) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::BaseCorrBucketPts {
                surface_id,
                detachment_bps,
                maturities,
                points,
            },
        }
    }

    /// Parallel percent shift to volatility surface.
    ///
    /// # Arguments
    /// * `surface_kind` - Type of volatility surface
    /// * `surface_id` - Surface identifier
    /// * `pct` - Percentage change to apply
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = volSurfaceParallelPct)]
    pub fn vol_surface_parallel_pct(
        surface_kind: &JsVolSurfaceKind,
        surface_id: String,
        pct: f64,
    ) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::VolSurfaceParallelPct {
                surface_kind: surface_kind.inner,
                surface_id,
                pct,
            },
        }
    }

    /// Bucketed volatility surface shock.
    ///
    /// # Arguments
    /// * `surface_kind` - Type of volatility surface
    /// * `surface_id` - Surface identifier
    /// * `tenors` - Optional array of tenor strings (e.g., ["1M", "3M"])
    /// * `strikes` - Optional array of strike values
    /// * `pct` - Percentage change to apply to matching buckets
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = volSurfaceBucketPct)]
    pub fn vol_surface_bucket_pct(
        surface_kind: &JsVolSurfaceKind,
        surface_id: String,
        tenors: Option<Vec<String>>,
        strikes: Option<Vec<f64>>,
        pct: f64,
    ) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::VolSurfaceBucketPct {
                surface_kind: surface_kind.inner,
                surface_id,
                tenors,
                strikes,
                pct,
            },
        }
    }

    /// Statement forecast percent change.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the statement node
    /// * `pct` - Percentage change to apply
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = stmtForecastPercent)]
    pub fn stmt_forecast_percent(node_id: String, pct: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::StmtForecastPercent {
                node_id: node_id.into(),
                pct,
            },
        }
    }

    /// Statement forecast value assignment.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the statement node
    /// * `value` - Value to assign
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = stmtForecastAssign)]
    pub fn stmt_forecast_assign(node_id: String, value: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::StmtForecastAssign {
                node_id: node_id.into(),
                value,
            },
        }
    }

    /// Instrument spread shock by exact attribute match.
    ///
    /// # Arguments
    /// * `attrs` - JavaScript object with attribute filters
    /// * `bp` - Basis points to add
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = instrumentSpreadBpByAttr)]
    pub fn instrument_spread_bp_by_attr(
        attrs: &JsValue,
        bp: f64,
    ) -> Result<JsOperationSpec, JsValue> {
        let attrs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(attrs.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse attributes: {}", e)))?;
        let index_attrs: indexmap::IndexMap<String, String> = attrs_map.into_iter().collect();
        Ok(JsOperationSpec {
            inner: OperationSpec::InstrumentSpreadBpByAttr {
                attrs: index_attrs,
                bp,
            },
        })
    }

    /// Instrument price shock by type.
    ///
    /// # Arguments
    /// * `instrument_types` - Array of instrument type strings
    /// * `pct` - Percentage change to apply
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = instrumentPricePctByType)]
    pub fn instrument_price_pct_by_type(
        instrument_types: &JsValue,
        pct: f64,
    ) -> Result<JsOperationSpec, JsValue> {
        let types: Vec<InstrumentType> = serde_wasm_bindgen::from_value(instrument_types.clone())
            .map_err(|e| {
            JsValue::from_str(&format!("Failed to parse instrument types: {}", e))
        })?;
        Ok(JsOperationSpec {
            inner: OperationSpec::InstrumentPricePctByType {
                instrument_types: types,
                pct,
            },
        })
    }

    /// Instrument spread shock by type.
    ///
    /// # Arguments
    /// * `instrument_types` - Array of instrument type strings
    /// * `bp` - Basis points to add
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = instrumentSpreadBpByType)]
    pub fn instrument_spread_bp_by_type(
        instrument_types: &JsValue,
        bp: f64,
    ) -> Result<JsOperationSpec, JsValue> {
        let types: Vec<InstrumentType> = serde_wasm_bindgen::from_value(instrument_types.clone())
            .map_err(|e| {
            JsValue::from_str(&format!("Failed to parse instrument types: {}", e))
        })?;
        Ok(JsOperationSpec {
            inner: OperationSpec::InstrumentSpreadBpByType {
                instrument_types: types,
                bp,
            },
        })
    }

    /// Shock asset correlation for structured credit instruments.
    ///
    /// # Arguments
    /// * `delta_pts` - Additive shock in correlation points (e.g., 0.05 for +5%)
    #[wasm_bindgen(js_name = assetCorrelationPts)]
    pub fn asset_correlation_pts(delta_pts: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::AssetCorrelationPts { delta_pts },
        }
    }

    /// Shock prepay-default correlation for structured credit instruments.
    ///
    /// # Arguments
    /// * `delta_pts` - Additive shock in correlation points
    #[wasm_bindgen(js_name = prepayDefaultCorrelationPts)]
    pub fn prepay_default_correlation_pts(delta_pts: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::PrepayDefaultCorrelationPts { delta_pts },
        }
    }

    /// Shock recovery-default correlation for structured credit instruments.
    ///
    /// # Arguments
    /// * `delta_pts` - Additive shock in correlation points
    #[wasm_bindgen(js_name = recoveryCorrelationPts)]
    pub fn recovery_correlation_pts(delta_pts: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::RecoveryCorrelationPts { delta_pts },
        }
    }

    /// Shock prepayment factor loading (systematic factor sensitivity).
    ///
    /// # Arguments
    /// * `delta_pts` - Additive shock to factor loading
    #[wasm_bindgen(js_name = prepayFactorLoadingPts)]
    pub fn prepay_factor_loading_pts(delta_pts: f64) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::PrepayFactorLoadingPts { delta_pts },
        }
    }

    /// Roll forward horizon by a period with carry/theta.
    ///
    /// # Arguments
    /// * `period` - Period string (e.g., "1D", "1W", "1M", "1Y")
    /// * `apply_shocks` - Whether to apply market shocks after rolling (default: true)
    /// * `roll_mode` - Optional roll interpretation (defaults to business days)
    ///
    /// # Returns
    /// Operation specification
    #[wasm_bindgen(js_name = timeRollForward)]
    pub fn time_roll_forward(
        period: String,
        apply_shocks: Option<bool>,
        roll_mode: Option<JsTimeRollMode>,
    ) -> JsOperationSpec {
        JsOperationSpec {
            inner: OperationSpec::TimeRollForward {
                period,
                apply_shocks: apply_shocks.unwrap_or(true),
                roll_mode: roll_mode.map(|m| m.inner).unwrap_or_default(),
            },
        }
    }
}

impl From<OperationSpec> for JsOperationSpec {
    fn from(inner: OperationSpec) -> Self {
        Self { inner }
    }
}

/// A complete scenario specification with metadata and ordered operations.
#[wasm_bindgen]
pub struct JsScenarioSpec {
    pub(crate) inner: ScenarioSpec,
}

#[wasm_bindgen]
impl JsScenarioSpec {
    /// Create a new scenario specification.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this scenario
    /// * `operations` - Array of operation specifications (as JSON)
    ///
    /// # Returns
    /// Scenario specification instance
    #[wasm_bindgen(constructor)]
    pub fn new(id: String, operations: &JsValue) -> Result<JsScenarioSpec, JsValue> {
        let ops: Vec<OperationSpec> = serde_wasm_bindgen::from_value(operations.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse operations: {}", e)))?;

        Ok(JsScenarioSpec {
            inner: ScenarioSpec {
                id,
                name: None,
                description: None,
                operations: ops,
                priority: 0,
                resolution_mode: Default::default(),
            },
        })
    }

    /// Create from JSON object.
    ///
    /// # Arguments
    /// * `value` - JavaScript object representing the scenario spec
    ///
    /// # Returns
    /// Scenario specification instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: &JsValue) -> Result<JsScenarioSpec, JsValue> {
        let inner: ScenarioSpec = serde_wasm_bindgen::from_value(value.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to parse ScenarioSpec: {}", e)))?;
        Ok(JsScenarioSpec { inner })
    }

    /// Convert to JSON object.
    ///
    /// # Returns
    /// JavaScript object representation
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize ScenarioSpec: {}", e)))
    }

    /// Get the scenario ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Set the scenario ID.
    #[wasm_bindgen(setter)]
    pub fn set_id(&mut self, id: String) {
        self.inner.id = id;
    }

    /// Get the scenario name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// Set the scenario name.
    #[wasm_bindgen(setter)]
    pub fn set_name(&mut self, name: Option<String>) {
        self.inner.name = name;
    }

    /// Get the scenario description.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// Set the scenario description.
    #[wasm_bindgen(setter)]
    pub fn set_description(&mut self, description: Option<String>) {
        self.inner.description = description;
    }

    /// Get the scenario priority.
    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> i32 {
        self.inner.priority
    }

    /// Set the scenario priority (lower value = higher priority).
    #[wasm_bindgen(setter)]
    pub fn set_priority(&mut self, priority: i32) {
        self.inner.priority = priority;
    }

    /// Get the number of operations in this scenario.
    #[wasm_bindgen(js_name = operationCount)]
    pub fn operation_count(&self) -> usize {
        self.inner.operations.len()
    }
}

impl From<ScenarioSpec> for JsScenarioSpec {
    fn from(inner: ScenarioSpec) -> Self {
        Self { inner }
    }
}
