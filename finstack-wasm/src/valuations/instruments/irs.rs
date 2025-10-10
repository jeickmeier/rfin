use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::instrument_id_from_str;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateSwap)]
#[derive(Clone, Debug)]
pub struct JsInterestRateSwap(InterestRateSwap);

impl InstrumentWrapper for JsInterestRateSwap {
    type Inner = InterestRateSwap;
    fn from_inner(inner: InterestRateSwap) -> Self {
        JsInterestRateSwap(inner)
    }
    fn inner(&self) -> InterestRateSwap {
        self.0.clone()
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
        use finstack_valuations::instruments::common::parameters::PayReceive;
        let swap = InterestRateSwap::new(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            fixed_rate,
            start.inner(),
            end.inner(),
            PayReceive::PayFixed,
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
        use finstack_valuations::instruments::common::parameters::PayReceive;
        let swap = InterestRateSwap::new(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            fixed_rate,
            start.inner(),
            end.inner(),
            PayReceive::ReceiveFixed,
        );
        JsInterestRateSwap::from_inner(swap)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.0.notional)
    }

    #[wasm_bindgen(getter, js_name = fixedRate)]
    pub fn fixed_rate(&self) -> f64 {
        self.0.fixed.rate
    }

    #[wasm_bindgen(getter, js_name = floatSpreadBp)]
    pub fn float_spread_bp(&self) -> f64 {
        self.0.float.spread_bp
    }

    #[wasm_bindgen(getter)]
    pub fn start(&self) -> JsDate {
        JsDate::from_core(self.0.fixed.start)
    }

    #[wasm_bindgen(getter)]
    pub fn end(&self) -> JsDate {
        JsDate::from_core(self.0.fixed.end)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.0.fixed.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.0.float.fwd_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::IRS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InterestRateSwap(id='{}', notional={}, fixed_rate={:.4})",
            self.0.id, self.0.notional, self.0.fixed.rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInterestRateSwap {
        JsInterestRateSwap::from_inner(self.0.clone())
    }
}
