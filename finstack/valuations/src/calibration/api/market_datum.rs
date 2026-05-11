//! Flat-list market-data inputs for `CalibrationEnvelope` v3.
//!
//! `MarketDatum` is the single id-addressable datum type consumed by the v3
//! calibration envelope. Each variant wraps an existing market-data primitive
//! (quote, scalar, surface, etc.) and is serialized with a `kind` tag so the
//! envelope can carry a heterogeneous list in JSON/YAML.

use crate::market::quotes::bond::BondQuote;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::cds_tranche::CDSTrancheQuote;
use crate::market::quotes::fx::FxQuote;
use crate::market::quotes::inflation::InflationQuote;
use crate::market::quotes::rates::RateQuote;
use crate::market::quotes::vol::VolQuote;
use crate::market::quotes::xccy::XccyQuote;
use finstack_core::currency::Currency;
use finstack_core::market_data::context::CreditIndexState;
use finstack_core::market_data::dividends::DividendSchedule;
use finstack_core::market_data::scalars::{InflationIndex, MarketScalar, ScalarTimeSeries};
use finstack_core::market_data::surfaces::{FxDeltaVolSurface, VolCube};
use finstack_core::money::fx::FxRate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single id-addressable input to the calibrator.
///
/// Each variant is tagged via serde as `{"kind": "<snake_case_variant>", ...}`
/// so callers can author flat heterogeneous lists in JSON/YAML.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketDatum {
    /// Interest-rate quote (deposit, FRA, future, swap, ...).
    RateQuote(RateQuote),
    /// Single-name credit-default-swap quote.
    CdsQuote(CdsQuote),
    /// CDS index tranche quote.
    CdsTrancheQuote(CDSTrancheQuote),
    /// FX quote (forward, swap, ...).
    FxQuote(FxQuote),
    /// Inflation quote (zero-coupon swap, year-on-year, ...).
    InflationQuote(InflationQuote),
    /// Volatility quote (cap/floor, swaption, ...).
    VolQuote(VolQuote),
    /// Cross-currency basis-swap quote.
    XccyQuote(XccyQuote),
    /// Bond quote (price or yield).
    BondQuote(BondQuote),
    /// FX spot quote.
    FxSpot(FxSpotDatum),
    /// Spot price for a single asset.
    Price(PriceDatum),
    /// Dividend schedule for an underlier.
    DividendSchedule(DividendScheduleDatum),
    /// Generic scalar time series (CPI, historical fixings, ...).
    FixingSeries(
        #[schemars(with = "serde_json::Value")] //
        ScalarTimeSeries,
    ),
    /// Inflation index fixings.
    InflationFixings(InflationIndex),
    /// Credit-index reference state.
    CreditIndex(
        #[schemars(with = "serde_json::Value")] //
        CreditIndexState,
    ),
    /// FX delta-vol surface.
    FxVolSurface(
        #[schemars(with = "serde_json::Value")] //
        FxDeltaVolSurface,
    ),
    /// Generic vol cube.
    VolCube(
        #[schemars(with = "serde_json::Value")] //
        VolCube,
    ),
    /// Collateral / CSA mapping entry.
    Collateral(CollateralEntry),
}

/// FX-spot quote payload for [`MarketDatum::FxSpot`].
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FxSpotDatum {
    /// Stable identifier for this datum.
    pub id: String,
    /// Quote currency (numerator of `to / from`).
    pub from: Currency,
    /// Base currency (denominator of `to / from`).
    pub to: Currency,
    /// Spot rate value (1 `from` = `rate` `to`).
    pub rate: FxRate,
}

/// Single-name spot-price payload for [`MarketDatum::Price`].
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PriceDatum {
    /// Stable identifier (e.g., asset ticker).
    pub id: String,
    /// Scalar value (unitless or monetary).
    #[schemars(with = "serde_json::Value")]
    pub scalar: MarketScalar,
}

/// Dividend-schedule payload for [`MarketDatum::DividendSchedule`].
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DividendScheduleDatum {
    /// The dividend schedule itself.
    #[schemars(with = "serde_json::Value")]
    pub schedule: DividendSchedule,
}

/// Collateral / CSA mapping payload for [`MarketDatum::Collateral`].
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CollateralEntry {
    /// Trade-leg currency this CSA mapping applies to.
    pub id: Currency,
    /// Collateral / CSA currency.
    pub csa_currency: Currency,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_price_datum() {
        let datum = MarketDatum::Price(PriceDatum {
            id: "AAPL".into(),
            scalar: MarketScalar::Unitless(175.42),
        });
        let json = serde_json::to_string(&datum).unwrap();
        assert!(json.contains(r#""kind":"price""#));
        let back: MarketDatum = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, MarketDatum::Price(_)));
    }
}
