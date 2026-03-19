use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::deposit::Deposit;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = DepositBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsDepositBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    start: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    day_count: Option<finstack_core::dates::DayCount>,
    discount_curve: Option<String>,
    quote_rate: Option<f64>,
}

#[wasm_bindgen(js_class = DepositBuilder)]
impl JsDepositBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsDepositBuilder {
        JsDepositBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsDepositBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = start)]
    pub fn start(mut self, start: &JsDate) -> JsDepositBuilder {
        self.start = Some(start.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsDepositBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: &JsDayCount) -> JsDepositBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsDepositBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = quoteRate)]
    pub fn quote_rate(mut self, quote_rate: f64) -> JsDepositBuilder {
        self.quote_rate = Some(quote_rate);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsDeposit, JsValue> {
        let notional = self
            .notional
            .ok_or_else(|| js_error("DepositBuilder: notional (money) is required".to_string()))?;
        let start = self
            .start
            .ok_or_else(|| js_error("DepositBuilder: start is required".to_string()))?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("DepositBuilder: maturity is required".to_string()))?;
        let day_count = self
            .day_count
            .ok_or_else(|| js_error("DepositBuilder: dayCount is required".to_string()))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("DepositBuilder: discountCurve is required".to_string()))?;

        Deposit::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .start_date(start)
            .maturity(maturity)
            .day_count(day_count)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .quote_rate_opt({
                self.quote_rate
                    .map(|rate| crate::valuations::common::f64_to_decimal(rate, "quote_rate"))
                    .transpose()?
            })
            .build()
            .map(JsDeposit::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = Deposit)]
#[derive(Clone, Debug)]
pub struct JsDeposit {
    pub(crate) inner: Deposit,
}

impl InstrumentWrapper for JsDeposit {
    type Inner = Deposit;
    fn from_inner(inner: Deposit) -> Self {
        JsDeposit { inner }
    }
    fn inner(&self) -> Deposit {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Deposit)]
impl JsDeposit {
    /// Create a money-market deposit accruing simple interest over a date range.
    ///
    /// Conventions:
    /// - `quote_rate` is a **decimal rate** (e.g. `0.0475` for 4.75%).
    /// - Accrual is computed using the provided `day_count`.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Deposit notional (currency-tagged)
    /// @param start - Start date (trade/settlement start)
    /// @param maturity - Maturity date
    /// @param day_count - Day count convention for accrual (e.g. `DayCount.act360()`)
    /// @param discount_curve - Discount curve ID (must exist in `MarketContext` when pricing)
    /// @param quote_rate - Optional quoted deposit rate (decimal). If omitted, some models may treat it as 0 or infer from curves.
    /// @returns A new `Deposit`
    /// @throws {Error} If inputs are invalid (e.g., start > end)
    ///
    /// @example
    /// ```javascript
    /// import init, { Deposit, Money, FsDate, DayCount } from "finstack-wasm";
    ///
    /// await init();
    /// const dep = new Deposit(
    ///   "dep_1",
    ///   Money.fromCode(10_000_000, "USD"),
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2024, 4, 2),
    ///   DayCount.act360(),
    ///   "USD-OIS",
    ///   0.0475
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        start: &JsDate,
        maturity: &JsDate,
        day_count: &JsDayCount,
        discount_curve: &str,
        quote_rate: Option<f64>,
    ) -> Result<JsDeposit, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "Deposit constructor is deprecated; use DepositBuilder instead.",
        ));
        Deposit::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .start_date(start.inner())
            .maturity(maturity.inner())
            .day_count(day_count.inner())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .quote_rate_opt(
                quote_rate
                    .map(|rate| crate::valuations::common::f64_to_decimal(rate, "quote_rate"))
                    .transpose()?,
            )
            .build()
            .map(JsDeposit::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsDeposit, JsValue> {
        from_js_value(value).map(JsDeposit::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get the cashflow schedule for this deposit.
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

    #[wasm_bindgen(getter)]
    pub fn start(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    #[wasm_bindgen(getter, js_name = quoteRate)]
    pub fn quote_rate(&self) -> Option<f64> {
        self.inner
            .quote_rate
            .as_ref()
            .and_then(rust_decimal::prelude::ToPrimitive::to_f64)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::Deposit.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Deposit(id='{}', start='{}', maturity='{}', quote_rate={:?})",
            self.inner.id, self.inner.start_date, self.inner.maturity, self.inner.quote_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsDeposit {
        JsDeposit::from_inner(self.inner.clone())
    }
}
