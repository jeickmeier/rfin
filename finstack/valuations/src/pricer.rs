//! Pricer infrastructure: type-safe pricing dispatch via registry pattern.
//!
//! This module provides a registry-based pricing system that maps
//! (instrument type, model) pairs to specific pricer implementations.
//! The system uses enum-based dispatch for type safety rather than string
//! comparisons.

use crate::instruments::common::traits::Instrument as Priceable;
use finstack_core::market_data::MarketContext as Market;

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
    CLO = 26,
    ABS = 27,
    RMBS = 28,
    CMBS = 29,
    PrivateMarketsFund = 30,
}

impl InstrumentType {
    /// Returns the canonical string representation for metrics registry lookups.
    ///
    /// This format matches the instrument type tags used in the metrics registry
    /// and is TitleCase (e.g., "Bond", "InterestRateSwap").
    pub fn as_str(&self) -> &'static str {
        match self {
            InstrumentType::Bond => "Bond",
            InstrumentType::Loan => "Loan",
            InstrumentType::CDS => "CDS",
            InstrumentType::CDSIndex => "CDSIndex",
            InstrumentType::CDSTranche => "CDSTranche",
            InstrumentType::CDSOption => "CdsOption",
            InstrumentType::IRS => "InterestRateSwap",
            InstrumentType::CapFloor => "InterestRateOption",
            InstrumentType::Swaption => "Swaption",
            InstrumentType::TRS => "TRS",
            InstrumentType::BasisSwap => "BasisSwap",
            InstrumentType::Basket => "Basket",
            InstrumentType::Convertible => "ConvertibleBond",
            InstrumentType::Deposit => "Deposit",
            InstrumentType::EquityOption => "EquityOption",
            InstrumentType::FxOption => "FxOption",
            InstrumentType::FxSpot => "FxSpot",
            InstrumentType::FxSwap => "FxSwap",
            InstrumentType::InflationLinkedBond => "InflationLinkedBond",
            InstrumentType::InflationSwap => "InflationSwap",
            InstrumentType::InterestRateFuture => "InterestRateFuture",
            InstrumentType::VarianceSwap => "VarianceSwap",
            InstrumentType::Equity => "Equity",
            InstrumentType::Repo => "Repo",
            InstrumentType::FRA => "FRA",
            InstrumentType::CLO => "CLO",
            InstrumentType::ABS => "ABS",
            InstrumentType::RMBS => "RMBS",
            InstrumentType::CMBS => "CMBS",
            InstrumentType::PrivateMarketsFund => "PrivateMarketsFund",
        }
    }
}

impl std::fmt::Display for InstrumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            InstrumentType::Bond => "bond",
            InstrumentType::Loan => "loan",
            InstrumentType::CDS => "cds",
            InstrumentType::CDSIndex => "cds_index",
            InstrumentType::CDSTranche => "cds_tranche",
            InstrumentType::CDSOption => "cds_option",
            InstrumentType::IRS => "irs",
            InstrumentType::CapFloor => "cap_floor",
            InstrumentType::Swaption => "swaption",
            InstrumentType::TRS => "trs",
            InstrumentType::BasisSwap => "basis_swap",
            InstrumentType::Basket => "basket",
            InstrumentType::Convertible => "convertible",
            InstrumentType::Deposit => "deposit",
            InstrumentType::EquityOption => "equity_option",
            InstrumentType::FxOption => "fx_option",
            InstrumentType::FxSpot => "fx_spot",
            InstrumentType::FxSwap => "fx_swap",
            InstrumentType::InflationLinkedBond => "inflation_linked_bond",
            InstrumentType::InflationSwap => "inflation_swap",
            InstrumentType::InterestRateFuture => "interest_rate_future",
            InstrumentType::VarianceSwap => "variance_swap",
            InstrumentType::Equity => "equity",
            InstrumentType::Repo => "repo",
            InstrumentType::FRA => "fra",
            InstrumentType::CLO => "clo",
            InstrumentType::ABS => "abs",
            InstrumentType::RMBS => "rmbs",
            InstrumentType::CMBS => "cmbs",
            InstrumentType::PrivateMarketsFund => "private_markets_fund",
        };
        write!(f, "{}", label)
    }
}

impl std::str::FromStr for InstrumentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "bond" => Ok(InstrumentType::Bond),
            "loan" => Ok(InstrumentType::Loan),
            "cds" => Ok(InstrumentType::CDS),
            "cds_index" | "cdsindex" => Ok(InstrumentType::CDSIndex),
            "cds_tranche" | "cdstranche" => Ok(InstrumentType::CDSTranche),
            "cds_option" | "cdsoption" => Ok(InstrumentType::CDSOption),
            "irs" | "swap" | "interest_rate_swap" => Ok(InstrumentType::IRS),
            "cap_floor" | "capfloor" | "interest_rate_option" => Ok(InstrumentType::CapFloor),
            "swaption" => Ok(InstrumentType::Swaption),
            "trs" | "total_return_swap" => Ok(InstrumentType::TRS),
            "basis_swap" | "basisswap" => Ok(InstrumentType::BasisSwap),
            "basket" => Ok(InstrumentType::Basket),
            "convertible" | "convertible_bond" => Ok(InstrumentType::Convertible),
            "deposit" => Ok(InstrumentType::Deposit),
            "equity_option" | "equityoption" => Ok(InstrumentType::EquityOption),
            "fx_option" | "fxoption" => Ok(InstrumentType::FxOption),
            "fx_spot" | "fxspot" => Ok(InstrumentType::FxSpot),
            "fx_swap" | "fxswap" => Ok(InstrumentType::FxSwap),
            "inflation_linked_bond" | "ilb" => Ok(InstrumentType::InflationLinkedBond),
            "inflation_swap" => Ok(InstrumentType::InflationSwap),
            "interest_rate_future" | "ir_future" | "irfuture" => {
                Ok(InstrumentType::InterestRateFuture)
            }
            "variance_swap" | "varianceswap" => Ok(InstrumentType::VarianceSwap),
            "equity" => Ok(InstrumentType::Equity),
            "repo" => Ok(InstrumentType::Repo),
            "fra" => Ok(InstrumentType::FRA),
            "clo" => Ok(InstrumentType::CLO),
            "abs" => Ok(InstrumentType::ABS),
            "rmbs" => Ok(InstrumentType::RMBS),
            "cmbs" => Ok(InstrumentType::CMBS),
            "private_markets_fund" | "pmf" => Ok(InstrumentType::PrivateMarketsFund),
            other => Err(format!("Unknown instrument type: {}", other)),
        }
    }
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

impl std::fmt::Display for ModelKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ModelKey::Discounting => "discounting",
            ModelKey::Tree => "tree",
            ModelKey::Black76 => "black76",
            ModelKey::HullWhite1F => "hull_white_1f",
            ModelKey::HazardRate => "hazard_rate",
        };
        write!(f, "{}", label)
    }
}

impl std::str::FromStr for ModelKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "discounting" => Ok(ModelKey::Discounting),
            "tree" | "lattice" => Ok(ModelKey::Tree),
            "black76" | "black" | "black_76" => Ok(ModelKey::Black76),
            "hull_white_1f" | "hullwhite1f" | "hw1f" => Ok(ModelKey::HullWhite1F),
            "hazard_rate" | "hazard" => Ok(ModelKey::HazardRate),
            other => Err(format!("Unknown model key: {}", other)),
        }
    }
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

/// Pricing-specific errors returned by pricer implementations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PricingError {
    /// No pricer registered for the requested (instrument, model) combination.
    #[error("No pricer found for instrument={} model={}", .0.instrument, .0.model)]
    UnknownPricer(PricerKey),
    
    /// Instrument type mismatch during downcasting.
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch {
        /// Expected instrument type
        expected: InstrumentType,
        /// Actual instrument type
        got: InstrumentType,
    },
    
    /// Pricing model computation failed.
    #[error("Model failure: {0}")]
    ModelFailure(String),
}

impl From<finstack_core::Error> for PricingError {
    fn from(err: finstack_core::Error) -> Self {
        PricingError::ModelFailure(err.to_string())
    }
}

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

// ========================= AUTO-REGISTRATION =========================

/// Pricer registration for auto-discovery via inventory crate.
///
/// Each pricer implementation annotated with `#[register_pricer]` will
/// automatically submit a registration that can be collected at runtime.
pub struct PricerRegistration {
    pub ctor: fn() -> Box<dyn Pricer>,
}

inventory::collect!(PricerRegistration);

/// Create a standard pricer registry with all auto-registered pricers.
///
/// This function collects all pricers that were marked with `#[register_pricer]`
/// or manually registered via `inventory::submit!` and automatically registers
/// them based on their `key()` implementation.
///
/// All 40+ instrument pricers are now self-registering at compile time.
pub fn create_standard_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();

    // Collect all auto-registered pricers
    for registration in inventory::iter::<PricerRegistration> {
        let pricer = (registration.ctor)();
        let key = pricer.key();
        registry.register_pricer(key, pricer);
    }

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

    #[test]
    fn auto_registration_test() {
        // Test that ALL pricers are auto-registered correctly
        let registry = create_standard_registry();

        // Bond pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::Discounting))
                .is_some(),
            "Bond Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::Tree))
                .is_some(),
            "Bond OAS pricer should be auto-registered"
        );

        // Interest Rate pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::IRS, ModelKey::Discounting))
                .is_some(),
            "IRS Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::FRA, ModelKey::Discounting))
                .is_some(),
            "FRA Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CapFloor, ModelKey::Black76))
                .is_some(),
            "CapFloor Black76 pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CapFloor,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CapFloor Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Swaption, ModelKey::Black76))
                .is_some(),
            "Swaption Black76 pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Swaption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Swaption Discounting pricer should be auto-registered"
        );

        // Credit pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDS, ModelKey::HazardRate))
                .is_some(),
            "CDS HazardRate pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDS, ModelKey::Discounting))
                .is_some(),
            "CDS Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSIndex,
                    ModelKey::HazardRate
                ))
                .is_some(),
            "CDSIndex HazardRate pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSIndex,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSIndex Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDSOption, ModelKey::Black76))
                .is_some(),
            "CDSOption Black76 pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSOption Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSTranche,
                    ModelKey::HazardRate
                ))
                .is_some(),
            "CDSTranche HazardRate pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSTranche,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSTranche Discounting pricer should be auto-registered"
        );

        // FX pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxSpot,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxSpot Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::FxOption, ModelKey::Black76))
                .is_some(),
            "FxOption Black76 pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxOption Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxSwap Discounting pricer should be auto-registered"
        );

        // Equity pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Equity,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Equity Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityOption,
                    ModelKey::Black76
                ))
                .is_some(),
            "EquityOption Black76 pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "EquityOption Discounting pricer should be auto-registered"
        );

        // Basic pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Deposit,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Deposit Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InterestRateFuture,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InterestRateFuture Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::BasisSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "BasisSwap Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Repo, ModelKey::Discounting))
                .is_some(),
            "Repo Discounting pricer should be auto-registered"
        );

        // Inflation pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InflationSwap Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationLinkedBond,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InflationLinkedBond Discounting pricer should be auto-registered"
        );

        // Complex pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::VarianceSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "VarianceSwap Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Basket,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Basket Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Convertible,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Convertible Discounting pricer should be auto-registered"
        );

        // Structured credit pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::ABS, ModelKey::Discounting))
                .is_some(),
            "ABS Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CLO, ModelKey::Discounting))
                .is_some(),
            "CLO Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CMBS, ModelKey::Discounting))
                .is_some(),
            "CMBS Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::RMBS, ModelKey::Discounting))
                .is_some(),
            "RMBS Discounting pricer should be auto-registered"
        );

        // TRS and Private Markets
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::TRS, ModelKey::Discounting))
                .is_some(),
            "TRS Discounting pricer should be auto-registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::PrivateMarketsFund,
                    ModelKey::Discounting
                ))
                .is_some(),
            "PrivateMarketsFund Discounting pricer should be auto-registered"
        );
    }
}
