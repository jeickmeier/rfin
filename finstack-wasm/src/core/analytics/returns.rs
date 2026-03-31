//! WASM bindings for return computation utilities.

use wasm_bindgen::prelude::*;

/// Compute simple (percentage-change) returns from a price series.
///
/// For prices `[p0, p1, p2, ...]` returns `[0.0, p1/p0 - 1, p2/p1 - 1, ...]`.
/// The leading zero keeps output length equal to input length.
///
/// @param {Float64Array} prices - Asset prices in chronological order
/// @returns {Float64Array} Simple returns (same length as prices)
#[wasm_bindgen(js_name = simpleReturns)]
pub fn simple_returns(prices: &[f64]) -> Vec<f64> {
    finstack_analytics::returns::simple_returns(prices)
}

/// Compute excess returns (portfolio minus risk-free).
///
/// When `nperiods` is provided, the risk-free rate is de-compounded:
/// `rf_adj = (1 + rf)^(1/nperiods) - 1`
///
/// @param {Float64Array} returns - Portfolio return series
/// @param {Float64Array} rf - Risk-free rate series (aligned with returns)
/// @param {number | undefined} nperiods - Optional compounding periods per year
/// @returns {Float64Array} Excess returns
#[wasm_bindgen(js_name = excessReturns)]
pub fn excess_returns(returns: &[f64], rf: &[f64], nperiods: Option<f64>) -> Vec<f64> {
    finstack_analytics::returns::excess_returns(returns, rf, nperiods)
}

/// Convert simple returns back to a price series starting at `base`.
///
/// @param {Float64Array} returns - Simple period returns
/// @param {number} base - Starting price level (e.g. 100.0)
/// @returns {Float64Array} Reconstructed prices (length = returns.length + 1)
#[wasm_bindgen(js_name = convertToPrices)]
pub fn convert_to_prices(returns: &[f64], base: f64) -> Vec<f64> {
    finstack_analytics::returns::convert_to_prices(returns, base)
}

/// Rebase a price series so the first value equals `base`.
///
/// @param {Float64Array} prices - Asset prices
/// @param {number} base - Desired starting value (e.g. 100.0)
/// @returns {Float64Array} Rebased prices
#[wasm_bindgen(js_name = rebasePrices)]
pub fn rebase_prices(prices: &[f64], base: f64) -> Vec<f64> {
    finstack_analytics::returns::rebase(prices, base)
}

/// Cumulative compounded returns: `(1+r).cumprod() - 1`.
///
/// @param {Float64Array} returns - Simple period returns
/// @returns {Float64Array} Cumulative returns (same length)
#[wasm_bindgen(js_name = compoundedCumulativeReturns)]
pub fn compounded_cumulative_returns(returns: &[f64]) -> Vec<f64> {
    finstack_analytics::returns::comp_sum(returns)
}

/// Total compounded return over the full slice: `prod(1 + r_i) - 1`.
///
/// @param {Float64Array} returns - Simple period returns
/// @returns {number} Total compounded return
#[wasm_bindgen(js_name = compoundedTotalReturn)]
pub fn compounded_total_return(returns: &[f64]) -> f64 {
    finstack_analytics::returns::comp_total(returns)
}
