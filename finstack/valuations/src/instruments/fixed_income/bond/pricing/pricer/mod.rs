//! Bond pricer implementations for the pricing registry.
//!
//! Each pricer integrates a bond pricing engine with the instrument pricing
//! registry system. Pricers are organized by model:
//!
//! - [`SimpleBondHazardPricer`]: Credit-adjusted PV via hazard curves (FRP)
//! - [`SimpleBondOasPricer`]: OAS for callable/putable bonds via tree pricing
//! - [`SimpleBondMertonMcPricer`]: Structural credit MC for PIK bonds (feature-gated)

mod discount;
mod hazard;
#[cfg(feature = "mc")]
mod merton_mc;
mod oas;

pub(crate) use hazard::SimpleBondHazardPricer;
#[cfg(feature = "mc")]
pub use merton_mc::SimpleBondMertonMcPricer;
pub(crate) use oas::SimpleBondOasPricer;
