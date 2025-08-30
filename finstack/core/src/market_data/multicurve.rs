//! Deprecated multi-curve container. This module now aliases `CurveSet` to the
//! unified `MarketContext` type so downstream code keeps working unchanged.
//! New code should use `market_data::context::MarketContext` directly.

#![allow(dead_code)]

use crate::currency::Currency;
use crate::dates::Date;
use crate::money::fx::{FxConversionPolicy, FxProvider, FxRate};

/// Minimal FX provider used by the `CurveSet` alias. It returns identity for
/// same-currency requests and fails otherwise.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFx;

impl FxProvider for NoFx {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> crate::Result<FxRate> {
        if from == to {
            #[cfg(feature = "decimal128")]
            { return Ok(rust_decimal::Decimal::ONE); }
            #[cfg(not(feature = "decimal128"))]
            { return Ok(1.0); }
        }
        Err(crate::error::InputError::NotFound.into())
    }
}

/// Backwards-compatible alias. Prefer `MarketContext` moving forward.
pub type CurveSet = crate::market_data::context::MarketContext<NoFx>;

