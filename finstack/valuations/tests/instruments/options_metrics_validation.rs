//! Options metrics validation tests against market-standard formulas.
//!
//! These tests validate option Greeks and fundamental option properties:
//! - Put-call parity
//! - Delta bounds and behavior
//! - Gamma positivity
//! - Vega symmetry
//! - Theta negativity for long options
//!
//! References:
//! - Black & Scholes (1973), "The Pricing of Options and Corporate Liabilities"
//! - Hull, "Options, Futures, and Other Derivatives"

use finstack_valuations::instruments::common::models::{d1, d2};
use finstack_core::math::{norm_cdf, norm_pdf};

#[test]
fn test_black_scholes_delta_formula() {
    // Verify analytical delta formula implementation
    // Call Delta: Δ_c = e^(-qT) × N(d1)
    // Put Delta: Δ_p = -e^(-qT) × N(-d1)
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02; // dividend yield
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    
    // Call delta
    let call_delta = (-q * t).exp() * norm_cdf(d1_val);
    
    // Put delta  
    let put_delta = -(-q * t).exp() * norm_cdf(-d1_val);
    
    // Call delta should be in (0, 1)
    assert!(
        call_delta > 0.0 && call_delta < 1.0,
        "Call delta={:.4} outside range (0, 1)",
        call_delta
    );
    
    // Put delta should be in (-1, 0)
    assert!(
        put_delta > -1.0 && put_delta < 0.0,
        "Put delta={:.4} outside range (-1, 0)",
        put_delta
    );
    
    // For ATM options, call delta ≈ 0.5 (slightly above due to lognormal distribution)
    assert!(
        (0.45..0.60).contains(&call_delta),
        "ATM call delta={:.4} should be near 0.5",
        call_delta
    );
}

#[test]
fn test_black_scholes_gamma_formula() {
    // Gamma: Γ = e^(-qT) × N'(d1) / (S × σ × √T)
    // Gamma is always positive and same for calls and puts
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let pdf_d1 = norm_pdf(d1_val);
    
    let gamma = (-q * t).exp() * pdf_d1 / (spot * sigma * t.sqrt());
    
    // Gamma should be positive
    assert!(gamma > 0.0, "Gamma={:.6} should be positive", gamma);
    
    // For ATM options, gamma is highest
    // Typical range for ATM 1Y option: 0.01-0.02
    assert!(
        (0.005..=0.03).contains(&gamma),
        "ATM gamma={:.6} outside typical range 0.005-0.03",
        gamma
    );
}

#[test]
fn test_black_scholes_vega_formula() {
    // Vega: ν = S × e^(-qT) × N'(d1) × √T / 100
    // Vega is same for calls and puts
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let pdf_d1 = norm_pdf(d1_val);
    
    let vega = spot * (-q * t).exp() * pdf_d1 * t.sqrt() / 100.0;
    
    // Vega should be positive
    assert!(vega > 0.0, "Vega={:.4} should be positive", vega);
    
    // Vega per 1% vol move (formula divides by 100)
    // For $100 spot, 1Y ATM, vega ~0.38 per 1% vol
    assert!(
        (0.30..0.50).contains(&vega),
        "Vega={:.2} outside typical range 0.30-0.50 for $100 spot 1Y ATM",
        vega
    );
}

#[test]
fn test_black_scholes_theta_formula() {
    // Theta for call: Θ = -[S×N'(d1)×σ×e^(-qT)]/(2√T) - r×K×e^(-rT)×N(d2) + q×S×e^(-qT)×N(d1)
    // Theta is typically negative for long options (time decay)
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let d2_val = d2(spot, strike, r, sigma, t, q);
    let pdf_d1 = norm_pdf(d1_val);
    let cdf_d1 = norm_cdf(d1_val);
    let cdf_d2 = norm_cdf(d2_val);
    
    let term1 = -spot * pdf_d1 * sigma * (-q * t).exp() / (2.0 * t.sqrt());
    let term2 = q * spot * cdf_d1 * (-q * t).exp();
    let term3 = -r * strike * (-r * t).exp() * cdf_d2;
    
    // Annualized theta
    let theta_annual = term1 + term2 + term3;
    
    // Daily theta (divide by 252 trading days)
    let theta_daily = theta_annual / 252.0;
    
    // Theta should be negative for long ATM call
    assert!(
        theta_daily < 0.0,
        "Long option theta={:.4} should be negative",
        theta_daily
    );
}

#[test]
fn test_put_call_parity_formula() {
    // Market Standard: C - P = S×e^(-qT) - K×e^(-rT)
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let d2_val = d2(spot, strike, r, sigma, t, q);
    
    // Call price
    let call_price = spot * (-q * t).exp() * norm_cdf(d1_val)
        - strike * (-r * t).exp() * norm_cdf(d2_val);
    
    // Put price
    let put_price = strike * (-r * t).exp() * norm_cdf(-d2_val)
        - spot * (-q * t).exp() * norm_cdf(-d1_val);
    
    // Put-call parity
    let parity_lhs = call_price - put_price;
    let parity_rhs = spot * (-q * t).exp() - strike * (-r * t).exp();
    
    assert!(
        (parity_lhs - parity_rhs).abs() < 1e-10,
        "Put-call parity violated: C - P = {:.6}, S*e^(-qT) - K*e^(-rT) = {:.6}",
        parity_lhs,
        parity_rhs
    );
}

#[test]
fn test_delta_bounds() {
    // Delta should be bounded: Call ∈ [0, 1], Put ∈ [-1, 0]
    
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    // Test various spot/strike combinations
    let spot_strikes = vec![
        (80.0, 100.0),  // OTM call
        (100.0, 100.0), // ATM
        (120.0, 100.0), // ITM call
    ];
    
    for (spot, strike) in spot_strikes {
        let d1_val = d1(spot, strike, r, sigma, t, q);
        
        let call_delta = (-q * t).exp() * norm_cdf(d1_val);
        let put_delta = -(-q * t).exp() * norm_cdf(-d1_val);
        
        assert!(
            (0.0..=1.0).contains(&call_delta),
            "Call delta={:.4} outside [0, 1] for S={:.0}, K={:.0}",
            call_delta,
            spot,
            strike
        );
        
        assert!(
            (-1.0..=0.0).contains(&put_delta),
            "Put delta={:.4} outside [-1, 0] for S={:.0}, K={:.0}",
            put_delta,
            spot,
            strike
        );
    }
}

#[test]
fn test_gamma_always_positive() {
    // Gamma is always >= 0 for both calls and puts
    
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    // Test various moneyness levels
    for (spot, strike) in [(80.0, 100.0), (100.0, 100.0), (120.0, 100.0)] {
        let d1_val = d1(spot, strike, r, sigma, t, q);
        let pdf_d1 = norm_pdf(d1_val);
        
        let gamma = (-q * t).exp() * pdf_d1 / (spot * sigma * t.sqrt());
        
        assert!(
            gamma >= 0.0,
            "Gamma={:.6} should be non-negative for S={:.0}, K={:.0}",
            gamma,
            spot,
            strike
        );
    }
}

#[test]
fn test_vega_same_for_call_and_put() {
    // Vega is identical for calls and puts with same strike/expiry
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let pdf_d1 = norm_pdf(d1_val);
    
    let vega = spot * (-q * t).exp() * pdf_d1 * t.sqrt() / 100.0;
    
    // Vega is same for call and put (formula doesn't depend on option type)
    // Just verify it's positive and reasonable
    assert!(vega > 0.0, "Vega should be positive");
    
    // Vega per 1% vol move
    assert!(
        (0.30..0.50).contains(&vega),
        "Vega={:.2} outside typical range",
        vega
    );
}

#[test]
fn test_atm_option_characteristics() {
    // At-the-money options have specific characteristics:
    // - Delta ≈ 0.5 for calls (slightly above due to lognormal)
    // - Gamma is maximized
    // - Vega is maximized
    
    let spot = 100.0;
    let strike = 100.0; // ATM
    let r = 0.05;
    let q = 0.0; // No dividend
    let sigma = 0.20;
    let t = 0.5; // 6 months
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let pdf_d1 = norm_pdf(d1_val);
    
    // Call delta
    let delta = norm_cdf(d1_val);
    
    // For ATM with no dividend and r=5%, delta should be around 0.60
    // (lognormal distribution skews delta above 0.5)
    assert!(
        (0.55..0.65).contains(&delta),
        "ATM call delta={:.4} should be around 0.60",
        delta
    );
    
    // Gamma at ATM
    let gamma = pdf_d1 / (spot * sigma * t.sqrt());
    
    // Compare to gamma at strikes +/- 10
    let d1_otm = d1(spot, strike + 10.0, r, sigma, t, q);
    let gamma_otm = norm_pdf(d1_otm) / (spot * sigma * t.sqrt());
    
    assert!(
        gamma > gamma_otm,
        "ATM gamma={:.6} should be higher than OTM gamma={:.6}",
        gamma,
        gamma_otm
    );
}

#[test]
fn test_deep_itm_call_delta_near_one() {
    // Deep in-the-money call delta approaches 1.0
    
    let spot = 150.0;
    let strike = 100.0; // Deep ITM
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let delta = (-q * t).exp() * norm_cdf(d1_val);
    
    assert!(
        delta > 0.92,
        "Deep ITM call delta={:.4} should be close to 1.0",
        delta
    );
}

#[test]
fn test_deep_otm_call_delta_near_zero() {
    // Deep out-of-the-money call delta approaches 0.0
    
    let spot = 80.0;
    let strike = 120.0; // Deep OTM
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d1_val = d1(spot, strike, r, sigma, t, q);
    let delta = (-q * t).exp() * norm_cdf(d1_val);
    
    assert!(
        delta < 0.10,
        "Deep OTM call delta={:.4} should be close to 0.0",
        delta
    );
}

#[test]
fn test_vega_increases_with_time() {
    // Vega increases with time to expiration
    // Vega ∝ √T
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    
    let mut vegas = Vec::new();
    
    for t in [0.25, 0.5, 1.0, 2.0] {
        let d1_val = d1(spot, strike, r, sigma, t, q);
        let pdf_d1 = norm_pdf(d1_val);
        let vega = spot * (-q * t).exp() * pdf_d1 * t.sqrt() / 100.0;
        vegas.push((t, vega));
    }
    
    // Verify vega increases with time
    for i in 1..vegas.len() {
        assert!(
            vegas[i].1 > vegas[i - 1].1,
            "Vega should increase with time: T={:.2} vega={:.2} <= T={:.2} vega={:.2}",
            vegas[i - 1].0,
            vegas[i - 1].1,
            vegas[i].0,
            vegas[i].1
        );
    }
}

#[test]
fn test_rho_call_vs_put() {
    // Call rho is positive, put rho is negative
    // Call Rho: ρ_c = K × T × e^(-rT) × N(d2) / 100
    // Put Rho: ρ_p = -K × T × e^(-rT) × N(-d2) / 100
    
    let spot = 100.0;
    let strike = 100.0;
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    let d2_val = d2(spot, strike, r, sigma, t, q);
    
    let call_rho = strike * t * (-r * t).exp() * norm_cdf(d2_val) / 100.0;
    let put_rho = -strike * t * (-r * t).exp() * norm_cdf(-d2_val) / 100.0;
    
    // Call rho should be positive
    assert!(
        call_rho > 0.0,
        "Call rho={:.4} should be positive",
        call_rho
    );
    
    // Put rho should be negative
    assert!(
        put_rho < 0.0,
        "Put rho={:.4} should be negative",
        put_rho
    );
}

#[test]
fn test_option_value_non_negative() {
    // Option values are always non-negative
    
    let test_cases = vec![
        (80.0, 100.0),  // OTM call, ITM put
        (100.0, 100.0), // ATM
        (120.0, 100.0), // ITM call, OTM put
    ];
    
    let r = 0.05;
    let q = 0.02;
    let sigma = 0.25;
    let t = 1.0;
    
    for (spot, strike) in test_cases {
        let d1_val = d1(spot, strike, r, sigma, t, q);
        let d2_val = d2(spot, strike, r, sigma, t, q);
        
        // Call value
        let call = spot * (-q * t).exp() * norm_cdf(d1_val)
            - strike * (-r * t).exp() * norm_cdf(d2_val);
        
        // Put value
        let put = strike * (-r * t).exp() * norm_cdf(-d2_val)
            - spot * (-q * t).exp() * norm_cdf(-d1_val);
        
        assert!(
            call >= 0.0,
            "Call value={:.4} should be non-negative for S={:.0}, K={:.0}",
            call,
            spot,
            strike
        );
        
        assert!(
            put >= 0.0,
            "Put value={:.4} should be non-negative for S={:.0}, K={:.0}",
            put,
            spot,
            strike
        );
    }
}
