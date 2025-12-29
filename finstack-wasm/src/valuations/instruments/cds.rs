use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use finstack_valuations::pricer::InstrumentType;
use rust_decimal::prelude::ToPrimitive;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CreditDefaultSwap)]
#[derive(Clone, Debug)]
pub struct JsCreditDefaultSwap {
    pub(crate) inner: CreditDefaultSwap,
}

impl InstrumentWrapper for JsCreditDefaultSwap {
    type Inner = CreditDefaultSwap;
    fn from_inner(inner: CreditDefaultSwap) -> Self {
        JsCreditDefaultSwap { inner }
    }
    fn inner(&self) -> CreditDefaultSwap {
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
    ) -> Result<JsCreditDefaultSwap, JsValue> {
        let mut cds = CreditDefaultSwap::buy_protection(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            spread_bp,
            start_date.inner(),
            maturity.inner(),
            curve_id_from_str(discount_curve),
            curve_id_from_str(credit_curve),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if let Some(rr) = recovery_rate {
            cds.protection.recovery_rate = rr;
        }
        Ok(JsCreditDefaultSwap::from_inner(cds))
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
    ) -> Result<JsCreditDefaultSwap, JsValue> {
        let mut cds = CreditDefaultSwap::sell_protection(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            spread_bp,
            start_date.inner(),
            maturity.inner(),
            curve_id_from_str(discount_curve),
            curve_id_from_str(credit_curve),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if let Some(rr) = recovery_rate {
            cds.protection.recovery_rate = rr;
        }
        Ok(JsCreditDefaultSwap::from_inner(cds))
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
        self.inner.premium.spread_bp.to_f64().unwrap_or(0.0)
    }

    #[wasm_bindgen(getter, js_name = recoveryRate)]
    pub fn recovery_rate(&self) -> f64 {
        self.inner.protection.recovery_rate
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.premium.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = creditCurve)]
    pub fn credit_curve(&self) -> String {
        self.inner.protection.credit_curve_id.as_str().to_string()
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
