use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::instrument_id_from_str;
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateSwap)]
#[derive(Clone, Debug)]
pub struct JsInterestRateSwap {
    inner: InterestRateSwap,
}

impl JsInterestRateSwap {
    pub(crate) fn from_inner(inner: InterestRateSwap) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> InterestRateSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateSwap)]
impl JsInterestRateSwap {
    #[wasm_bindgen(js_name = usdPayFixed)]
    pub fn usd_pay_fixed(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start: &JsDate,
        end: &JsDate,
    ) -> JsInterestRateSwap {
        let swap = InterestRateSwap::usd_pay_fixed(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            fixed_rate,
            start.inner(),
            end.inner(),
        );
        JsInterestRateSwap::from_inner(swap)
    }

    #[wasm_bindgen(js_name = usdReceiveFixed)]
    pub fn usd_receive_fixed(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start: &JsDate,
        end: &JsDate,
    ) -> JsInterestRateSwap {
        let swap = InterestRateSwap::usd_receive_fixed(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            fixed_rate,
            start.inner(),
            end.inner(),
        );
        JsInterestRateSwap::from_inner(swap)
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
        self.inner.fixed.rate
    }

    #[wasm_bindgen(getter, js_name = floatSpreadBp)]
    pub fn float_spread_bp(&self) -> f64 {
        self.inner.float.spread_bp
    }

    #[wasm_bindgen(getter)]
    pub fn start(&self) -> JsDate {
        JsDate::from_core(self.inner.fixed.start)
    }

    #[wasm_bindgen(getter)]
    pub fn end(&self) -> JsDate {
        JsDate::from_core(self.inner.fixed.end)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.fixed.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.float.fwd_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::IRS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InterestRateSwap(id='{}', notional={}, fixed_rate={:.4})",
            self.inner.id, self.inner.notional, self.inner.fixed.rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInterestRateSwap {
        JsInterestRateSwap::from_inner(self.inner.clone())
    }
}

