//! FX Spot pricer engine.
//!
//! Provides deterministic PV for `FxSpot` instruments. The PV is the base
//! notional converted to the quote currency at the applicable rate:
//! - If an explicit `spot_rate` is set on the instrument, that is used directly
//!   to compute `quote_amount = base_notional.amount() * spot_rate`.
//! - Otherwise, the rate is obtained from the `MarketContext`'s `FxMatrix`
//!   using the `FxProvider` interface from `finstack_core`.
//!
//! All arithmetic is done using the core `Money` type to preserve currency
//! safety and respect rounding policies configured in the core library.

use crate::instruments::fx_spot::FxSpot;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::{FxConversionPolicy, FxProvider};
use finstack_core::money::Money;
use finstack_core::Result;

/// Stateless pricing engine for `FxSpot` instruments.
#[derive(Debug, Default, Clone, Copy)]
pub struct FxSpotPricer;

impl FxSpotPricer {
    /// Compute present value in the instrument's quote currency.
    ///
    /// # Parameters
    /// - `inst`: reference to the `FxSpot` instrument
    /// - `curves`: market context supplying the FX matrix when `spot_rate` is not set
    /// - `as_of`: valuation date used for FX lookup
    pub fn pv(&self, inst: &FxSpot, curves: &MarketContext, as_of: Date) -> Result<Money> {
        if let Some(rate) = inst.spot_rate {
            let quote_amount = inst.effective_notional().amount() * rate;
            return Ok(Money::new(quote_amount, inst.quote));
        }

        let matrix = curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        struct MatrixProvider<'a> {
            m: &'a finstack_core::money::fx::FxMatrix,
        }

        impl FxProvider for MatrixProvider<'_> {
            fn rate(
                &self,
                from: Currency,
                to: Currency,
                on: Date,
                policy: finstack_core::money::fx::FxConversionPolicy,
            ) -> finstack_core::Result<finstack_core::money::fx::FxRate> {
                let result = self.m.rate(finstack_core::money::fx::FxQuery::with_policy(
                    from, to, on, policy,
                ))?;
                Ok(result.rate)
            }
        }

        let provider = MatrixProvider { m: matrix };
        let policy = FxConversionPolicy::CashflowDate;
        inst.effective_notional()
            .convert(inst.quote, as_of, &provider, policy)
    }
}
