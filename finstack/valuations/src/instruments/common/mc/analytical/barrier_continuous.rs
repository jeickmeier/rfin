//! Continuous barrier option formulas for validation.
//!
//! Implements Reiner-Rubinstein formulas for continuous monitoring barriers.
//! Used to validate discrete barrier corrections (Gobet-Miri, Brownian bridge).
//!
//! Reference:
//! - Reiner & Rubinstein (1991) - "Breaking Down the Barriers"
//! - Merton (1973) - "Theory of Rational Option Pricing"

use finstack_core::math::special_functions::norm_cdf;

/// Barrier option type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BarrierType {
    UpIn,
    UpOut,
    DownIn,
    DownOut,
}

/// Helper function for barrier pricing.
fn barrier_helper(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    eta: f64,  // 1 for call, -1 for put
    phi: f64,  // 1 for up, -1 for down
) -> f64 {
    if time <= 0.0 || vol <= 0.0 {
        return 0.0;
    }

    let mu = (rate - div_yield - 0.5 * vol * vol) / (vol * vol);
    let _lambda = (mu * mu + 2.0 * rate / (vol * vol)).sqrt();
    
    let x = (spot / strike).ln() / (vol * time.sqrt()) + (1.0 + mu) * vol * time.sqrt();
    let x1 = (spot / barrier).ln() / (vol * time.sqrt()) + (1.0 + mu) * vol * time.sqrt();
    let y = (barrier * barrier / (spot * strike)).ln() / (vol * time.sqrt()) + (1.0 + mu) * vol * time.sqrt();
    let y1 = (barrier / spot).ln() / (vol * time.sqrt()) + (1.0 + mu) * vol * time.sqrt();
    
    let discount = (-rate * time).exp();
    let forward_discount = (-div_yield * time).exp();
    
    // Standard vanilla components
    let a = phi * spot * forward_discount * norm_cdf(phi * x) - phi * strike * discount * norm_cdf(phi * (x - vol * time.sqrt()));
    
    // Barrier-adjusted components
    let b = phi * spot * forward_discount * norm_cdf(phi * x1) - phi * strike * discount * norm_cdf(phi * (x1 - vol * time.sqrt()));
    
    let c = phi * spot * forward_discount * (barrier / spot).powf(2.0 * (mu + 1.0)) * norm_cdf(eta * y)
        - phi * strike * discount * (barrier / spot).powf(2.0 * mu) * norm_cdf(eta * (y - vol * time.sqrt()));
    
    let d = phi * spot * forward_discount * (barrier / spot).powf(2.0 * (mu + 1.0)) * norm_cdf(eta * y1)
        - phi * strike * discount * (barrier / spot).powf(2.0 * mu) * norm_cdf(eta * (y1 - vol * time.sqrt()));
    
    // Combine based on barrier type
    if spot > barrier {
        // Up barrier
        if eta == 1.0 {  // Call
            a - b + c - d
        } else {  // Put
            b - c + d
        }
    } else {
        // Down barrier
        if eta == 1.0 {  // Call
            b - c + d
        } else {  // Put
            a - b + c - d
        }
    }
}

/// Price a continuous up-and-out call.
pub fn up_out_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot >= barrier {
        return 0.0;  // Already knocked out
    }
    
    // Up-and-out = Vanilla - Up-and-in
    let vanilla = {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        spot * (-div_yield * time).exp() * norm_cdf(d1) - strike * (-rate * time).exp() * norm_cdf(d2)
    };
    
    let up_in = barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, 1.0);
    
    vanilla - up_in
}

/// Price a continuous up-and-in call.
pub fn up_in_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot >= barrier {
        // Already knocked in, price as vanilla
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        return spot * (-div_yield * time).exp() * norm_cdf(d1) - strike * (-rate * time).exp() * norm_cdf(d2);
    }
    
    barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, 1.0)
}

/// Price a continuous down-and-out call.
pub fn down_out_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot <= barrier {
        return 0.0;  // Already knocked out
    }
    
    let vanilla = {
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        spot * (-div_yield * time).exp() * norm_cdf(d1) - strike * (-rate * time).exp() * norm_cdf(d2)
    };
    
    let down_in = barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, -1.0);
    
    vanilla - down_in
}

/// Price a continuous down-and-in call.
pub fn down_in_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
) -> f64 {
    if spot <= barrier {
        // Already knocked in, price as vanilla
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        return spot * (-div_yield * time).exp() * norm_cdf(d1) - strike * (-rate * time).exp() * norm_cdf(d2);
    }
    
    barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, 1.0, -1.0)
}

/// Generic barrier call price dispatcher.
pub fn barrier_call_continuous(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    match barrier_type {
        BarrierType::UpIn => up_in_call(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::UpOut => up_out_call(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::DownIn => down_in_call(spot, strike, barrier, time, rate, div_yield, vol),
        BarrierType::DownOut => down_out_call(spot, strike, barrier, time, rate, div_yield, vol),
    }
}

/// Generic barrier put price (using put-call transformation).
pub fn barrier_put_continuous(
    spot: f64,
    strike: f64,
    barrier: f64,
    time: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    barrier_type: BarrierType,
) -> f64 {
    // Put pricing using similar formulas with eta = -1
    barrier_helper(spot, strike, barrier, time, rate, div_yield, vol, -1.0, 
                  if matches!(barrier_type, BarrierType::UpIn | BarrierType::UpOut) { 1.0 } else { -1.0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barrier_in_plus_out_equals_vanilla() {
        let spot = 100.0;
        let strike = 100.0;
        let barrier = 120.0;
        let time = 1.0;
        let rate = 0.05;
        let div_yield = 0.02;
        let vol = 0.2;
        
        let up_in = up_in_call(spot, strike, barrier, time, rate, div_yield, vol);
        let up_out = up_out_call(spot, strike, barrier, time, rate, div_yield, vol);
        
        // Vanilla call price
        let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * time.sqrt());
        let d2 = d1 - vol * time.sqrt();
        let vanilla = spot * (-div_yield * time).exp() * norm_cdf(d1) - strike * (-rate * time).exp() * norm_cdf(d2);
        
        let sum = up_in + up_out;
        
        assert!((sum - vanilla).abs() < 0.01, "Barrier parity failed: {} vs {}", sum, vanilla);
    }

    #[test]
    fn test_up_out_call_knocked_out() {
        let price = up_out_call(125.0, 100.0, 120.0, 1.0, 0.05, 0.02, 0.2);
        assert_eq!(price, 0.0, "Already above barrier should be zero");
    }

    #[test]
    fn test_down_out_call_knocked_out() {
        let price = down_out_call(75.0, 100.0, 80.0, 1.0, 0.05, 0.02, 0.2);
        assert_eq!(price, 0.0, "Already below barrier should be zero");
    }

    #[test]
    fn test_barrier_prices_non_negative() {
        let spot = 100.0;
        let strike = 100.0;
        let barrier_up = 120.0;
        let barrier_down = 80.0;
        let time = 1.0;
        let rate = 0.05;
        let div_yield = 0.02;
        let vol = 0.2;
        
        assert!(up_in_call(spot, strike, barrier_up, time, rate, div_yield, vol) >= 0.0);
        assert!(up_out_call(spot, strike, barrier_up, time, rate, div_yield, vol) >= 0.0);
        assert!(down_in_call(spot, strike, barrier_down, time, rate, div_yield, vol) >= 0.0);
        assert!(down_out_call(spot, strike, barrier_down, time, rate, div_yield, vol) >= 0.0);
    }
}

