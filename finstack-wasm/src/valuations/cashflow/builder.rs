//! WASM bindings for the composable cashflow builder.
//!
//! Provides a builder pattern for constructing complex cashflow schedules with:
//! - Fixed and floating coupons
//! - Cash/PIK/split payment types
//! - Amortization schedules
//! - Step-up coupon programs
//! - Payment split programs (cash-to-PIK transitions)

use super::JsAmortizationSpec;
use crate::core::cashflow::primitives::JsCashFlow;
use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::{JsDayCount, JsTenor};
use crate::core::dates::schedule::JsStubKind;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::curve_id_from_str;
use finstack_valuations::cashflow::builder::specs::{
    CouponType as CoreCouponType, FixedCouponSpec as CoreFixedCouponSpec,
    FloatCouponParams as CoreFloatCouponParams, FloatingCouponSpec as CoreFloatingCouponSpec,
    ScheduleParams as CoreScheduleParams,
};
use finstack_valuations::cashflow::builder::{
    CashFlowBuilder as CoreCashFlowBuilder, CashFlowSchedule as CoreCashFlowSchedule,
};
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Coupon payment type: cash, PIK (payment-in-kind), or split between cash and PIK.
#[wasm_bindgen(js_name = CouponType)]
#[derive(Clone, Copy, Debug)]
pub struct JsCouponType {
    inner: CoreCouponType,
}

impl JsCouponType {
    pub(crate) fn inner(&self) -> CoreCouponType {
        self.inner
    }
}

#[wasm_bindgen(js_class = CouponType)]
impl JsCouponType {
    /// Create a cash coupon type (100% paid in cash).
    #[wasm_bindgen(js_name = Cash)]
    pub fn cash() -> JsCouponType {
        JsCouponType {
            inner: CoreCouponType::Cash,
        }
    }

    /// Create a PIK coupon type (100% capitalized into principal).
    #[wasm_bindgen(js_name = PIK)]
    pub fn pik() -> JsCouponType {
        JsCouponType {
            inner: CoreCouponType::PIK,
        }
    }

    /// Create a split coupon type with specified cash and PIK percentages.
    ///
    /// # Arguments
    /// * `cashPct` - Percentage paid in cash (0.0 to 1.0)
    /// * `pikPct` - Percentage capitalized into principal (0.0 to 1.0)
    ///
    /// Note: cash_pct + pik_pct should sum to ~1.0
    #[wasm_bindgen(js_name = split)]
    pub fn split(cash_pct: f64, pik_pct: f64) -> JsCouponType {
        JsCouponType {
            inner: CoreCouponType::Split { cash_pct, pik_pct },
        }
    }
}

/// Schedule parameters bundle for coupon generation.
#[wasm_bindgen(js_name = ScheduleParams)]
#[derive(Clone, Debug)]
pub struct JsScheduleParams {
    inner: CoreScheduleParams,
}

impl JsScheduleParams {
    pub(crate) fn inner(&self) -> CoreScheduleParams {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = ScheduleParams)]
impl JsScheduleParams {
    /// Create schedule parameters with explicit settings.
    #[wasm_bindgen(constructor)]
    pub fn new(
        frequency: &JsTenor,
        day_count: &JsDayCount,
        business_day_convention: JsBusinessDayConvention,
        calendar_id: Option<String>,
        stub_kind: Option<JsStubKind>,
    ) -> JsScheduleParams {
        JsScheduleParams {
            inner: CoreScheduleParams {
                freq: frequency.inner(),
                dc: day_count.inner(),
                bdc: business_day_convention.into(),
                calendar_id,
                stub: stub_kind
                    .map(|s| s.inner())
                    .unwrap_or(finstack_core::dates::StubKind::None),
            },
        }
    }

    /// Quarterly payments with Act/360 day count and Following BDC.
    #[wasm_bindgen(js_name = quarterlyAct360)]
    pub fn quarterly_act360() -> JsScheduleParams {
        JsScheduleParams {
            inner: CoreScheduleParams::quarterly_act360(),
        }
    }

    /// Semi-annual payments with 30/360 day count and Modified Following BDC.
    #[wasm_bindgen(js_name = semiannual30360)]
    pub fn semiannual_30360() -> JsScheduleParams {
        JsScheduleParams {
            inner: CoreScheduleParams::semiannual_30360(),
        }
    }

    /// Annual payments with Act/Act day count and Following BDC.
    #[wasm_bindgen(js_name = annualActAct)]
    pub fn annual_actact() -> JsScheduleParams {
        JsScheduleParams {
            inner: CoreScheduleParams::annual_actact(),
        }
    }
}

/// Fixed coupon specification combining rate, schedule, and payment type.
#[wasm_bindgen(js_name = FixedCouponSpec)]
#[derive(Clone, Debug)]
pub struct JsFixedCouponSpec {
    inner: CoreFixedCouponSpec,
}

#[wasm_bindgen(js_class = FixedCouponSpec)]
impl JsFixedCouponSpec {
    /// Create a fixed coupon specification.
    ///
    /// # Arguments
    /// * `rate` - Annual coupon rate in decimal form (e.g., 0.05 for 5%)
    /// * `schedule` - Schedule parameters (frequency, day count, etc.)
    /// * `couponType` - Payment type (cash, PIK, or split)
    #[wasm_bindgen(constructor)]
    pub fn new(
        rate: f64,
        schedule: &JsScheduleParams,
        coupon_type: &JsCouponType,
    ) -> JsFixedCouponSpec {
        let sched = schedule.inner();
        JsFixedCouponSpec {
            inner: CoreFixedCouponSpec {
                coupon_type: coupon_type.inner(),
                rate,
                freq: sched.freq,
                dc: sched.dc,
                bdc: sched.bdc,
                calendar_id: sched.calendar_id,
                stub: sched.stub,
            },
        }
    }
}

/// Floating coupon parameters (index, margin, gearing, reset lag).
#[wasm_bindgen(js_name = FloatCouponParams)]
#[derive(Clone, Debug)]
pub struct JsFloatCouponParams {
    inner: CoreFloatCouponParams,
}

#[wasm_bindgen(js_class = FloatCouponParams)]
impl JsFloatCouponParams {
    /// Create floating coupon parameters.
    ///
    /// # Arguments
    /// * `indexId` - Forward curve identifier (e.g., "USD-SOFR-3M")
    /// * `marginBp` - Margin in basis points (e.g., 150.0 for +150 bps)
    /// * `gearing` - Optional gearing multiplier (default: 1.0)
    /// * `resetLagDays` - Optional reset lag in days (default: 2)
    #[wasm_bindgen(constructor)]
    pub fn new(
        index_id: &str,
        margin_bp: f64,
        gearing: Option<f64>,
        reset_lag_days: Option<i32>,
    ) -> JsFloatCouponParams {
        JsFloatCouponParams {
            inner: CoreFloatCouponParams {
                index_id: curve_id_from_str(index_id),
                margin_bp,
                gearing: gearing.unwrap_or(1.0),
                reset_lag_days: reset_lag_days.unwrap_or(2),
            },
        }
    }
}

/// Floating coupon specification combining parameters, schedule, and payment type.
#[wasm_bindgen(js_name = FloatingCouponSpec)]
#[derive(Clone, Debug)]
pub struct JsFloatingCouponSpec {
    inner: CoreFloatingCouponSpec,
}

#[wasm_bindgen(js_class = FloatingCouponSpec)]
impl JsFloatingCouponSpec {
    /// Create a floating coupon specification.
    ///
    /// # Arguments
    /// * `params` - Floating coupon parameters (index, margin, gearing, reset lag)
    /// * `schedule` - Schedule parameters (frequency, day count, etc.)
    /// * `couponType` - Payment type (cash, PIK, or split)
    #[wasm_bindgen(constructor)]
    pub fn new(
        params: &JsFloatCouponParams,
        schedule: &JsScheduleParams,
        coupon_type: &JsCouponType,
    ) -> JsFloatingCouponSpec {
        let sched = schedule.inner();
        let calendar_id = sched.calendar_id.clone();
        JsFloatingCouponSpec {
            inner: CoreFloatingCouponSpec {
                rate_spec: finstack_valuations::cashflow::builder::FloatingRateSpec {
                    index_id: params.inner.index_id.clone(),
                    spread_bp: params.inner.margin_bp,
                    gearing: params.inner.gearing,
                    gearing_includes_spread: true,
                    floor_bp: None,
                    all_in_floor_bp: None,
                    cap_bp: None,
                    index_cap_bp: None,
                    reset_freq: sched.freq,
                    reset_lag_days: params.inner.reset_lag_days,
                    dc: sched.dc,
                    bdc: sched.bdc,
                    calendar_id: calendar_id.clone(),
                    fixing_calendar_id: calendar_id,
                },
                coupon_type: coupon_type.inner(),
                freq: sched.freq,
                stub: sched.stub,
            },
        }
    }
}

/// Composable cashflow builder for creating complex coupon structures.
///
/// Supports:
/// - Fixed and floating coupons
/// - Cash/PIK/split payment types
/// - Amortization (linear, step schedules)
/// - Step-up coupon programs
/// - Payment split programs (cash-to-PIK transitions)
#[wasm_bindgen(js_name = CashflowBuilder)]
pub struct JsCashflowBuilder {
    inner: CoreCashFlowBuilder,
}

impl Default for JsCashflowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = CashflowBuilder)]
impl JsCashflowBuilder {
    /// Create a new cashflow builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsCashflowBuilder {
        JsCashflowBuilder {
            inner: CoreCashFlowBuilder::new(),
        }
    }

    /// Set principal details and instrument horizon.
    ///
    /// # Arguments
    /// * `notional` - Initial notional amount
    /// * `issue` - Issue date
    /// * `maturity` - Maturity date
    #[wasm_bindgen]
    pub fn principal(
        mut self,
        notional: &JsMoney,
        issue: &JsDate,
        maturity: &JsDate,
    ) -> JsCashflowBuilder {
        let _ = self
            .inner
            .principal(notional.inner(), issue.inner(), maturity.inner());
        self
    }

    /// Configure amortization on the current notional.
    ///
    /// # Arguments
    /// * `spec` - Amortization specification (linear, step, etc.)
    #[wasm_bindgen]
    pub fn amortization(mut self, spec: &JsAmortizationSpec) -> JsCashflowBuilder {
        let _ = self.inner.amortization(spec.inner());
        self
    }

    /// Add a fixed coupon specification.
    ///
    /// # Arguments
    /// * `spec` - Fixed coupon specification (rate, schedule, payment type)
    #[wasm_bindgen(js_name = fixedCf)]
    pub fn fixed_cf(mut self, spec: &JsFixedCouponSpec) -> JsCashflowBuilder {
        let _ = self.inner.fixed_cf(spec.inner.clone());
        self
    }

    /// Add a floating coupon specification.
    ///
    /// # Arguments
    /// * `spec` - Floating coupon specification (index, margin, schedule, payment type)
    #[wasm_bindgen(js_name = floatingCf)]
    pub fn floating_cf(mut self, spec: &JsFloatingCouponSpec) -> JsCashflowBuilder {
        let _ = self.inner.floating_cf(spec.inner.clone());
        self
    }

    /// Add a fixed step-up coupon program.
    ///
    /// # Arguments
    /// * `steps` - Array of [dateString, rate] pairs defining rate changes
    /// * `schedule` - Schedule parameters for all periods
    /// * `defaultSplit` - Default payment split type (cash/PIK)
    #[wasm_bindgen(js_name = fixedStepup)]
    pub fn fixed_stepup(
        mut self,
        steps: &Array,
        schedule: &JsScheduleParams,
        default_split: &JsCouponType,
    ) -> Result<JsCashflowBuilder, JsValue> {
        let mut parsed_steps = Vec::with_capacity(steps.length() as usize);

        for item in steps.iter() {
            if !Array::is_array(&item) {
                return Err(js_error("Each step must be an array [dateString, rate]"));
            }
            let pair = Array::from(&item);
            if pair.length() != 2 {
                return Err(js_error("Each step must be [dateString, rate]"));
            }

            let date_str = pair
                .get(0)
                .as_string()
                .ok_or_else(|| js_error("Step date must be a string in YYYY-MM-DD format"))?;
            let rate = pair
                .get(1)
                .as_f64()
                .ok_or_else(|| js_error("Step rate must be a number"))?;

            let date = parse_iso_date(&date_str)?;
            parsed_steps.push((date, rate));
        }

        let _ = self
            .inner
            .fixed_stepup(&parsed_steps, schedule.inner(), default_split.inner());
        Ok(self)
    }

    /// Add a payment split program defining cash/PIK transitions.
    ///
    /// # Arguments
    /// * `steps` - Array of [dateString, CouponType] pairs defining payment type changes
    #[wasm_bindgen(js_name = paymentSplitProgram)]
    pub fn payment_split_program(mut self, steps: &Array) -> Result<JsCashflowBuilder, JsValue> {
        let mut parsed_steps = Vec::with_capacity(steps.length() as usize);

        for item in steps.iter() {
            if !Array::is_array(&item) {
                return Err(js_error(
                    "Each step must be an array [dateString, CouponType]",
                ));
            }
            let pair = Array::from(&item);
            if pair.length() != 2 {
                return Err(js_error("Each step must be [dateString, CouponType]"));
            }

            let date_str = pair
                .get(0)
                .as_string()
                .ok_or_else(|| js_error("Step date must be a string in YYYY-MM-DD format"))?;

            // Extract CouponType from the second element
            // Since JsCouponType is a struct, we need to check string representation
            // For now, we'll expect users to pass the CouponType object directly
            // We'll document this pattern in examples
            let coupon_type_str = pair.get(1).as_string().ok_or_else(|| {
                js_error("Second element must be 'cash', 'pik', or 'split:X:Y' string")
            })?;

            let coupon_type = if coupon_type_str.eq_ignore_ascii_case("cash") {
                CoreCouponType::Cash
            } else if coupon_type_str.eq_ignore_ascii_case("pik") {
                CoreCouponType::PIK
            } else if coupon_type_str.starts_with("split:") {
                let parts: Vec<&str> = coupon_type_str.split(':').collect();
                if parts.len() != 3 {
                    return Err(js_error(
                        "Split format must be 'split:cashPct:pikPct' (e.g., 'split:0.7:0.3')",
                    ));
                }
                let cash_pct: f64 = parts[1]
                    .parse()
                    .map_err(|_| js_error("Invalid cash percentage in split"))?;
                let pik_pct: f64 = parts[2]
                    .parse()
                    .map_err(|_| js_error("Invalid PIK percentage in split"))?;
                CoreCouponType::Split { cash_pct, pik_pct }
            } else {
                return Err(js_error(
                    "Coupon type must be 'cash', 'pik', or 'split:X:Y'",
                ));
            };

            let date = parse_iso_date(&date_str)?;
            parsed_steps.push((date, coupon_type));
        }

        let _ = self.inner.payment_split_program(&parsed_steps);
        Ok(self)
    }

    /// Build the complete cashflow schedule without market curves.
    ///
    /// For floating rate coupons, only the margin is used in the calculation.
    /// To incorporate forward rates from market curves, use `buildWithCurves()` instead.
    ///
    /// Returns a CashFlowSchedule containing all generated cashflows.
    #[wasm_bindgen]
    pub fn build(self) -> Result<JsCashFlowSchedule, JsValue> {
        let schedule = self.inner.build().map_err(|e| js_error(e.to_string()))?;
        Ok(JsCashFlowSchedule::new(schedule))
    }

    /// Build the cashflow schedule with market curves for floating rate computation.
    ///
    /// When a market context is provided, floating rate coupons include the forward rate:
    /// `coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction`
    ///
    /// Without curves (or using `build()`), only the margin is used:
    /// `coupon = outstanding * (margin_bp * 1e-4 * gearing) * year_fraction`
    ///
    /// # Arguments
    /// * `market` - Market context containing forward curves
    #[wasm_bindgen(js_name = buildWithCurves)]
    pub fn build_with_curves(
        self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<JsCashFlowSchedule, JsValue> {
        let schedule = self
            .inner
            .build_with_curves(Some(market.inner()))
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsCashFlowSchedule::new(schedule))
    }
}

/// Cashflow schedule containing generated cashflows and metadata.
#[wasm_bindgen(js_name = CashFlowSchedule)]
#[derive(Clone, Debug)]
pub struct JsCashFlowSchedule {
    inner: CoreCashFlowSchedule,
}

impl JsCashFlowSchedule {
    pub(crate) fn new(inner: CoreCashFlowSchedule) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &CoreCashFlowSchedule {
        &self.inner
    }
}

#[wasm_bindgen(js_class = CashFlowSchedule)]
impl JsCashFlowSchedule {
    /// Get the notional amount for this schedule.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional.initial)
    }

    /// Get the day count convention used.
    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> JsDayCount {
        // Construct JsDayCount from the inner value
        match self.inner.day_count {
            finstack_core::dates::DayCount::Act360 => JsDayCount::act_360(),
            finstack_core::dates::DayCount::Act365F => JsDayCount::act_365f(),
            finstack_core::dates::DayCount::Act365L => JsDayCount::act_365l(),
            finstack_core::dates::DayCount::Thirty360 => JsDayCount::thirty_360(),
            finstack_core::dates::DayCount::ThirtyE360 => JsDayCount::thirty_e_360(),
            finstack_core::dates::DayCount::ActAct => JsDayCount::act_act(),
            finstack_core::dates::DayCount::ActActIsma => JsDayCount::act_act_isma(),
            finstack_core::dates::DayCount::Bus252 => JsDayCount::bus_252(),
            _ => JsDayCount::act_365f(), // Default fallback
        }
    }

    /// Get all cashflows in the schedule.
    ///
    /// Returns an array of CashFlow objects.
    #[wasm_bindgen]
    pub fn flows(&self) -> Array {
        let result = Array::new();
        for cf in &self.inner.flows {
            result.push(&JsCashFlow::from_inner(*cf).into());
        }
        result
    }

    /// Get the number of cashflows in the schedule.
    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.inner.flows.len()
    }
}

// Helper function to parse ISO date strings (YYYY-MM-DD)
fn parse_iso_date(value: &str) -> Result<finstack_core::dates::Date, JsValue> {
    use time::Month;

    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 {
        return Err(js_error(format!(
            "Date '{value}' must be in ISO format YYYY-MM-DD"
        )));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| js_error(format!("Invalid year in date '{value}'")))?;
    let month: u8 = parts[1]
        .parse()
        .map_err(|_| js_error(format!("Invalid month in date '{value}'")))?;
    let day: u8 = parts[2]
        .parse()
        .map_err(|_| js_error(format!("Invalid day in date '{value}'")))?;

    let month_enum = Month::try_from(month)
        .map_err(|_| js_error(format!("Month must be 1-12 in date '{value}'")))?;

    finstack_core::dates::Date::from_calendar_date(year, month_enum, day)
        .map_err(|e| js_error(format!("Invalid date '{value}': {e}")))
}
