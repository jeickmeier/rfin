//! WASM bindings for consecutive streak counting.

use wasm_bindgen::prelude::*;

/// Count the maximum consecutive elements where the value exceeds the threshold.
///
/// Scans `values` left-to-right and returns the length of the longest
/// unbroken streak of elements above `threshold`. Typical usage: longest
/// run of positive returns (wins) in a return series.
///
/// @param {Float64Array} values - Return series to scan
/// @param {number} threshold - Value above which an element counts as a "hit"
/// @returns {number} Length of the longest consecutive streak
#[wasm_bindgen(js_name = countConsecutiveAbove)]
pub fn count_consecutive_above(values: &[f64], threshold: f64) -> usize {
    finstack_analytics::consecutive::count_consecutive(values, |v| v > threshold)
}

/// Count the maximum consecutive elements where the value is below the threshold.
///
/// Scans `values` left-to-right and returns the length of the longest
/// unbroken streak of elements below `threshold`. Typical usage: longest
/// run of negative returns (losses) in a return series.
///
/// @param {Float64Array} values - Return series to scan
/// @param {number} threshold - Value below which an element counts as a "hit"
/// @returns {number} Length of the longest consecutive streak
#[wasm_bindgen(js_name = countConsecutiveBelow)]
pub fn count_consecutive_below(values: &[f64], threshold: f64) -> usize {
    finstack_analytics::consecutive::count_consecutive(values, |v| v < threshold)
}
