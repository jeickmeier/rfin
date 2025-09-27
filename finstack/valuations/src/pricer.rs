//! Pricer infrastructure: type-safe pricing dispatch via registry pattern.
//! 
//! This module provides a registry-based pricing system that maps 
//! (instrument type, model) pairs to specific pricer implementations.
//! The system uses enum-based dispatch for type safety rather than string
//! comparisons.

use finstack_core::market_data::MarketContext as Market;
use crate::instruments::common::traits::Instrument as Priceable;

// ========================= KEYS =========================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum InstrumentType {
    Bond = 1,
    Loan = 2,
    CDS = 3,
    CDSIndex = 4,
    CDSTranche = 5,
    CDSOption = 6,
    IRS = 7,
    CapFloor = 8,
    Swaption = 9,
    TRS = 10,
    BasisSwap = 11,
    Basket = 12,
    Convertible = 13,
    Deposit = 14,
    EquityOption = 15,
    FxOption = 16,
    FxSpot = 17,
    FxSwap = 18,
    InflationLinkedBond = 19,
    InflationSwap = 20,
    InterestRateFuture = 21,
    VarianceSwap = 22,
    Equity = 23,
    Repo = 24,
    FRA = 25,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ModelKey {
    Discounting = 1,
    Tree = 2,
    Black76 = 3,
    HullWhite1F = 4,
    HazardRate = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PricerKey {
    pub instrument: InstrumentType,
    pub model: ModelKey,
}

impl PricerKey {
    pub const fn new(instrument: InstrumentType, model: ModelKey) -> Self {
        Self { instrument, model }
    }
}

// ========================= ERRORS =========================

#[derive(Debug)]
pub enum PricingError {
    UnknownPricer(PricerKey),
    TypeMismatch {
        expected: InstrumentType,
        got: InstrumentType,
    },
    ModelFailure(String),
}

impl From<finstack_core::Error> for PricingError {
    fn from(err: finstack_core::Error) -> Self {
        PricingError::ModelFailure(err.to_string())
    }
}

impl std::fmt::Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PricingError::UnknownPricer(key) => {
                write!(f, "No pricer found for {:?}", key)
            }
            PricingError::TypeMismatch { expected, got } => {
                write!(f, "Type mismatch: expected {:?}, got {:?}", expected, got)
            }
            PricingError::ModelFailure(msg) => {
                write!(f, "Model failure: {}", msg)
            }
        }
    }
}

impl std::error::Error for PricingError {}

// ========================= TRAITS =========================

/// Helper function to safely downcast a trait object to a concrete instrument type.
/// 
/// This performs both enum-based type checking and actual type downcasting,
/// ensuring type safety at both levels.
pub fn expect_inst<T: Priceable + 'static>(
    inst: &dyn Priceable,
    expected: InstrumentType,
) -> std::result::Result<&T, PricingError> {
    // First check the enum-based type
    if inst.key() != expected {
        return Err(PricingError::TypeMismatch {
            expected,
            got: inst.key(),
        });
    }
    
    // Then perform actual downcast
    inst.as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| PricingError::TypeMismatch {
            expected,
            got: inst.key(),
        })
}

/// Trait for instrument pricers.
/// 
/// Each pricer handles a specific (instrument, model) combination and knows
/// how to price that instrument using the specified model.
pub trait Pricer: Send + Sync {
    /// Get the (instrument, model) key this pricer handles
    fn key(&self) -> PricerKey;
    
    /// Price an instrument using this pricer's model
    fn price_dyn(
        &self,
        instrument: &dyn Priceable,
        market: &Market,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError>;
}


// ========================= REGISTRY =========================

use std::collections::HashMap;

/// Trait-based pricer registry
#[derive(Default)]
pub struct PricerRegistry {
    pricers: HashMap<PricerKey, Box<dyn Pricer>>,
}

impl PricerRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn register_pricer(&mut self, key: PricerKey, pricer: Box<dyn Pricer>) {
        self.pricers.insert(key, pricer);
    }
    
    pub fn get_pricer(&self, key: PricerKey) -> Option<&dyn Pricer> {
        self.pricers.get(&key).map(|p| p.as_ref())
    }
    
    pub fn price_with_registry(
        &self,
        instrument: &dyn Priceable,
        model: ModelKey,
        market: &Market,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let key = PricerKey::new(instrument.key(), model);
        if let Some(pricer) = self.get_pricer(key) {
            pricer.price_dyn(instrument, market)
        } else {
            Err(PricingError::UnknownPricer(key))
        }
    }
}

/// Create a standard pricer registry with all simplified pricers
pub fn create_standard_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    
    // Register simplified Bond pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::Bond, ModelKey::Discounting),
        Box::new(crate::instruments::bond::pricing::pricer::SimpleBondDiscountingPricer::bond())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Bond, ModelKey::Tree),
        Box::new(crate::instruments::bond::pricing::pricer::SimpleBondOasPricer::new())
    );
    
    // Register simplified Interest Rate pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::IRS, ModelKey::Discounting),
        Box::new(crate::instruments::irs::pricing::pricer::SimpleIrsDiscountingPricer::irs())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::FRA, ModelKey::Discounting),
        Box::new(crate::instruments::fra::pricer::SimpleFraDiscountingPricer::fra())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CapFloor, ModelKey::Black76),
        Box::new(crate::instruments::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Swaption, ModelKey::Black76),
        Box::new(crate::instruments::swaption::pricing::pricer::SimpleSwaptionBlackPricer::new())
    );
    
    // Register simplified Credit pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDS, ModelKey::HazardRate),
        Box::new(crate::instruments::common::GenericInstrumentPricer::cds())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSIndex, ModelKey::HazardRate),
        Box::new(crate::instruments::cds_index::pricer::SimpleCdsIndexHazardPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSOption, ModelKey::Black76),
        Box::new(crate::instruments::cds_option::pricer::SimpleCdsOptionBlackPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSTranche, ModelKey::HazardRate),
        Box::new(crate::instruments::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::new())
    );
    
    // Register simplified FX pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxSpot, ModelKey::Discounting),
        Box::new(crate::instruments::fx_spot::pricer::SimpleFxSpotDiscountingPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxOption, ModelKey::Black76),
        Box::new(crate::instruments::fx_option::SimpleFxOptionBlackPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxSwap, ModelKey::Discounting),
        Box::new(crate::instruments::fx_swap::pricer::SimpleFxSwapDiscountingPricer::new())
    );
    
    // Register simplified Equity pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::Equity, ModelKey::Discounting),
        Box::new(crate::instruments::equity::pricer::SimpleEquityDiscountingPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::EquityOption, ModelKey::Black76),
        Box::new(crate::instruments::equity_option::pricer::SimpleEquityOptionBlackPricer::new())
    );
    
    // Register simplified Basic pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting),
        Box::new(crate::instruments::deposit::pricer::SimpleDepositDiscountingPricer::deposit())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::InterestRateFuture, ModelKey::Discounting),
        Box::new(crate::instruments::ir_future::pricer::SimpleIrFutureDiscountingPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::BasisSwap, ModelKey::Discounting),
        Box::new(crate::instruments::basis_swap::SimpleBasisSwapDiscountingPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Repo, ModelKey::Discounting),
        Box::new(crate::instruments::repo::pricer::SimpleRepoDiscountingPricer::repo())
    );
    
    // Register simplified Inflation pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::InflationSwap, ModelKey::Discounting),
        Box::new(crate::instruments::inflation_swap::SimpleInflationSwapDiscountingPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::InflationLinkedBond, ModelKey::Discounting),
        Box::new(crate::instruments::inflation_linked_bond::pricer::SimpleInflationLinkedBondDiscountingPricer::new_discounting(InstrumentType::InflationLinkedBond))
    );
    
    // Register simplified Complex pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::VarianceSwap, ModelKey::Discounting),
        Box::new(crate::instruments::variance_swap::pricing::pricer::SimpleVarianceSwapDiscountingPricer::new())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Basket, ModelKey::Discounting),
        Box::new(crate::instruments::basket::SimpleBasketDiscountingPricer::basket())
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Convertible, ModelKey::Discounting),
        Box::new(crate::instruments::convertible::pricer::SimpleConvertibleDiscountingPricer::new())
    );
    
    // Register simplified TRS pricer (handles both Equity and FI Index variants)
    registry.register_pricer(
        PricerKey::new(InstrumentType::TRS, ModelKey::Discounting),
        Box::new(crate::instruments::trs::pricing::pricer::SimpleTrsDiscountingPricer::new())
    );
    
    registry
}

// ========================= TESTS =========================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_is_stable() {
        use core::mem::size_of;
        assert_eq!(size_of::<InstrumentType>(), 2);
        assert_eq!(size_of::<ModelKey>(), 2);
        assert_eq!(size_of::<PricerKey>(), 4);
    }

    #[test]
    fn registry_creation_test() {
        // Test that the standard registry can be created without errors
        let registry = create_standard_registry();
        
        // Test that we can retrieve a registered pricer
        let key = PricerKey::new(InstrumentType::Bond, ModelKey::Discounting);
        assert!(registry.get_pricer(key).is_some());
    }
}
