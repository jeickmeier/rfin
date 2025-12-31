use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InflationSwap)]
#[derive(Clone, Debug)]
pub struct JsInflationSwap {
    pub(crate) inner: InflationSwap,
}

impl InstrumentWrapper for JsInflationSwap {
    type Inner = InflationSwap;
    fn from_inner(inner: InflationSwap) -> Self {
        JsInflationSwap { inner }
    }
    fn inner(&self) -> InflationSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InflationSwap)]
impl JsInflationSwap {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        inflation_curve: &str,
        side: Option<String>,
        day_count: Option<String>,
    ) -> Result<JsInflationSwap, JsValue> {
        let side_value = parse_optional_with_default(side, PayReceiveInflation::PayFixed)?;
        let dc = parse_optional_with_default(day_count, DayCount::ActAct)?;

        let builder = InflationSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .fixed_rate(fixed_rate)
            .start(start_date.inner())
            .maturity(maturity.inner())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .inflation_index_id(inflation_curve.into())
            .dc(dc)
            .side(side_value)
            .attributes(Default::default());

        builder
            .build()
            .map(JsInflationSwap::from_inner)
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

    #[wasm_bindgen(getter, js_name = fixedRate)]
    pub fn fixed_rate(&self) -> f64 {
        self.inner.fixed_rate
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::InflationSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InflationSwap(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInflationSwap {
        JsInflationSwap::from_inner(self.inner.clone())
    }
}
