//! WASM bindings for common parameter types (OptionType, ExerciseStyle, etc.)

use finstack_valuations::instruments::{
    BarrierType, ExerciseStyle, OptionType, PayReceive, SettlementType,
};
use wasm_bindgen::prelude::*;

/// Option type for pricing (Call or Put).
#[wasm_bindgen(js_name = OptionType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsOptionType {
    inner: OptionType,
}

#[wasm_bindgen(js_class = OptionType)]
impl JsOptionType {
    #[wasm_bindgen(constructor)]
    pub fn new(value: &str) -> Result<JsOptionType, JsValue> {
        value
            .parse()
            .map(|inner| JsOptionType { inner })
            .map_err(|e: String| JsValue::from_str(&e))
    }

    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        self.inner.to_string()
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name = isCall)]
    pub fn is_call(&self) -> bool {
        matches!(self.inner, OptionType::Call)
    }

    #[wasm_bindgen(js_name = isPut)]
    pub fn is_put(&self) -> bool {
        matches!(self.inner, OptionType::Put)
    }
}

impl JsOptionType {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: OptionType) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> OptionType {
        self.inner
    }
}

impl From<OptionType> for JsOptionType {
    fn from(value: OptionType) -> Self {
        Self::from_inner(value)
    }
}

/// Exercise style for options (European, American, Bermudan).
#[wasm_bindgen(js_name = ExerciseStyle)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsExerciseStyle {
    inner: ExerciseStyle,
}

#[wasm_bindgen(js_class = ExerciseStyle)]
impl JsExerciseStyle {
    #[wasm_bindgen(constructor)]
    pub fn new(value: &str) -> Result<JsExerciseStyle, JsValue> {
        value
            .parse()
            .map(|inner| JsExerciseStyle { inner })
            .map_err(|e: String| JsValue::from_str(&e))
    }

    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        self.inner.to_string()
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name = isEuropean)]
    pub fn is_european(&self) -> bool {
        matches!(self.inner, ExerciseStyle::European)
    }

    #[wasm_bindgen(js_name = isAmerican)]
    pub fn is_american(&self) -> bool {
        matches!(self.inner, ExerciseStyle::American)
    }

    #[wasm_bindgen(js_name = isBermudan)]
    pub fn is_bermudan(&self) -> bool {
        matches!(self.inner, ExerciseStyle::Bermudan)
    }
}

impl JsExerciseStyle {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: ExerciseStyle) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> ExerciseStyle {
        self.inner
    }
}

impl From<ExerciseStyle> for JsExerciseStyle {
    fn from(value: ExerciseStyle) -> Self {
        Self::from_inner(value)
    }
}

/// Settlement type for options (Physical or Cash).
#[wasm_bindgen(js_name = SettlementType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsSettlementType {
    inner: SettlementType,
}

#[wasm_bindgen(js_class = SettlementType)]
impl JsSettlementType {
    #[wasm_bindgen(constructor)]
    pub fn new(value: &str) -> Result<JsSettlementType, JsValue> {
        value
            .parse()
            .map(|inner| JsSettlementType { inner })
            .map_err(|e: String| JsValue::from_str(&e))
    }

    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        self.inner.to_string()
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name = isPhysical)]
    pub fn is_physical(&self) -> bool {
        matches!(self.inner, SettlementType::Physical)
    }

    #[wasm_bindgen(js_name = isCash)]
    pub fn is_cash(&self) -> bool {
        matches!(self.inner, SettlementType::Cash)
    }
}

impl JsSettlementType {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: SettlementType) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> SettlementType {
        self.inner
    }
}

/// Pay/Receive direction for instrument legs.
#[wasm_bindgen(js_name = PayReceive)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsPayReceive {
    inner: PayReceive,
}

#[wasm_bindgen(js_class = PayReceive)]
impl JsPayReceive {
    #[wasm_bindgen(constructor)]
    pub fn new(value: &str) -> Result<JsPayReceive, JsValue> {
        value
            .parse()
            .map(|inner| JsPayReceive { inner })
            .map_err(|e: String| JsValue::from_str(&e))
    }

    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        self.inner.to_string()
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name = isPayer)]
    pub fn is_payer(&self) -> bool {
        self.inner.is_payer()
    }

    #[wasm_bindgen(js_name = isReceiver)]
    pub fn is_receiver(&self) -> bool {
        self.inner.is_receiver()
    }
}

impl JsPayReceive {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: PayReceive) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> PayReceive {
        self.inner
    }
}

impl From<PayReceive> for JsPayReceive {
    fn from(value: PayReceive) -> Self {
        Self::from_inner(value)
    }
}

/// Barrier type for barrier options.
#[wasm_bindgen(js_name = BarrierType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsBarrierType {
    inner: BarrierType,
}

#[wasm_bindgen(js_class = BarrierType)]
impl JsBarrierType {
    #[wasm_bindgen(js_name = upAndOut)]
    pub fn up_and_out() -> Self {
        Self {
            inner: BarrierType::UpAndOut,
        }
    }

    #[wasm_bindgen(js_name = upAndIn)]
    pub fn up_and_in() -> Self {
        Self {
            inner: BarrierType::UpAndIn,
        }
    }

    #[wasm_bindgen(js_name = downAndOut)]
    pub fn down_and_out() -> Self {
        Self {
            inner: BarrierType::DownAndOut,
        }
    }

    #[wasm_bindgen(js_name = downAndIn)]
    pub fn down_and_in() -> Self {
        Self {
            inner: BarrierType::DownAndIn,
        }
    }

    #[wasm_bindgen(js_name = isKnockOut)]
    pub fn is_knock_out(&self) -> bool {
        matches!(self.inner, BarrierType::UpAndOut | BarrierType::DownAndOut)
    }

    #[wasm_bindgen(js_name = isKnockIn)]
    pub fn is_knock_in(&self) -> bool {
        matches!(self.inner, BarrierType::UpAndIn | BarrierType::DownAndIn)
    }

    #[wasm_bindgen(js_name = isUp)]
    pub fn is_up(&self) -> bool {
        matches!(self.inner, BarrierType::UpAndOut | BarrierType::UpAndIn)
    }
}

impl JsBarrierType {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: BarrierType) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> BarrierType {
        self.inner
    }
}

impl From<BarrierType> for JsBarrierType {
    fn from(value: BarrierType) -> Self {
        Self::from_inner(value)
    }
}

// =============================================================================
// FixedLegSpec
// =============================================================================

/// Specification for fixed rate legs in interest rate swaps.
#[wasm_bindgen(js_name = FixedLegSpec)]
pub struct JsFixedLegSpec {
    inner: finstack_valuations::instruments::FixedLegSpec,
}

#[wasm_bindgen(js_class = FixedLegSpec)]
impl JsFixedLegSpec {
    /// Create a FixedLegSpec from a JSON object.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsFixedLegSpec, JsValue> {
        let inner: finstack_valuations::instruments::FixedLegSpec = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;
        Ok(JsFixedLegSpec { inner })
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
    }

    /// Discount curve identifier.
    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.to_string()
    }

    /// Fixed rate (decimal).
    #[wasm_bindgen(getter)]
    pub fn rate(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.rate.to_f64().unwrap_or(0.0)
    }

    /// Payment frequency code.
    #[wasm_bindgen(getter)]
    pub fn frequency(&self) -> String {
        self.inner.frequency.to_string()
    }

    /// Day count convention code.
    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    /// Start date as ISO string.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> String {
        self.inner.start.to_string()
    }

    /// End date as ISO string.
    #[wasm_bindgen(getter)]
    pub fn end(&self) -> String {
        self.inner.end.to_string()
    }
}

#[allow(dead_code)]
impl JsFixedLegSpec {
    pub(crate) fn from_inner(inner: finstack_valuations::instruments::FixedLegSpec) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> &finstack_valuations::instruments::FixedLegSpec {
        &self.inner
    }
}

// =============================================================================
// FloatLegSpec
// =============================================================================

/// Specification for floating rate legs in interest rate swaps.
#[wasm_bindgen(js_name = FloatLegSpec)]
pub struct JsFloatLegSpec {
    inner: finstack_valuations::instruments::FloatLegSpec,
}

#[wasm_bindgen(js_class = FloatLegSpec)]
impl JsFloatLegSpec {
    /// Create a FloatLegSpec from a JSON object.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsFloatLegSpec, JsValue> {
        let inner: finstack_valuations::instruments::FloatLegSpec = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;
        Ok(JsFloatLegSpec { inner })
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
    }

    /// Discount curve identifier.
    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.to_string()
    }

    /// Forward curve identifier.
    #[wasm_bindgen(getter, js_name = forwardCurveId)]
    pub fn forward_curve_id(&self) -> String {
        self.inner.forward_curve_id.to_string()
    }

    /// Spread in basis points.
    #[wasm_bindgen(getter, js_name = spreadBp)]
    pub fn spread_bp(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.spread_bp.to_f64().unwrap_or(0.0)
    }

    /// Payment frequency code.
    #[wasm_bindgen(getter)]
    pub fn frequency(&self) -> String {
        self.inner.frequency.to_string()
    }

    /// Day count convention code.
    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    /// Start date as ISO string.
    #[wasm_bindgen(getter)]
    pub fn start(&self) -> String {
        self.inner.start.to_string()
    }

    /// End date as ISO string.
    #[wasm_bindgen(getter)]
    pub fn end(&self) -> String {
        self.inner.end.to_string()
    }
}

#[allow(dead_code)]
impl JsFloatLegSpec {
    pub(crate) fn from_inner(inner: finstack_valuations::instruments::FloatLegSpec) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> &finstack_valuations::instruments::FloatLegSpec {
        &self.inner
    }
}

// =============================================================================
// FxPair
// =============================================================================

/// A currency pair for FX operations.
#[wasm_bindgen(js_name = FxPair)]
pub struct JsFxPair {
    base: String,
    quote: String,
}

#[wasm_bindgen(js_class = FxPair)]
impl JsFxPair {
    /// Create a new FX pair from base and quote currency codes.
    #[wasm_bindgen(constructor)]
    pub fn new(base: &str, quote: &str) -> JsFxPair {
        JsFxPair {
            base: base.to_string(),
            quote: quote.to_string(),
        }
    }

    /// Parse an FX pair from "BASE/QUOTE" notation (e.g., "EUR/USD").
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_string(pair: &str) -> Result<JsFxPair, JsValue> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return Err(JsValue::from_str(&format!(
                "Invalid FX pair format '{}'; expected 'BASE/QUOTE'",
                pair
            )));
        }
        Ok(JsFxPair {
            base: parts[0].to_string(),
            quote: parts[1].to_string(),
        })
    }

    /// Base currency code.
    #[wasm_bindgen(getter)]
    pub fn base(&self) -> String {
        self.base.clone()
    }

    /// Quote currency code.
    #[wasm_bindgen(getter)]
    pub fn quote(&self) -> String {
        self.quote.clone()
    }

    /// String representation "BASE/QUOTE".
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{}/{}", self.base, self.quote)
    }
}
