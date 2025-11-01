//! WASM bindings for common parameter types (OptionType, ExerciseStyle, etc.)

use finstack_valuations::instruments::common::models::monte_carlo::payoff::barrier::BarrierType;
use finstack_valuations::instruments::common::parameters::{
    legs::PayReceive,
    market::{ExerciseStyle, OptionType, SettlementType},
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
        self.inner.is_knock_out()
    }

    #[wasm_bindgen(js_name = isKnockIn)]
    pub fn is_knock_in(&self) -> bool {
        self.inner.is_knock_in()
    }

    #[wasm_bindgen(js_name = isUp)]
    pub fn is_up(&self) -> bool {
        self.inner.is_up()
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
