//! IR Future pricing facade and engine re-export.
//!
//! Exposes the pricing entrypoints for `InterestRateFuture`. Core pricing
//! logic lives in `engine`. Instruments and metrics should depend on this
//! module rather than private files to keep the public API stable.

pub mod engine;

// local to this pricing module; not #[macro_export]
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
    }
}

pub mod pricer;

pub use engine::IrFutureEngine;
