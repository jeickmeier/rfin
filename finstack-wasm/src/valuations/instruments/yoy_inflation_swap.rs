//! Year-on-year inflation swap WASM bindings.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{DayCount, Tenor};
use finstack_valuations::instruments::rates::inflation_swap::{
    PayReceiveInflation, YoYInflationSwap,
};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Pay/receive direction for inflation swaps.
#[wasm_bindgen(js_name = PayReceiveInflation)]
#[derive(Clone, Copy)]
pub struct JsPayReceiveInflation {
    inner: PayReceiveInflation,
}

#[wasm_bindgen(js_class = PayReceiveInflation)]
impl JsPayReceiveInflation {
    /// Pay fixed (real) leg, receive inflation leg.
    #[wasm_bindgen(js_name = PayFixed)]
    pub fn pay_fixed() -> JsPayReceiveInflation {
        JsPayReceiveInflation {
            inner: PayReceiveInflation::PayFixed,
        }
    }

    /// Receive fixed (real) leg, pay inflation leg.
    #[wasm_bindgen(js_name = ReceiveFixed)]
    pub fn receive_fixed() -> JsPayReceiveInflation {
        JsPayReceiveInflation {
            inner: PayReceiveInflation::ReceiveFixed,
        }
    }

    /// Check if this is pay-fixed.
    #[wasm_bindgen(js_name = isPayFixed)]
    pub fn is_pay_fixed(&self) -> bool {
        matches!(self.inner, PayReceiveInflation::PayFixed)
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

impl JsPayReceiveInflation {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> PayReceiveInflation {
        self.inner
    }
}

/// Year-on-year (YoY) Inflation Swap instrument.
///
/// Pays periodic inflation rates (CPI ratios over each period) versus a fixed rate.
#[wasm_bindgen(js_name = YoYInflationSwap)]
#[derive(Clone, Debug)]
pub struct JsYoYInflationSwap {
    pub(crate) inner: YoYInflationSwap,
}

impl InstrumentWrapper for JsYoYInflationSwap {
    type Inner = YoYInflationSwap;
    fn from_inner(inner: YoYInflationSwap) -> Self {
        JsYoYInflationSwap { inner }
    }
    fn inner(&self) -> YoYInflationSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = YoYInflationSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsYoYInflationSwapBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    fixed_rate: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    inflation_index_id: Option<String>,
    frequency: Option<String>,
    side: Option<String>,
    day_count: Option<String>,
}

#[wasm_bindgen(js_class = YoYInflationSwapBuilder)]
impl JsYoYInflationSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsYoYInflationSwapBuilder {
        JsYoYInflationSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsYoYInflationSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = fixedRate)]
    pub fn fixed_rate(mut self, fixed_rate: f64) -> JsYoYInflationSwapBuilder {
        self.fixed_rate = Some(fixed_rate);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsYoYInflationSwapBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsYoYInflationSwapBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsYoYInflationSwapBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = inflationIndexId)]
    pub fn inflation_index_id(mut self, inflation_index_id: &str) -> JsYoYInflationSwapBuilder {
        self.inflation_index_id = Some(inflation_index_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = frequency)]
    pub fn frequency(mut self, frequency: String) -> JsYoYInflationSwapBuilder {
        self.frequency = Some(frequency);
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: String) -> JsYoYInflationSwapBuilder {
        self.side = Some(side);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: String) -> JsYoYInflationSwapBuilder {
        self.day_count = Some(day_count);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsYoYInflationSwap, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("YoYInflationSwapBuilder: notional (money) is required".to_string())
        })?;
        let fixed_rate = self.fixed_rate.ok_or_else(|| {
            js_error("YoYInflationSwapBuilder: fixedRate is required".to_string())
        })?;
        let start_date = self.start_date.ok_or_else(|| {
            js_error("YoYInflationSwapBuilder: startDate is required".to_string())
        })?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("YoYInflationSwapBuilder: maturity is required".to_string()))?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("YoYInflationSwapBuilder: discountCurve is required".to_string())
        })?;
        let inflation_index_id = self.inflation_index_id.as_deref().ok_or_else(|| {
            js_error("YoYInflationSwapBuilder: inflationIndexId is required".to_string())
        })?;

        let freq = parse_optional_with_default(self.frequency, Tenor::annual())?;
        let side_value = parse_optional_with_default(self.side, PayReceiveInflation::PayFixed)?;
        let dc = parse_optional_with_default(self.day_count, DayCount::ActAct)?;

        YoYInflationSwap::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .fixed_rate(fixed_rate)
            .start(start_date)
            .maturity(maturity)
            .frequency(freq)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .inflation_index_id(curve_id_from_str(inflation_index_id))
            .dc(dc)
            .side(side_value)
            .attributes(Default::default())
            .build()
            .map(JsYoYInflationSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_class = YoYInflationSwap)]
impl JsYoYInflationSwap {
    /// Create a new YoY inflation swap.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        inflation_index_id: &str,
        frequency: Option<String>,
        side: Option<String>,
        day_count: Option<String>,
    ) -> Result<JsYoYInflationSwap, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "YoYInflationSwap constructor is deprecated; use YoYInflationSwapBuilder instead.",
        ));
        let freq = parse_optional_with_default(frequency, Tenor::annual())?;
        let side_value = parse_optional_with_default(side, PayReceiveInflation::PayFixed)?;
        let dc = parse_optional_with_default(day_count, DayCount::ActAct)?;

        let builder = YoYInflationSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .fixed_rate(fixed_rate)
            .start(start_date.inner())
            .maturity(maturity.inner())
            .frequency(freq)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .inflation_index_id(curve_id_from_str(inflation_index_id))
            .dc(dc)
            .side(side_value)
            .attributes(Default::default());

        builder
            .build()
            .map(JsYoYInflationSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the notional.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    /// Get the fixed rate.
    #[wasm_bindgen(getter, js_name = fixedRate)]
    pub fn fixed_rate(&self) -> f64 {
        self.inner.fixed_rate
    }

    /// Get the start date.
    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start)
    }

    /// Get the maturity date.
    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
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
        InstrumentType::YoYInflationSwap as u16
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsYoYInflationSwap, JsValue> {
        from_js_value(value).map(|inner| JsYoYInflationSwap { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get projected cashflows for this YoY inflation swap (inflation leg + fixed leg).
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self, market: &JsMarketContext) -> Result<Array, JsValue> {
        use finstack_core::dates::{DayCountCtx, StubKind};
        use finstack_valuations::cashflow::builder::build_dates;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let sched = build_dates(
            self.inner.start,
            self.inner.maturity,
            self.inner.frequency,
            StubKind::None,
            finstack_core::dates::BusinessDayConvention::Unadjusted,
            None,
        )
        .map_err(|e| js_error(e.to_string()))?;

        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(Array::new());
        }

        let result = Array::new();
        let mut prev = dates[0];
        for &d in &dates[1..] {
            if d <= as_of {
                prev = d;
                continue;
            }

            let accrual = self
                .inner
                .dc
                .year_fraction(prev, d, DayCountCtx::default())
                .map_err(|e| js_error(e.to_string()))?;

            let mut yoy = 0.0;
            if let Some(index) = market
                .inner()
                .inflation_index(self.inner.inflation_index_id.as_str())
            {
                let s = index.value_on(prev).unwrap_or(1.0);
                let e = index.value_on(d).unwrap_or(s);
                if s > 0.0 {
                    yoy = e / s - 1.0;
                }
            }

            let notional = self.inner.notional.amount();
            let ccy = self.inner.notional.currency();

            let infl_amt = notional * yoy * accrual;
            let fixed_amt = notional * self.inner.fixed_rate * accrual;

            let (infl_sign, fixed_sign) = match self.inner.side {
                PayReceiveInflation::PayFixed => (1.0, -1.0),
                PayReceiveInflation::ReceiveFixed => (-1.0, 1.0),
            };

            for (kind, amt) in [
                ("InflationLeg", infl_sign * infl_amt),
                ("FixedLeg", fixed_sign * fixed_amt),
            ] {
                let entry = Array::new();
                entry.push(&JsDate::from_core(d).into());
                entry.push(&JsMoney::from_inner(finstack_core::money::Money::new(amt, ccy)).into());
                entry.push(&JsValue::from_str(kind));
                entry.push(&JsValue::NULL);
                result.push(&entry);
            }

            prev = d;
        }

        Ok(result)
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "YoYInflationSwap(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsYoYInflationSwap {
        JsYoYInflationSwap::from_inner(self.inner.clone())
    }
}
