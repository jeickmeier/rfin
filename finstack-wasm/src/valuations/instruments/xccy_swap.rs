//! Cross-currency swap WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::curve_id_from_str;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_valuations::instruments::rates::xccy_swap::{
    LegSide, NotionalExchange, XccySwap, XccySwapLeg,
};
use finstack_valuations::prelude::InstrumentNpvExt;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Notional exchange convention for XCCY swaps.
#[wasm_bindgen(js_name = NotionalExchange)]
#[derive(Clone, Copy)]
pub struct JsNotionalExchange {
    inner: NotionalExchange,
}

#[wasm_bindgen(js_class = NotionalExchange)]
impl JsNotionalExchange {
    /// No principal exchange.
    #[wasm_bindgen(js_name = None)]
    pub fn none() -> JsNotionalExchange {
        JsNotionalExchange {
            inner: NotionalExchange::None,
        }
    }

    /// Exchange principal at maturity only.
    #[wasm_bindgen(js_name = Final)]
    pub fn final_only() -> JsNotionalExchange {
        JsNotionalExchange {
            inner: NotionalExchange::Final,
        }
    }

    /// Exchange principal at start and maturity (typical for XCCY basis swaps).
    #[wasm_bindgen(js_name = InitialAndFinal)]
    pub fn initial_and_final() -> JsNotionalExchange {
        JsNotionalExchange {
            inner: NotionalExchange::InitialAndFinal,
        }
    }
}

impl JsNotionalExchange {
    pub(crate) fn inner(&self) -> NotionalExchange {
        self.inner
    }
}

/// Leg side (pay or receive).
#[wasm_bindgen(js_name = LegSide)]
#[derive(Clone, Copy)]
pub struct JsLegSide {
    inner: LegSide,
}

#[wasm_bindgen(js_class = LegSide)]
impl JsLegSide {
    /// Receive the leg's coupons.
    #[wasm_bindgen(js_name = Receive)]
    pub fn receive() -> JsLegSide {
        JsLegSide {
            inner: LegSide::Receive,
        }
    }

    /// Pay the leg's coupons.
    #[wasm_bindgen(js_name = Pay)]
    pub fn pay() -> JsLegSide {
        JsLegSide {
            inner: LegSide::Pay,
        }
    }
}

impl JsLegSide {
    pub(crate) fn inner(&self) -> LegSide {
        self.inner
    }
}

/// One floating leg of an XCCY swap.
#[wasm_bindgen(js_name = XccySwapLeg)]
#[derive(Clone)]
pub struct JsXccySwapLeg {
    inner: XccySwapLeg,
}

#[wasm_bindgen(js_name = XccySwapLegBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsXccySwapLegBuilder {
    currency: Option<finstack_core::currency::Currency>,
    notional: Option<finstack_core::money::Money>,
    side: Option<LegSide>,
    forward_curve_id: Option<String>,
    discount_curve_id: Option<String>,
    frequency: Option<String>,
    day_count: Option<String>,
    bdc: Option<String>,
    spread: Option<f64>,
    payment_lag_days: Option<i32>,
    calendar_id: Option<String>,
}

#[wasm_bindgen(js_class = XccySwapLegBuilder)]
impl JsXccySwapLegBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsXccySwapLegBuilder {
        JsXccySwapLegBuilder::default()
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: &JsCurrency) -> JsXccySwapLegBuilder {
        self.currency = Some(currency.inner());
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: &JsMoney) -> JsXccySwapLegBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: &JsLegSide) -> JsXccySwapLegBuilder {
        self.side = Some(side.inner());
        self
    }

    #[wasm_bindgen(js_name = forwardCurveId)]
    pub fn forward_curve_id(mut self, forward_curve_id: &str) -> JsXccySwapLegBuilder {
        self.forward_curve_id = Some(forward_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, discount_curve_id: &str) -> JsXccySwapLegBuilder {
        self.discount_curve_id = Some(discount_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = frequency)]
    pub fn frequency(mut self, frequency: String) -> JsXccySwapLegBuilder {
        self.frequency = Some(frequency);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: String) -> JsXccySwapLegBuilder {
        self.day_count = Some(day_count);
        self
    }

    #[wasm_bindgen(js_name = businessDayConvention)]
    pub fn bdc(mut self, bdc: String) -> JsXccySwapLegBuilder {
        self.bdc = Some(bdc);
        self
    }

    #[wasm_bindgen(js_name = spread)]
    pub fn spread(mut self, spread: f64) -> JsXccySwapLegBuilder {
        self.spread = Some(spread);
        self
    }

    #[wasm_bindgen(js_name = paymentLagDays)]
    pub fn payment_lag_days(mut self, payment_lag_days: i32) -> JsXccySwapLegBuilder {
        self.payment_lag_days = Some(payment_lag_days);
        self
    }

    #[wasm_bindgen(js_name = calendarId)]
    pub fn calendar_id(mut self, calendar_id: String) -> JsXccySwapLegBuilder {
        self.calendar_id = Some(calendar_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsXccySwapLeg, JsValue> {
        let currency = self
            .currency
            .ok_or_else(|| js_error("XccySwapLegBuilder: currency is required".to_string()))?;
        let notional = self
            .notional
            .ok_or_else(|| js_error("XccySwapLegBuilder: notional is required".to_string()))?;
        let side = self
            .side
            .ok_or_else(|| js_error("XccySwapLegBuilder: side is required".to_string()))?;
        let forward_curve_id = self.forward_curve_id.as_deref().ok_or_else(|| {
            js_error("XccySwapLegBuilder: forwardCurveId is required".to_string())
        })?;
        let discount_curve_id = self.discount_curve_id.as_deref().ok_or_else(|| {
            js_error("XccySwapLegBuilder: discountCurveId is required".to_string())
        })?;

        let freq = parse_optional_with_default(self.frequency, Tenor::quarterly())?;
        let dc = parse_optional_with_default(self.day_count, DayCount::ActAct)?;
        let bdc_value =
            parse_optional_with_default(self.bdc, BusinessDayConvention::ModifiedFollowing)?;

        Ok(JsXccySwapLeg {
            inner: XccySwapLeg {
                currency,
                notional,
                side,
                forward_curve_id: curve_id_from_str(forward_curve_id),
                discount_curve_id: curve_id_from_str(discount_curve_id),
                frequency: freq,
                day_count: dc,
                bdc: bdc_value,
                spread: self.spread.unwrap_or(0.0),
                payment_lag_days: self.payment_lag_days.unwrap_or(0),
                calendar_id: self.calendar_id,
                allow_calendar_fallback: true,
            },
        })
    }
}

#[wasm_bindgen(js_class = XccySwapLeg)]
impl JsXccySwapLeg {
    /// Create a new XCCY swap leg.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        currency: &JsCurrency,
        notional: &JsMoney,
        side: &JsLegSide,
        forward_curve_id: &str,
        discount_curve_id: &str,
        frequency: Option<String>,
        day_count: Option<String>,
        bdc: Option<String>,
        spread: Option<f64>,
        payment_lag_days: Option<i32>,
        calendar_id: Option<String>,
    ) -> Result<JsXccySwapLeg, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "XccySwapLeg constructor is deprecated; use XccySwapLegBuilder instead.",
        ));
        let freq = parse_optional_with_default(frequency, Tenor::quarterly())?;
        let dc = parse_optional_with_default(day_count, DayCount::ActAct)?;
        let bdc_value = parse_optional_with_default(bdc, BusinessDayConvention::ModifiedFollowing)?;

        Ok(JsXccySwapLeg {
            inner: XccySwapLeg {
                currency: currency.inner(),
                notional: notional.inner(),
                side: side.inner(),
                forward_curve_id: curve_id_from_str(forward_curve_id),
                discount_curve_id: curve_id_from_str(discount_curve_id),
                frequency: freq,
                day_count: dc,
                bdc: bdc_value,
                spread: spread.unwrap_or(0.0),
                payment_lag_days: payment_lag_days.unwrap_or(0),
                calendar_id,
                allow_calendar_fallback: true,
            },
        })
    }

    /// Get the leg currency.
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency)
    }

    /// Get the leg notional.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    /// Get the spread (in decimal form).
    #[wasm_bindgen(getter)]
    pub fn spread(&self) -> f64 {
        self.inner.spread
    }
}

impl JsXccySwapLeg {
    pub(crate) fn inner(&self) -> XccySwapLeg {
        self.inner.clone()
    }
}

/// Cross-currency floating-for-floating swap.
#[wasm_bindgen(js_name = XccySwap)]
#[derive(Clone, Debug)]
pub struct JsXccySwap {
    pub(crate) inner: XccySwap,
}

impl InstrumentWrapper for JsXccySwap {
    type Inner = XccySwap;
    fn from_inner(inner: XccySwap) -> Self {
        JsXccySwap { inner }
    }
    fn inner(&self) -> XccySwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = XccySwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsXccySwapBuilder {
    instrument_id: String,
    start_date: Option<finstack_core::dates::Date>,
    maturity_date: Option<finstack_core::dates::Date>,
    leg1: Option<XccySwapLeg>,
    leg2: Option<XccySwapLeg>,
    reporting_currency: Option<finstack_core::currency::Currency>,
    notional_exchange: Option<NotionalExchange>,
    stub_kind: Option<String>,
}

#[wasm_bindgen(js_class = XccySwapBuilder)]
impl JsXccySwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsXccySwapBuilder {
        JsXccySwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsXccySwapBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturityDate)]
    pub fn maturity_date(mut self, maturity_date: &JsDate) -> JsXccySwapBuilder {
        self.maturity_date = Some(maturity_date.inner());
        self
    }

    #[wasm_bindgen(js_name = leg1)]
    pub fn leg1(mut self, leg1: &JsXccySwapLeg) -> JsXccySwapBuilder {
        self.leg1 = Some(leg1.inner());
        self
    }

    #[wasm_bindgen(js_name = leg2)]
    pub fn leg2(mut self, leg2: &JsXccySwapLeg) -> JsXccySwapBuilder {
        self.leg2 = Some(leg2.inner());
        self
    }

    #[wasm_bindgen(js_name = reportingCurrency)]
    pub fn reporting_currency(mut self, reporting_currency: &JsCurrency) -> JsXccySwapBuilder {
        self.reporting_currency = Some(reporting_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = notionalExchange)]
    pub fn notional_exchange(mut self, notional_exchange: JsNotionalExchange) -> JsXccySwapBuilder {
        self.notional_exchange = Some(notional_exchange.inner());
        self
    }

    #[wasm_bindgen(js_name = stubKind)]
    pub fn stub_kind(mut self, stub_kind: String) -> JsXccySwapBuilder {
        self.stub_kind = Some(stub_kind);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsXccySwap, JsValue> {
        let start_date = self
            .start_date
            .ok_or_else(|| js_error("XccySwapBuilder: startDate is required".to_string()))?;
        let maturity_date = self
            .maturity_date
            .ok_or_else(|| js_error("XccySwapBuilder: maturityDate is required".to_string()))?;
        let leg1 = self
            .leg1
            .ok_or_else(|| js_error("XccySwapBuilder: leg1 is required".to_string()))?;
        let leg2 = self
            .leg2
            .ok_or_else(|| js_error("XccySwapBuilder: leg2 is required".to_string()))?;
        let reporting_currency = self.reporting_currency.ok_or_else(|| {
            js_error("XccySwapBuilder: reportingCurrency is required".to_string())
        })?;

        let stub = parse_optional_with_default(self.stub_kind, StubKind::None)?;
        let exchange = self
            .notional_exchange
            .unwrap_or(NotionalExchange::InitialAndFinal);

        let swap = XccySwap::new(
            &self.instrument_id,
            start_date,
            maturity_date,
            leg1,
            leg2,
            reporting_currency,
        )
        .with_notional_exchange(exchange)
        .with_stub(stub);

        Ok(JsXccySwap { inner: swap })
    }
}

#[wasm_bindgen(js_class = XccySwap)]
impl JsXccySwap {
    /// Create a new cross-currency swap.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        start_date: &JsDate,
        maturity_date: &JsDate,
        leg1: &JsXccySwapLeg,
        leg2: &JsXccySwapLeg,
        reporting_currency: &JsCurrency,
        notional_exchange: Option<JsNotionalExchange>,
        stub_kind: Option<String>,
    ) -> Result<JsXccySwap, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "XccySwap constructor is deprecated; use XccySwapBuilder instead.",
        ));
        let stub = parse_optional_with_default(stub_kind, StubKind::None)?;
        let exchange = notional_exchange
            .map(|e| e.inner())
            .unwrap_or(NotionalExchange::InitialAndFinal);

        let swap = XccySwap::new(
            instrument_id,
            start_date.inner(),
            maturity_date.inner(),
            leg1.inner(),
            leg2.inner(),
            reporting_currency.inner(),
        )
        .with_notional_exchange(exchange)
        .with_stub(stub);

        Ok(JsXccySwap { inner: swap })
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the start date.
    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    /// Get the maturity date.
    #[wasm_bindgen(getter, js_name = maturityDate)]
    pub fn maturity_date(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity_date)
    }

    /// Get the reporting currency.
    #[wasm_bindgen(getter, js_name = reportingCurrency)]
    pub fn reporting_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.reporting_currency)
    }

    /// Calculate the NPV.
    pub fn npv(&self, market: &JsMarketContext, as_of: &JsDate) -> Result<JsMoney, JsValue> {
        self.inner
            .npv(market.inner(), as_of.inner())
            .map(JsMoney::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the instrument type.
    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::XccySwap as u16
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsXccySwap, JsValue> {
        from_js_value(value).map(|inner| JsXccySwap { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get projected cashflows for this XCCY swap (both legs, leg currencies).
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self, market: &JsMarketContext) -> Result<Array, JsValue> {
        use finstack_core::dates::{CalendarRegistry, DateExt, DayCountCtx};
        use finstack_valuations::cashflow::builder::build_dates;

        // Use leg1 discount curve base date as the projection / filter anchor.
        let disc = market
            .inner()
            .get_discount(self.inner.leg1.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let result = Array::new();

        for (leg_label, leg) in [("Leg1", &self.inner.leg1), ("Leg2", &self.inner.leg2)] {
            // Resolve calendar (if any)
            let cal = leg
                .calendar_id
                .as_deref()
                .and_then(|id| CalendarRegistry::global().resolve_str(id));

            // Principal exchanges
            let principal_sign = match leg.side {
                LegSide::Receive => (-1.0, 1.0), // pay at start, receive at end
                LegSide::Pay => (1.0, -1.0),
            };

            if matches!(
                self.inner.notional_exchange,
                NotionalExchange::InitialAndFinal
            ) && self.inner.start_date > as_of
            {
                let entry = Array::new();
                entry.push(&JsDate::from_core(self.inner.start_date).into());
                entry.push(
                    &JsMoney::from_inner(finstack_core::money::Money::new(
                        principal_sign.0 * leg.notional.amount(),
                        leg.currency,
                    ))
                    .into(),
                );
                entry.push(&JsValue::from_str(&format!("{leg_label}:Principal")));
                entry.push(&JsValue::NULL);
                result.push(&entry);
            }

            if matches!(
                self.inner.notional_exchange,
                NotionalExchange::Final | NotionalExchange::InitialAndFinal
            ) && self.inner.maturity_date > as_of
            {
                let entry = Array::new();
                entry.push(&JsDate::from_core(self.inner.maturity_date).into());
                entry.push(
                    &JsMoney::from_inner(finstack_core::money::Money::new(
                        principal_sign.1 * leg.notional.amount(),
                        leg.currency,
                    ))
                    .into(),
                );
                entry.push(&JsValue::from_str(&format!("{leg_label}:Principal")));
                entry.push(&JsValue::NULL);
                result.push(&entry);
            }

            // Floating coupons
            let sched = build_dates(
                self.inner.start_date,
                self.inner.maturity_date,
                leg.frequency,
                self.inner.stub_kind,
                leg.bdc,
                leg.calendar_id.as_deref(),
            )
            .map_err(|e| js_error(e.to_string()))?;

            let dates = sched.dates;
            if dates.len() < 2 {
                continue;
            }

            let fwd = market
                .inner()
                .get_forward(leg.forward_curve_id.as_str())
                .map_err(|e| js_error(e.to_string()))?;
            let fwd_dc = fwd.day_count();
            let fwd_base = fwd.base_date();

            let coupon_sign = match leg.side {
                LegSide::Receive => 1.0,
                LegSide::Pay => -1.0,
            };

            for i in 1..dates.len() {
                let period_start = dates[i - 1];
                let period_end = dates[i];

                let payment_date = if leg.payment_lag_days == 0 {
                    period_end
                } else if let Some(cal) = cal {
                    period_end
                        .add_business_days(leg.payment_lag_days, cal)
                        .map_err(|e| js_error(e.to_string()))?
                } else {
                    period_end + time::Duration::days(leg.payment_lag_days as i64)
                };

                if payment_date <= as_of {
                    continue;
                }

                let t_start = fwd_dc
                    .year_fraction(fwd_base, period_start, DayCountCtx::default())
                    .map_err(|e| js_error(e.to_string()))?;
                let t_end = fwd_dc
                    .year_fraction(fwd_base, period_end, DayCountCtx::default())
                    .map_err(|e| js_error(e.to_string()))?;
                let forward_rate = fwd.rate_period(t_start, t_end);

                let accrual = leg
                    .day_count
                    .year_fraction(period_start, period_end, DayCountCtx::default())
                    .map_err(|e| js_error(e.to_string()))?;

                let amount =
                    coupon_sign * leg.notional.amount() * (forward_rate + leg.spread) * accrual;

                let entry = Array::new();
                entry.push(&JsDate::from_core(payment_date).into());
                entry.push(
                    &JsMoney::from_inner(finstack_core::money::Money::new(amount, leg.currency))
                        .into(),
                );
                entry.push(&JsValue::from_str(&format!("{leg_label}:Coupon")));
                entry.push(&JsValue::NULL);
                result.push(&entry);
            }
        }

        Ok(result)
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "XccySwap(id='{}', leg1={}, leg2={})",
            self.inner.id, self.inner.leg1.currency, self.inner.leg2.currency
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsXccySwap {
        JsXccySwap::from_inner(self.inner.clone())
    }
}
