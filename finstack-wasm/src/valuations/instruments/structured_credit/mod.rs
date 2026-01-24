use crate::core::error::js_error;
use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::exotics::basket::Basket;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    Pool, StructuredCredit, TrancheStructure,
};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

pub mod waterfall;

// ===========================
// Basket
// ===========================

#[wasm_bindgen(js_name = BasketBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsBasketBuilder {
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = BasketBuilder)]
impl JsBasketBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsBasketBuilder {
        JsBasketBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsBasketBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsBasket, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("BasketBuilder: jsonString is required"))?;
        JsBasket::from_json(json_str)
    }
}

/// Basket instrument (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
#[wasm_bindgen(js_name = Basket)]
#[derive(Clone, Debug)]
pub struct JsBasket {
    pub(crate) inner: Basket,
}

impl InstrumentWrapper for JsBasket {
    type Inner = Basket;
    fn from_inner(inner: Basket) -> Self {
        JsBasket { inner }
    }
    fn inner(&self) -> Basket {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Basket)]
impl JsBasket {
    /// Parse a basket instrument from a JSON string.
    ///
    /// @param json_str - JSON payload matching the basket schema
    /// @returns A new `Basket`
    /// @throws {Error} If JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsBasket, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "Basket.fromJson is deprecated; use BasketBuilder instead.",
        ));
        serde_json::from_str(json_str)
            .map(JsBasket::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize this instrument to a pretty-printed JSON string.
    ///
    /// @returns JSON string
    /// @throws {Error} If serialization fails
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Basket as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Basket(id='{}', constituents={})",
            self.inner.id,
            self.inner.constituents.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBasket {
        JsBasket::from_inner(self.inner.clone())
    }
}

// ===========================
// Unified Structured Credit
// ===========================

/// Structured credit instrument (ABS/CLO/CMBS/RMBS-style) (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
#[wasm_bindgen(js_name = StructuredCredit)]
#[derive(Clone, Debug)]
pub struct JsStructuredCredit {
    pub(crate) inner: StructuredCredit,
}

impl InstrumentWrapper for JsStructuredCredit {
    type Inner = StructuredCredit;
    fn from_inner(inner: StructuredCredit) -> Self {
        JsStructuredCredit { inner }
    }
    fn inner(&self) -> StructuredCredit {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = StructuredCreditBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsStructuredCreditBuilder {
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = StructuredCreditBuilder)]
impl JsStructuredCreditBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsStructuredCreditBuilder {
        JsStructuredCreditBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsStructuredCreditBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsStructuredCredit, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("StructuredCreditBuilder: jsonString is required"))?;
        JsStructuredCredit::from_json(json_str)
    }
}

#[wasm_bindgen(js_class = StructuredCredit)]
impl JsStructuredCredit {
    /// Parse a structured credit deal from a JSON string.
    ///
    /// @param json_str - JSON payload matching the structured credit schema
    /// @returns A new `StructuredCredit`
    /// @throws {Error} If JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsStructuredCredit, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "StructuredCredit.fromJson is deprecated; use StructuredCreditBuilder instead.",
        ));
        serde_json::from_str(json_str)
            .map(JsStructuredCredit::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = dealType)]
    pub fn deal_type(&self) -> String {
        format!("{:?}", self.inner.deal_type)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this structured credit deal (per-tranche engine schedule).
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use crate::core::money::JsMoney;
        use finstack_valuations::cashflow::CashflowProvider;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let sched = self
            .inner
            .build_full_schedule(market.inner(), as_of)
            .map_err(|e| js_error(e.to_string()))?;
        let outstanding_path = sched
            .outstanding_path_per_flow()
            .map_err(|e| js_error(e.to_string()))?;

        let result = Array::new();
        for (idx, cf) in sched.flows.iter().enumerate() {
            let entry = Array::new();
            entry.push(&JsDate::from_core(cf.date).into());
            entry.push(&JsMoney::from_inner(cf.amount).into());
            entry.push(&JsValue::from_str(&format!("{:?}", cf.kind)));
            let outstanding = outstanding_path
                .get(idx)
                .map(|(_, m)| m.amount())
                .unwrap_or(0.0);
            entry.push(&JsValue::from_f64(outstanding));
            result.push(&entry);
        }
        Ok(result)
    }

    /// Serialize this instrument to a pretty-printed JSON string.
    ///
    /// @returns JSON string
    /// @throws {Error} If serialization fails
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::StructuredCredit as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "StructuredCredit({:?}, id='{}', tranches={})",
            self.inner.deal_type,
            self.inner.id,
            self.inner.tranches.tranches.len()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsStructuredCredit {
        JsStructuredCredit::from_inner(self.inner.clone())
    }

    #[wasm_bindgen(getter, js_name = trancheCount)]
    pub fn tranche_count(&self) -> usize {
        self.inner.tranches.tranches.len()
    }
}

// ===========================
// Waterfall / Pool helpers
// ===========================

/// Tranche structure wrapper (JSON-based).
#[wasm_bindgen(js_name = TrancheStructure)]
#[derive(Clone, Debug)]
pub struct JsTrancheStructure {
    pub(crate) inner: TrancheStructure,
}

#[wasm_bindgen(js_class = TrancheStructure)]
impl JsTrancheStructure {
    /// Parse a tranche structure from a JSON string.
    ///
    /// @param json_str - JSON payload matching the tranche structure schema
    /// @returns A `TrancheStructure`
    /// @throws {Error} If JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsTrancheStructure, JsValue> {
        serde_json::from_str(json_str)
            .map(|inner| JsTrancheStructure { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }
}

/// Pool wrapper (JSON-based).
#[wasm_bindgen(js_name = Pool)]
#[derive(Clone, Debug)]
pub struct JsPool {
    pub(crate) inner: Pool,
}

#[wasm_bindgen(js_class = Pool)]
impl JsPool {
    /// Parse a collateral pool from a JSON string.
    ///
    /// @param json_str - JSON payload matching the pool schema
    /// @returns A `Pool`
    /// @throws {Error} If JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsPool, JsValue> {
        serde_json::from_str(json_str)
            .map(|inner| JsPool { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }
}

// Re-export waterfall helpers from sibling module
pub use waterfall::{JsCoverageTestRules, JsCoverageTrigger, JsWaterfall, JsWaterfallDistribution};
