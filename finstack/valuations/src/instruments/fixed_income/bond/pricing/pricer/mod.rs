//! Bond pricer implementations for the pricing registry.
//!
//! Each pricer integrates a bond pricing engine with the instrument pricing
//! registry system. Pricers are organized by model:
//!
//! - [`SimpleBondDiscountingPricer`]: Risk-free PV via discount curves
//! - [`SimpleBondHazardPricer`]: Credit-adjusted PV via hazard curves (FRP)
//! - [`SimpleBondOasPricer`]: OAS for callable/putable bonds via tree pricing
//! - [`SimpleBondMertonMcPricer`]: Structural credit MC for PIK bonds (feature-gated)

mod discount;
mod hazard;
#[cfg(feature = "mc")]
mod merton_mc;
mod oas;

pub use discount::SimpleBondDiscountingPricer;
pub use hazard::SimpleBondHazardPricer;
#[cfg(feature = "mc")]
pub use merton_mc::SimpleBondMertonMcPricer;
pub use oas::SimpleBondOasPricer;
