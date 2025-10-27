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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u16)]
/// Instrument Type enumeration.
pub enum InstrumentType {
    /// Bond variant.
    Bond = 1,
    /// Loan variant.
    Loan = 2,
    /// C D S variant.
    CDS = 3,
    /// C D S Index variant.
    CDSIndex = 4,
    /// C D S Tranche variant.
    CDSTranche = 5,
    /// C D S Option variant.
    CDSOption = 6,
    /// I R S variant.
    IRS = 7,
    /// Cap Floor variant.
    CapFloor = 8,
    /// Swaption variant.
    Swaption = 9,
    /// T R S variant.
    TRS = 10,
    /// Basis Swap variant.
    BasisSwap = 11,
    /// Basket variant.
    Basket = 12,
    /// Convertible variant.
    Convertible = 13,
    /// Deposit variant.
    Deposit = 14,
    /// Equity Option variant.
    EquityOption = 15,
    /// Fx Option variant.
    FxOption = 16,
    /// Fx Spot variant.
    FxSpot = 17,
    /// Fx Swap variant.
    FxSwap = 18,
    /// Inflation Linked Bond variant.
    InflationLinkedBond = 19,
    /// Inflation Swap variant.
    InflationSwap = 20,
    /// Interest Rate Future variant.
    InterestRateFuture = 21,
    /// Variance Swap variant.
    VarianceSwap = 22,
    /// Equity variant.
    Equity = 23,
    /// Repo variant.
    Repo = 24,
    /// F R A variant.
    FRA = 25,
    /// Structured Credit variant.
    StructuredCredit = 26,
    /// Private Markets Fund variant.
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
            InstrumentType::StructuredCredit => "StructuredCredit",
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
            InstrumentType::StructuredCredit => "structured_credit",
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
            // Unified structured credit type (accepts legacy names for backward compatibility)
            "structured_credit" | "clo" | "abs" | "rmbs" | "cmbs" => {
                Ok(InstrumentType::StructuredCredit)
            }
            "private_markets_fund" | "pmf" => Ok(InstrumentType::PrivateMarketsFund),
            other => Err(format!("Unknown instrument type: {}", other)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
/// Model Key enumeration.
pub enum ModelKey {
    /// Discounting variant.
    Discounting = 1,
    /// Tree variant.
    Tree = 2,
    /// Black76 variant.
    Black76 = 3,
    /// Hull White1 F variant.
    HullWhite1F = 4,
    /// Hazard Rate variant.
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
/// Pricer Key structure.
pub struct PricerKey {
    /// instrument.
    pub instrument: InstrumentType,
    /// model.
    pub model: ModelKey,
}

impl PricerKey {
    /// fn constant.
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
/// Pricer Registry structure.
pub struct PricerRegistry {
    pricers: HashMap<PricerKey, Box<dyn Pricer>>,
}

impl PricerRegistry {
    /// new.
    pub fn new() -> Self {
        Self::default()
    }

    /// register pricer.
    pub fn register_pricer(&mut self, key: PricerKey, pricer: Box<dyn Pricer>) {
        self.pricers.insert(key, pricer);
    }

    /// get pricer.
    pub fn get_pricer(&self, key: PricerKey) -> Option<&dyn Pricer> {
        self.pricers.get(&key).map(|p| p.as_ref())
    }

    /// price with registry.
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

// ========================= REGISTRATION =========================

/// Register all standard pricers explicitly.
///
/// This function registers all instrument pricers in a single, visible location.
/// This explicit approach provides better IDE support, easier debugging, and
/// clearer dependency tracking compared to auto-registration.
fn register_all_pricers(registry: &mut PricerRegistry) {
    // Bond pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::Bond, ModelKey::Discounting),
        Box::new(crate::instruments::bond::pricing::pricer::SimpleBondDiscountingPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Bond, ModelKey::Tree),
        Box::new(crate::instruments::bond::pricing::pricer::SimpleBondOasPricer),
    );

    // Interest Rate Swaps
    registry.register_pricer(
        PricerKey::new(InstrumentType::IRS, ModelKey::Discounting),
        Box::new(crate::instruments::irs::pricer::SimpleIrsDiscountingPricer::default()),
    );

    // FRA
    registry.register_pricer(
        PricerKey::new(InstrumentType::FRA, ModelKey::Discounting),
        Box::new(crate::instruments::fra::pricer::SimpleFraDiscountingPricer::default()),
    );

    // Basis Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::BasisSwap, ModelKey::Discounting),
        Box::new(
            crate::instruments::basis_swap::pricer::SimpleBasisSwapDiscountingPricer::default(),
        ),
    );

    // Deposit
    registry.register_pricer(
        PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting),
        Box::new(crate::instruments::deposit::pricer::SimpleDepositDiscountingPricer::default()),
    );

    // Interest Rate Future
    registry.register_pricer(
        PricerKey::new(InstrumentType::InterestRateFuture, ModelKey::Discounting),
        Box::new(crate::instruments::ir_future::pricer::SimpleIrFutureDiscountingPricer::default()),
    );

    // Cap/Floor
    registry.register_pricer(
        PricerKey::new(InstrumentType::CapFloor, ModelKey::Black76),
        Box::new(
            crate::instruments::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::default(),
        ),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CapFloor, ModelKey::Discounting),
        Box::new(
            crate::instruments::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // Swaption
    registry.register_pricer(
        PricerKey::new(InstrumentType::Swaption, ModelKey::Black76),
        Box::new(crate::instruments::swaption::pricer::SimpleSwaptionBlackPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Swaption, ModelKey::Discounting),
        Box::new(
            crate::instruments::swaption::pricer::SimpleSwaptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // CDS
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDS, ModelKey::HazardRate),
        Box::new(crate::instruments::common::GenericInstrumentPricer::cds()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDS, ModelKey::Discounting),
        Box::new(crate::instruments::common::GenericInstrumentPricer::<
            crate::instruments::CreditDefaultSwap,
        >::new(InstrumentType::CDS, ModelKey::Discounting)),
    );

    // CDS Index
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSIndex, ModelKey::HazardRate),
        Box::new(crate::instruments::cds_index::pricer::SimpleCdsIndexHazardPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSIndex, ModelKey::Discounting),
        Box::new(
            crate::instruments::cds_index::pricer::SimpleCdsIndexHazardPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // CDS Tranche
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSTranche, ModelKey::HazardRate),
        Box::new(crate::instruments::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSTranche, ModelKey::Discounting),
        Box::new(
            crate::instruments::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // CDS Option
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSOption, ModelKey::Black76),
        Box::new(crate::instruments::cds_option::pricer::SimpleCdsOptionBlackPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSOption, ModelKey::Discounting),
        Box::new(
            crate::instruments::cds_option::pricer::SimpleCdsOptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // FX Spot
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxSpot, ModelKey::Discounting),
        Box::new(crate::instruments::fx_spot::pricer::SimpleFxSpotDiscountingPricer),
    );

    // FX Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxSwap, ModelKey::Discounting),
        Box::new(crate::instruments::fx_swap::pricer::SimpleFxSwapDiscountingPricer::default()),
    );

    // FX Option
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxOption, ModelKey::Black76),
        Box::new(crate::instruments::fx_option::pricer::SimpleFxOptionBlackPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxOption, ModelKey::Discounting),
        Box::new(
            crate::instruments::fx_option::pricer::SimpleFxOptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // Equity
    registry.register_pricer(
        PricerKey::new(InstrumentType::Equity, ModelKey::Discounting),
        Box::new(crate::instruments::equity::pricer::SimpleEquityDiscountingPricer),
    );

    // Equity Option
    registry.register_pricer(
        PricerKey::new(InstrumentType::EquityOption, ModelKey::Black76),
        Box::new(
            crate::instruments::equity_option::pricer::SimpleEquityOptionBlackPricer::default(),
        ),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::EquityOption, ModelKey::Discounting),
        Box::new(
            crate::instruments::equity_option::pricer::SimpleEquityOptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // TRS (Total Return Swap)
    registry.register_pricer(
        PricerKey::new(InstrumentType::TRS, ModelKey::Discounting),
        Box::new(crate::instruments::trs::pricing::pricer::SimpleTrsDiscountingPricer),
    );

    // Convertible Bond
    registry.register_pricer(
        PricerKey::new(InstrumentType::Convertible, ModelKey::Discounting),
        Box::new(crate::instruments::convertible::pricer::SimpleConvertibleDiscountingPricer),
    );

    // Private Markets Fund
    registry.register_pricer(
        PricerKey::new(InstrumentType::PrivateMarketsFund, ModelKey::Discounting),
        Box::new(
            crate::instruments::private_markets_fund::pricer::PrivateMarketsFundDiscountingPricer,
        ),
    );

    // Inflation Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::InflationSwap, ModelKey::Discounting),
        Box::new(crate::instruments::inflation_swap::pricer::SimpleInflationSwapDiscountingPricer::default()),
    );

    // Inflation Linked Bond
    registry.register_pricer(
        PricerKey::new(InstrumentType::InflationLinkedBond, ModelKey::Discounting),
        Box::new(crate::instruments::inflation_linked_bond::pricer::SimpleInflationLinkedBondDiscountingPricer::default()),
    );

    // Variance Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::VarianceSwap, ModelKey::Discounting),
        Box::new(
            crate::instruments::variance_swap::pricer::SimpleVarianceSwapDiscountingPricer::default(
            ),
        ),
    );

    // Repo
    registry.register_pricer(
        PricerKey::new(InstrumentType::Repo, ModelKey::Discounting),
        Box::new(crate::instruments::repo::pricer::SimpleRepoDiscountingPricer::default()),
    );

    // Basket
    registry.register_pricer(
        PricerKey::new(InstrumentType::Basket, ModelKey::Discounting),
        Box::new(crate::instruments::basket::SimpleBasketDiscountingPricer::default()),
    );

    // Structured Credit - unified pricer for ABS, CLO, CMBS, RMBS
    registry.register_pricer(
        PricerKey::new(InstrumentType::StructuredCredit, ModelKey::Discounting),
        Box::new(
            crate::instruments::structured_credit::StructuredCreditDiscountingPricer::default(),
        ),
    );
}

/// Create a standard pricer registry with all registered pricers.
///
/// This function creates a registry and explicitly registers all instrument pricers.
/// The explicit registration approach provides better visibility, IDE support, and
/// debugging capabilities compared to the previous auto-registration system.
///
/// All 40+ instrument pricers are registered in the `register_all_pricers` function.
pub fn create_standard_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_all_pricers(&mut registry);
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

        // Structured credit pricer (unified for ABS, CLO, CMBS, RMBS)
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::StructuredCredit,
                    ModelKey::Discounting
                ))
                .is_some(),
            "StructuredCredit Discounting pricer should be auto-registered"
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
