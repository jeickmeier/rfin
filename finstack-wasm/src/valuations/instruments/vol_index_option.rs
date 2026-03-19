//! WASM bindings for VolatilityIndexOption.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::vol_index_option::{
    VolIndexOptionSpecs, VolatilityIndexOption,
};
use finstack_valuations::instruments::{ExerciseStyle, OptionType};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_exercise_style(label: Option<String>) -> Result<ExerciseStyle, JsValue> {
    match label {
        None => Ok(ExerciseStyle::European),
        Some(s) => {
            let normalized = s.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "european" => Ok(ExerciseStyle::European),
                "american" => Ok(ExerciseStyle::American),
                "bermudan" => Ok(ExerciseStyle::Bermudan),
                _ => Err(js_error(format!(
                    "Invalid exercise style: {}. Use 'european', 'american', or 'bermudan'",
                    s
                ))),
            }
        }
    }
}

#[wasm_bindgen(js_name = VolatilityIndexOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsVolatilityIndexOptionBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    strike: Option<f64>,
    expiry: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    vol_index_curve: Option<String>,
    vol_of_vol_surface: Option<String>,
    option_type: Option<String>,
    exercise_style: Option<String>,
    multiplier: Option<f64>,
    index_id: Option<String>,
}

#[wasm_bindgen(js_class = VolatilityIndexOptionBuilder)]
impl JsVolatilityIndexOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsVolatilityIndexOptionBuilder {
        JsVolatilityIndexOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsVolatilityIndexOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsVolatilityIndexOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsVolatilityIndexOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsVolatilityIndexOptionBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = volIndexCurve)]
    pub fn vol_index_curve(mut self, vol_index_curve: &str) -> JsVolatilityIndexOptionBuilder {
        self.vol_index_curve = Some(vol_index_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = volOfVolSurface)]
    pub fn vol_of_vol_surface(
        mut self,
        vol_of_vol_surface: &str,
    ) -> JsVolatilityIndexOptionBuilder {
        self.vol_of_vol_surface = Some(vol_of_vol_surface.to_string());
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: String) -> JsVolatilityIndexOptionBuilder {
        self.option_type = Some(option_type);
        self
    }

    #[wasm_bindgen(js_name = exerciseStyle)]
    pub fn exercise_style(mut self, exercise_style: String) -> JsVolatilityIndexOptionBuilder {
        self.exercise_style = Some(exercise_style);
        self
    }

    #[wasm_bindgen(js_name = multiplier)]
    pub fn multiplier(mut self, multiplier: f64) -> JsVolatilityIndexOptionBuilder {
        self.multiplier = Some(multiplier);
        self
    }

    #[wasm_bindgen(js_name = indexId)]
    pub fn index_id(mut self, index_id: String) -> JsVolatilityIndexOptionBuilder {
        self.index_id = Some(index_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsVolatilityIndexOption, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("VolatilityIndexOptionBuilder: notional (money) is required".to_string())
        })?;
        let strike = self.strike.ok_or_else(|| {
            js_error("VolatilityIndexOptionBuilder: strike is required".to_string())
        })?;
        let expiry = self.expiry.ok_or_else(|| {
            js_error("VolatilityIndexOptionBuilder: expiry is required".to_string())
        })?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("VolatilityIndexOptionBuilder: discountCurve is required".to_string())
        })?;
        let vol_index_curve = self.vol_index_curve.as_deref().ok_or_else(|| {
            js_error("VolatilityIndexOptionBuilder: volIndexCurve is required".to_string())
        })?;
        let vol_of_vol_surface = self.vol_of_vol_surface.as_deref().ok_or_else(|| {
            js_error("VolatilityIndexOptionBuilder: volOfVolSurface is required".to_string())
        })?;

        let option_type_value = parse_optional_with_default(self.option_type, OptionType::Call)?;
        let exercise_style_value = parse_exercise_style(self.exercise_style)?;

        let specs = VolIndexOptionSpecs {
            multiplier: self.multiplier.unwrap_or(100.0),
            index_id: self.index_id.unwrap_or_else(|| "VIX".to_string()),
        };

        VolatilityIndexOption::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .strike(strike)
            .expiry(expiry)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .vol_index_curve_id(curve_id_from_str(vol_index_curve))
            .vol_of_vol_surface_id(curve_id_from_str(vol_of_vol_surface))
            .option_type(option_type_value)
            .exercise_style(exercise_style_value)
            .contract_specs(specs)
            .attributes(Default::default())
            .build()
            .map(JsVolatilityIndexOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = VolatilityIndexOption)]
#[derive(Clone, Debug)]
pub struct JsVolatilityIndexOption {
    pub(crate) inner: VolatilityIndexOption,
}

impl InstrumentWrapper for JsVolatilityIndexOption {
    type Inner = VolatilityIndexOption;
    fn from_inner(inner: VolatilityIndexOption) -> Self {
        JsVolatilityIndexOption { inner }
    }
    fn inner(&self) -> VolatilityIndexOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = VolatilityIndexOption)]
impl JsVolatilityIndexOption {
    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[wasm_bindgen(getter, js_name = optionType)]
    pub fn option_type(&self) -> String {
        match self.inner.option_type {
            OptionType::Call => "call".to_string(),
            OptionType::Put => "put".to_string(),
        }
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsVolatilityIndexOption, JsValue> {
        from_js_value(value).map(JsVolatilityIndexOption::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::VolatilityIndexOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        let opt_type = match self.inner.option_type {
            OptionType::Call => "Call",
            OptionType::Put => "Put",
        };
        format!(
            "VolatilityIndexOption(id='{}', strike={:.2}, type={})",
            self.inner.id, self.inner.strike, opt_type
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsVolatilityIndexOption {
        JsVolatilityIndexOption::from_inner(self.inner.clone())
    }
}
