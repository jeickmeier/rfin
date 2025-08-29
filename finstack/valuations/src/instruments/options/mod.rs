//! Option instruments including equity, FX, interest rate, and credit options.
//!
//! Provides comprehensive option valuation using Black-Scholes, Garman-Kohlhagen,
//! Black model, and credit option models with full Greeks calculation.

use finstack_core::F;
use std::f64::consts::PI;

pub mod equity_option;
pub mod fx_option;
pub mod interest_rate_option;
pub mod credit_option;
pub mod greeks;
pub mod greeks_enhanced;
pub mod metrics;
pub mod binomial_tree;
pub mod implied_vol;

pub use equity_option::EquityOption;
pub use fx_option::FxOption;
pub use interest_rate_option::{InterestRateOption, RateOptionType};
pub use credit_option::CreditOption;
pub use greeks::{GreeksCalculator, Greeks};
pub use greeks_enhanced::{EnhancedGreeksCalculator, CrossGreeks, CrossGreeksCalculator};
pub use binomial_tree::{BinomialTree, TreeType};
pub use implied_vol::{ImpliedVolSolver, implied_volatility};

/// Option type (Call or Put)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionType {
    /// Call option (right to buy)
    Call,
    /// Put option (right to sell)
    Put,
}

/// Option exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExerciseStyle {
    /// European option (exercise only at maturity)
    European,
    /// American option (exercise any time before maturity)
    American,
    /// Bermudan option (exercise on specific dates)
    Bermudan,
}

/// Settlement type for options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettlementType {
    /// Physical delivery of underlying
    Physical,
    /// Cash settlement
    Cash,
}

/// Common functions for Black-Scholes-type models
pub(crate) mod black_scholes_common {
    use super::*;
    
    /// Cumulative standard normal distribution function
    pub fn norm_cdf(x: F) -> F {
        0.5 * (1.0 + erf(x / (2.0_f64).sqrt()))
    }
    
    /// Standard normal probability density function
    pub fn norm_pdf(x: F) -> F {
        (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
    }
    
    /// Error function approximation
    fn erf(x: F) -> F {
        // Abramowitz and Stegun approximation
        let a1 =  0.254829592;
        let a2 = -0.284496736;
        let a3 =  1.421413741;
        let a4 = -1.453152027;
        let a5 =  1.061405429;
        let p  =  0.3275911;
        
        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();
        
        let t = 1.0 / (1.0 + p * x);
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;
        let t5 = t4 * t;
        
        let y = 1.0 - ((((a5 * t5 + a4 * t4) + a3 * t3) + a2 * t2) + a1 * t) * (-x * x).exp();
        
        sign * y
    }
    
    /// Calculate d1 for Black-Scholes formula
    pub fn d1(spot: F, strike: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }
        ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt())
    }
    
    /// Calculate d2 for Black-Scholes formula
    pub fn d2(spot: F, strike: F, r: F, sigma: F, t: F, q: F) -> F {
        d1(spot, strike, r, sigma, t, q) - sigma * t.sqrt()
    }
}
