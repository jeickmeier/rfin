use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::inflation_swap::{InflationSwap, PayReceiveInflation};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InflationSwap)]
#[derive(Clone, Debug)]
pub struct JsInflationSwap {
    pub(crate) inner: InflationSwap,
}

impl InstrumentWrapper for JsInflationSwap {
    type Inner = InflationSwap;
    fn from_inner(inner: InflationSwap) -> Self {
        JsInflationSwap { inner }
    }
    fn inner(&self) -> InflationSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InflationSwap)]
impl JsInflationSwap {
    /// Create a zero-coupon inflation swap.
    ///
    /// Conventions:
    /// - `fixed_rate` is a **decimal rate** (e.g. `0.025` for 2.5%).
    /// - `side` defaults to paying fixed if omitted.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Swap notional (currency-tagged)
    /// @param fixed_rate - Fixed inflation rate (decimal)
    /// @param start_date - Start date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID
    /// @param inflation_curve - Inflation index/curve ID
    /// @param side - Optional pay/receive side string
    /// @param day_count - Optional day count name (defaults to Act/Act)
    /// @returns A new `InflationSwap`
    /// @throws {Error} If inputs are invalid or parsing fails
    ///
    /// @example
    /// ```javascript
    /// import init, { InflationSwap, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const swap = new InflationSwap(
    ///   "infl_1",
    ///   Money.fromCode(10_000_000, "USD"),
    ///   0.025,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2029, 1, 2),
    ///   "USD-OIS",
    ///   "US-CPI",
    ///   "pay_fixed",
    ///   "act_act"
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        inflation_curve: &str,
        side: Option<String>,
        day_count: Option<String>,
    ) -> Result<JsInflationSwap, JsValue> {
        let side_value = parse_optional_with_default(side, PayReceiveInflation::PayFixed)?;
        let dc = parse_optional_with_default(day_count, DayCount::ActAct)?;

        let builder = InflationSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .fixed_rate(fixed_rate)
            .start(start_date.inner())
            .maturity(maturity.inner())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .inflation_index_id(inflation_curve.into())
            .dc(dc)
            .side(side_value)
            .attributes(Default::default());

        builder
            .build()
            .map(JsInflationSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInflationSwap, JsValue> {
        from_js_value(value).map(JsInflationSwap::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this zero-coupon inflation swap (fixed + inflation legs at maturity).
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use finstack_core::dates::DayCountCtx;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        // If already matured relative to curve base, return empty.
        if self.inner.maturity <= as_of {
            return Ok(Array::new());
        }

        // Very simple forward projection: use inflation index if present, otherwise assume index_ratio=1.
        let mut index_ratio = 1.0;
        if let Some(index) = market
            .inner()
            .inflation_index(self.inner.inflation_index_id.as_str())
        {
            let start_v = index.value_on(self.inner.start).unwrap_or(1.0);
            let end_v = index.value_on(self.inner.maturity).unwrap_or(start_v);
            if start_v > 0.0 {
                index_ratio = end_v / start_v;
            }
        }

        let tau = self
            .inner
            .dc
            .year_fraction(
                self.inner.start,
                self.inner.maturity,
                DayCountCtx::default(),
            )
            .map_err(|e| js_error(e.to_string()))?;

        let notional = self.inner.notional.amount();
        let ccy = self.inner.notional.currency();

        let inflation_leg = notional * (index_ratio - 1.0);
        let fixed_leg = notional * ((1.0 + self.inner.fixed_rate).powf(tau) - 1.0);

        let (infl_sign, fixed_sign) = match self.inner.side {
            PayReceiveInflation::PayFixed => (1.0, -1.0),
            PayReceiveInflation::ReceiveFixed => (-1.0, 1.0),
        };

        let result = Array::new();
        for (kind, amt) in [
            ("InflationLeg", infl_sign * inflation_leg),
            ("FixedLeg", fixed_sign * fixed_leg),
        ] {
            let entry = Array::new();
            entry.push(&JsDate::from_core(self.inner.maturity).into());
            entry.push(&JsMoney::from_inner(finstack_core::money::Money::new(amt, ccy)).into());
            entry.push(&JsValue::from_str(kind));
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

    #[wasm_bindgen(getter, js_name = fixedRate)]
    pub fn fixed_rate(&self) -> f64 {
        self.inner.fixed_rate
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::InflationSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InflationSwap(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInflationSwap {
        JsInflationSwap::from_inner(self.inner.clone())
    }
}
