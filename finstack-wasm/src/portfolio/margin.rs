//! Portfolio margin aggregation bindings for WASM.

use crate::core::currency::JsCurrency;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::portfolio::positions::JsPortfolio;
use crate::portfolio::types::JsPosition;
use finstack_portfolio::margin::{
    NettingSet, NettingSetManager, NettingSetMargin, PortfolioMarginAggregator,
    PortfolioMarginResult,
};
use finstack_valuations::margin::NettingSetId;
use js_sys::{Array, Object};
use wasm_bindgen::prelude::*;

/// Identifier for a margin netting set.
#[wasm_bindgen(js_name = NettingSetId)]
pub struct JsNettingSetId {
    inner: NettingSetId,
}

#[wasm_bindgen(js_class = NettingSetId)]
impl JsNettingSetId {
    /// Create a bilateral netting set identifier.
    #[wasm_bindgen(js_name = bilateral)]
    pub fn bilateral(counterparty_id: String, csa_id: String) -> JsNettingSetId {
        JsNettingSetId {
            inner: NettingSetId::bilateral(counterparty_id, csa_id),
        }
    }

    /// Create a cleared netting set identifier.
    #[wasm_bindgen(js_name = cleared)]
    pub fn cleared(ccp_id: String) -> JsNettingSetId {
        JsNettingSetId {
            inner: NettingSetId::cleared(ccp_id),
        }
    }

    /// Counterparty identifier (if bilateral).
    #[wasm_bindgen(getter, js_name = counterpartyId)]
    pub fn counterparty_id(&self) -> String {
        self.inner.counterparty_id.clone()
    }

    /// CSA identifier (if bilateral).
    #[wasm_bindgen(getter, js_name = csaId)]
    pub fn csa_id(&self) -> Option<String> {
        self.inner.csa_id.clone()
    }

    /// CCP identifier (if cleared).
    #[wasm_bindgen(getter, js_name = ccpId)]
    pub fn ccp_id(&self) -> Option<String> {
        self.inner.ccp_id.clone()
    }

    /// Whether this netting set is cleared.
    #[wasm_bindgen(js_name = isCleared)]
    pub fn is_cleared(&self) -> bool {
        self.inner.is_cleared()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

impl JsNettingSetId {
    pub(crate) fn inner(&self) -> &NettingSetId {
        &self.inner
    }

    pub(crate) fn clone_inner(&self) -> NettingSetId {
        self.inner.clone()
    }
}

/// A netting set containing positions for margin aggregation.
#[wasm_bindgen(js_name = NettingSet)]
pub struct JsNettingSet {
    inner: NettingSet,
}

#[wasm_bindgen(js_class = NettingSet)]
impl JsNettingSet {
    #[wasm_bindgen(constructor)]
    pub fn new(id: &JsNettingSetId) -> JsNettingSet {
        JsNettingSet {
            inner: NettingSet::new(id.clone_inner()),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[wasm_bindgen(js_name = addPosition)]
    pub fn add_position(&mut self, position_id: String) {
        self.inner.add_position(position_id.into());
    }

    #[wasm_bindgen(js_name = positionCount)]
    pub fn position_count(&self) -> usize {
        self.inner.position_count()
    }

    #[wasm_bindgen(js_name = isCleared)]
    pub fn is_cleared(&self) -> bool {
        self.inner.is_cleared()
    }

    /// Merge SIMM sensitivities into the netting set.
    #[wasm_bindgen(js_name = mergeSensitivities)]
    pub fn merge_sensitivities(&mut self, sensitivities: JsValue) -> Result<(), JsValue> {
        let sens: finstack_valuations::margin::SimmSensitivities =
            serde_wasm_bindgen::from_value(sensitivities)
                .map_err(|e| JsValue::from_str(&format!("Invalid sensitivities: {}", e)))?;
        self.inner.merge_sensitivities(&sens);
        Ok(())
    }
}

impl JsNettingSet {
    pub(crate) fn from_inner(inner: NettingSet) -> Self {
        Self { inner }
    }
}

/// Manager for organizing positions into netting sets.
#[wasm_bindgen(js_name = NettingSetManager)]
pub struct JsNettingSetManager {
    inner: NettingSetManager,
}

impl Default for JsNettingSetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = NettingSetManager)]
impl JsNettingSetManager {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsNettingSetManager {
        JsNettingSetManager {
            inner: NettingSetManager::new(),
        }
    }

    #[wasm_bindgen(js_name = withDefaultSet)]
    pub fn with_default_set(mut self, id: &JsNettingSetId) -> JsNettingSetManager {
        self.inner = self.inner.with_default_set(id.clone_inner());
        self
    }

    #[wasm_bindgen(js_name = addPosition)]
    pub fn add_position(&mut self, position: &JsPosition, netting_set_id: Option<JsNettingSetId>) {
        let ns_id = netting_set_id.map(|ns| ns.clone_inner());
        self.inner.add_position(&position.inner, ns_id);
    }

    pub fn count(&self) -> usize {
        self.inner.count()
    }

    pub fn ids(&self) -> Array {
        let arr = Array::new();
        for id in self.inner.ids() {
            arr.push(&JsValue::from(JsNettingSetId { inner: id.clone() }));
        }
        arr
    }

    pub fn get(&self, id: &JsNettingSetId) -> Option<JsNettingSet> {
        self.inner
            .get(id.inner())
            .cloned()
            .map(JsNettingSet::from_inner)
    }
}

/// Margin results for a single netting set.
#[wasm_bindgen(js_name = NettingSetMargin)]
pub struct JsNettingSetMargin {
    inner: NettingSetMargin,
}

#[wasm_bindgen(js_class = NettingSetMargin)]
impl JsNettingSetMargin {
    #[wasm_bindgen(getter, js_name = nettingSetId)]
    pub fn netting_set_id(&self) -> String {
        self.inner.netting_set_id.to_string()
    }

    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> FsDate {
        FsDate::from_core(self.inner.as_of)
    }

    #[wasm_bindgen(getter, js_name = initialMargin)]
    pub fn initial_margin(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.initial_margin)
    }

    #[wasm_bindgen(getter, js_name = variationMargin)]
    pub fn variation_margin(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.variation_margin)
    }

    #[wasm_bindgen(getter, js_name = totalMargin)]
    pub fn total_margin(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_margin)
    }

    #[wasm_bindgen(getter, js_name = positionCount)]
    pub fn position_count(&self) -> usize {
        self.inner.position_count
    }

    #[wasm_bindgen(getter, js_name = imMethodology)]
    pub fn im_methodology(&self) -> String {
        self.inner.im_methodology.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn sensitivities(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.sensitivities)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize sensitivities: {}", e)))
    }

    #[wasm_bindgen(getter, js_name = imBreakdown)]
    pub fn im_breakdown(&self) -> Result<JsValue, JsValue> {
        let dict = Object::new();
        for (bucket, money) in &self.inner.im_breakdown {
            js_sys::Reflect::set(
                &dict,
                &JsValue::from_str(bucket),
                &JsValue::from(JsMoney::from_inner(*money)),
            )?;
        }
        Ok(JsValue::from(dict))
    }

    #[wasm_bindgen(js_name = isCleared)]
    pub fn is_cleared(&self) -> bool {
        self.inner.is_cleared()
    }
}

impl JsNettingSetMargin {
    pub(crate) fn from_inner(inner: NettingSetMargin) -> Self {
        Self { inner }
    }
}

/// Portfolio-wide margin calculation results.
#[wasm_bindgen(js_name = PortfolioMarginResult)]
pub struct JsPortfolioMarginResult {
    inner: PortfolioMarginResult,
}

#[wasm_bindgen(js_class = PortfolioMarginResult)]
impl JsPortfolioMarginResult {
    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> FsDate {
        FsDate::from_core(self.inner.as_of)
    }

    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.base_currency)
    }

    #[wasm_bindgen(getter, js_name = totalInitialMargin)]
    pub fn total_initial_margin(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_initial_margin)
    }

    #[wasm_bindgen(getter, js_name = totalVariationMargin)]
    pub fn total_variation_margin(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_variation_margin)
    }

    #[wasm_bindgen(getter, js_name = totalMargin)]
    pub fn total_margin(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_margin)
    }

    #[wasm_bindgen(getter, js_name = byNettingSet)]
    pub fn by_netting_set(&self) -> Result<JsValue, JsValue> {
        let dict = Object::new();
        for (id, margin) in &self.inner.by_netting_set {
            js_sys::Reflect::set(
                &dict,
                &JsValue::from_str(&id.to_string()),
                &JsValue::from(JsNettingSetMargin::from_inner(margin.clone())),
            )?;
        }
        Ok(JsValue::from(dict))
    }

    #[wasm_bindgen(getter, js_name = totalPositions)]
    pub fn total_positions(&self) -> usize {
        self.inner.total_positions
    }

    #[wasm_bindgen(getter, js_name = positionsWithoutMargin)]
    pub fn positions_without_margin(&self) -> usize {
        self.inner.positions_without_margin
    }

    #[wasm_bindgen(js_name = clearedBilateralSplit)]
    pub fn cleared_bilateral_split(&self) -> Array {
        let (cleared, bilateral) = self.inner.cleared_bilateral_split();
        let arr = Array::new();
        arr.push(&JsValue::from(JsMoney::from_inner(cleared)));
        arr.push(&JsValue::from(JsMoney::from_inner(bilateral)));
        arr
    }
}

impl JsPortfolioMarginResult {
    pub(crate) fn from_inner(inner: PortfolioMarginResult) -> Self {
        Self { inner }
    }
}

/// Aggregates margin requirements across a portfolio.
#[wasm_bindgen(js_name = PortfolioMarginAggregator)]
pub struct JsPortfolioMarginAggregator {
    inner: PortfolioMarginAggregator,
}

#[wasm_bindgen(js_class = PortfolioMarginAggregator)]
impl JsPortfolioMarginAggregator {
    #[wasm_bindgen(constructor)]
    pub fn new(base_ccy: JsCurrency) -> JsPortfolioMarginAggregator {
        JsPortfolioMarginAggregator {
            inner: PortfolioMarginAggregator::new(base_ccy.inner()),
        }
    }

    #[wasm_bindgen(js_name = fromPortfolio)]
    pub fn from_portfolio(portfolio: &JsPortfolio) -> JsPortfolioMarginAggregator {
        JsPortfolioMarginAggregator {
            inner: PortfolioMarginAggregator::from_portfolio(&portfolio.inner),
        }
    }

    #[wasm_bindgen(js_name = addPosition)]
    pub fn add_position(&mut self, position: &JsPosition) {
        self.inner.add_position(&position.inner);
    }

    pub fn calculate(
        &mut self,
        portfolio: &JsPortfolio,
        market_context: &JsMarketContext,
        as_of: &FsDate,
    ) -> Result<JsPortfolioMarginResult, JsValue> {
        self.inner
            .calculate(&portfolio.inner, market_context.inner(), as_of.inner())
            .map(JsPortfolioMarginResult::from_inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
