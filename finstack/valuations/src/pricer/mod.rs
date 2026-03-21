//! Pricer infrastructure: type-safe pricing dispatch via registry pattern.
//!
//! This module provides a registry-based pricing system that maps
//! (instrument type, model) pairs to specific pricer implementations.
//! The system uses enum-based dispatch for type safety rather than string
//! comparisons.
//!
//! # Module structure
//!
//! Core types are split into focused submodules:
//! - [`keys`]: `InstrumentType`, `ModelKey`, `PricerKey`
//! - [`errors`]: `PricingError`, `PricingErrorContext`, `PricingResult`, `PricingContextExt`
//! - [`registry`]: `Pricer` trait, `PricerRegistry`, `expect_inst`
//!
//! The registration logic is split into asset-class submodules:
//! - [`rates`]: Bond, IRS, FRA, BasisSwap, Deposit, CapFloor, Swaption, Repo, DCF, IR futures
//! - [`credit`]: CDS, CDSIndex, CDSTranche, CDSOption, StructuredCredit
//! - [`equity`]: Equity, EquityOption, EquityTRS, VarianceSwap, EquityIndexFuture, RealEstate, PE fund
//! - [`fx`]: FxSpot, FxSwap, XccySwap, FxOption, FxVarianceSwap, FxForward, NDF, FX barrier/digital/touch
//! - [`fixed_income`]: FIIndexTRS, Convertible, InflationLinkedBond, RevolvingCredit, TermLoan, MBS, TBA, CMO
//! - [`inflation`]: InflationSwap, YoYInflationSwap, InflationCapFloor
//! - [`exotics`]: Basket, Asian, Barrier, Lookback, Quanto, Autocallable, CMS, Cliquet, RangeAccrual, BermudanSwaption
//! - [`commodity`]: CommodityForward, CommoditySwap, CommodityOption, CommoditySwaption, CommoditySpreadOption

// Core submodules
mod errors;
mod keys;
mod registry;

pub use errors::*;
pub use keys::*;
pub use registry::*;

// Asset-class registration submodules
mod commodity;
mod credit;
mod equity;
mod exotics;
mod fixed_income;
mod fx;
mod inflation;
mod rates;

use std::sync::{Arc, OnceLock};

/// Register all standard pricers explicitly.
///
/// This function registers all instrument pricers in a single, visible location.
/// This explicit approach provides better IDE support, easier debugging, and
/// clearer dependency tracking compared to auto-registration.
fn register_all_pricers(registry: &mut PricerRegistry) {
    register_rates_pricers(registry);
    register_credit_pricers(registry);
    register_equity_pricers(registry);
    register_fx_pricers(registry);
    register_fixed_income_pricers(registry);
    register_inflation_pricers(registry);
    register_exotic_pricers(registry);
    register_commodity_pricers(registry);
}

/// Register a minimal set of pricers for rates instruments.
///
/// Intended for environments (like WASM) where registering *all* pricers may be
/// too memory intensive.
pub fn register_rates_pricers(registry: &mut PricerRegistry) {
    rates::register_rates_pricers(registry);
}

/// Register pricers for credit instruments.
pub fn register_credit_pricers(registry: &mut PricerRegistry) {
    credit::register_credit_pricers(registry);
}

/// Register pricers for equity instruments.
pub fn register_equity_pricers(registry: &mut PricerRegistry) {
    equity::register_equity_pricers(registry);
}

/// Register pricers for FX instruments.
pub fn register_fx_pricers(registry: &mut PricerRegistry) {
    fx::register_fx_pricers(registry);
}

/// Register pricers for additional fixed-income instruments (convertibles, MBS,
/// revolving credit, term loans) not included in the minimal rates set.
pub fn register_fixed_income_pricers(registry: &mut PricerRegistry) {
    fixed_income::register_fixed_income_pricers(registry);
}

/// Register pricers for inflation instruments (swaps, caps/floors).
pub fn register_inflation_pricers(registry: &mut PricerRegistry) {
    inflation::register_inflation_pricers(registry);
}

/// Register pricers for exotic instruments (barriers, lookbacks, Asians,
/// autocallables, quantos, cliquets, range accruals, Bermudan swaptions).
pub fn register_exotic_pricers(registry: &mut PricerRegistry) {
    exotics::register_exotic_pricers(registry);
}

/// Register pricers for commodity instruments.
pub fn register_commodity_pricers(registry: &mut PricerRegistry) {
    commodity::register_commodity_pricers(registry);
}

/// Create a standard pricer registry with all registered pricers.
///
/// This function creates a registry and explicitly registers all instrument pricers.
/// The explicit registration approach provides better visibility, IDE support, and
/// debugging capabilities compared to the previous auto-registration system.
///
/// All 40+ instrument pricers are registered in the `register_all_pricers` function.
/// Note: All pricers now use standardized parameter ordering: (instrument, market, as_of).
pub fn create_standard_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_all_pricers(&mut registry);
    registry
}

static STANDARD_PRICER_REGISTRY: OnceLock<Arc<PricerRegistry>> = OnceLock::new();

/// Return the shared standard pricer registry.
///
/// The registry is initialized once and then cloned via `Arc` for cheap reuse
/// across instrument-side pricing calls.
pub fn shared_standard_registry() -> Arc<PricerRegistry> {
    STANDARD_PRICER_REGISTRY
        .get_or_init(|| Arc::new(create_standard_registry()))
        .clone()
}
