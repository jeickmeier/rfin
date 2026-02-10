use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::instrument_id_from_str;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::equity_trs::EquityTotalReturnSwap;
use finstack_valuations::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;
use finstack_valuations::instruments::FinancingLegSpec;
use finstack_valuations::instruments::{EquityUnderlyingParams, IndexUnderlyingParams};
use finstack_valuations::instruments::{TrsScheduleSpec, TrsSide};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use rust_decimal::Decimal;
use wasm_bindgen::prelude::*;

// Simplified TRS schedule spec for WASM
#[wasm_bindgen(js_name = TrsScheduleSpec)]
#[derive(Clone, Debug)]
pub struct JsTrsScheduleSpec {
    pub(crate) inner: TrsScheduleSpec,
}

#[wasm_bindgen(js_class = TrsScheduleSpec)]
impl JsTrsScheduleSpec {
    /// Create a schedule specification for a total return swap.
    ///
    /// @param start - Schedule start date
    /// @param end - Schedule end date
    /// @param schedule_params - Schedule parameters (frequency/day count/bdc/etc.)
    /// @returns A `TrsScheduleSpec`
    /// @throws {Error} If `end` is not after `start`
    #[wasm_bindgen(constructor)]
    pub fn new(
        start: &JsDate,
        end: &JsDate,
        schedule_params: &crate::valuations::cashflow::builder::JsScheduleParams,
    ) -> Result<JsTrsScheduleSpec, JsValue> {
        if end.inner() <= start.inner() {
            return Err(js_error("Schedule end must be after start".to_string()));
        }

        let spec =
            TrsScheduleSpec::from_params(start.inner(), end.inner(), schedule_params.inner());
        Ok(JsTrsScheduleSpec { inner: spec })
    }
}

// Financing leg specification
#[wasm_bindgen(js_name = TrsFinancingLegSpec)]
#[derive(Clone, Debug)]
pub struct JsFinancingLegSpec {
    pub(crate) inner: FinancingLegSpec,
}

#[wasm_bindgen(js_class = TrsFinancingLegSpec)]
impl JsFinancingLegSpec {
    /// Create a financing leg specification for a TRS.
    ///
    /// Conventions:
    /// - `spread_bp` is in **basis points**.
    ///
    /// @param discount_curve - Discount curve ID
    /// @param forward_curve - Forward curve ID
    /// @param day_count - Day count for financing accrual
    /// @param spread_bp - Optional spread in basis points (default 0)
    /// @returns A `TrsFinancingLegSpec`
    #[wasm_bindgen(constructor)]
    pub fn new(
        discount_curve: &str,
        forward_curve: &str,
        day_count: &crate::core::dates::daycount::JsDayCount,
        spread_bp: Option<f64>,
    ) -> JsFinancingLegSpec {
        // Convert f64 to Decimal; fallback to ZERO if conversion fails (shouldn't happen for valid bp values)
        let spread_decimal = Decimal::try_from(spread_bp.unwrap_or(0.0)).unwrap_or(Decimal::ZERO);
        JsFinancingLegSpec {
            inner: FinancingLegSpec::new(
                discount_curve.to_string(),
                forward_curve.to_string(),
                spread_decimal,
                day_count.inner(),
            ),
        }
    }
}

// Equity underlying parameters
#[wasm_bindgen(js_name = EquityUnderlying)]
#[derive(Clone, Debug)]
pub struct JsEquityUnderlying {
    pub(crate) inner: EquityUnderlyingParams,
}

#[wasm_bindgen(js_class = EquityUnderlying)]
impl JsEquityUnderlying {
    /// Create equity underlying parameters for an equity TRS.
    ///
    /// @param ticker - Equity ticker/symbol
    /// @param spot_id - Market scalar/price id for spot
    /// @param currency - Equity currency
    /// @param div_yield_id - Optional dividend yield id
    /// @returns An `EquityUnderlying`
    #[wasm_bindgen(constructor)]
    pub fn new(
        ticker: &str,
        spot_id: &str,
        currency: &JsCurrency,
        div_yield_id: Option<String>,
    ) -> JsEquityUnderlying {
        let mut params = EquityUnderlyingParams::new(ticker, spot_id, currency.inner());
        if let Some(div) = div_yield_id {
            params = params.with_dividend_yield(&div);
        }
        JsEquityUnderlying { inner: params }
    }
}

// Index underlying parameters
#[wasm_bindgen(js_name = IndexUnderlying)]
#[derive(Clone, Debug)]
pub struct JsIndexUnderlying {
    pub(crate) inner: IndexUnderlyingParams,
}

#[wasm_bindgen(js_class = IndexUnderlying)]
impl JsIndexUnderlying {
    /// Create index underlying parameters for an index TRS.
    ///
    /// @param index_id - Index identifier
    /// @param base_currency - Index currency
    /// @param yield_id - Optional yield id
    /// @returns An `IndexUnderlying`
    #[wasm_bindgen(constructor)]
    pub fn new(
        index_id: &str,
        base_currency: &JsCurrency,
        yield_id: Option<String>,
    ) -> JsIndexUnderlying {
        let mut params = IndexUnderlyingParams::new(index_id, base_currency.inner());
        if let Some(y) = yield_id {
            params = params.with_yield(&y);
        }
        JsIndexUnderlying { inner: params }
    }
}

// Equity TRS
#[wasm_bindgen(js_name = EquityTotalReturnSwap)]
#[derive(Clone, Debug)]
pub struct JsEquityTotalReturnSwap {
    pub(crate) inner: EquityTotalReturnSwap,
}

impl InstrumentWrapper for JsEquityTotalReturnSwap {
    type Inner = EquityTotalReturnSwap;
    fn from_inner(inner: EquityTotalReturnSwap) -> Self {
        JsEquityTotalReturnSwap { inner }
    }
    fn inner(&self) -> EquityTotalReturnSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = EquityTotalReturnSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsEquityTotalReturnSwapBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    underlying: Option<EquityUnderlyingParams>,
    financing: Option<FinancingLegSpec>,
    schedule: Option<TrsScheduleSpec>,
    receive_total_return: Option<bool>,
    initial_level: Option<f64>,
    dividend_tax_rate: Option<f64>,
    discrete_dividends: Vec<(finstack_core::dates::Date, f64)>,
}

#[wasm_bindgen(js_class = EquityTotalReturnSwapBuilder)]
impl JsEquityTotalReturnSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsEquityTotalReturnSwapBuilder {
        JsEquityTotalReturnSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsEquityTotalReturnSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = underlying)]
    pub fn underlying(mut self, underlying: &JsEquityUnderlying) -> JsEquityTotalReturnSwapBuilder {
        self.underlying = Some(underlying.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = financing)]
    pub fn financing(mut self, financing: &JsFinancingLegSpec) -> JsEquityTotalReturnSwapBuilder {
        self.financing = Some(financing.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = schedule)]
    pub fn schedule(mut self, schedule: &JsTrsScheduleSpec) -> JsEquityTotalReturnSwapBuilder {
        self.schedule = Some(schedule.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = receiveTotalReturn)]
    pub fn receive_total_return(
        mut self,
        receive_total_return: bool,
    ) -> JsEquityTotalReturnSwapBuilder {
        self.receive_total_return = Some(receive_total_return);
        self
    }

    #[wasm_bindgen(js_name = initialLevel)]
    pub fn initial_level(mut self, initial_level: f64) -> JsEquityTotalReturnSwapBuilder {
        self.initial_level = Some(initial_level);
        self
    }

    #[wasm_bindgen(js_name = dividendTaxRate)]
    pub fn dividend_tax_rate(
        mut self,
        dividend_tax_rate: f64,
    ) -> Result<JsEquityTotalReturnSwapBuilder, JsValue> {
        if !dividend_tax_rate.is_finite() || !(0.0..=1.0).contains(&dividend_tax_rate) {
            return Err(js_error(
                "EquityTotalReturnSwapBuilder: dividendTaxRate must be finite and in [0, 1]"
                    .to_string(),
            ));
        }
        self.dividend_tax_rate = Some(dividend_tax_rate);
        Ok(self)
    }

    #[wasm_bindgen(js_name = addDiscreteDividend)]
    pub fn add_discrete_dividend(
        mut self,
        ex_date: &JsDate,
        amount: f64,
    ) -> Result<JsEquityTotalReturnSwapBuilder, JsValue> {
        if !amount.is_finite() {
            return Err(js_error(
                "EquityTotalReturnSwapBuilder: discrete dividend amount must be finite".to_string(),
            ));
        }
        self.discrete_dividends.push((ex_date.inner(), amount));
        Ok(self)
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsEquityTotalReturnSwap, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("EquityTotalReturnSwapBuilder: notional (money) is required".to_string())
        })?;
        let underlying = self.underlying.ok_or_else(|| {
            js_error("EquityTotalReturnSwapBuilder: underlying is required".to_string())
        })?;
        let financing = self.financing.ok_or_else(|| {
            js_error("EquityTotalReturnSwapBuilder: financing is required".to_string())
        })?;
        let schedule = self.schedule.ok_or_else(|| {
            js_error("EquityTotalReturnSwapBuilder: schedule is required".to_string())
        })?;
        let receive_total_return = self.receive_total_return.ok_or_else(|| {
            js_error("EquityTotalReturnSwapBuilder: receiveTotalReturn is required".to_string())
        })?;

        let side = if receive_total_return {
            TrsSide::ReceiveTotalReturn
        } else {
            TrsSide::PayTotalReturn
        };

        let trs = EquityTotalReturnSwap {
            id: instrument_id_from_str(&self.instrument_id),
            notional,
            underlying,
            financing,
            schedule,
            side,
            initial_level: self.initial_level,
            dividend_tax_rate: self.dividend_tax_rate.unwrap_or(0.0),
            discrete_dividends: self.discrete_dividends,
            attributes: Default::default(),
            margin_spec: None,
        };

        Ok(JsEquityTotalReturnSwap::from_inner(trs))
    }
}

#[wasm_bindgen(js_class = EquityTotalReturnSwap)]
impl JsEquityTotalReturnSwap {
    /// Create an equity total return swap (TRS).
    ///
    /// Conventions:
    /// - `receive_total_return = true` means you receive the underlying total return and pay financing.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - TRS notional (currency-tagged)
    /// @param underlying - Equity underlying parameters
    /// @param financing - Financing leg specification
    /// @param schedule - Payment/reset schedule specification
    /// @param receive_total_return - Direction flag
    /// @param initial_level - Optional initial level/spot override
    /// @returns A new `EquityTotalReturnSwap`
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        underlying: &JsEquityUnderlying,
        financing: &JsFinancingLegSpec,
        schedule: &JsTrsScheduleSpec,
        receive_total_return: bool,
        initial_level: Option<f64>,
    ) -> JsEquityTotalReturnSwap {
        web_sys::console::warn_1(&JsValue::from_str(
            "EquityTotalReturnSwap constructor is deprecated; use EquityTotalReturnSwapBuilder instead.",
        ));
        let side = if receive_total_return {
            TrsSide::ReceiveTotalReturn
        } else {
            TrsSide::PayTotalReturn
        };

        let trs = EquityTotalReturnSwap {
            id: instrument_id_from_str(instrument_id),
            notional: notional.inner(),
            underlying: underlying.inner.clone(),
            financing: financing.inner.clone(),
            schedule: schedule.inner.clone(),
            side,
            initial_level,
            dividend_tax_rate: 0.0, // Default: no withholding tax
            discrete_dividends: Vec::new(),
            attributes: Default::default(),
            margin_spec: None,
        };

        JsEquityTotalReturnSwap::from_inner(trs)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsEquityTotalReturnSwap, JsValue> {
        from_js_value(value).map(JsEquityTotalReturnSwap::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this equity TRS.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use finstack_valuations::cashflow::CashflowProvider;

        let disc = market
            .inner()
            .get_discount(self.inner.financing.discount_curve_id.as_str())
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

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::EquityTotalReturnSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "EquityTotalReturnSwap(id='{}', notional={})",
            self.inner.id, self.inner.notional
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsEquityTotalReturnSwap {
        JsEquityTotalReturnSwap::from_inner(self.inner.clone())
    }
}

// FI Index TRS
#[wasm_bindgen(js_name = FiIndexTotalReturnSwap)]
#[derive(Clone, Debug)]
pub struct JsFiIndexTotalReturnSwap {
    pub(crate) inner: FIIndexTotalReturnSwap,
}

impl InstrumentWrapper for JsFiIndexTotalReturnSwap {
    type Inner = FIIndexTotalReturnSwap;
    fn from_inner(inner: FIIndexTotalReturnSwap) -> Self {
        JsFiIndexTotalReturnSwap { inner }
    }
    fn inner(&self) -> FIIndexTotalReturnSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = FiIndexTotalReturnSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFiIndexTotalReturnSwapBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    underlying: Option<IndexUnderlyingParams>,
    financing: Option<FinancingLegSpec>,
    schedule: Option<TrsScheduleSpec>,
    receive_total_return: Option<bool>,
    initial_level: Option<f64>,
}

#[wasm_bindgen(js_class = FiIndexTotalReturnSwapBuilder)]
impl JsFiIndexTotalReturnSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsFiIndexTotalReturnSwapBuilder {
        JsFiIndexTotalReturnSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsFiIndexTotalReturnSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = underlying)]
    pub fn underlying(mut self, underlying: &JsIndexUnderlying) -> JsFiIndexTotalReturnSwapBuilder {
        self.underlying = Some(underlying.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = financing)]
    pub fn financing(mut self, financing: &JsFinancingLegSpec) -> JsFiIndexTotalReturnSwapBuilder {
        self.financing = Some(financing.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = schedule)]
    pub fn schedule(mut self, schedule: &JsTrsScheduleSpec) -> JsFiIndexTotalReturnSwapBuilder {
        self.schedule = Some(schedule.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = receiveTotalReturn)]
    pub fn receive_total_return(
        mut self,
        receive_total_return: bool,
    ) -> JsFiIndexTotalReturnSwapBuilder {
        self.receive_total_return = Some(receive_total_return);
        self
    }

    #[wasm_bindgen(js_name = initialLevel)]
    pub fn initial_level(mut self, initial_level: f64) -> JsFiIndexTotalReturnSwapBuilder {
        self.initial_level = Some(initial_level);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFiIndexTotalReturnSwap, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("FiIndexTotalReturnSwapBuilder: notional (money) is required".to_string())
        })?;
        let underlying = self.underlying.ok_or_else(|| {
            js_error("FiIndexTotalReturnSwapBuilder: underlying is required".to_string())
        })?;
        let financing = self.financing.ok_or_else(|| {
            js_error("FiIndexTotalReturnSwapBuilder: financing is required".to_string())
        })?;
        let schedule = self.schedule.ok_or_else(|| {
            js_error("FiIndexTotalReturnSwapBuilder: schedule is required".to_string())
        })?;
        let receive_total_return = self.receive_total_return.ok_or_else(|| {
            js_error("FiIndexTotalReturnSwapBuilder: receiveTotalReturn is required".to_string())
        })?;

        let side = if receive_total_return {
            TrsSide::ReceiveTotalReturn
        } else {
            TrsSide::PayTotalReturn
        };

        let trs = FIIndexTotalReturnSwap {
            id: instrument_id_from_str(&self.instrument_id),
            notional,
            underlying,
            financing,
            schedule,
            side,
            initial_level: self.initial_level,
            attributes: Default::default(),
            margin_spec: None,
        };

        Ok(JsFiIndexTotalReturnSwap::from_inner(trs))
    }
}

#[wasm_bindgen(js_class = FiIndexTotalReturnSwap)]
impl JsFiIndexTotalReturnSwap {
    /// Create a fixed-income index total return swap (TRS).
    ///
    /// Conventions:
    /// - `receive_total_return = true` means you receive the index total return and pay financing.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - TRS notional (currency-tagged)
    /// @param underlying - Index underlying parameters
    /// @param financing - Financing leg specification
    /// @param schedule - Payment/reset schedule specification
    /// @param receive_total_return - Direction flag
    /// @param initial_level - Optional initial level override
    /// @returns A new `FiIndexTotalReturnSwap`
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        underlying: &JsIndexUnderlying,
        financing: &JsFinancingLegSpec,
        schedule: &JsTrsScheduleSpec,
        receive_total_return: bool,
        initial_level: Option<f64>,
    ) -> JsFiIndexTotalReturnSwap {
        web_sys::console::warn_1(&JsValue::from_str(
            "FiIndexTotalReturnSwap constructor is deprecated; use FiIndexTotalReturnSwapBuilder instead.",
        ));
        let side = if receive_total_return {
            TrsSide::ReceiveTotalReturn
        } else {
            TrsSide::PayTotalReturn
        };

        let trs = FIIndexTotalReturnSwap {
            id: instrument_id_from_str(instrument_id),
            notional: notional.inner(),
            underlying: underlying.inner.clone(),
            financing: financing.inner.clone(),
            schedule: schedule.inner.clone(),
            side,
            initial_level,
            attributes: Default::default(),
            margin_spec: None,
        };

        JsFiIndexTotalReturnSwap::from_inner(trs)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsFiIndexTotalReturnSwap, JsValue> {
        from_js_value(value).map(JsFiIndexTotalReturnSwap::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this FI index TRS.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use finstack_valuations::cashflow::CashflowProvider;

        let disc = market
            .inner()
            .get_discount(self.inner.financing.discount_curve_id.as_str())
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

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::FIIndexTotalReturnSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FiIndexTotalReturnSwap(id='{}', notional={})",
            self.inner.id, self.inner.notional
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFiIndexTotalReturnSwap {
        JsFiIndexTotalReturnSwap::from_inner(self.inner.clone())
    }
}
