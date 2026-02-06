use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::{JsDayCount, JsTenor};
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::irs::InterestRateSwap;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsInterestRateSwapBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    fixed_rate: Option<f64>,
    start: Option<finstack_core::dates::Date>,
    end: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    forward_curve: Option<String>,
    side: Option<String>,
    fixed_frequency: Option<finstack_core::dates::Tenor>,
    fixed_day_count: Option<finstack_core::dates::DayCount>,
    float_frequency: Option<finstack_core::dates::Tenor>,
    float_day_count: Option<finstack_core::dates::DayCount>,
    business_day_convention: Option<finstack_core::dates::BusinessDayConvention>,
    calendar_id: Option<String>,
    stub_kind: Option<finstack_core::dates::StubKind>,
    reset_lag_days: Option<i32>,
    float_spread_bp: Option<f64>,
}

#[wasm_bindgen(js_class = InterestRateSwapBuilder)]
impl JsInterestRateSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsInterestRateSwapBuilder {
        JsInterestRateSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsInterestRateSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = fixedRate)]
    pub fn fixed_rate(mut self, fixed_rate: f64) -> JsInterestRateSwapBuilder {
        self.fixed_rate = Some(fixed_rate);
        self
    }

    #[wasm_bindgen(js_name = start)]
    pub fn start(mut self, start: &JsDate) -> JsInterestRateSwapBuilder {
        self.start = Some(start.inner());
        self
    }

    #[wasm_bindgen(js_name = end)]
    pub fn end(mut self, end: &JsDate) -> JsInterestRateSwapBuilder {
        self.end = Some(end.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsInterestRateSwapBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = forwardCurve)]
    pub fn forward_curve(mut self, forward_curve: &str) -> JsInterestRateSwapBuilder {
        self.forward_curve = Some(forward_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: &str) -> JsInterestRateSwapBuilder {
        self.side = Some(side.to_string());
        self
    }

    #[wasm_bindgen(js_name = fixedFrequency)]
    pub fn fixed_frequency(mut self, fixed_frequency: JsTenor) -> JsInterestRateSwapBuilder {
        self.fixed_frequency = Some(fixed_frequency.inner());
        self
    }

    #[wasm_bindgen(js_name = fixedDayCount)]
    pub fn fixed_day_count(mut self, fixed_day_count: JsDayCount) -> JsInterestRateSwapBuilder {
        self.fixed_day_count = Some(fixed_day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = floatFrequency)]
    pub fn float_frequency(mut self, float_frequency: JsTenor) -> JsInterestRateSwapBuilder {
        self.float_frequency = Some(float_frequency.inner());
        self
    }

    #[wasm_bindgen(js_name = floatDayCount)]
    pub fn float_day_count(mut self, float_day_count: JsDayCount) -> JsInterestRateSwapBuilder {
        self.float_day_count = Some(float_day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = businessDayConvention)]
    pub fn business_day_convention(
        mut self,
        business_day_convention: JsBusinessDayConvention,
    ) -> JsInterestRateSwapBuilder {
        self.business_day_convention = Some(business_day_convention.into());
        self
    }

    #[wasm_bindgen(js_name = calendarId)]
    pub fn calendar_id(mut self, calendar_id: String) -> JsInterestRateSwapBuilder {
        self.calendar_id = Some(calendar_id);
        self
    }

    #[wasm_bindgen(js_name = stubKind)]
    pub fn stub_kind(
        mut self,
        stub_kind: crate::core::dates::schedule::JsStubKind,
    ) -> JsInterestRateSwapBuilder {
        self.stub_kind = Some(stub_kind.inner());
        self
    }

    #[wasm_bindgen(js_name = resetLagDays)]
    pub fn reset_lag_days(mut self, reset_lag_days: i32) -> JsInterestRateSwapBuilder {
        self.reset_lag_days = Some(reset_lag_days);
        self
    }

    #[wasm_bindgen(js_name = floatSpreadBp)]
    pub fn float_spread_bp(mut self, float_spread_bp: f64) -> JsInterestRateSwapBuilder {
        self.float_spread_bp = Some(float_spread_bp);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsInterestRateSwap, JsValue> {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind};
        use finstack_valuations::instruments::{FixedLegSpec, FloatLegSpec, PayReceive};

        let notional = self.notional.ok_or_else(|| {
            js_error("InterestRateSwapBuilder: notional (money) is required".to_string())
        })?;
        let fixed_rate = self.fixed_rate.ok_or_else(|| {
            js_error("InterestRateSwapBuilder: fixedRate is required".to_string())
        })?;
        let start = self
            .start
            .ok_or_else(|| js_error("InterestRateSwapBuilder: start is required".to_string()))?;
        let end = self
            .end
            .ok_or_else(|| js_error("InterestRateSwapBuilder: end is required".to_string()))?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("InterestRateSwapBuilder: discountCurve is required".to_string())
        })?;
        let forward_curve = self.forward_curve.as_deref().ok_or_else(|| {
            js_error("InterestRateSwapBuilder: forwardCurve is required".to_string())
        })?;
        let side_str = self
            .side
            .as_deref()
            .ok_or_else(|| js_error("InterestRateSwapBuilder: side is required".to_string()))?;

        let side_parsed: PayReceive = side_str.parse().map_err(js_error)?;

        let bdc = self
            .business_day_convention
            .unwrap_or(BusinessDayConvention::ModifiedFollowing);
        let fixed_freq = self
            .fixed_frequency
            .unwrap_or_else(finstack_core::dates::Tenor::semi_annual);
        let float_freq = self
            .float_frequency
            .unwrap_or_else(finstack_core::dates::Tenor::quarterly);
        let fixed_dc = self.fixed_day_count.unwrap_or(DayCount::Thirty360);
        let float_dc = self.float_day_count.unwrap_or(DayCount::Act360);
        let stub = self.stub_kind.unwrap_or(StubKind::None);
        let reset_lag_days = self.reset_lag_days.unwrap_or(2);
        let spread_bp = rust_decimal::Decimal::from_f64_retain(self.float_spread_bp.unwrap_or(0.0))
            .unwrap_or_default();

        let fixed = FixedLegSpec {
            discount_curve_id: curve_id_from_str(discount_curve),
            rate: rust_decimal::Decimal::from_f64_retain(fixed_rate).unwrap_or_default(),
            freq: fixed_freq,
            dc: fixed_dc,
            bdc,
            calendar_id: self.calendar_id.clone(),
            stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        };
        let float = FloatLegSpec {
            discount_curve_id: curve_id_from_str(discount_curve),
            forward_curve_id: curve_id_from_str(forward_curve),
            spread_bp,
            freq: float_freq,
            dc: float_dc,
            bdc,
            calendar_id: self.calendar_id.clone(),
            fixing_calendar_id: self.calendar_id,
            stub,
            reset_lag_days,
            start,
            end,
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
        };

        let swap = InterestRateSwap::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .side(side_parsed)
            .fixed(fixed)
            .float(float)
            .build()
            .map_err(|e| js_error(e.to_string()))?;

        Ok(JsInterestRateSwap::from_inner(swap))
    }
}

#[wasm_bindgen(js_name = InterestRateSwap)]
#[derive(Clone, Debug)]
pub struct JsInterestRateSwap {
    pub(crate) inner: InterestRateSwap,
}

impl InstrumentWrapper for JsInterestRateSwap {
    type Inner = InterestRateSwap;
    fn from_inner(inner: InterestRateSwap) -> Self {
        JsInterestRateSwap { inner }
    }
    fn inner(&self) -> InterestRateSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateSwap)]
impl JsInterestRateSwap {
    /// Create a plain-vanilla fixed-for-floating interest rate swap.
    ///
    /// Conventions:
    /// - `fixed_rate` is a **decimal rate** (e.g. `0.0325` for 3.25%).
    /// - `side` controls pay/receive direction (parsed by the Rust `PayReceive` enum).
    ///   Common values: `"pay_fixed"` or `"receive_fixed"`.
    /// - Default fixed leg: semi-annual, 30/360.
    /// - Default float leg: quarterly, Act/360, reset lag 2.
    /// - Business day convention defaults to Modified Following.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Swap notional (currency-tagged)
    /// @param fixed_rate - Fixed leg rate (decimal)
    /// @param start - Swap start date
    /// @param end - Swap end/maturity date
    /// @param discount_curve - Discount curve ID
    /// @param forward_curve - Forward curve ID (e.g. `"USD-SOFR-3M"`)
    /// @param side - Pay/receive direction (`"pay_fixed"` | `"receive_fixed"`)
    /// @param fixed_frequency - Optional fixed leg frequency (Tenor)
    /// @param fixed_day_count - Optional fixed leg day count
    /// @param float_frequency - Optional float leg frequency (Tenor)
    /// @param float_day_count - Optional float leg day count
    /// @param business_day_convention - Optional business day convention
    /// @param calendar_id - Optional calendar code (e.g. `"usny"`)
    /// @param stub_kind - Optional stub rule
    /// @param reset_lag_days - Optional reset lag (default 2)
    /// @returns A new `InterestRateSwap`
    /// @throws {Error} If `side` is invalid or inputs are inconsistent
    ///
    /// @example
    /// ```javascript
    /// import init, { InterestRateSwap, Money, FsDate, DayCount } from "finstack-wasm";
    ///
    /// await init();
    /// const swap = new InterestRateSwap(
    ///   "swap_1",
    ///   Money.fromCode(10_000_000, "USD"),
    ///   0.0325,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2029, 1, 2),
    ///   "USD-OIS",
    ///   "USD-SOFR-3M",
    ///   "pay_fixed",
    ///   null,
    ///   DayCount.thirty360(),
    ///   null,
    ///   DayCount.act360(),
    ///   null,
    ///   "usny",
    ///   null,
    ///   2
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start: &JsDate,
        end: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        side: &str,
        fixed_frequency: Option<JsTenor>,
        fixed_day_count: Option<JsDayCount>,
        float_frequency: Option<JsTenor>,
        float_day_count: Option<JsDayCount>,
        business_day_convention: Option<JsBusinessDayConvention>,
        calendar_id: Option<String>,
        stub_kind: Option<crate::core::dates::schedule::JsStubKind>,
        reset_lag_days: Option<i32>,
    ) -> Result<JsInterestRateSwap, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "InterestRateSwap constructor is deprecated; use InterestRateSwapBuilder instead.",
        ));
        use finstack_core::dates::BusinessDayConvention;
        use finstack_core::dates::StubKind;
        use finstack_valuations::instruments::{FixedLegSpec, FloatLegSpec, PayReceive};

        let side_parsed: PayReceive = side.parse().map_err(js_error)?;
        let bdc = business_day_convention
            .map(Into::<BusinessDayConvention>::into)
            .unwrap_or(BusinessDayConvention::ModifiedFollowing);
        let fixed_freq = fixed_frequency
            .map(|f| f.inner())
            .unwrap_or(finstack_core::dates::Tenor::semi_annual());
        let float_freq = float_frequency
            .map(|f| f.inner())
            .unwrap_or(finstack_core::dates::Tenor::quarterly());
        let fixed_dc = fixed_day_count
            .map(|d| d.inner())
            .unwrap_or(finstack_core::dates::DayCount::Thirty360);
        let float_dc = float_day_count
            .map(|d| d.inner())
            .unwrap_or(finstack_core::dates::DayCount::Act360);
        let stub = stub_kind.map(|s| s.inner()).unwrap_or(StubKind::None);
        let fixed = FixedLegSpec {
            discount_curve_id: curve_id_from_str(discount_curve),
            rate: rust_decimal::Decimal::from_f64_retain(fixed_rate).unwrap_or_default(),
            freq: fixed_freq,
            dc: fixed_dc,
            bdc,
            calendar_id: calendar_id.clone(),
            stub,
            start: start.inner(),
            end: end.inner(),
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            end_of_month: false,
        };
        let float = FloatLegSpec {
            discount_curve_id: curve_id_from_str(discount_curve),
            forward_curve_id: curve_id_from_str(forward_curve),
            spread_bp: rust_decimal::Decimal::ZERO,
            freq: float_freq,
            dc: float_dc,
            bdc,
            calendar_id: calendar_id.clone(),
            fixing_calendar_id: calendar_id,
            stub,
            reset_lag_days: reset_lag_days.unwrap_or(2),
            start: start.inner(),
            end: end.inner(),
            compounding: Default::default(),
            payment_delay_days: 0,
            end_of_month: false,
        };
        let swap = InterestRateSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .side(side_parsed)
            .fixed(fixed)
            .float(float)
            .build()
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsInterestRateSwap::from_inner(swap))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInterestRateSwap, JsValue> {
        from_js_value(value).map(JsInterestRateSwap::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this swap (both legs).
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
            .get_discount(self.inner.fixed.discount_curve_id.as_str())
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
        use rust_decimal::prelude::ToPrimitive;
        self.inner.fixed.rate.to_f64().unwrap_or(0.0)
    }

    #[wasm_bindgen(getter, js_name = floatSpreadBp)]
    pub fn float_spread_bp(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.float.spread_bp.to_f64().unwrap_or(0.0)
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
        self.inner.fixed.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.float.forward_curve_id.as_str().to_string()
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
