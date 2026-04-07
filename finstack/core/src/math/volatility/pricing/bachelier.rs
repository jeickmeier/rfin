use crate::math::{norm_cdf, norm_pdf};

#[derive(Clone, Copy, Debug)]
struct BachelierState {
    st: f64,
    d: f64,
}

#[inline]
fn bachelier_state(forward: f64, strike: f64, sigma_n: f64, t: f64) -> Option<BachelierState> {
    if t <= 0.0 || sigma_n <= 0.0 {
        return None;
    }

    let st = sigma_n * t.sqrt();
    Some(BachelierState {
        st,
        d: (forward - strike) / st,
    })
}

/// Bachelier (normal) call price with unit annuity.
pub fn bachelier_call(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    match bachelier_state(forward, strike, sigma_n, t) {
        Some(state) => (forward - strike) * norm_cdf(state.d) + state.st * norm_pdf(state.d),
        None => (forward - strike).max(0.0),
    }
}

/// Bachelier (normal) put price with unit annuity.
pub fn bachelier_put(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    match bachelier_state(forward, strike, sigma_n, t) {
        Some(state) => (strike - forward) * norm_cdf(-state.d) + state.st * norm_pdf(state.d),
        None => (strike - forward).max(0.0),
    }
}

/// Bachelier vega.
pub fn bachelier_vega(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    match bachelier_state(forward, strike, sigma_n, t) {
        Some(state) => t.sqrt() * norm_pdf(state.d),
        None => 0.0,
    }
}

/// Bachelier call delta with respect to the forward.
pub fn bachelier_delta_call(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    match bachelier_state(forward, strike, sigma_n, t) {
        Some(state) => norm_cdf(state.d),
        None => {
            if forward >= strike {
                1.0
            } else {
                0.0
            }
        }
    }
}

/// Bachelier put delta with respect to the forward.
pub fn bachelier_delta_put(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_delta_call(forward, strike, sigma_n, t) - 1.0
}

/// Bachelier gamma with respect to the forward.
pub fn bachelier_gamma(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    match bachelier_state(forward, strike, sigma_n, t) {
        Some(state) => norm_pdf(state.d) / state.st,
        None => 0.0,
    }
}
