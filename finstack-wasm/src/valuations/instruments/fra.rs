use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::money::JsMoney;
use crate::core::utils::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_valuations::instruments::fra::ForwardRateAgreement;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = ForwardRateAgreement)]
#[derive(Clone, Debug)]
pub struct JsForwardRateAgreement {
    inner: ForwardRateAgreement,
}

impl JsForwardRateAgreement {
    pub(crate) fn from_inner(inner: ForwardRateAgreement) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> ForwardRateAgreement {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = ForwardRateAgreement)]
impl JsForwardRateAgreement {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        fixing_date: &JsDate,
        start_date: &JsDate,
        end_date: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        day_count: Option<JsDayCount>,
        reset_lag: Option<i32>,
        pay_fixed: Option<bool>,
    ) -> Result<JsForwardRateAgreement, JsValue> {
        let mut builder = ForwardRateAgreement::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .fixed_rate(fixed_rate)
            .fixing_date(fixing_date.inner())
            .start_date(start_date.inner())
            .end_date(end_date.inner())
            .disc_id(curve_id_from_str(discount_curve))
            .forward_id(curve_id_from_str(forward_curve));

        if let Some(dc) = day_count {
            builder = builder.day_count(dc.inner());
        }
        if let Some(lag) = reset_lag {
            builder = builder.reset_lag(lag);
        }
        if let Some(pay) = pay_fixed {
            builder = builder.pay_fixed(pay);
        }

        builder
            .build()
            .map(JsForwardRateAgreement::from_inner)
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

    #[wasm_bindgen(getter, js_name = fixingDate)]
    pub fn fixing_date(&self) -> JsDate {
        JsDate::from_core(self.inner.fixing_date)
    }

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    #[wasm_bindgen(getter, js_name = endDate)]
    pub fn end_date(&self) -> JsDate {
        JsDate::from_core(self.inner.end_date)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.forward_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::FRA as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ForwardRateAgreement(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsForwardRateAgreement {
        JsForwardRateAgreement::from_inner(self.inner.clone())
    }
}

