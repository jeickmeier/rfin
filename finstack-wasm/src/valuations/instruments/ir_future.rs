use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::ir_future::{
    FutureContractSpecs, InterestRateFuture, Position,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateFutureBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsInterestRateFutureBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    quoted_price: Option<f64>,
    expiry: Option<finstack_core::dates::Date>,
    fixing_date: Option<finstack_core::dates::Date>,
    period_start: Option<finstack_core::dates::Date>,
    period_end: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    forward_curve: Option<String>,
    position: Option<String>,
    day_count: Option<DayCount>,
}

#[wasm_bindgen(js_class = InterestRateFutureBuilder)]
impl JsInterestRateFutureBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsInterestRateFutureBuilder {
        JsInterestRateFutureBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsInterestRateFutureBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = quotedPrice)]
    pub fn quoted_price(mut self, quoted_price: f64) -> JsInterestRateFutureBuilder {
        self.quoted_price = Some(quoted_price);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsInterestRateFutureBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = fixingDate)]
    pub fn fixing_date(mut self, fixing_date: &JsDate) -> JsInterestRateFutureBuilder {
        self.fixing_date = Some(fixing_date.inner());
        self
    }

    #[wasm_bindgen(js_name = periodStart)]
    pub fn period_start(mut self, period_start: &JsDate) -> JsInterestRateFutureBuilder {
        self.period_start = Some(period_start.inner());
        self
    }

    #[wasm_bindgen(js_name = periodEnd)]
    pub fn period_end(mut self, period_end: &JsDate) -> JsInterestRateFutureBuilder {
        self.period_end = Some(period_end.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsInterestRateFutureBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = forwardCurve)]
    pub fn forward_curve(mut self, forward_curve: &str) -> JsInterestRateFutureBuilder {
        self.forward_curve = Some(forward_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = position)]
    pub fn position(mut self, position: String) -> JsInterestRateFutureBuilder {
        self.position = Some(position);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsInterestRateFutureBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsInterestRateFuture, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("InterestRateFutureBuilder: notional (money) is required".to_string())
        })?;
        let quoted_price = self.quoted_price.ok_or_else(|| {
            js_error("InterestRateFutureBuilder: quotedPrice is required".to_string())
        })?;
        let expiry = self
            .expiry
            .ok_or_else(|| js_error("InterestRateFutureBuilder: expiry is required".to_string()))?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("InterestRateFutureBuilder: discountCurve is required".to_string())
        })?;
        let forward_curve = self.forward_curve.as_deref().ok_or_else(|| {
            js_error("InterestRateFutureBuilder: forwardCurve is required".to_string())
        })?;

        let position_value = parse_optional_with_default(self.position, Position::Long)?;
        let dc = self.day_count.unwrap_or(DayCount::Act360);
        let contract_specs = FutureContractSpecs {
            convexity_adjustment: Some(0.0),
            ..Default::default()
        };

        InterestRateFuture::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .quoted_price(quoted_price)
            .expiry(expiry)
            .fixing_date_opt(self.fixing_date)
            .period_start_opt(self.period_start)
            .period_end_opt(self.period_end)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .forward_curve_id(curve_id_from_str(forward_curve))
            .day_count(dc)
            .position(position_value)
            .contract_specs(contract_specs)
            .attributes(Default::default())
            .build()
            .map(JsInterestRateFuture::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = InterestRateFuture)]
#[derive(Clone, Debug)]
pub struct JsInterestRateFuture {
    pub(crate) inner: InterestRateFuture,
}

impl InstrumentWrapper for JsInterestRateFuture {
    type Inner = InterestRateFuture;
    fn from_inner(inner: InterestRateFuture) -> Self {
        JsInterestRateFuture { inner }
    }
    fn inner(&self) -> InterestRateFuture {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateFuture)]
impl JsInterestRateFuture {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInterestRateFuture, JsValue> {
        from_js_value(value).map(JsInterestRateFuture::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
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
    pub fn instrument_type(&self) -> String {
        InstrumentType::InterestRateFuture.to_string()
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
