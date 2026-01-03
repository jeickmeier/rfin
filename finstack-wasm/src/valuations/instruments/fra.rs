use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::fra::ForwardRateAgreement;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = ForwardRateAgreement)]
#[derive(Clone, Debug)]
pub struct JsForwardRateAgreement {
    pub(crate) inner: ForwardRateAgreement,
}

impl InstrumentWrapper for JsForwardRateAgreement {
    type Inner = ForwardRateAgreement;
    fn from_inner(inner: ForwardRateAgreement) -> Self {
        JsForwardRateAgreement { inner }
    }
    fn inner(&self) -> ForwardRateAgreement {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = ForwardRateAgreement)]
impl JsForwardRateAgreement {
    /// Create a forward rate agreement (FRA).
    ///
    /// Conventions:
    /// - `fixed_rate` is a **decimal rate** (e.g. `0.05` for 5%).
    /// - `reset_lag` is in days (defaults depend on model conventions if omitted).
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Notional (currency-tagged)
    /// @param fixed_rate - FRA fixed rate (decimal)
    /// @param fixing_date - Fixing date
    /// @param start_date - Accrual start date
    /// @param end_date - Accrual end date
    /// @param discount_curve - Discount curve ID
    /// @param forward_curve - Forward curve ID
    /// @param day_count - Optional day count (if omitted, library default applies)
    /// @param reset_lag - Optional reset lag in days
    /// @param pay_fixed - Optional direction (true pays fixed)
    /// @returns A new `ForwardRateAgreement`
    /// @throws {Error} If inputs are invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { ForwardRateAgreement, Money, FsDate, DayCount } from "finstack-wasm";
    ///
    /// await init();
    /// const fra = new ForwardRateAgreement(
    ///   "fra_1",
    ///   Money.fromCode(10_000_000, "USD"),
    ///   0.045,
    ///   new FsDate(2024, 3, 29),
    ///   new FsDate(2024, 4, 2),
    ///   new FsDate(2024, 7, 2),
    ///   "USD-OIS",
    ///   "USD-SOFR-3M",
    ///   DayCount.act360(),
    ///   2,
    ///   true
    /// );
    /// ```
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
            .discount_curve_id(curve_id_from_str(discount_curve))
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

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsForwardRateAgreement, JsValue> {
        from_js_value(value).map(JsForwardRateAgreement::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this FRA (settlement cashflow at start/end).
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::money::JsMoney;
        use finstack_valuations::cashflow::CashflowProvider;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let sched = self
            .inner
            .build_full_schedule(market.inner(), as_of)
            .map_err(|e| js_error(e.to_string()))?;
        let outstanding_path = sched
            .outstanding_path_per_flow()
            .map_err(|e| js_error(e.to_string()))?;

        let result = Array::new();
        for (idx, cf) in sched.flows.iter().enumerate() {
            let entry = Array::new();
            entry.push(&JsDate::from_core(cf.date).into());
            entry.push(&JsMoney::from_inner(cf.amount).into());
            entry.push(&JsValue::from_str(&format!("{:?}", cf.kind)));
            let outstanding = outstanding_path
                .get(idx)
                .map(|(_, m)| m.amount())
                .unwrap_or(0.0);
            entry.push(&JsValue::from_f64(outstanding));
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
        self.inner.discount_curve_id.as_str().to_string()
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
