//! Comprehensive equity option test suite.
//!
//! This module organizes tests by functional area following market standards:
//! - Pricing: Black-Scholes PV validation
//! - Greeks: Individual greek tests with bounds and relationships
//! - Implied Vol: Solver accuracy and edge cases
//! - Edge Cases: Expiry, extreme strikes, zero vol, etc.
//! - Parity: Put-call parity validation
//! - Moneyness: ITM/ATM/OTM behavior

mod helpers;
mod integration;
mod test_constructors;
mod test_edge_cases;
mod test_greeks;
mod test_implied_vol;
mod test_moneyness;
mod test_near_expiry;
mod test_pricing;
mod test_put_call_parity;

mod test_option_pricing;