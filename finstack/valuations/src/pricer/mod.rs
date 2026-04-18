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
//! - `keys`: [`crate::pricer::InstrumentType`], [`crate::pricer::ModelKey`],
//!   [`crate::pricer::PricerKey`]
//! - `errors`: [`crate::pricer::PricingError`],
//!   [`crate::pricer::PricingErrorContext`], [`crate::pricer::PricingResult`],
//!   [`crate::pricer::PricingContextExt`]
//! - `registry`: [`crate::pricer::Pricer`], [`crate::pricer::PricerRegistry`],
//!   `expect_inst`
//!
//! The registration logic is split into asset-class submodules:
//! - `rates`: Bond, IRS, FRA, BasisSwap, Deposit, CapFloor, Swaption, Repo, DCF, IR futures
//! - `credit`: CDS, CDSIndex, CDSTranche, CDSOption, StructuredCredit
//! - `equity`: Equity, EquityOption, EquityTRS, VarianceSwap, EquityIndexFuture, RealEstate, PE fund
//! - `fx`: FxSpot, FxSwap, XccySwap, FxOption, FxVarianceSwap, FxForward, NDF, FX barrier/digital/touch
//! - `fixed_income`: FIIndexTRS, Convertible, InflationLinkedBond, RevolvingCredit, TermLoan, MBS, TBA, CMO
//! - `inflation`: InflationSwap, YoYInflationSwap, InflationCapFloor
//! - `exotics`: Basket, Asian, Barrier, Lookback, Quanto, Autocallable, CMS, Cliquet, RangeAccrual, BermudanSwaption
//! - `commodity`: CommodityForward, CommoditySwap, CommodityOption, CommoditySwaption, CommoditySpreadOption

// Core submodules
mod errors;
mod keys;
mod registry;

pub(crate) use errors::actionable_unknown_pricer_message;
pub use errors::{PricingContextExt, PricingError, PricingErrorContext, PricingResult};
pub use keys::{InstrumentType, ModelKey, PricerKey};
pub use registry::{expect_inst, Pricer, PricerRegistry};

// Fourier pricing methods (COS, Lewis)
pub mod fourier;

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

/// Build a standard pricer registry with all registered pricers.
///
/// This helper explicitly registers all instrument pricers into a fresh registry.
/// The explicit registration approach provides better visibility, IDE support, and
/// debugging capabilities compared to the previous auto-registration system.
///
/// All 40+ instrument pricers are registered in the `register_all_pricers` function.
/// Note: All pricers now use standardized parameter ordering: (instrument, market, as_of).
fn build_standard_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_all_pricers(&mut registry);
    registry
}

static STANDARD_PRICER_REGISTRY: OnceLock<Arc<PricerRegistry>> = OnceLock::new();

/// Return the shared standard pricer registry by reference.
///
/// This is the primary public entry point for accessing the built-in pricer set.
/// Callers that need to mutate a registry should start from `standard_registry().clone()`.
pub fn standard_registry() -> &'static PricerRegistry {
    STANDARD_PRICER_REGISTRY
        .get_or_init(|| Arc::new(build_standard_registry()))
        .as_ref()
}

/// Return the shared standard pricer registry.
///
/// The registry is initialized once and then cloned via `Arc` for cheap reuse
/// across instrument-side pricing calls.
pub fn shared_standard_registry() -> Arc<PricerRegistry> {
    STANDARD_PRICER_REGISTRY
        .get_or_init(|| Arc::new(build_standard_registry()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::ValuationResult;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use std::ptr;

    struct DummyPricer;

    impl Pricer for DummyPricer {
        fn key(&self) -> PricerKey {
            PricerKey::new(InstrumentType::Deposit, ModelKey::Tree)
        }

        fn price_dyn(
            &self,
            _instrument: &dyn crate::instruments::internal::InstrumentExt,
            _market: &finstack_core::market_data::MarketContext,
            as_of: finstack_core::dates::Date,
        ) -> PricingResult<ValuationResult> {
            Ok(ValuationResult::stamped(
                "dummy",
                as_of,
                Money::new(0.0, Currency::USD),
            ))
        }
    }

    #[test]
    fn standard_registry_returns_shared_singleton() {
        assert!(ptr::eq(standard_registry(), standard_registry()));
        assert!(ptr::eq(
            standard_registry(),
            shared_standard_registry().as_ref(),
        ));
    }

    #[test]
    fn cloned_standard_registry_is_independently_mutable() {
        let key = PricerKey::new(InstrumentType::Deposit, ModelKey::Tree);
        assert!(standard_registry().get_pricer(key).is_none());

        let mut cloned = standard_registry().clone();
        cloned.register_pricer(key, Arc::new(DummyPricer));

        assert!(cloned.get_pricer(key).is_some());
        assert!(standard_registry().get_pricer(key).is_none());
    }
}
