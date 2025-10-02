use finstack_core::math::distributions::{
    binomial_probability, log_binomial_coefficient, log_factorial,
};
use wasm_bindgen::prelude::*;

/// Calculate binomial probability for exact number of successes.
///
/// Computes P(X = k) where X ~ Binomial(n, p) for n trials with success probability p.
///
/// @param {number} trials - Total number of trials (n)
/// @param {number} successes - Number of successes (k, must be ≤ trials)
/// @param {number} probability - Success probability per trial (0 ≤ p ≤ 1)
/// @returns {number} Binomial probability P(X = successes)
/// @throws {Error} If probability is outside [0,1] or successes > trials
///
/// @example
/// ```javascript
/// // Fair coin flipped 10 times, probability of exactly 7 heads
/// const prob = binomialProbability(10, 7, 0.5);
/// console.log(prob);  // ~0.1171875
///
/// // Biased die (1/6 chance), 5 rolls, exactly 2 sixes
/// const diceProb = binomialProbability(5, 2, 1/6);
/// console.log(diceProb);  // ~0.160751
/// ```
#[wasm_bindgen(js_name = binomialProbability)]
pub fn binomial_probability_js(trials: usize, successes: usize, probability: f64) -> f64 {
    binomial_probability(trials, successes, probability)
}

/// Calculate natural logarithm of binomial coefficient C(n,k).
///
/// Computes log(C(n,k)) = log(n! / (k! * (n-k)!)) for numerical stability
/// when dealing with large factorials.
///
/// @param {number} trials - Total number of trials (n)
/// @param {number} successes - Number of successes (k, must be ≤ trials)
/// @returns {number} Natural log of binomial coefficient
/// @throws {Error} If successes > trials
///
/// @example
/// ```javascript
/// // Log of "10 choose 3"
/// const logCoeff = logBinomialCoefficient(10, 3);
/// console.log(logCoeff);  // ~2.302585 (log(120))
///
/// // For large numbers, use log version to avoid overflow
/// const largeLogCoeff = logBinomialCoefficient(1000, 250);
/// console.log(largeLogCoeff);  // ~663.813 (avoids factorial overflow)
/// ```
#[wasm_bindgen(js_name = logBinomialCoefficient)]
pub fn log_binomial_coefficient_js(trials: usize, successes: usize) -> f64 {
    log_binomial_coefficient(trials, successes)
}

/// Calculate natural logarithm of factorial for numerical stability.
///
/// Computes log(n!) = log(n) + log(n-1) + ... + log(2) + log(1).
/// Used to avoid overflow when working with large factorials.
///
/// @param {number} value - Non-negative integer to compute log factorial of
/// @returns {number} Natural logarithm of factorial
/// @throws {Error} If value is negative
///
/// @example
/// ```javascript
/// // Log of 5! = log(120)
/// const logFact5 = logFactorial(5);
/// console.log(logFact5);  // ~4.787492 (log(120))
///
/// // For large factorials, use log version
/// const logFact100 = logFactorial(100);
/// console.log(logFact100);  // ~363.739 (avoids overflow of 100!)
/// ```
#[wasm_bindgen(js_name = logFactorial)]
pub fn log_factorial_js(value: usize) -> f64 {
    log_factorial(value)
}
