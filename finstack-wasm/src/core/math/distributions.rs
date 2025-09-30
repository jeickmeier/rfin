use finstack_core::math::distributions::{
    binomial_probability, log_binomial_coefficient, log_factorial,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = binomialProbability)]
pub fn binomial_probability_js(trials: usize, successes: usize, probability: f64) -> f64 {
    binomial_probability(trials, successes, probability)
}

#[wasm_bindgen(js_name = logBinomialCoefficient)]
pub fn log_binomial_coefficient_js(trials: usize, successes: usize) -> f64 {
    log_binomial_coefficient(trials, successes)
}

#[wasm_bindgen(js_name = logFactorial)]
pub fn log_factorial_js(value: usize) -> f64 {
    log_factorial(value)
}
