use wasm_bindgen::prelude::*;

use std::sync::Arc;

use finstack_core::dates::DayCount as CoreDayCount;
use finstack_valuations::cashflow::builder::{CashFlowSchedule, FixedCouponSpec, CouponType, cf};
use finstack_core::dates::{BusinessDayConvention, StubKind};
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
    inner: Arc<CashFlowSchedule>,
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
        let dc: CoreDayCount = day_count.into();

        let fixed_spec = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate,
            freq: frequency.into(),
            dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let leg = cf()
            .principal_amount(notional_amount, core_currency, start.inner(), end.inner())
            .fixed_cf(fixed_spec)
            .build()
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
        // Implement accrued calculation for CashFlowSchedule
        let val_date_inner = val_date.inner();
        
        // No accrual before first period
        if val_date_inner <= self.inner.flows.first().map(|cf| cf.date).unwrap_or(val_date_inner) {
            return 0.0;
        }

        // Find index of first flow after valuation date
        let idx = match self.inner.flows.iter().position(|cf| cf.date > val_date_inner) {
            Some(i) => i,
            None => return 0.0, // past last payment
        };

        if idx == 0 {
            return 0.0;
        }

        let prev_date = self.inner.flows[idx - 1].date;
        let curr_flow = &self.inner.flows[idx];

        // Only calculate accrued for coupon flows
        use finstack_valuations::cashflow::primitives::CFKind;
        if !matches!(curr_flow.kind, CFKind::Fixed | CFKind::Stub) {
            return 0.0;
        }

        // Derive coupon rate from stored amount and accrual factor
        let coupon_rate = curr_flow.amount.amount() / (self.inner.notional.initial.amount() * curr_flow.accrual_factor);

        let elapsed_yf = self.inner.day_count
            .year_fraction(prev_date, val_date_inner)
            .unwrap_or(0.0);

        (self.inner.notional.initial * (coupon_rate * elapsed_yf)).amount()
    }

    /// Number of underlying cash-flows.
    #[wasm_bindgen(getter, js_name = "numFlows")]
    pub fn num_flows(&self) -> usize {
        self.inner.flows.len()
    }
}
