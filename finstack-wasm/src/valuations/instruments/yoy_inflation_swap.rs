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
use finstack_valuations::instruments::inflation_swap::{PayReceiveInflation, YoYInflationSwap};
use finstack_valuations::pricer::InstrumentType;
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
            .npv(&market.inner(), as_of.inner())
            .map(JsMoney::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the instrument type.
    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::YoYInflationSwap as u16
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsYoYInflationSwap, JsValue> {
        from_js_value(value).map(|inner| JsYoYInflationSwap { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
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
