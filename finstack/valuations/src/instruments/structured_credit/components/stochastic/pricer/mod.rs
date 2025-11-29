//! Stochastic pricing engine for structured credit.
//!
//! This module provides tree-based and Monte Carlo pricing for structured credit
//! instruments with stochastic prepayment and default models.
//!
//! # Pricing Modes
//!
//! - **Tree-based**: Exact pricing using non-recombining scenario tree
//! - **Monte Carlo**: Statistical pricing with variance reduction
//!
//! # Features
//!
//! - Path-dependent state tracking (burnout, cumulative losses)
//! - Tranche-level cashflow generation
//! - Multi-currency support via FX rates
//! - Risk metric computation (EL, UL, ES)
//!
//! # Example
//!
//! ```ignore
//! let config = StochasticPricerConfig::rmbs_standard(
//!     valuation_date,
//!     discount_curve,
//!     pool_coupon,
//! );
//!
//! let pricer = StochasticPricer::new(config);
//! let result = pricer.price(&instrument)?;
//!
//! println!("NPV: {}", result.npv);
//! println!("Expected Loss: {}", result.expected_loss);
//! println!("Unexpected Loss: {}", result.unexpected_loss);
//! ```

mod config;
mod engine;
mod result;

pub use config::{PricingMode, StochasticPricerConfig};
pub use engine::StochasticPricer;
pub use result::{StochasticPricingResult, TranchePricingResult};
