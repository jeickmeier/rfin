use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;
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
    /// Create a CDS position that **buys protection** (long credit protection).
    ///
    /// Conventions:
    /// - `spread_bp` is in **basis points** (e.g. `120.0` for 120bp running spread).
    /// - `recovery_rate` is a fraction in **decimal** (e.g. `0.4` for 40%).
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - CDS notional (currency-tagged)
    /// @param spread_bp - Running premium spread in bps
    /// @param start_date - Effective start date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID (PV discounting)
    /// @param credit_curve - Hazard/credit curve ID (default probabilities)
    /// @param recovery_rate - Optional recovery rate override (decimal)
    /// @returns A new `CreditDefaultSwap`
    /// @throws {Error} If construction fails (e.g. invalid dates)
    ///
    /// @example
    /// ```javascript
    /// import init, { CreditDefaultSwap, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const cds = CreditDefaultSwap.buyProtection(
    ///   "cds_1",
    ///   Money.fromCode(5_000_000, "USD"),
    ///   120.0,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2029, 1, 2),
    ///   "USD-OIS",
    ///   "ACME-HAZARD",
    ///   0.4
    /// );
    /// ```
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

    /// Create a CDS position that **sells protection** (short credit protection).
    ///
    /// Conventions:
    /// - `spread_bp` is in **basis points**.
    /// - `recovery_rate` is a fraction in **decimal**.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - CDS notional (currency-tagged)
    /// @param spread_bp - Running premium spread in bps
    /// @param start_date - Effective start date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID (PV discounting)
    /// @param credit_curve - Hazard/credit curve ID (default probabilities)
    /// @param recovery_rate - Optional recovery rate override (decimal)
    /// @returns A new `CreditDefaultSwap`
    /// @throws {Error} If construction fails (e.g. invalid dates)
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
