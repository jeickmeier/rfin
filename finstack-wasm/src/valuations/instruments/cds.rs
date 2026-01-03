use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_valuations::pricer::InstrumentType;
use rust_decimal::prelude::ToPrimitive;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

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
    /// Create a CDS position.
    ///
    /// Conventions:
    /// - `spread_bp` is in **basis points** (e.g. `120.0` for 120bp running spread).
    /// - `recovery_rate` is a fraction in **decimal** (e.g. `0.4` for 40%).
    /// - `side`: `"buy_protection"` or `"sell_protection"`.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - CDS notional (currency-tagged)
    /// @param spread_bp - Running premium spread in bps
    /// @param start_date - Effective start date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID (PV discounting)
    /// @param credit_curve - Hazard/credit curve ID (default probabilities)
    /// @param side - `"buy_protection"` or `"sell_protection"`
    /// @param recovery_rate - Optional recovery rate override (decimal)
    /// @returns A new `CreditDefaultSwap`
    /// @throws {Error} If construction fails (e.g. invalid dates)
    ///
    /// @example
    /// ```javascript
    /// import init, { CreditDefaultSwap, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const cds = new CreditDefaultSwap(
    ///   "cds_1",
    ///   Money.fromCode(5_000_000, "USD"),
    ///   120.0,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2029, 1, 2),
    ///   "USD-OIS",
    ///   "ACME-HAZARD",
    ///   "buy_protection",
    ///   0.4
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        spread_bp: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        credit_curve: &str,
        side: &str,
        recovery_rate: Option<f64>,
    ) -> Result<JsCreditDefaultSwap, JsValue> {
        let id = instrument_id_from_str(instrument_id);
        let disc = curve_id_from_str(discount_curve);
        let credit = curve_id_from_str(credit_curve);
        let mut cds = match side.to_lowercase().as_str() {
            "buy_protection" => CreditDefaultSwap::buy_protection(
                id,
                notional.inner(),
                spread_bp,
                start_date.inner(),
                maturity.inner(),
                disc,
                credit,
            ),
            "sell_protection" => CreditDefaultSwap::sell_protection(
                id,
                notional.inner(),
                spread_bp,
                start_date.inner(),
                maturity.inner(),
                disc,
                credit,
            ),
            other => {
                return Err(JsValue::from_str(&format!(
                    "Invalid side '{other}'; expected 'buy_protection' or 'sell_protection'"
                )));
            }
        }
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if let Some(rr) = recovery_rate {
            cds.protection.recovery_rate = rr;
        }
        Ok(JsCreditDefaultSwap::from_inner(cds))
    }

    /// Parse a CDS from a JSON value (as produced by `toJson`).
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCreditDefaultSwap, JsValue> {
        from_js_value(value).map(JsCreditDefaultSwap::from_inner)
    }

    /// Serialize this CDS to a JSON value.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get premium-leg cashflows for this CDS.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    /// - kind is always \"Premium\" for CDS cashflows
    /// - outstanding_balance is null (not applicable)
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<js_sys::Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use js_sys::Array;

        let flows = self
            .inner
            .build_premium_schedule(market.inner(), self.inner.premium.start)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let result = Array::new();
        for (d, amt) in flows {
            let entry = Array::new();
            entry.push(&JsDate::from_core(d).into());
            entry.push(&JsMoney::from_inner(amt).into());
            entry.push(&JsValue::from_str("Premium"));
            entry.push(&JsValue::NULL);
            result.push(&entry);
        }
        Ok(result)
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
