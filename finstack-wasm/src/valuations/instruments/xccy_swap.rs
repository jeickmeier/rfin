//! Cross-currency swap WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::curve_id_from_str;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_valuations::instruments::xccy_swap::{LegSide, NotionalExchange, XccySwap, XccySwapLeg};
use finstack_valuations::pricer::InstrumentType;
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
        let freq = parse_optional_with_default(frequency, Tenor::quarterly())?;
        let dc = parse_optional_with_default(day_count, DayCount::ActAct)?;
        let bdc_value =
            parse_optional_with_default(bdc, BusinessDayConvention::ModifiedFollowing)?;

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
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsXccySwap, JsValue> {
        from_js_value(value).map(|inner| JsXccySwap { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
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
