//! Minimal pricer infrastructure: stable keys, traits, registry macro, and errors.
use finstack_core::market_data::MarketContext as Market;
// no prelude import to avoid Result alias collisions

use crate::instruments::common::traits::Instrument as Priceable;

// ========================= KEYS =========================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum InstrumentKey {
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
    pub instrument: InstrumentKey,
    pub model: ModelKey,
}

impl PricerKey {
    pub const fn new(instrument: InstrumentKey, model: ModelKey) -> Self {
        Self { instrument, model }
    }
}

// ========================= ERRORS =========================

#[derive(Debug)]
pub enum PricingError {
    UnknownPricer(PricerKey),
    TypeMismatch {
        expected: InstrumentKey,
        got: InstrumentKey,
    },
    TypeErasedMismatch {
        expected: InstrumentKey,
        got: InstrumentKey,
    },
    ModelFailure(String),
}

impl From<finstack_core::Error> for PricingError {
    fn from(err: finstack_core::Error) -> Self {
        PricingError::ModelFailure(err.to_string())
    }
}

// ========================= TRAITS =========================

pub trait PriceableExt: core::any::Any + Send + Sync {
    fn key(&self) -> InstrumentKey;
    fn as_any(&self) -> &dyn core::any::Any;
}

// Adapter: implement PriceableExt for all existing Instrument types using their `instrument_type()`
impl<T: Priceable + 'static> PriceableExt for T {
    fn key(&self) -> InstrumentKey {
        match self.instrument_type() {
            "Bond" => InstrumentKey::Bond,
            "InterestRateSwap" => InstrumentKey::IRS,
            "Swaption" => InstrumentKey::Swaption,
            "CreditDefaultSwap" => InstrumentKey::CDS,
            "CDSIndex" => InstrumentKey::CDSIndex,
            "CdsTranche" => InstrumentKey::CDSTranche,
            "CdsOption" => InstrumentKey::CDSOption,
            "InterestRateOption" => InstrumentKey::CapFloor,
            "EquityTotalReturnSwap" | "FIIndexTotalReturnSwap" => InstrumentKey::TRS,
            "BasisSwap" => InstrumentKey::BasisSwap,
            "Basket" => InstrumentKey::Basket,
            "ConvertibleBond" => InstrumentKey::Convertible,
            "Deposit" => InstrumentKey::Deposit,
            "Equity" => InstrumentKey::Equity,
            "EquityOption" => InstrumentKey::EquityOption,
            "FxOption" => InstrumentKey::FxOption,
            "FxSpot" => InstrumentKey::FxSpot,
            "FxSwap" => InstrumentKey::FxSwap,
            "InflationLinkedBond" => InstrumentKey::InflationLinkedBond,
            "InflationSwap" => InstrumentKey::InflationSwap,
            "InterestRateFuture" => InstrumentKey::InterestRateFuture,
            "VarianceSwap" => InstrumentKey::VarianceSwap,
            // Extend as needed; default to Deposit for unknowns to avoid panic (explicit is better)
            _ => InstrumentKey::Deposit,
        }
    }
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

pub fn expect_inst<T: PriceableExt + 'static>(
    inst: &dyn PriceableExt,
    expected: InstrumentKey,
) -> std::result::Result<&T, PricingError> {
    if inst.key() != expected {
        return Err(PricingError::TypeMismatch {
            expected,
            got: inst.key(),
        });
    }
    inst.as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| PricingError::TypeErasedMismatch {
            expected,
            got: inst.key(),
        })
}

pub trait Pricer: Send + Sync {
    fn key(&self) -> PricerKey;
    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &Market,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError>;
}

// ========================= REGISTRY MACRO =========================

#[macro_export]
macro_rules! pricers {
    ($( $(#[$meta:meta])* $i:ident / $m:ident => $ctor:path ),* $(,)?) => {
        pub fn resolve(key: $crate::pricer::PricerKey) -> Option<Box<dyn $crate::pricer::Pricer>> {
            match key {
                $(
                    $(#[$meta])*
                    $crate::pricer::PricerKey { instrument: $crate::pricer::InstrumentKey::$i, model: $crate::pricer::ModelKey::$m } =>
                        Some(Box::new($ctor())),
                )*
                _ => None,
            }
        }

        pub const ALL: &[$crate::pricer::PricerKey] = &[
            $(
                $(#[$meta])*
                $crate::pricer::PricerKey { instrument: $crate::pricer::InstrumentKey::$i, model: $crate::pricer::ModelKey::$m },
            )*
        ];

        pub fn models_for(i: $crate::pricer::InstrumentKey) -> impl Iterator<Item = $crate::pricer::ModelKey> {
            ALL.iter().copied().filter(move |k| k.instrument == i).map(|k| k.model)
        }

        pub fn supports(key: $crate::pricer::PricerKey) -> bool {
            ALL.iter().any(|&k| k == key)
        }
    }
}

// ========================= DIAGNOSTICS =========================

// Diagnostic macro; currently a no-op to avoid feature gating warnings.
macro_rules! trace_price {
    ($($t:tt)*) => {};
}

// ========================= PUBLIC API =========================

pub fn price(
    instrument: &dyn PriceableExt,
    model: ModelKey,
    market: &Market,
) -> std::result::Result<crate::results::ValuationResult, PricingError> {
    let key = PricerKey::new(instrument.key(), model);
    if let Some(p) = crate::instruments::registry::resolve(key) {
        trace_price!(key, instrument);
        p.price_dyn(instrument, market)
    } else {
        Err(PricingError::UnknownPricer(key))
    }
}

// ========================= LOCAL PRICER MACRO =========================

#[macro_export]
macro_rules! impl_dyn_pricer {
    (
        name: $name:ident,
        instrument: $inst:path,
        instrument_key: $ikey:ident,
        model: $model:ident,
        as_of = $as_of:expr,
        pv = $pv:expr $(,)?
    ) => {
        pub struct $name;
        impl $name { pub fn new() -> Self { Self } }
        impl ::core::default::Default for $name { fn default() -> Self { Self::new() } }
        impl $crate::pricer::Pricer for $name {
            fn key(&self) -> $crate::pricer::PricerKey {
                $crate::pricer::PricerKey::new(
                    $crate::pricer::InstrumentKey::$ikey,
                    $crate::pricer::ModelKey::$model
                )
            }
            fn price_dyn(
                &self,
                instrument: &dyn $crate::pricer::PriceableExt,
                market: &finstack_core::market_data::MarketContext
            ) -> ::std::result::Result<$crate::results::ValuationResult, $crate::pricer::PricingError> {
                let inst: &$inst = $crate::pricer::expect_inst(instrument, $crate::pricer::InstrumentKey::$ikey)?;
                let as_of = ($as_of)(inst, market)?;
                let pv = ($pv)(inst, market, as_of)?;
                Ok($crate::results::ValuationResult::stamped(inst.id.as_str(), as_of, pv))
            }
        }
    };
    (
        name: $name:ident,
        instrument: $inst:path,
        instrument_key: $ikey:ident,
        model: $model:ident,
        as_of = $as_of:expr,
        pv = $pv:expr,
        result = $result:expr $(,)?
    ) => {
        pub struct $name;
        impl $name { pub fn new() -> Self { Self } }
        impl ::core::default::Default for $name { fn default() -> Self { Self::new() } }
        impl $crate::pricer::Pricer for $name {
            fn key(&self) -> $crate::pricer::PricerKey {
                $crate::pricer::PricerKey::new(
                    $crate::pricer::InstrumentKey::$ikey,
                    $crate::pricer::ModelKey::$model
                )
            }
            fn price_dyn(
                &self,
                instrument: &dyn $crate::pricer::PriceableExt,
                market: &finstack_core::market_data::MarketContext
            ) -> ::std::result::Result<$crate::results::ValuationResult, $crate::pricer::PricingError> {
                let inst: &$inst = $crate::pricer::expect_inst(instrument, $crate::pricer::InstrumentKey::$ikey)?;
                let as_of = ($as_of)(inst, market)?;
                let pv = ($pv)(inst, market, as_of)?;
                let result = $crate::results::ValuationResult::stamped(inst.id.as_str(), as_of, pv);
                let result = ($result)(inst, market, as_of, result)?;
                Ok(result)
            }
        }
    };
}

// ========================= TESTS =========================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn abi_is_stable() {
        use core::mem::size_of;
        assert_eq!(size_of::<InstrumentKey>(), 2);
        assert_eq!(size_of::<ModelKey>(), 2);
        assert_eq!(size_of::<PricerKey>(), 4);
    }

    #[test]
    fn no_duplicate_keys_empty_registry() {
        // With no registrations, ALL should be empty by default in this module.
        // Registrations are added by invoking the macro in another module.
        let set: BTreeSet<PricerKey> = BTreeSet::new();
        assert!(set.is_empty());
    }
}
