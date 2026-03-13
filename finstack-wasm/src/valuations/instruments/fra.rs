use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::common::parameters::PayReceive;
use finstack_valuations::instruments::rates::fra::ForwardRateAgreement;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = ForwardRateAgreementBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsForwardRateAgreementBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    fixed_rate: Option<f64>,
    fixing_date: Option<finstack_core::dates::Date>,
    start_date: Option<finstack_core::dates::Date>,
    end_date: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    forward_curve: Option<String>,
    day_count: Option<finstack_core::dates::DayCount>,
    reset_lag: Option<i32>,
    receive_fixed: Option<bool>,
}

#[wasm_bindgen(js_class = ForwardRateAgreementBuilder)]
impl JsForwardRateAgreementBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsForwardRateAgreementBuilder {
        JsForwardRateAgreementBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsForwardRateAgreementBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = fixedRate)]
    pub fn fixed_rate(mut self, fixed_rate: f64) -> JsForwardRateAgreementBuilder {
        self.fixed_rate = Some(fixed_rate);
        self
    }

    #[wasm_bindgen(js_name = fixingDate)]
    pub fn fixing_date(mut self, fixing_date: &JsDate) -> JsForwardRateAgreementBuilder {
        self.fixing_date = Some(fixing_date.inner());
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsForwardRateAgreementBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = endDate)]
    pub fn end_date(mut self, end_date: &JsDate) -> JsForwardRateAgreementBuilder {
        self.end_date = Some(end_date.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsForwardRateAgreementBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = forwardCurve)]
    pub fn forward_curve(mut self, forward_curve: &str) -> JsForwardRateAgreementBuilder {
        self.forward_curve = Some(forward_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsForwardRateAgreementBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = resetLag)]
    pub fn reset_lag(mut self, reset_lag: i32) -> JsForwardRateAgreementBuilder {
        self.reset_lag = Some(reset_lag);
        self
    }

    /// Set the FRA direction: true = receive fixed rate, false = pay fixed rate.
    #[wasm_bindgen(js_name = receiveFixed)]
    pub fn receive_fixed(mut self, receive_fixed: bool) -> JsForwardRateAgreementBuilder {
        self.receive_fixed = Some(receive_fixed);
        self
    }

    /// Deprecated alias for receiveFixed(). Use receiveFixed() instead.
    #[wasm_bindgen(js_name = payFixed)]
    pub fn pay_fixed(mut self, pay_fixed: bool) -> JsForwardRateAgreementBuilder {
        self.receive_fixed = Some(pay_fixed);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsForwardRateAgreement, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("ForwardRateAgreementBuilder: notional (money) is required".to_string())
        })?;
        let fixed_rate = self.fixed_rate.ok_or_else(|| {
            js_error("ForwardRateAgreementBuilder: fixedRate is required".to_string())
        })?;
        let fixing_date = self.fixing_date.ok_or_else(|| {
            js_error("ForwardRateAgreementBuilder: fixingDate is required".to_string())
        })?;
        let start_date = self.start_date.ok_or_else(|| {
            js_error("ForwardRateAgreementBuilder: startDate is required".to_string())
        })?;
        let end_date = self.end_date.ok_or_else(|| {
            js_error("ForwardRateAgreementBuilder: endDate is required".to_string())
        })?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("ForwardRateAgreementBuilder: discountCurve is required".to_string())
        })?;
        let forward_curve = self.forward_curve.as_deref().ok_or_else(|| {
            js_error("ForwardRateAgreementBuilder: forwardCurve is required".to_string())
        })?;

        let mut builder = ForwardRateAgreement::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .fixed_rate(rust_decimal::Decimal::try_from(fixed_rate).unwrap_or_default())
            .fixing_date(fixing_date)
            .start_date(start_date)
            .maturity(end_date)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .forward_curve_id(curve_id_from_str(forward_curve));

        if let Some(dc) = self.day_count {
            builder = builder.day_count(dc);
        }
        if let Some(lag) = self.reset_lag {
            builder = builder.reset_lag(lag);
        }
        if let Some(receive) = self.receive_fixed {
            builder = builder.side(if receive {
                PayReceive::ReceiveFixed
            } else {
                PayReceive::PayFixed
            });
        }

        builder
            .build()
            .map(JsForwardRateAgreement::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

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
    /// @param receive_fixed - Optional direction (true = receive fixed rate)
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
        receive_fixed: Option<bool>,
    ) -> Result<JsForwardRateAgreement, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "ForwardRateAgreement constructor is deprecated; use ForwardRateAgreementBuilder instead.",
        ));
        let mut builder = ForwardRateAgreement::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .fixed_rate(rust_decimal::Decimal::try_from(fixed_rate).unwrap_or_default())
            .fixing_date(fixing_date.inner())
            .start_date(start_date.inner())
            .maturity(end_date.inner())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .forward_curve_id(curve_id_from_str(forward_curve));

        if let Some(dc) = day_count {
            builder = builder.day_count(dc.inner());
        }
        if let Some(lag) = reset_lag {
            builder = builder.reset_lag(lag);
        }
        if let Some(receive) = receive_fixed {
            builder = builder.side(if receive {
                PayReceive::ReceiveFixed
            } else {
                PayReceive::PayFixed
            });
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
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.fixed_rate).unwrap_or_default()
    }

    #[wasm_bindgen(getter, js_name = fixingDate)]
    pub fn fixing_date(&self) -> Option<JsDate> {
        self.inner.fixing_date.map(JsDate::from_core)
    }

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    #[wasm_bindgen(getter, js_name = endDate)]
    pub fn end_date(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
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
