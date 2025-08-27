use wasm_bindgen::prelude::*;

use std::sync::Arc;

use finstack_core::dates::DayCount as CoreDayCount;
use finstack_core::dates::ScheduleBuilder;
use finstack_valuations::cashflow::leg::CashFlowLeg;
use finstack_valuations::cashflow::notional::Notional;
use finstack_valuations::pricing::discountable::Discountable;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve as CoreDiscCurve;
use finstack_core::market_data::traits::Discount as _;

use crate::currency::Currency;
use crate::dates::{Date, DayCount};
use crate::schedule::Frequency;


/// Fixed-rate cash-flow leg exposed to JavaScript.
#[wasm_bindgen]
#[derive(Clone)]
pub struct FixedRateLeg {
    inner: Arc<CashFlowLeg>,
}

#[wasm_bindgen]
impl FixedRateLeg {
    /// Construct a new fixed-rate leg.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        notional_amount: f64,
        currency: &Currency,
        rate: f64,
        start: &Date,
        end: &Date,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Result<FixedRateLeg, JsValue> {
        let core_currency = currency.inner();
        let notional = Notional::par(notional_amount, core_currency);

        let sched = ScheduleBuilder::new(start.inner(), end.inner())
            .frequency(frequency.into())
            .build_raw();

        let dc: CoreDayCount = day_count.into();

        let leg = CashFlowLeg::fixed_rate(notional, rate, sched, dc)
            .map_err(|e| JsValue::from_str(&format!("Failed to build leg: {:?}", e)))?;

        Ok(FixedRateLeg {
            inner: Arc::new(leg),
        })
    }

    /// Present value (flat discount 1.0).
    #[wasm_bindgen(js_name = "npv")]
    pub fn npv_js(&self) -> f64 {
        let base = self.inner.flows.first().map(|cf| cf.date).unwrap_or_else(|| finstack_core::dates::Date::from_ordinal_date(1970, 1).unwrap());
        let curve = CoreDiscCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (30.0, 1.0)])
            .linear_df()
            .build()
            .unwrap();
        self.inner
            .npv(&curve, curve.base_date(), self.inner.day_count)
            .map(|m| m.amount())
            .unwrap_or(0.0)
    }

    /// Accrued interest up to (but excluding) `val_date`.
    #[wasm_bindgen(js_name = "accrued")]
    pub fn accrued_js(&self, val_date: &Date) -> f64 {
        self.inner.accrued(val_date.inner()).amount()
    }

    /// Number of underlying cash-flows.
    #[wasm_bindgen(getter, js_name = "numFlows")]
    pub fn num_flows(&self) -> usize {
        self.inner.flows.len()
    }
}
