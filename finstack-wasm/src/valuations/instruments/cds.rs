use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CreditDefaultSwap)]
#[derive(Clone, Debug)]
pub struct JsCreditDefaultSwap {
    inner: CreditDefaultSwap,
}

impl JsCreditDefaultSwap {
    pub(crate) fn from_inner(inner: CreditDefaultSwap) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> CreditDefaultSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CreditDefaultSwap)]
impl JsCreditDefaultSwap {
    #[wasm_bindgen(js_name = buyProtection)]
    #[allow(clippy::too_many_arguments)]
    pub fn buy_protection(
        instrument_id: &str,
        notional: &JsMoney,
        spread_bp: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        credit_curve: &str,
        recovery_rate: Option<f64>,
    ) -> JsCreditDefaultSwap {
        let mut cds = CreditDefaultSwap::buy_protection(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            spread_bp,
            start_date.inner(),
            maturity.inner(),
            curve_id_from_str(discount_curve),
            curve_id_from_str(credit_curve),
        );
        if let Some(rr) = recovery_rate {
            cds.protection.recovery_rate = rr;
        }
        JsCreditDefaultSwap::from_inner(cds)
    }

    #[wasm_bindgen(js_name = sellProtection)]
    #[allow(clippy::too_many_arguments)]
    pub fn sell_protection(
        instrument_id: &str,
        notional: &JsMoney,
        spread_bp: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        credit_curve: &str,
        recovery_rate: Option<f64>,
    ) -> JsCreditDefaultSwap {
        let mut cds = CreditDefaultSwap::sell_protection(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            spread_bp,
            start_date.inner(),
            maturity.inner(),
            curve_id_from_str(discount_curve),
            curve_id_from_str(credit_curve),
        );
        if let Some(rr) = recovery_rate {
            cds.protection.recovery_rate = rr;
        }
        JsCreditDefaultSwap::from_inner(cds)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = spreadBp)]
    pub fn spread_bp(&self) -> f64 {
        self.inner.premium.spread_bp
    }

    #[wasm_bindgen(getter, js_name = recoveryRate)]
    pub fn recovery_rate(&self) -> f64 {
        self.inner.protection.recovery_rate
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.premium.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = creditCurve)]
    pub fn credit_curve(&self) -> String {
        self.inner.protection.credit_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.premium.start)
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.premium.end)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CDS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CreditDefaultSwap(id='{}', spread_bp={:.1})",
            self.inner.id, self.inner.premium.spread_bp
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCreditDefaultSwap {
        JsCreditDefaultSwap::from_inner(self.inner.clone())
    }
}

