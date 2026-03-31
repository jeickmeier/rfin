//! Performance measurement bindings for WASM.
//!
//! Provides XIRR/IRR (Internal Rate of Return) and NPV calculations for
//! cash flows with irregular and periodic timing.

use crate::core::error::js_error;
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
    use finstack_core::cashflow::InternalRateOfReturn;
    flows
        .irr(guess)
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

    // Delegate defaults + calculation to core library
    finstack_core::cashflow::npv_amounts(&flows, discount_rate, None, None)
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
    use finstack_core::cashflow::InternalRateOfReturn;
    amounts
        .irr(guess)
        .map_err(|e| JsValue::from_str(&format!("IRR calculation failed: {}", e)))
}

/// Extended IRR result with root-ambiguity metadata.
///
/// Contains the IRR rate plus diagnostic information about whether
/// multiple solutions may exist (Descartes' rule of signs).
///
/// @example
/// ```javascript
/// const result = irrDetailed([-100, 230, -132, 5], null);
/// console.log(result.rate);
/// console.log(result.signChanges);
/// console.log(result.multipleRootsPossible);
/// ```
#[wasm_bindgen(js_name = IrrResult)]
#[derive(Clone, Debug)]
pub struct JsIrrResult {
    rate: f64,
    sign_changes: usize,
    multiple_roots_possible: bool,
}

#[wasm_bindgen(js_class = IrrResult)]
impl JsIrrResult {
    /// The computed internal rate of return.
    #[wasm_bindgen(getter)]
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Number of sign changes in the cashflow sequence (upper bound on positive roots).
    #[wasm_bindgen(getter, js_name = signChanges)]
    pub fn sign_changes(&self) -> usize {
        self.sign_changes
    }

    /// Whether multiple roots are possible (sign_changes > 1).
    #[wasm_bindgen(getter, js_name = multipleRootsPossible)]
    pub fn multiple_roots_possible(&self) -> bool {
        self.multiple_roots_possible
    }
}

/// Calculate IRR with root-ambiguity metadata for periodic cashflows.
///
/// @param {Float64Array} amounts - Periodic cashflow amounts
/// @param {number | null} guess - Optional initial guess
/// @returns {IrrResult} IRR result with diagnostic metadata
#[wasm_bindgen(js_name = irrDetailed)]
pub fn irr_detailed_wasm(amounts: Vec<f64>, guess: Option<f64>) -> Result<JsIrrResult, JsValue> {
    let result = finstack_core::cashflow::irr_detailed(&amounts, guess)
        .map_err(|e| js_error(format!("IRR calculation failed: {e}")))?;
    Ok(JsIrrResult {
        rate: result.rate,
        sign_changes: result.sign_changes,
        multiple_roots_possible: result.multiple_roots_possible,
    })
}

/// Calculate XIRR with root-ambiguity metadata for dated cashflows.
///
/// @param {Array} cashFlows - Array of [Date, amount] tuples
/// @param {string} dayCount - Day count convention name (e.g. "act_365f")
/// @param {number | null} guess - Optional initial guess
/// @returns {IrrResult} XIRR result with diagnostic metadata
#[wasm_bindgen(js_name = xirrDetailed)]
pub fn xirr_detailed_wasm(
    cash_flows: Array,
    day_count: &str,
    guess: Option<f64>,
) -> Result<JsIrrResult, JsValue> {
    use crate::core::common::parse::ParseFromString;

    let dc = finstack_core::dates::DayCount::parse_from_string(day_count)?;

    let mut flows: Vec<(finstack_core::dates::Date, f64)> = Vec::new();
    for item in cash_flows.iter() {
        if !Array::is_array(&item) {
            return Err(js_error("Each cash flow must be an array [Date, amount]"));
        }
        let pair = Array::from(&item);
        if pair.length() != 2 {
            return Err(js_error(
                "Each cash flow must have exactly 2 elements: [Date, amount]",
            ));
        }

        let date_value = pair.get(0);
        let js_date = date_value
            .dyn_ref::<JsDate>()
            .ok_or_else(|| js_error("First element must be a JavaScript Date object"))?;

        let year = js_date.get_full_year() as i32;
        let month = (js_date.get_month() + 1) as u8;
        let day = js_date.get_date() as u8;

        let month_enum = time::Month::try_from(month)
            .map_err(|_| js_error("Invalid month from JavaScript Date"))?;
        let core_date = finstack_core::dates::Date::from_calendar_date(year, month_enum, day)
            .map_err(|e| js_error(format!("Invalid date: {e}")))?;

        let amount = pair
            .get(1)
            .as_f64()
            .ok_or_else(|| js_error("Second element must be a number"))?;

        flows.push((core_date, amount));
    }

    let result = finstack_core::cashflow::xirr_detailed(&flows, dc, guess)
        .map_err(|e| js_error(format!("XIRR calculation failed: {e}")))?;

    Ok(JsIrrResult {
        rate: result.rate,
        sign_changes: result.sign_changes,
        multiple_roots_possible: result.multiple_roots_possible,
    })
}

/// Count sign changes in a numeric sequence.
///
/// Useful for Descartes' rule of signs to bound the number of real roots.
///
/// @param {Float64Array} values - Numeric sequence
/// @returns {number} Number of sign changes
#[wasm_bindgen(js_name = countSignChanges)]
pub fn count_sign_changes_wasm(values: Vec<f64>) -> usize {
    finstack_core::cashflow::count_sign_changes(values)
}
