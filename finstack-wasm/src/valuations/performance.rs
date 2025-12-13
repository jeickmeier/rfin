//! Performance measurement bindings for WASM.
//!
//! Provides XIRR (Extended Internal Rate of Return) calculation for
//! cash flows with irregular timing.

use js_sys::{Array, Date as JsDate};
use wasm_bindgen::prelude::*;

/// Calculate XIRR (Extended Internal Rate of Return) for a series of cash flows.
///
/// XIRR finds the discount rate that makes the net present value of all cash flows
/// equal to zero. It's particularly useful for investments with irregular timing.
///
/// @param {Array} cashFlows - Array of [Date, amount] tuples. Negative amounts represent
///                             outflows (investments), positive amounts represent inflows (returns).
/// @param {number | null} guess - Optional initial guess for the IRR (defaults to 0.1 = 10%)
/// @returns {number} The XIRR as a decimal (e.g., 0.15 for 15% annual return)
/// @throws {Error} If less than 2 cash flows, no sign change, or cannot converge
///
/// @example
/// ```typescript
/// const cashFlows = [
///   [new Date(2024, 0, 1), -100000],  // Initial investment
///   [new Date(2024, 6, 1), 5000],     // Mid-year dividend
///   [new Date(2025, 0, 1), 110000]    // Final value
/// ];
///
/// const irr = xirr(cashFlows, null);
/// console.log(`IRR: ${(irr * 100).toFixed(2)}%`);
/// ```
#[wasm_bindgen(js_name = xirr)]
pub fn xirr_wasm(cash_flows: Array, guess: Option<f64>) -> Result<f64, JsValue> {
    // Parse cash flows from JavaScript array
    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::new();

    for item in cash_flows.iter() {
        if !Array::is_array(&item) {
            return Err(JsValue::from_str(
                "Each cash flow must be an array [Date, amount]",
            ));
        }

        let pair = Array::from(&item);
        if pair.length() != 2 {
            return Err(JsValue::from_str(
                "Each cash flow must have exactly 2 elements: [Date, amount]",
            ));
        }

        // Extract date
        let date_value = pair.get(0);

        // Parse as native JS Date
        let js_date = date_value
            .dyn_ref::<JsDate>()
            .ok_or_else(|| JsValue::from_str("First element must be a JavaScript Date object"))?;

        // Extract year, month, day from JavaScript Date
        let year = js_date.get_full_year() as i32;
        let month = (js_date.get_month() + 1) as u8; // JS months are 0-based
        let day = js_date.get_date() as u8;

        // Convert to our core Date type
        let month_enum = time::Month::try_from(month)
            .map_err(|_| JsValue::from_str("Invalid month from JavaScript Date"))?;
        let core_date = finstack_core::dates::Date::from_calendar_date(year, month_enum, day)
            .map_err(|e| JsValue::from_str(&format!("Invalid date: {}", e)))?;

        // Extract amount
        let amount = pair
            .get(1)
            .as_f64()
            .ok_or_else(|| JsValue::from_str("Second element must be a number"))?;

        flows.push((core_date, amount));
    }

    // Call the core XIRR function
    finstack_core::cashflow::xirr(&flows, guess)
        .map_err(|e| JsValue::from_str(&format!("XIRR calculation failed: {}", e)))
}

/// Calculate NPV (Net Present Value) for a series of cash flows at a given discount rate.
///
/// @param {Array} cashFlows - Array of [Date, amount] tuples
/// @param {number} discountRate - Annual discount rate as a decimal (e.g., 0.1 for 10%)
/// @returns {number} The net present value
///
/// @example
/// ```typescript
/// const cashFlows = [
///   [new Date(2024, 0, 1), -100000],
///   [new Date(2025, 0, 1), 110000]
/// ];
///
/// const npv = calculateNpv(cashFlows, 0.05);
/// console.log(`NPV at 5%: $${npv.toFixed(2)}`);
/// ```
#[wasm_bindgen(js_name = calculateNpv)]
pub fn calculate_npv_wasm(cash_flows: Array, discount_rate: f64) -> Result<f64, JsValue> {
    // Parse cash flows
    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::new();

    for item in cash_flows.iter() {
        if !Array::is_array(&item) {
            return Err(JsValue::from_str(
                "Each cash flow must be an array [Date, amount]",
            ));
        }

        let pair = Array::from(&item);
        if pair.length() != 2 {
            return Err(JsValue::from_str(
                "Each cash flow must have exactly 2 elements: [Date, amount]",
            ));
        }

        let date_value = pair.get(0);

        // Parse as native JS Date
        let js_date = date_value
            .dyn_ref::<JsDate>()
            .ok_or_else(|| JsValue::from_str("First element must be a JavaScript Date object"))?;

        // Extract year, month, day from JavaScript Date
        let year = js_date.get_full_year() as i32;
        let month = (js_date.get_month() + 1) as u8; // JS months are 0-based
        let day = js_date.get_date() as u8;

        // Convert to our core Date type
        let month_enum = time::Month::try_from(month)
            .map_err(|_| JsValue::from_str("Invalid month from JavaScript Date"))?;
        let core_date = finstack_core::dates::Date::from_calendar_date(year, month_enum, day)
            .map_err(|e| JsValue::from_str(&format!("Invalid date: {}", e)))?;

        let amount = pair
            .get(1)
            .as_f64()
            .ok_or_else(|| JsValue::from_str("Second element must be a number"))?;

        flows.push((core_date, amount));
    }

    // Convert f64 flows to Money (using arbitrary currency for calculation)
    let money_flows: Vec<(finstack_core::dates::Date, finstack_core::money::Money)> = flows
        .into_iter()
        .map(|(d, a)| {
            (
                d,
                finstack_core::money::Money::new(a, finstack_core::currency::Currency::USD),
            )
        })
        .collect();

    // Default base date to first flow date if available, else today (arbitrary)
    let base_date = money_flows.first().map(|(d, _)| *d).unwrap_or_else(|| {
        finstack_core::dates::Date::from_calendar_date(2000, time::Month::January, 1).unwrap()
    });

    // Use Act365F as default day count for simple scalar NPV
    let dc = finstack_core::dates::DayCount::Act365F;

    // Use the core NPV function
    finstack_core::cashflow::performance::npv(&money_flows, discount_rate, base_date, dc)
        .map(|m| m.amount())
        .map_err(|e| JsValue::from_str(&format!("NPV calculation failed: {}", e)))
}

/// Calculate IRR (Internal Rate of Return) for evenly-spaced periodic cash flows.
///
/// This is a simplified version of XIRR for cash flows that occur at regular intervals
/// (e.g., monthly, quarterly, or annual).
///
/// @param {Array<number>} amounts - Array of cash flow amounts (negative for outflows, positive for inflows)
/// @param {number | null} guess - Optional initial guess for the IRR (defaults to 0.1 = 10%)
/// @returns {number} The IRR as a decimal
/// @throws {Error} If less than 2 cash flows or no sign change
///
/// @example
/// ```typescript
/// // Quarterly cash flows over 2 years
/// const amounts = [-100000, 3000, 3000, 3000, 3000, 3000, 3000, 3000, 90000];
/// const irr = irrPeriodic(amounts, null);
/// console.log(`Quarterly IRR: ${(irr * 100).toFixed(2)}%`);
/// console.log(`Annual IRR: ${(Math.pow(1 + irr, 4) - 1) * 100).toFixed(2)}%`);
/// ```
#[wasm_bindgen(js_name = irrPeriodic)]
pub fn irr_periodic_wasm(amounts: Vec<f64>, guess: Option<f64>) -> Result<f64, JsValue> {
    // Use the core IRR periodic function
    finstack_core::cashflow::irr_periodic(&amounts, guess)
        .map_err(|e| JsValue::from_str(&format!("IRR calculation failed: {}", e)))
}
