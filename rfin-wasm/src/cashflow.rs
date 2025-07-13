use wasm_bindgen::prelude::*;

use std::sync::Arc;

use rfin_core::cashflow::leg::CashFlowLeg;
use rfin_core::cashflow::notional::Notional;
use rfin_core::cashflow::npv::{DiscountCurve, Discountable};
use rfin_core::dates::DayCount as CoreDayCount;
use rfin_core::dates::ScheduleBuilder;

use crate::currency::Currency;
use crate::dates::{Date, DayCount};
use crate::schedule::Frequency;

/// Simple flat discount curve (df = 1.0) placeholder.
struct FlatCurve;
impl DiscountCurve for FlatCurve {
    fn df(&self, _date: rfin_core::dates::Date) -> f64 {
        1.0
    }
}

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
        let curve = FlatCurve;
        self.inner.npv(&curve).amount()
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
