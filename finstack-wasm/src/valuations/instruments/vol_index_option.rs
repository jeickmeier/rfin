//! WASM bindings for VolatilityIndexOption.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::vol_index_option::{
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
    /// Create a volatility index option (e.g., VIX option).
    ///
    /// @param {string} instrumentId - Unique identifier for the instrument
    /// @param {Money} notional - Notional amount (e.g., $100,000 USD)
    /// @param {number} strike - Strike price (e.g., 20.0 for VIX at 20)
    /// @param {Date} expiry - Expiry date of the option
    /// @param {string} discountCurve - ID of the discount curve for NPV calculations
    /// @param {string} volIndexCurve - ID of the volatility index curve for forward levels
    /// @param {string} volOfVolSurface - ID of the volatility-of-volatility surface
    /// @param {string} optionType - Option type: "call" (default) or "put"
    /// @param {string} exerciseStyle - Exercise style: "european" (default), "american", or "bermudan"
    /// @param {number} multiplier - Contract multiplier (default: 100 for VIX options)
    /// @param {string} indexId - Index identifier (default: "VIX")
    /// @returns {VolatilityIndexOption} The constructed option instrument
    ///
    /// @example
    /// ```javascript
    /// const option = new VolatilityIndexOption(
    ///   "VIX-OPT-MAR25-C20",
    ///   new Money("USD", 100000),
    ///   20.0,
    ///   new Date(2025, 2, 15),
    ///   "USD-OIS",
    ///   "VIX-Forward",
    ///   "VIX-VolOfVol",
    ///   "call",
    ///   "european",
    ///   100,      // multiplier
    ///   "VIX"     // index id
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        strike: f64,
        expiry: &JsDate,
        discount_curve: &str,
        vol_index_curve: &str,
        vol_of_vol_surface: &str,
        option_type: Option<String>,
        exercise_style: Option<String>,
        multiplier: Option<f64>,
        index_id: Option<String>,
    ) -> Result<JsVolatilityIndexOption, JsValue> {
        let option_type_value = parse_optional_with_default(option_type, OptionType::Call)?;
        let exercise_style_value = parse_exercise_style(exercise_style)?;

        let specs = VolIndexOptionSpecs {
            multiplier: multiplier.unwrap_or(100.0),
            index_id: index_id.unwrap_or_else(|| "VIX".to_string()),
        };

        let builder = VolatilityIndexOption::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .strike(strike)
            .expiry(expiry.inner())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .vol_index_curve_id(curve_id_from_str(vol_index_curve))
            .vol_of_vol_surface_id(curve_id_from_str(vol_of_vol_surface))
            .option_type(option_type_value)
            .exercise_style(exercise_style_value)
            .contract_specs(specs)
            .attributes(Default::default());

        builder
            .build()
            .map(JsVolatilityIndexOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

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

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::VolatilityIndexOption as u16
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
