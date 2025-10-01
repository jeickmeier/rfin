use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::money::JsMoney;
use crate::core::utils::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::ir_future::{
    FutureContractSpecs, InterestRateFuture, Position,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_position(label: Option<String>) -> Result<Position, JsValue> {
    match label.as_deref() {
        None | Some("long") => Ok(Position::Long),
        Some("short") => Ok(Position::Short),
        Some(s) => s
            .parse()
            .map_err(|e: String| js_error(format!("Invalid position: {e}"))),
    }
}

#[wasm_bindgen(js_name = InterestRateFuture)]
#[derive(Clone, Debug)]
pub struct JsInterestRateFuture {
    inner: InterestRateFuture,
}

impl JsInterestRateFuture {
    pub(crate) fn from_inner(inner: InterestRateFuture) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> InterestRateFuture {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateFuture)]
impl JsInterestRateFuture {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        quoted_price: f64,
        expiry: &JsDate,
        fixing_date: &JsDate,
        period_start: &JsDate,
        period_end: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        position: Option<String>,
        day_count: Option<JsDayCount>,
    ) -> Result<JsInterestRateFuture, JsValue> {
        let position_value = parse_position(position)?;
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::Act360);

        let builder = InterestRateFuture::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .quoted_price(quoted_price)
            .expiry_date(expiry.inner())
            .fixing_date(fixing_date.inner())
            .period_start(period_start.inner())
            .period_end(period_end.inner())
            .disc_id(curve_id_from_str(discount_curve))
            .forward_id(curve_id_from_str(forward_curve))
            .day_count(dc)
            .position(position_value)
            .contract_specs(FutureContractSpecs::default())
            .attributes(Default::default());

        builder
            .build()
            .map(JsInterestRateFuture::from_inner)
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

    #[wasm_bindgen(getter, js_name = quotedPrice)]
    pub fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::InterestRateFuture as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InterestRateFuture(id='{}', price={:.2})",
            self.inner.id, self.inner.quoted_price
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInterestRateFuture {
        JsInterestRateFuture::from_inner(self.inner.clone())
    }
}

