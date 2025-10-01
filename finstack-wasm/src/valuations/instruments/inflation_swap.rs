use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::core::utils::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, optional_static_str};
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_side(label: Option<String>) -> Result<PayReceiveInflation, JsValue> {
    match label.as_deref() {
        None | Some("pay_fixed") => Ok(PayReceiveInflation::PayFixed),
        Some("receive_fixed") => Ok(PayReceiveInflation::ReceiveFixed),
        Some(s) => s
            .parse()
            .map_err(|e: String| js_error(format!("Invalid side: {e}"))),
    }
}

fn parse_day_count(label: Option<String>) -> Result<DayCount, JsValue> {
    match label.as_deref() {
        None | Some("act_act") => Ok(DayCount::ActAct),
        Some("act_360") => Ok(DayCount::Act360),
        Some("act_365f") => Ok(DayCount::Act365F),
        Some(other) => Err(js_error(format!("Unsupported day_count: {other}"))),
    }
}

#[wasm_bindgen(js_name = InflationSwap)]
#[derive(Clone, Debug)]
pub struct JsInflationSwap {
    inner: InflationSwap,
}

impl JsInflationSwap {
    pub(crate) fn from_inner(inner: InflationSwap) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> InflationSwap {
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
        let side_value = parse_side(side)?;
        let dc = parse_day_count(day_count)?;
        
        let inflation_id = optional_static_str(Some(inflation_curve.to_string()))
            .ok_or_else(|| js_error("inflation_curve required".to_string()))?;

        let builder = InflationSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .fixed_rate(fixed_rate)
            .start(start_date.inner())
            .maturity(maturity.inner())
            .disc_id(curve_id_from_str(discount_curve))
            .inflation_id(inflation_id)
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

