use crate::core::market_data::context::JsMarketContext;
use crate::core::utils::js_error;
use crate::valuations::instruments::{Bond as JsBond, Deposit as JsDeposit};
use crate::valuations::results::JsValuationResult;
use finstack_valuations::instruments::build_with_metrics_dyn;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use finstack_valuations::pricer::{create_standard_registry, ModelKey, PricerRegistry};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

fn parse_model_key(model: &str) -> Result<ModelKey, JsValue> {
    ModelKey::from_str(model).map_err(js_error)
}

fn metrics_from_array(array: &js_sys::Array) -> Vec<MetricId> {
    array
        .iter()
        .filter_map(|value| value.as_string())
        .map(|name| MetricId::from_str(&name).unwrap_or_else(|_| MetricId::custom(name)))
        .collect()
}

fn pricing_error_to_js(err: finstack_valuations::pricer::PricingError) -> JsValue {
    js_error(err.to_string())
}

fn core_error_to_js(err: finstack_core::Error) -> JsValue {
    js_error(err.to_string())
}

fn price_with_optional_metrics(
    registry: &PricerRegistry,
    instrument: &dyn Instrument,
    model_key: ModelKey,
    market: &JsMarketContext,
    metrics: Option<&js_sys::Array>,
) -> Result<JsValuationResult, JsValue> {
    let base = registry
        .price_with_registry(instrument, model_key, market.inner())
        .map_err(pricing_error_to_js)?;

    if let Some(list) = metrics {
        if list.length() == 0 {
            return Ok(JsValuationResult::new(base));
        }
        let metric_ids = metrics_from_array(list);
        return build_with_metrics_dyn(
            instrument,
            market.inner(),
            base.as_of,
            base.value,
            &metric_ids,
        )
        .map(JsValuationResult::new)
        .map_err(core_error_to_js);
    }

    Ok(JsValuationResult::new(base))
}

#[wasm_bindgen(js_name = PricerRegistry)]
pub struct JsPricerRegistry {
    inner: PricerRegistry,
}

impl JsPricerRegistry {
    pub(crate) fn new(inner: PricerRegistry) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = PricerRegistry)]
impl JsPricerRegistry {
    #[wasm_bindgen(constructor)]
    pub fn new_empty() -> JsPricerRegistry {
        JsPricerRegistry::new(PricerRegistry::new())
    }

    #[wasm_bindgen(js_name = priceBond)]
    pub fn price_bond(
        &self,
        bond: &JsBond,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceBondWithMetrics)]
    pub fn price_bond_with_metrics(
        &self,
        bond: &JsBond,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = bond.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }

    #[wasm_bindgen(js_name = priceDeposit)]
    pub fn price_deposit(
        &self,
        deposit: &JsDeposit,
        model: &str,
        market: &JsMarketContext,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = deposit.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, None)
    }

    #[wasm_bindgen(js_name = priceDepositWithMetrics)]
    pub fn price_deposit_with_metrics(
        &self,
        deposit: &JsDeposit,
        model: &str,
        market: &JsMarketContext,
        metrics: &js_sys::Array,
    ) -> Result<JsValuationResult, JsValue> {
        let model_key = parse_model_key(model)?;
        let instrument = deposit.inner();
        price_with_optional_metrics(&self.inner, &instrument, model_key, market, Some(metrics))
    }
}

#[wasm_bindgen(js_name = createStandardRegistry)]
pub fn create_standard_registry_js() -> JsPricerRegistry {
    JsPricerRegistry::new(create_standard_registry())
}
