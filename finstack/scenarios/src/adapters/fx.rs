//! Foreign exchange shock adapter.
//!
//! Provides helper functions used by `OperationSpec::MarketFxPct` to apply
//! multiplicative shocks to FX matrices held inside the market context. The
//! logic intentionally focuses on the `SimpleFxProvider` implementation so the
//! operation remains deterministic and easy to audit.

use crate::error::{Error, Result};
use finstack_core::currency::Currency;
use finstack_core::market_data::MarketContext;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use std::sync::Arc;

/// Apply a percentage shock to an FX rate.
///
/// Positive percentages strengthen the base currency (increase the quoted rate).
/// The function expects the FX matrix inside the [`MarketContext`] to be powered
/// by a [`SimpleFxProvider`]; in that case it clones the matrix with the updated
/// rate and replaces the original.
///
/// # Arguments
/// - `market`: Market context whose FX matrix will be shocked.
/// - `base`: Base currency of the rate to shock.
/// - `quote`: Quote currency of the rate to shock.
/// - `pct`: Percentage change to apply (e.g., `5.0` means +5%).
///
/// # Returns
/// [`Result`](crate::error::Result) indicating success. The Ok variant is unit
/// typed because the changes are applied directly to `market`.
///
/// # Errors
/// - [`Error::MarketDataNotFound`](crate::error::Error::MarketDataNotFound) if
///   no FX matrix is present.
/// - [`Error::Core`](crate::error::Error::Core) if retrieving the existing rate
///   fails.
///
/// # Examples
/// ```rust,no_run
/// use finstack_scenarios::adapters::fx::apply_fx_shock;
/// use finstack_core::currency::Currency;
/// use finstack_core::market_data::MarketContext;
///
/// # fn main() -> finstack_scenarios::Result<()> {
/// let mut market = MarketContext::new();
/// // ... populate `market` with an FX matrix ...
/// apply_fx_shock(&mut market, Currency::USD, Currency::EUR, 2.5)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_fx_shock(
    market: &mut MarketContext,
    base: Currency,
    quote: Currency,
    pct: f64,
) -> Result<()> {
    // Get the current FX matrix
    let fx = market
        .fx
        .as_ref()
        .ok_or_else(|| Error::MarketDataNotFound {
            id: "FX matrix".to_string(),
        })?;

    // Try to get the current rate
    let current_rate = fx
        .rate(finstack_core::money::fx::FxQuery::new(
            base,
            quote,
            finstack_core::dates::Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
        ))
        .map_err(Error::Core)?
        .rate;

    // Calculate shocked rate
    let factor = 1.0 + (pct / 100.0);
    let shocked_rate = current_rate * factor;

    // Create a new SimpleFxProvider with the shocked rate
    let provider = Arc::new(SimpleFxProvider::new());
    provider.set_quote(base, quote, shocked_rate);

    // Create and insert new FxMatrix
    let new_matrix = FxMatrix::new(provider);
    market.insert_fx_mut(Arc::new(new_matrix));

    Ok(())
}
