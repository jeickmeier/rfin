use finstack_core::math::distributions::{
    binomial_distribution, binomial_probability, chi_squared_cdf, chi_squared_pdf,
    chi_squared_quantile, exponential_cdf, exponential_pdf, exponential_quantile,
    log_binomial_coefficient, log_factorial, lognormal_cdf, lognormal_pdf, lognormal_quantile,
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

/// Calculate complete binomial distribution P(X=k) for k = 0, 1, ..., n.
///
/// @param {number} n - Number of trials
/// @param {number} p - Success probability per trial
/// @returns {Float64Array} Array of probabilities [P(X=0), P(X=1), ..., P(X=n)]
///
/// @example
/// ```javascript
/// const dist = binomialDistribution(5, 0.5);
/// // dist[2] = P(X=2) for 5 coin flips
/// ```
#[wasm_bindgen(js_name = binomialDistribution)]
pub fn binomial_distribution_js(n: usize, p: f64) -> Vec<f64> {
    binomial_distribution(n, p)
}

// =============================================================================
// Chi-Squared Distribution
// =============================================================================

/// Chi-squared probability density function.
///
/// @param {number} x - Value (must be non-negative)
/// @param {number} df - Degrees of freedom (must be positive)
/// @returns {number} Probability density at x
///
/// @example
/// ```javascript
/// const density = chiSquaredPdf(5.0, 3);  // χ²(3) density at x=5
/// ```
#[wasm_bindgen(js_name = chiSquaredPdf)]
pub fn chi_squared_pdf_js(x: f64, df: f64) -> f64 {
    chi_squared_pdf(x, df)
}

/// Chi-squared cumulative distribution function.
///
/// @param {number} x - Value (must be non-negative)
/// @param {number} df - Degrees of freedom (must be positive)
/// @returns {number} P(X ≤ x) where X ~ χ²(df)
///
/// @example
/// ```javascript
/// const prob = chiSquaredCdf(7.81, 3);  // ≈ 0.95 (95th percentile for df=3)
/// ```
#[wasm_bindgen(js_name = chiSquaredCdf)]
pub fn chi_squared_cdf_js(x: f64, df: f64) -> f64 {
    chi_squared_cdf(x, df)
}

/// Chi-squared quantile function (inverse CDF).
///
/// @param {number} p - Probability in (0, 1)
/// @param {number} df - Degrees of freedom (must be positive)
/// @returns {number} x such that P(X ≤ x) = p
///
/// @example
/// ```javascript
/// const critical = chiSquaredQuantile(0.95, 3);  // ≈ 7.81 (critical value)
/// ```
#[wasm_bindgen(js_name = chiSquaredQuantile)]
pub fn chi_squared_quantile_js(p: f64, df: f64) -> f64 {
    chi_squared_quantile(p, df)
}

// =============================================================================
// Lognormal Distribution
// =============================================================================

/// Lognormal probability density function.
///
/// @param {number} x - Value (must be positive)
/// @param {number} mu - Mean of the underlying normal distribution
/// @param {number} sigma - Standard deviation of the underlying normal (must be positive)
/// @returns {number} Probability density at x
///
/// @example
/// ```javascript
/// const density = lognormalPdf(1.0, 0.0, 1.0);  // Standard lognormal at x=1
/// ```
#[wasm_bindgen(js_name = lognormalPdf)]
pub fn lognormal_pdf_js(x: f64, mu: f64, sigma: f64) -> f64 {
    lognormal_pdf(x, mu, sigma)
}

/// Lognormal cumulative distribution function.
///
/// @param {number} x - Value (must be positive)
/// @param {number} mu - Mean of the underlying normal distribution
/// @param {number} sigma - Standard deviation of the underlying normal (must be positive)
/// @returns {number} P(X ≤ x)
///
/// @example
/// ```javascript
/// const prob = lognormalCdf(1.0, 0.0, 1.0);  // ≈ 0.5 (median of standard lognormal)
/// ```
#[wasm_bindgen(js_name = lognormalCdf)]
pub fn lognormal_cdf_js(x: f64, mu: f64, sigma: f64) -> f64 {
    lognormal_cdf(x, mu, sigma)
}

/// Lognormal quantile function (inverse CDF).
///
/// @param {number} p - Probability in (0, 1)
/// @param {number} mu - Mean of the underlying normal distribution
/// @param {number} sigma - Standard deviation of the underlying normal (must be positive)
/// @returns {number} x such that P(X ≤ x) = p
///
/// @example
/// ```javascript
/// const median = lognormalQuantile(0.5, 0.0, 1.0);  // = 1.0 (median)
/// ```
#[wasm_bindgen(js_name = lognormalQuantile)]
pub fn lognormal_quantile_js(p: f64, mu: f64, sigma: f64) -> f64 {
    lognormal_quantile(p, mu, sigma)
}

// =============================================================================
// Exponential Distribution
// =============================================================================

/// Exponential probability density function.
///
/// @param {number} x - Value (must be non-negative)
/// @param {number} lambda - Rate parameter (must be positive)
/// @returns {number} Probability density at x
///
/// @example
/// ```javascript
/// const density = exponentialPdf(1.0, 0.5);  // λ=0.5, mean=2
/// ```
#[wasm_bindgen(js_name = exponentialPdf)]
pub fn exponential_pdf_js(x: f64, lambda: f64) -> f64 {
    exponential_pdf(x, lambda)
}

/// Exponential cumulative distribution function.
///
/// @param {number} x - Value (must be non-negative)
/// @param {number} lambda - Rate parameter (must be positive)
/// @returns {number} P(X ≤ x) = 1 - e^(-λx)
///
/// @example
/// ```javascript
/// const prob = exponentialCdf(2.0, 0.5);  // P(X ≤ 2) where mean = 2
/// ```
#[wasm_bindgen(js_name = exponentialCdf)]
pub fn exponential_cdf_js(x: f64, lambda: f64) -> f64 {
    exponential_cdf(x, lambda)
}

/// Exponential quantile function (inverse CDF).
///
/// @param {number} p - Probability in (0, 1)
/// @param {number} lambda - Rate parameter (must be positive)
/// @returns {number} x such that P(X ≤ x) = p
///
/// @example
/// ```javascript
/// const median = exponentialQuantile(0.5, 1.0);  // ≈ 0.693 (ln(2))
/// ```
#[wasm_bindgen(js_name = exponentialQuantile)]
pub fn exponential_quantile_js(p: f64, lambda: f64) -> f64 {
    exponential_quantile(p, lambda)
}
