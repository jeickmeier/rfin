//! Implied volatility solver for options.
//!
//! Implements robust implied volatility calculation using Brenner-Subrahmanyam
//! initial guess and Halley's method for fast convergence.

use finstack_core::{F, Result, Error};
use crate::instruments::options::{OptionType, black_scholes_common};
use std::f64::consts::PI;

/// Configuration for implied volatility solver
#[derive(Clone, Debug)]
pub struct ImpliedVolConfig {
    /// Tolerance for convergence
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Minimum volatility bound
    pub min_vol: F,
    /// Maximum volatility bound  
    pub max_vol: F,
    /// Use Halley's method (vs Newton)
    pub use_halley: bool,
}

impl Default for ImpliedVolConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 20,
            min_vol: 0.001,  // 0.1%
            max_vol: 5.0,     // 500%
            use_halley: true, // Halley converges faster for implied vol
        }
    }
}

/// Implied volatility solver
pub struct ImpliedVolSolver {
    config: ImpliedVolConfig,
}

impl ImpliedVolSolver {
    /// Create new solver with default config
    pub fn new() -> Self {
        Self {
            config: ImpliedVolConfig::default(),
        }
    }
    
    /// Create solver with custom config
    pub fn with_config(config: ImpliedVolConfig) -> Self {
        Self { config }
    }
    
    /// Brenner-Subrahmanyam initial volatility guess
    /// This approximation works well for near-ATM options
    #[allow(clippy::too_many_arguments)]
    fn initial_guess(
        &self,
        market_price: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> F {
        // Calculate intrinsic value
        let intrinsic = match option_type {
            OptionType::Call => (spot * (-(q * t)).exp() - strike * (-(r * t)).exp()).max(0.0),
            OptionType::Put => (strike * (-(r * t)).exp() - spot * (-(q * t)).exp()).max(0.0),
        };
        
        // Time value
        let time_value = (market_price - intrinsic).max(0.01);
        
        // Brenner-Subrahmanyam approximation
        let forward = spot * ((r - q) * t).exp();
        (2.0 * PI / t).sqrt() * time_value / forward
    }
    
    /// Calculate option price, vega, and volga for given volatility
    #[allow(clippy::too_many_arguments)]
    fn price_vega_volga(
        &self,
        vol: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> (F, F, F) {
        if vol <= 0.0 || t <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        
        let sqrt_t = t.sqrt();
        let d1 = black_scholes_common::d1(spot, strike, r, vol, t, q);
        let d2 = black_scholes_common::d2(spot, strike, r, vol, t, q);
        
        let nd1 = black_scholes_common::norm_cdf(d1);
        let nd2 = black_scholes_common::norm_cdf(d2);
        let npd1 = black_scholes_common::norm_pdf(d1);
        
        let exp_qt = (-q * t).exp();
        let exp_rt = (-r * t).exp();
        
        // Price
        let price = match option_type {
            OptionType::Call => spot * exp_qt * nd1 - strike * exp_rt * nd2,
            OptionType::Put => strike * exp_rt * (1.0 - nd2) - spot * exp_qt * (1.0 - nd1),
        };
        
        // Vega
        let vega = spot * exp_qt * npd1 * sqrt_t;
        
        // Volga (dvega/dvol)
        let volga = vega * d1 * d2 / vol;
        
        (price, vega, volga)
    }
    
    /// Solve for implied volatility given market price
    #[allow(clippy::too_many_arguments)]
    pub fn solve(
        &self,
        market_price: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<F> {
        // Handle edge cases
        if t <= 0.0 {
            return Err(Error::Internal);
        }
        
        if market_price <= 0.0 {
            return Err(Error::Internal);
        }
        
        // Check if price is below intrinsic value
        let intrinsic = match option_type {
            OptionType::Call => {
                (spot * (-(q * t)).exp() - strike * (-(r * t)).exp()).max(0.0)
            },
            OptionType::Put => {
                (strike * (-(r * t)).exp() - spot * (-(q * t)).exp()).max(0.0)
            }
        };
        
        if market_price < intrinsic - 1e-10 {
            return Err(Error::Internal); // No solution exists
        }
        
        // If price equals intrinsic (deep ITM/OTM), return small vol
        if (market_price - intrinsic).abs() < 1e-10 {
            return Ok(self.config.min_vol);
        }
        
        // Initial guess
        let mut vol = self.initial_guess(
            market_price,
            spot,
            strike,
            r,
            t,
            q,
            option_type,
        );
        
        // Clamp initial guess to bounds
        vol = vol.clamp(self.config.min_vol, self.config.max_vol);
        
        // Iteration
        for _i in 0..self.config.max_iterations {
            let (price, vega, volga) = self.price_vega_volga(
                vol,
                spot,
                strike,
                r,
                t,
                q,
                option_type,
            );
            
            let diff = price - market_price;
            
            // Check convergence
            if diff.abs() < self.config.tolerance {
                return Ok(vol);
            }
            
            // Check if vega is too small
            if vega.abs() < 1e-10 {
                // Try bisection as fallback
                return self.bisection_fallback(
                    market_price,
                    spot,
                    strike,
                    r,
                    t,
                    q,
                    option_type,
                );
            }
            
            // Update volatility
            if self.config.use_halley {
                // Halley's method (third-order convergence)
                let h = diff / vega;
                let halley_adjustment = h * (1.0 + 0.5 * h * volga / vega);
                vol -= halley_adjustment;
            } else {
                // Newton's method (second-order convergence)
                vol -= diff / vega;
            }
            
            // Clamp to bounds
            vol = vol.clamp(self.config.min_vol, self.config.max_vol);
        }
        
        // If we didn't converge, try bisection
        self.bisection_fallback(
            market_price,
            spot,
            strike,
            r,
            t,
            q,
            option_type,
        )
    }
    
    /// Bisection method as fallback
    #[allow(clippy::too_many_arguments)]
    fn bisection_fallback(
        &self,
        market_price: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<F> {
        let mut low = self.config.min_vol;
        let mut high = self.config.max_vol;
        
        // Check bounds
        let (low_price, _, _) = self.price_vega_volga(low, spot, strike, r, t, q, option_type);
        let (high_price, _, _) = self.price_vega_volga(high, spot, strike, r, t, q, option_type);
        
        if low_price > market_price || high_price < market_price {
            return Err(Error::Internal); // Market price outside bounds
        }
        
        for _ in 0..50 {
            let mid = (low + high) / 2.0;
            let (price, _, _) = self.price_vega_volga(
                mid,
                spot,
                strike,
                r,
                t,
                q,
                option_type,
            );
            
            if (price - market_price).abs() < self.config.tolerance {
                return Ok(mid);
            }
            
            if price < market_price {
                low = mid;
            } else {
                high = mid;
            }
            
            // Check if interval is too small
            if (high - low) < 1e-10 {
                return Ok(mid);
            }
        }
        
        Err(Error::Internal)
    }
    
    /// Solve for implied volatility from option prices with moneyness
    /// Returns (implied_vol, moneyness)
    #[allow(clippy::too_many_arguments)]
    pub fn solve_with_moneyness(
        &self,
        market_price: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<(F, F)> {
        let iv = self.solve(market_price, spot, strike, r, t, q, option_type)?;
        let forward = spot * ((r - q) * t).exp();
        let moneyness = strike / forward;
        Ok((iv, moneyness))
    }
}

impl Default for ImpliedVolSolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to solve for implied volatility with default settings
pub fn implied_volatility(
    market_price: F,
    spot: F,
    strike: F,
    r: F,
    t: F,
    q: F,
    option_type: OptionType,
) -> Result<F> {
    let solver = ImpliedVolSolver::new();
    solver.solve(market_price, spot, strike, r, t, q, option_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_implied_vol_atm_call() {
        // Test ATM call option
        let spot = 100.0;
        let strike = 100.0;
        let r = 0.05;
        let t = 1.0;
        let q = 0.0;
        let true_vol = 0.20;
        
        // Calculate Black-Scholes price with known vol
        let d1 = black_scholes_common::d1(spot, strike, r, true_vol, t, q);
        let d2 = black_scholes_common::d2(spot, strike, r, true_vol, t, q);
        let market_price = spot * (-q * t).exp() * black_scholes_common::norm_cdf(d1)
            - strike * (-r * t).exp() * black_scholes_common::norm_cdf(d2);
        
        // Solve for implied vol
        let solver = ImpliedVolSolver::new();
        let iv = solver.solve(market_price, spot, strike, r, t, q, OptionType::Call).unwrap();
        
        // Should recover the original volatility
        assert!((iv - true_vol).abs() < 1e-8);
    }
    
    #[test]
    fn test_implied_vol_otm_put() {
        // Test OTM put option
        let spot = 100.0;
        let strike = 90.0;
        let r = 0.05;
        let t = 0.25;
        let q = 0.02;
        let true_vol = 0.30;
        
        // Calculate Black-Scholes price
        let d1 = black_scholes_common::d1(spot, strike, r, true_vol, t, q);
        let d2 = black_scholes_common::d2(spot, strike, r, true_vol, t, q);
        let market_price = strike * (-r * t).exp() * black_scholes_common::norm_cdf(-d2)
            - spot * (-q * t).exp() * black_scholes_common::norm_cdf(-d1);
        
        // Solve for implied vol
        let solver = ImpliedVolSolver::new();
        let iv = solver.solve(market_price, spot, strike, r, t, q, OptionType::Put).unwrap();
        
        // Should recover the original volatility
        assert!((iv - true_vol).abs() < 1e-8);
    }
    
    #[test]
    #[allow(clippy::unnecessary_cast)]
    fn test_implied_vol_deep_itm() {
        // Test deep ITM call (should handle gracefully)
        let spot = 100.0;
        let strike = 50.0;
        let r = 0.05;
        let t = 1.0;
        let q = 0.0;
        
        // Deep ITM price is approximately intrinsic value
        let intrinsic = spot * ((-q * t) as f64).exp() - strike * ((-r * t) as f64).exp();
        let market_price = intrinsic + 0.01; // Small time value
        
        let solver = ImpliedVolSolver::new();
        let result = solver.solve(market_price, spot, strike, r, t, q, OptionType::Call);
        
        // Should return a valid (small) volatility
        assert!(result.is_ok());
        let iv = result.unwrap();
        assert!(iv > 0.0 && iv < 1.0); // Should be a valid volatility for deep ITM
    }
    
    #[test]
    fn test_implied_vol_halley_vs_newton() {
        // Compare Halley's method vs Newton's method
        let spot = 100.0;
        let strike = 105.0;
        let r = 0.05;
        let t = 0.5;
        let q = 0.0;
        let true_vol = 0.25;
        
        // Calculate market price
        let d1 = black_scholes_common::d1(spot, strike, r, true_vol, t, q);
        let d2 = black_scholes_common::d2(spot, strike, r, true_vol, t, q);
        let market_price = spot * (-q * t).exp() * black_scholes_common::norm_cdf(d1)
            - strike * (-r * t).exp() * black_scholes_common::norm_cdf(d2);
        
        // Solve with Halley's method
        let config_halley = ImpliedVolConfig {
            use_halley: true,
            ..Default::default()
        };
        let solver_halley = ImpliedVolSolver::with_config(config_halley);
        let iv_halley = solver_halley.solve(market_price, spot, strike, r, t, q, OptionType::Call).unwrap();
        
        // Solve with Newton's method
        let config_newton = ImpliedVolConfig {
            use_halley: false,
            ..Default::default()
        };
        let solver_newton = ImpliedVolSolver::with_config(config_newton);
        let iv_newton = solver_newton.solve(market_price, spot, strike, r, t, q, OptionType::Call).unwrap();
        
        // Both should converge to same value
        assert!((iv_halley - true_vol).abs() < 1e-8);
        assert!((iv_newton - true_vol).abs() < 1e-8);
        assert!((iv_halley - iv_newton).abs() < 1e-10);
    }
    
    #[test]
    #[allow(clippy::unnecessary_cast)]
    fn test_implied_vol_below_intrinsic() {
        // Test that solver rejects prices below intrinsic value
        let spot = 100.0;
        let strike = 100.0;
        let r = 0.05;
        let t = 1.0;
        let q = 0.0;
        
        let intrinsic = (spot * ((-q * t) as f64).exp() - strike * ((-r * t) as f64).exp()).max(0.0_f64);
        let market_price = intrinsic - 1.0; // Below intrinsic
        
        let solver = ImpliedVolSolver::new();
        let result = solver.solve(market_price, spot, strike, r, t, q, OptionType::Call);
        
        // Should return an error
        assert!(result.is_err());
    }
    
    #[test]
    fn test_brenner_subrahmanyam_initial_guess() {
        // Test that initial guess is reasonable
        let solver = ImpliedVolSolver::new();
        let spot = 100.0;
        let strike = 100.0;
        let r = 0.05;
        let t = 1.0;
        let q = 0.0;
        
        // ATM option with reasonable time value
        let market_price = 10.0;
        
        let guess = solver.initial_guess(market_price, spot, strike, r, t, q, OptionType::Call);
        
        // Initial guess should be reasonable (between 10% and 50% for ATM)
        assert!(guess > 0.1 && guess < 0.5);
    }
}
