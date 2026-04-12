use crate::math::{norm_cdf, norm_pdf};

#[derive(Clone, Copy, Debug)]
struct BlackState {
    st: f64,
    d1: f64,
    d2: f64,
}

#[derive(Clone, Copy, Debug)]
struct SpotBlackState {
    d1: f64,
    d2: f64,
    df_r: f64,
    df_q: f64,
}

#[inline]
fn all_finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

#[inline]
fn black_state(forward: f64, strike: f64, sigma: f64, t: f64) -> Option<BlackState> {
    if t <= 0.0 || sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return None;
    }

    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    Some(BlackState {
        st,
        d1,
        d2: d1 - st,
    })
}

#[inline]
fn black_scholes_spot_state(
    spot: f64,
    strike: f64,
    rate: f64,
    dividend_yield: f64,
    sigma: f64,
    t: f64,
) -> Option<SpotBlackState> {
    if t <= 0.0 || sigma <= 0.0 || spot <= 0.0 || strike <= 0.0 {
        return None;
    }

    let st = sigma * t.sqrt();
    let ln_sk = (spot / strike).ln();
    let d1 = (ln_sk + (rate - dividend_yield + 0.5 * sigma * sigma) * t) / st;
    Some(SpotBlackState {
        d1,
        d2: d1 - st,
        df_r: (-rate * t).exp(),
        df_q: (-dividend_yield * t).exp(),
    })
}

/// Black-76 (lognormal) call price with unit annuity.
pub fn black_call(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    match black_state(forward, strike, sigma, t) {
        Some(state) => forward * norm_cdf(state.d1) - strike * norm_cdf(state.d2),
        None => (forward - strike).max(0.0),
    }
}

/// Black-76 (lognormal) put price with unit annuity.
pub fn black_put(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    match black_state(forward, strike, sigma, t) {
        Some(state) => strike * norm_cdf(-state.d2) - forward * norm_cdf(-state.d1),
        None => (strike - forward).max(0.0),
    }
}

/// Black-Scholes-Merton call price on spot with continuous carry.
#[must_use]
pub fn black_scholes_spot_call(
    spot: f64,
    strike: f64,
    rate: f64,
    dividend_yield: f64,
    sigma: f64,
    t: f64,
) -> f64 {
    if !all_finite(&[spot, strike, rate, dividend_yield, sigma, t]) {
        return f64::NAN;
    }

    match black_scholes_spot_state(spot, strike, rate, dividend_yield, sigma, t) {
        Some(state) => {
            spot * state.df_q * norm_cdf(state.d1) - strike * state.df_r * norm_cdf(state.d2)
        }
        None => (spot - strike).max(0.0),
    }
}

/// Black-Scholes-Merton put price on spot with continuous carry.
#[must_use]
pub fn black_scholes_spot_put(
    spot: f64,
    strike: f64,
    rate: f64,
    dividend_yield: f64,
    sigma: f64,
    t: f64,
) -> f64 {
    if !all_finite(&[spot, strike, rate, dividend_yield, sigma, t]) {
        return f64::NAN;
    }

    match black_scholes_spot_state(spot, strike, rate, dividend_yield, sigma, t) {
        Some(state) => {
            strike * state.df_r * norm_cdf(-state.d2) - spot * state.df_q * norm_cdf(-state.d1)
        }
        None => (strike - spot).max(0.0),
    }
}

/// Geometric-average Asian call under GBM with discrete fixings.
#[must_use]
pub fn geometric_asian_call(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    num_fixings: usize,
) -> f64 {
    if !all_finite(&[spot, strike, time, rate, div_yield, vol]) {
        return f64::NAN;
    }
    if time <= 0.0 {
        return (spot - strike).max(0.0);
    }
    if vol <= 0.0 || spot <= 0.0 || strike <= 0.0 || num_fixings == 0 {
        let fwd = spot * ((rate - div_yield) * time).exp();
        return (-rate * time).exp() * (fwd - strike).max(0.0);
    }

    let n = num_fixings as f64;
    let sigma_g = vol * ((n + 1.0) * (2.0 * n + 1.0) / (6.0 * n * n)).sqrt();
    let b_g = 0.5 * (rate - div_yield - 0.5 * vol * vol) * (n + 1.0) / n + 0.5 * sigma_g * sigma_g;
    let st = sigma_g * time.sqrt();
    if st <= 0.0 {
        return 0.0;
    }

    let ln_s_over_k = (spot / strike).ln();
    let d1 = (ln_s_over_k + (b_g + 0.5 * sigma_g * sigma_g) * time) / st;
    let d2 = d1 - st;
    let df_r = (-rate * time).exp();
    let growth = (b_g * time).exp();
    df_r * (spot * growth * norm_cdf(d1) - strike * norm_cdf(d2))
}

/// Black-76 vega.
pub fn black_vega(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    match black_state(forward, strike, sigma, t) {
        Some(state) => forward * t.sqrt() * norm_pdf(state.d1),
        None => 0.0,
    }
}

/// Black-76 call delta with respect to the forward.
pub fn black_delta_call(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    match black_state(forward, strike, sigma, t) {
        Some(state) => norm_cdf(state.d1),
        None => {
            if forward >= strike {
                1.0
            } else {
                0.0
            }
        }
    }
}

/// Black-76 put delta with respect to the forward.
pub fn black_delta_put(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_delta_call(forward, strike, sigma, t) - 1.0
}

/// Black-76 gamma with respect to the forward.
pub fn black_gamma(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    match black_state(forward, strike, sigma, t) {
        Some(state) => norm_pdf(state.d1) / (forward * state.st),
        None => 0.0,
    }
}

/// Black-76 d1: `(ln(F/K) + 0.5 * σ² * T) / (σ * √T)`.
///
/// Returns `0.0` for degenerate inputs (σ ≤ 0, T ≤ 0, F ≤ 0, or K ≤ 0)
/// to match the guarded `black_state` behaviour used by all pricer functions.
#[inline]
pub fn d1_black76(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return 0.0;
    }
    let sqrt_t = t.sqrt();
    ((forward / strike).ln() + 0.5 * sigma * sigma * t) / (sigma * sqrt_t)
}

/// Shifted Black call price with unit annuity.
#[inline]
pub fn black_shifted_call(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_call(forward + shift, strike + shift, sigma, t)
}

/// Shifted Black put price with unit annuity.
#[inline]
pub fn black_shifted_put(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_put(forward + shift, strike + shift, sigma, t)
}

/// Shifted Black vega with unit annuity.
#[inline]
pub fn black_shifted_vega(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_vega(forward + shift, strike + shift, sigma, t)
}
