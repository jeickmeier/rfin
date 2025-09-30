use crate::core::currency::JsCurrency;
use crate::core::utils::js_error;
use finstack_core::config::{FinstackConfig, RoundingMode};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = RoundingMode)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsRoundingMode {
    Bankers,
    AwayFromZero,
    TowardZero,
    Floor,
    Ceil,
}

impl From<JsRoundingMode> for RoundingMode {
    fn from(value: JsRoundingMode) -> Self {
        match value {
            JsRoundingMode::Bankers => RoundingMode::Bankers,
            JsRoundingMode::AwayFromZero => RoundingMode::AwayFromZero,
            JsRoundingMode::TowardZero => RoundingMode::TowardZero,
            JsRoundingMode::Floor => RoundingMode::Floor,
            JsRoundingMode::Ceil => RoundingMode::Ceil,
        }
    }
}

impl From<RoundingMode> for JsRoundingMode {
    fn from(value: RoundingMode) -> Self {
        match value {
            RoundingMode::Bankers => JsRoundingMode::Bankers,
            RoundingMode::AwayFromZero => JsRoundingMode::AwayFromZero,
            RoundingMode::TowardZero => JsRoundingMode::TowardZero,
            RoundingMode::Floor => JsRoundingMode::Floor,
            RoundingMode::Ceil => JsRoundingMode::Ceil,
            _ => JsRoundingMode::Bankers,
        }
    }
}

#[wasm_bindgen(js_name = FinstackConfig)]
#[derive(Clone, Debug)]
pub struct JsFinstackConfig {
    inner: FinstackConfig,
}

impl Default for JsFinstackConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FinstackConfig)]
impl JsFinstackConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsFinstackConfig {
        JsFinstackConfig {
            inner: FinstackConfig::default(),
        }
    }

    #[wasm_bindgen(js_name = copy)]
    pub fn copy(&self) -> JsFinstackConfig {
        JsFinstackConfig {
            inner: self.inner.clone(),
        }
    }

    #[wasm_bindgen(getter, js_name = roundingMode)]
    pub fn rounding_mode(&self) -> JsRoundingMode {
        self.inner.rounding.mode.into()
    }

    #[wasm_bindgen(js_name = setRoundingMode)]
    pub fn set_rounding_mode(&mut self, mode: JsRoundingMode) {
        self.inner.rounding.mode = mode.into();
    }

    #[wasm_bindgen(js_name = setRoundingModeLabel)]
    pub fn set_rounding_mode_label(&mut self, label: &str) -> Result<(), JsValue> {
        let mode = parse_rounding_label(label)
            .ok_or_else(|| js_error(format!("Unknown rounding mode: {label}")))?;
        self.inner.rounding.mode = mode;
        Ok(())
    }

    #[wasm_bindgen(js_name = ingestScale)]
    pub fn ingest_scale(&self, currency: &JsCurrency) -> u32 {
        self.inner.ingest_scale(currency.inner())
    }

    #[wasm_bindgen(js_name = setIngestScale)]
    pub fn set_ingest_scale(&mut self, currency: &JsCurrency, decimals: u32) {
        self.inner
            .rounding
            .ingest_scale
            .overrides
            .insert(currency.inner(), decimals);
    }

    #[wasm_bindgen(js_name = outputScale)]
    pub fn output_scale(&self, currency: &JsCurrency) -> u32 {
        self.inner.output_scale(currency.inner())
    }

    #[wasm_bindgen(js_name = setOutputScale)]
    pub fn set_output_scale(&mut self, currency: &JsCurrency, decimals: u32) {
        self.inner
            .rounding
            .output_scale
            .overrides
            .insert(currency.inner(), decimals);
    }
}

impl JsFinstackConfig {
    pub(crate) fn inner(&self) -> &FinstackConfig {
        &self.inner
    }
}

fn parse_rounding_label(label: &str) -> Option<RoundingMode> {
    match label.to_ascii_lowercase().as_str() {
        "bankers" | "bankers_rounding" | "bankersrounding" => Some(RoundingMode::Bankers),
        "away_from_zero" | "awayfromzero" => Some(RoundingMode::AwayFromZero),
        "toward_zero" | "towardzero" | "truncate" => Some(RoundingMode::TowardZero),
        "floor" => Some(RoundingMode::Floor),
        "ceil" | "ceiling" => Some(RoundingMode::Ceil),
        _ => None,
    }
}
