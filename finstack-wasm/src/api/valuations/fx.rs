//! Direct WASM wrappers for FX valuation instruments.

use crate::utils::to_js_err;
use serde_json::{Map, Value};
use wasm_bindgen::prelude::*;

fn canonical_payload(type_tag: &str, value: Value) -> Result<String, JsValue> {
    let payload = if value.get("type").is_some() {
        let actual = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| JsValue::from_str("instrument JSON field `type` must be a string"))?;
        if actual != type_tag {
            return Err(JsValue::from_str(&format!(
                "expected instrument type `{type_tag}`, got `{actual}`"
            )));
        }
        value
    } else {
        let mut payload = Map::new();
        payload.insert("type".to_string(), Value::String(type_tag.to_string()));
        payload.insert("spec".to_string(), value);
        Value::Object(payload)
    };

    let json = serde_json::to_string(&payload).map_err(to_js_err)?;
    finstack_valuations::pricer::validate_instrument_json(&json).map_err(to_js_err)
}

fn value_from_spec(spec: JsValue) -> Result<Value, JsValue> {
    if let Some(json) = spec.as_string() {
        serde_json::from_str(&json).map_err(to_js_err)
    } else {
        serde_wasm_bindgen::from_value(spec).map_err(to_js_err)
    }
}

fn from_spec(type_tag: &str, spec: JsValue) -> Result<String, JsValue> {
    canonical_payload(type_tag, value_from_spec(spec)?)
}

fn from_json_payload(type_tag: &str, json: &str) -> Result<String, JsValue> {
    let value: Value = serde_json::from_str(json).map_err(to_js_err)?;
    canonical_payload(type_tag, value)
}

fn pretty_json(json: &str) -> Result<String, JsValue> {
    let value: Value = serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string_pretty(&value).map_err(to_js_err)
}

fn price_payload(
    json: &str,
    market_json: &str,
    as_of: &str,
    model: Option<String>,
) -> Result<String, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let result = finstack_valuations::pricer::price_instrument_json(
        json,
        &market,
        as_of,
        model.as_deref().unwrap_or("default"),
    )
    .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

fn price_payload_with_metrics(
    json: &str,
    market_json: &str,
    as_of: &str,
    metrics: JsValue,
    model: Option<String>,
    pricing_options: Option<String>,
) -> Result<String, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let metrics: Vec<String> = serde_wasm_bindgen::from_value(metrics).map_err(to_js_err)?;
    let result = finstack_valuations::pricer::price_instrument_json_with_metrics(
        json,
        &market,
        as_of,
        model.as_deref().unwrap_or("default"),
        &metrics,
        pricing_options.as_deref(),
    )
    .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

fn metric_value(
    json: &str,
    market_json: &str,
    as_of: &str,
    model: Option<String>,
    metric: &str,
) -> Result<f64, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let result = finstack_valuations::pricer::price_instrument_json_with_metrics(
        json,
        &market,
        as_of,
        model.as_deref().unwrap_or("default"),
        &[metric.to_string()],
        None,
    )
    .map_err(to_js_err)?;
    result
        .metric_str(metric)
        .ok_or_else(|| JsValue::from_str(&format!("metric `{metric}` was not returned")))
}

macro_rules! fx_class {
    ($rust_name:ident, $js_name:literal, $type_tag:literal) => {
        #[wasm_bindgen(js_name = $js_name)]
        pub struct $rust_name {
            json: String,
        }

        #[wasm_bindgen(js_class = $js_name)]
        impl $rust_name {
            #[wasm_bindgen(constructor)]
            pub fn new(spec: JsValue) -> Result<$rust_name, JsValue> {
                Ok(Self {
                    json: from_spec($type_tag, spec)?,
                })
            }

            #[wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(json: &str) -> Result<$rust_name, JsValue> {
                Ok(Self {
                    json: from_json_payload($type_tag, json)?,
                })
            }

            #[wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<String, JsValue> {
                pretty_json(&self.json)
            }

            pub fn price(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<String, JsValue> {
                price_payload(&self.json, market_json, as_of, model)
            }

            #[wasm_bindgen(js_name = priceWithMetrics)]
            pub fn price_with_metrics(
                &self,
                market_json: &str,
                as_of: &str,
                metrics: JsValue,
                model: Option<String>,
                pricing_options: Option<String>,
            ) -> Result<String, JsValue> {
                price_payload_with_metrics(
                    &self.json,
                    market_json,
                    as_of,
                    metrics,
                    model,
                    pricing_options,
                )
            }
        }
    };
}

macro_rules! fx_option_class {
    ($rust_name:ident, $js_name:literal, $type_tag:literal) => {
        fx_class!($rust_name, $js_name, $type_tag);

        #[wasm_bindgen(js_class = $js_name)]
        impl $rust_name {
            pub fn delta(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "delta")
            }

            pub fn gamma(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "gamma")
            }

            pub fn vega(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "vega")
            }

            pub fn theta(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "theta")
            }

            pub fn rho(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "rho")
            }

            #[wasm_bindgen(js_name = foreignRho)]
            pub fn foreign_rho(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "foreign_rho")
            }

            pub fn vanna(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "vanna")
            }

            pub fn volga(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<f64, JsValue> {
                metric_value(&self.json, market_json, as_of, model, "volga")
            }

            pub fn greeks(
                &self,
                market_json: &str,
                as_of: &str,
                model: Option<String>,
            ) -> Result<JsValue, JsValue> {
                let mut out = Map::new();
                for metric in [
                    "delta",
                    "gamma",
                    "vega",
                    "theta",
                    "rho",
                    "foreign_rho",
                    "vanna",
                    "volga",
                ] {
                    if let Ok(value) =
                        metric_value(&self.json, market_json, as_of, model.clone(), metric)
                    {
                        out.insert(metric.to_string(), Value::from(value));
                    }
                }
                serde_wasm_bindgen::to_value(&Value::Object(out)).map_err(to_js_err)
            }
        }
    };
}

fx_class!(WasmFxSpot, "FxSpot", "fx_spot");
fx_class!(WasmFxForward, "FxForward", "fx_forward");
fx_class!(WasmFxSwap, "FxSwap", "fx_swap");
fx_class!(WasmNdf, "Ndf", "ndf");
fx_option_class!(WasmFxOption, "FxOption", "fx_option");
fx_option_class!(WasmFxDigitalOption, "FxDigitalOption", "fx_digital_option");
fx_option_class!(WasmFxTouchOption, "FxTouchOption", "fx_touch_option");
fx_option_class!(WasmFxBarrierOption, "FxBarrierOption", "fx_barrier_option");
fx_class!(WasmFxVarianceSwap, "FxVarianceSwap", "fx_variance_swap");
fx_option_class!(WasmQuantoOption, "QuantoOption", "quanto_option");
