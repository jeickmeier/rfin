//! JSON import/export for financial instruments.
//!
//! This module provides a tagged union for all instrument types and helpers
//! for loading instruments from JSON files with strict validation.

use super::*;
use finstack_core::Result;
use serde::{
    de::{Deserializer, Error as DeError},
    Deserialize, Serialize,
};
use std::io::Read;

/// Versioned envelope for JSON instrument definitions.
///
/// This wrapper allows for future schema evolution while maintaining
/// compatibility with existing JSON files.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
// Note: JsonSchema derive requires finstack-core types to implement JsonSchema
// #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InstrumentEnvelope {
    /// Schema version (e.g., "finstack.instrument/1")
    pub schema: String,
    /// The instrument definition
    pub instrument: InstrumentJson,
}

/// Tagged union of all instrument types.
///
/// This enum enables JSON deserialization of any supported instrument type
/// via a type discriminator field. All instruments can be losslessly
/// deserialized from JSON without additional programmatic parameters.
///
/// # JSON Format
///
/// ```json
/// {
///   "type": "bond",
///   "spec": {
///     "id": "BOND-001",
///     "notional": { "amount": 1000000.0, "ccy": "USD" },
///     // ... other Bond fields
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
// Note: JsonSchema derive requires finstack-core types to implement JsonSchema
// #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", content = "spec", rename_all = "snake_case")]
pub enum InstrumentJson {
    // Fixed Income
    /// Fixed or floating rate bond
    Bond(Bond),
    /// Convertible bond (hybrid debt-equity)
    ConvertibleBond(ConvertibleBond),
    /// Inflation-linked bond
    InflationLinkedBond(InflationLinkedBond),
    /// Term loan
    TermLoan(TermLoan),
    /// Revolving credit facility
    RevolvingCredit(RevolvingCredit),
    /// Bond future
    BondFuture(BondFuture),
    /// Agency MBS passthrough
    AgencyMbsPassthrough(AgencyMbsPassthrough),
    /// Agency TBA forward
    AgencyTba(AgencyTba),
    /// Agency CMO tranche
    AgencyCmo(AgencyCmo),
    /// Dollar roll
    DollarRoll(DollarRoll),

    // Rates
    /// Interest rate swap
    InterestRateSwap(InterestRateSwap),
    /// Basis swap
    BasisSwap(BasisSwap),
    /// Cross-currency swap
    XccySwap(XccySwap),
    /// Inflation swap
    InflationSwap(InflationSwap),
    /// Year-on-year inflation swap
    YoYInflationSwap(YoYInflationSwap),
    /// Inflation cap/floor
    InflationCapFloor(InflationCapFloor),
    /// Forward rate agreement (FRA)
    ForwardRateAgreement(ForwardRateAgreement),
    /// Swaption (option on swap)
    Swaption(Swaption),
    /// Interest rate future
    InterestRateFuture(InterestRateFuture),
    /// Interest rate option (cap/floor)
    InterestRateOption(InterestRateOption),
    /// Constant maturity swap (CMS) option
    CmsOption(CmsOption),
    /// Money market deposit
    Deposit(Deposit),
    /// Repurchase agreement
    Repo(Repo),

    // Credit
    /// Credit default swap (single-name CDS)
    CreditDefaultSwap(CreditDefaultSwap),
    /// CDS index (CDX, iTraxx)
    #[serde(rename = "cds_index")]
    CDSIndex(CDSIndex),
    /// CDS tranche (synthetic CDO)
    CdsTranche(CdsTranche),
    /// CDS option
    CdsOption(CdsOption),

    // Equity
    /// Equity spot position
    Equity(Equity),
    /// Vanilla equity option
    EquityOption(EquityOption),
    /// Asian option (path-dependent average)
    AsianOption(AsianOption),
    /// Barrier option (knock-in/out)
    BarrierOption(BarrierOption),
    /// Lookback option
    LookbackOption(LookbackOption),
    /// Variance swap
    VarianceSwap(VarianceSwap),
    /// Equity index future
    EquityIndexFuture(EquityIndexFuture),
    /// Volatility index future
    VolatilityIndexFuture(VolatilityIndexFuture),
    /// Volatility index option
    VolatilityIndexOption(VolatilityIndexOption),

    // FX
    /// FX spot position
    FxSpot(FxSpot),
    /// FX swap (forward)
    FxSwap(FxSwap),
    /// FX forward (outright)
    FxForward(FxForward),
    /// Non-deliverable forward
    Ndf(Ndf),
    /// Vanilla FX option
    FxOption(FxOption),
    /// FX barrier option
    FxBarrierOption(FxBarrierOption),
    /// FX variance swap
    FxVarianceSwap(FxVarianceSwap),
    /// Quanto option (cross-currency)
    QuantoOption(QuantoOption),

    // Commodity
    /// Commodity option
    CommodityOption(CommodityOption),
    /// Commodity forward
    CommodityForward(CommodityForward),
    /// Commodity swap
    CommoditySwap(CommoditySwap),

    // Exotic Options
    /// Autocallable note
    Autocallable(Autocallable),
    /// Cliquet/ratchet option
    CliquetOption(CliquetOption),
    /// Range accrual note
    RangeAccrual(RangeAccrual),

    // Total Return Swaps
    /// Equity total return swap
    TrsEquity(EquityTotalReturnSwap),
    /// Fixed income index total return swap
    TrsFixedIncomeIndex(FIIndexTotalReturnSwap),

    // Structured Credit
    /// Structured credit (ABS, RMBS, CMBS, CLO)
    StructuredCredit(Box<StructuredCredit>),

    // Other
    /// Multi-asset basket
    Basket(Basket),
    /// Private markets fund
    PrivateMarketsFund(PrivateMarketsFund),
    /// Real estate asset
    RealEstateAsset(RealEstateAsset),
}

impl InstrumentJson {
    /// Convert this JSON representation into a boxed instrument trait object.
    ///
    /// For instruments using a Spec pattern (e.g., TermLoan), this performs
    /// the spec-to-runtime conversion. For direct instrument types, it boxes
    /// them immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if spec validation fails during conversion.
    pub fn into_boxed(self) -> Result<Box<dyn Instrument>> {
        match self {
            // Fixed Income
            InstrumentJson::Bond(i) => Ok(Box::new(i)),
            InstrumentJson::ConvertibleBond(i) => Ok(Box::new(i)),
            InstrumentJson::InflationLinkedBond(i) => Ok(Box::new(i)),
            InstrumentJson::TermLoan(i) => Ok(Box::new(i)),
            InstrumentJson::BondFuture(i) => Ok(Box::new(i)),
            InstrumentJson::AgencyMbsPassthrough(i) => Ok(Box::new(i)),
            InstrumentJson::AgencyTba(i) => Ok(Box::new(i)),
            InstrumentJson::AgencyCmo(i) => Ok(Box::new(i)),
            InstrumentJson::DollarRoll(i) => Ok(Box::new(i)),

            // Swaps
            InstrumentJson::InterestRateSwap(i) => Ok(Box::new(i)),
            InstrumentJson::BasisSwap(i) => Ok(Box::new(i)),
            InstrumentJson::XccySwap(i) => Ok(Box::new(i)),
            InstrumentJson::InflationSwap(i) => Ok(Box::new(i)),
            InstrumentJson::YoYInflationSwap(i) => Ok(Box::new(i)),
            InstrumentJson::InflationCapFloor(i) => Ok(Box::new(i)),
            InstrumentJson::FxSwap(i) => Ok(Box::new(i)),
            InstrumentJson::VarianceSwap(i) => Ok(Box::new(i)),

            // Rates Derivatives
            InstrumentJson::ForwardRateAgreement(i) => Ok(Box::new(i)),
            InstrumentJson::Swaption(i) => Ok(Box::new(i)),
            InstrumentJson::InterestRateFuture(i) => Ok(Box::new(i)),
            InstrumentJson::InterestRateOption(i) => Ok(Box::new(i)),
            InstrumentJson::CmsOption(i) => Ok(Box::new(i)),

            // Credit
            InstrumentJson::CreditDefaultSwap(i) => Ok(Box::new(i)),
            InstrumentJson::CDSIndex(i) => Ok(Box::new(i)),
            InstrumentJson::CdsTranche(i) => Ok(Box::new(i)),
            InstrumentJson::CdsOption(i) => Ok(Box::new(i)),

            // Equity
            InstrumentJson::Equity(i) => Ok(Box::new(i)),
            InstrumentJson::EquityOption(i) => Ok(Box::new(i)),
            InstrumentJson::AsianOption(i) => Ok(Box::new(i)),
            InstrumentJson::BarrierOption(i) => Ok(Box::new(i)),
            InstrumentJson::LookbackOption(i) => Ok(Box::new(i)),
            InstrumentJson::EquityIndexFuture(i) => Ok(Box::new(i)),
            InstrumentJson::VolatilityIndexFuture(i) => Ok(Box::new(i)),
            InstrumentJson::VolatilityIndexOption(i) => Ok(Box::new(i)),

            // FX
            InstrumentJson::FxSpot(i) => Ok(Box::new(i)),
            InstrumentJson::FxForward(i) => Ok(Box::new(i)),
            InstrumentJson::Ndf(i) => Ok(Box::new(i)),
            InstrumentJson::FxOption(i) => Ok(Box::new(i)),
            InstrumentJson::FxBarrierOption(i) => Ok(Box::new(i)),
            InstrumentJson::FxVarianceSwap(i) => Ok(Box::new(i)),
            InstrumentJson::QuantoOption(i) => Ok(Box::new(i)),

            // Commodity
            InstrumentJson::CommodityOption(i) => Ok(Box::new(i)),
            InstrumentJson::CommodityForward(i) => Ok(Box::new(i)),
            InstrumentJson::CommoditySwap(i) => Ok(Box::new(i)),

            // Exotic Options
            InstrumentJson::Autocallable(i) => Ok(Box::new(i)),
            InstrumentJson::CliquetOption(i) => Ok(Box::new(i)),
            InstrumentJson::RangeAccrual(i) => Ok(Box::new(i)),

            // Total Return Swaps
            InstrumentJson::TrsEquity(i) => Ok(Box::new(i)),
            InstrumentJson::TrsFixedIncomeIndex(i) => Ok(Box::new(i)),

            // Structured Credit
            InstrumentJson::StructuredCredit(i) => Ok(Box::new(*i)),

            // Other
            InstrumentJson::Basket(i) => Ok(Box::new(i)),
            InstrumentJson::Deposit(i) => Ok(Box::new(i)),
            InstrumentJson::Repo(i) => Ok(Box::new(i)),
            InstrumentJson::PrivateMarketsFund(i) => Ok(Box::new(i)),
            InstrumentJson::RealEstateAsset(i) => Ok(Box::new(i)),
            InstrumentJson::RevolvingCredit(i) => Ok(Box::new(i)),
        }
    }
}

// Manual Deserialize implementation to avoid serde lifetime inference issues
impl<'de> Deserialize<'de> for InstrumentJson {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First deserialize to an owned serde_json::Value
        let value = serde_json::Value::deserialize(deserializer)?;

        // Convert back to string and re-parse to break lifetime connection
        let json_str = serde_json::to_string(&value).map_err(D::Error::custom)?;

        #[derive(Deserialize)]
        struct Tagged {
            #[serde(rename = "type")]
            ty: String,
            spec: serde_json::Value,
        }

        let tagged: Tagged = serde_json::from_str(&json_str).map_err(D::Error::custom)?;
        let ty = tagged.ty;
        let spec_str = serde_json::to_string(&tagged.spec).map_err(D::Error::custom)?;

        // Now parse spec into the appropriate type based on the tag
        // Using from_str on a fresh string to avoid lifetime issues
        match ty.as_str() {
            // Fixed Income
            "bond" => serde_json::from_str(&spec_str)
                .map(Self::Bond)
                .map_err(D::Error::custom),
            "convertible_bond" => serde_json::from_str(&spec_str)
                .map(Self::ConvertibleBond)
                .map_err(D::Error::custom),
            "inflation_linked_bond" => serde_json::from_str(&spec_str)
                .map(Self::InflationLinkedBond)
                .map_err(D::Error::custom),
            "term_loan" => serde_json::from_str(&spec_str)
                .map(Self::TermLoan)
                .map_err(D::Error::custom),
            "bond_future" => serde_json::from_str(&spec_str)
                .map(Self::BondFuture)
                .map_err(D::Error::custom),
            "agency_mbs_passthrough" => serde_json::from_str(&spec_str)
                .map(Self::AgencyMbsPassthrough)
                .map_err(D::Error::custom),
            "agency_tba" => serde_json::from_str(&spec_str)
                .map(Self::AgencyTba)
                .map_err(D::Error::custom),
            "agency_cmo" => serde_json::from_str(&spec_str)
                .map(Self::AgencyCmo)
                .map_err(D::Error::custom),
            "dollar_roll" => serde_json::from_str(&spec_str)
                .map(Self::DollarRoll)
                .map_err(D::Error::custom),

            // Swaps
            "interest_rate_swap" => serde_json::from_str(&spec_str)
                .map(Self::InterestRateSwap)
                .map_err(D::Error::custom),
            "basis_swap" => serde_json::from_str(&spec_str)
                .map(Self::BasisSwap)
                .map_err(D::Error::custom),
            "xccy_swap" => serde_json::from_str(&spec_str)
                .map(Self::XccySwap)
                .map_err(D::Error::custom),
            "inflation_swap" => serde_json::from_str(&spec_str)
                .map(Self::InflationSwap)
                .map_err(D::Error::custom),
            "yoy_inflation_swap" | "yo_y_inflation_swap" => serde_json::from_str(&spec_str)
                .map(Self::YoYInflationSwap)
                .map_err(D::Error::custom),
            "inflation_cap_floor" => serde_json::from_str(&spec_str)
                .map(Self::InflationCapFloor)
                .map_err(D::Error::custom),
            "fx_swap" => serde_json::from_str(&spec_str)
                .map(Self::FxSwap)
                .map_err(D::Error::custom),
            "variance_swap" => serde_json::from_str(&spec_str)
                .map(Self::VarianceSwap)
                .map_err(D::Error::custom),

            // Rates Derivatives
            "forward_rate_agreement" => serde_json::from_str(&spec_str)
                .map(Self::ForwardRateAgreement)
                .map_err(D::Error::custom),
            "swaption" => serde_json::from_str(&spec_str)
                .map(Self::Swaption)
                .map_err(D::Error::custom),
            "interest_rate_future" => serde_json::from_str(&spec_str)
                .map(Self::InterestRateFuture)
                .map_err(D::Error::custom),
            "interest_rate_option" => serde_json::from_str(&spec_str)
                .map(Self::InterestRateOption)
                .map_err(D::Error::custom),
            "cms_option" => serde_json::from_str(&spec_str)
                .map(Self::CmsOption)
                .map_err(D::Error::custom),

            // Credit
            "credit_default_swap" => serde_json::from_str(&spec_str)
                .map(Self::CreditDefaultSwap)
                .map_err(D::Error::custom),
            "cds_index" => serde_json::from_str(&spec_str)
                .map(Self::CDSIndex)
                .map_err(D::Error::custom),
            "cds_tranche" => serde_json::from_str(&spec_str)
                .map(Self::CdsTranche)
                .map_err(D::Error::custom),
            "cds_option" => serde_json::from_str(&spec_str)
                .map(Self::CdsOption)
                .map_err(D::Error::custom),

            // Equity
            "equity" => serde_json::from_str(&spec_str)
                .map(Self::Equity)
                .map_err(D::Error::custom),
            "equity_option" => serde_json::from_str(&spec_str)
                .map(Self::EquityOption)
                .map_err(D::Error::custom),
            "asian_option" => serde_json::from_str(&spec_str)
                .map(Self::AsianOption)
                .map_err(D::Error::custom),
            "barrier_option" => serde_json::from_str(&spec_str)
                .map(Self::BarrierOption)
                .map_err(D::Error::custom),
            "lookback_option" => serde_json::from_str(&spec_str)
                .map(Self::LookbackOption)
                .map_err(D::Error::custom),
            "equity_index_future" => serde_json::from_str(&spec_str)
                .map(Self::EquityIndexFuture)
                .map_err(D::Error::custom),
            "volatility_index_future" => serde_json::from_str(&spec_str)
                .map(Self::VolatilityIndexFuture)
                .map_err(D::Error::custom),
            "volatility_index_option" => serde_json::from_str(&spec_str)
                .map(Self::VolatilityIndexOption)
                .map_err(D::Error::custom),

            // FX
            "fx_spot" => serde_json::from_str(&spec_str)
                .map(Self::FxSpot)
                .map_err(D::Error::custom),
            "fx_forward" => serde_json::from_str(&spec_str)
                .map(Self::FxForward)
                .map_err(D::Error::custom),
            "ndf" => serde_json::from_str(&spec_str)
                .map(Self::Ndf)
                .map_err(D::Error::custom),
            "fx_option" => serde_json::from_str(&spec_str)
                .map(Self::FxOption)
                .map_err(D::Error::custom),
            "fx_barrier_option" => serde_json::from_str(&spec_str)
                .map(Self::FxBarrierOption)
                .map_err(D::Error::custom),
            "fx_variance_swap" => serde_json::from_str(&spec_str)
                .map(Self::FxVarianceSwap)
                .map_err(D::Error::custom),
            "quanto_option" => serde_json::from_str(&spec_str)
                .map(Self::QuantoOption)
                .map_err(D::Error::custom),

            // Commodity
            "commodity_option" => serde_json::from_str(&spec_str)
                .map(Self::CommodityOption)
                .map_err(D::Error::custom),
            "commodity_forward" => serde_json::from_str(&spec_str)
                .map(Self::CommodityForward)
                .map_err(D::Error::custom),
            "commodity_swap" => serde_json::from_str(&spec_str)
                .map(Self::CommoditySwap)
                .map_err(D::Error::custom),

            // Exotic Options
            "autocallable" => serde_json::from_str(&spec_str)
                .map(Self::Autocallable)
                .map_err(D::Error::custom),
            "cliquet_option" => serde_json::from_str(&spec_str)
                .map(Self::CliquetOption)
                .map_err(D::Error::custom),
            "range_accrual" => serde_json::from_str(&spec_str)
                .map(Self::RangeAccrual)
                .map_err(D::Error::custom),

            // Total Return Swaps
            "trs_equity" | "equity_trs" => serde_json::from_str(&spec_str)
                .map(Self::TrsEquity)
                .map_err(D::Error::custom),
            "trs_fixed_income_index" | "fi_trs" | "fixed_income_trs" => {
                serde_json::from_str(&spec_str)
                    .map(Self::TrsFixedIncomeIndex)
                    .map_err(D::Error::custom)
            }

            // Structured Credit
            "structured_credit" => serde_json::from_str(&spec_str)
                .map(|sc| Self::StructuredCredit(Box::new(sc)))
                .map_err(D::Error::custom),

            // Other
            "basket" => serde_json::from_str(&spec_str)
                .map(Self::Basket)
                .map_err(D::Error::custom),
            "deposit" => serde_json::from_str(&spec_str)
                .map(Self::Deposit)
                .map_err(D::Error::custom),
            "repo" => serde_json::from_str(&spec_str)
                .map(Self::Repo)
                .map_err(D::Error::custom),
            "private_markets_fund" => serde_json::from_str(&spec_str)
                .map(Self::PrivateMarketsFund)
                .map_err(D::Error::custom),
            "real_estate_asset" => serde_json::from_str(&spec_str)
                .map(Self::RealEstateAsset)
                .map_err(D::Error::custom),
            "revolving_credit" => serde_json::from_str(&spec_str)
                .map(Self::RevolvingCredit)
                .map_err(D::Error::custom),

            other => Err(D::Error::unknown_variant(
                other,
                &[
                    // Fixed Income
                    "bond",
                    "convertible_bond",
                    "inflation_linked_bond",
                    "term_loan",
                    "bond_future",
                    "agency_mbs_passthrough",
                    "agency_tba",
                    "agency_cmo",
                    "dollar_roll",
                    // Swaps
                    "interest_rate_swap",
                    "basis_swap",
                    "xccy_swap",
                    "inflation_swap",
                    "yoy_inflation_swap",
                    "yo_y_inflation_swap",
                    "inflation_cap_floor",
                    "fx_swap",
                    "variance_swap",
                    // Rates Derivatives
                    "forward_rate_agreement",
                    "swaption",
                    "interest_rate_future",
                    "interest_rate_option",
                    "cms_option",
                    // Credit
                    "credit_default_swap",
                    "cds_index",
                    "cds_tranche",
                    "cds_option",
                    // Equity
                    "equity",
                    "equity_option",
                    "asian_option",
                    "barrier_option",
                    "lookback_option",
                    "equity_index_future",
                    "volatility_index_future",
                    "volatility_index_option",
                    // FX
                    "fx_spot",
                    "fx_forward",
                    "ndf",
                    "fx_option",
                    "fx_barrier_option",
                    "fx_variance_swap",
                    "quanto_option",
                    // Commodity
                    "commodity_option",
                    "commodity_forward",
                    "commodity_swap",
                    // Exotics
                    "autocallable",
                    "cliquet_option",
                    "range_accrual",
                    // TRS
                    "trs_equity",
                    "trs_fixed_income_index",
                    // Structured
                    "structured_credit",
                    // Other
                    "basket",
                    "deposit",
                    "repo",
                    "private_markets_fund",
                    "real_estate_asset",
                    "revolving_credit",
                ],
            )),
        }
    }
}

impl InstrumentEnvelope {
    /// Load an instrument from a JSON reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - Any reader providing JSON bytes
    ///
    /// # Returns
    ///
    /// A boxed instrument trait object ready for pricing.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JSON is malformed
    /// - Schema version is unsupported
    /// - Required fields are missing
    /// - Unknown fields are present (strict mode)
    /// - Spec validation fails
    pub fn from_reader<R: Read>(reader: R) -> Result<Box<dyn Instrument>> {
        let envelope: Self =
            serde_json::from_reader(reader).map_err(|_| finstack_core::InputError::Invalid)?;

        // Validate schema version (currently we only support version 1)
        if !envelope.schema.starts_with("finstack.instrument/1") {
            return Err(finstack_core::InputError::Invalid.into());
        }

        envelope.instrument.into_boxed()
    }

    /// Load an instrument from a JSON string.
    ///
    /// Convenience wrapper around `from_reader`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Box<dyn Instrument>> {
        Self::from_reader(s.as_bytes())
    }

    /// Load an instrument from a JSON file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file
    ///
    /// # Returns
    ///
    /// A boxed instrument trait object ready for pricing.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Box<dyn Instrument>> {
        let file =
            std::fs::File::open(path.as_ref()).map_err(|_| finstack_core::InputError::Invalid)?;
        Self::from_reader(file)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[allow(clippy::expect_used, clippy::unwrap_used, dead_code, unused_imports)]
    mod test_utils {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/test_utils.rs"
        ));
    }

    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    #[test]
    fn test_bond_json_roundtrip() {
        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date"),
            Date::from_calendar_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let json = InstrumentJson::Bond(bond.clone());
        let serialized =
            serde_json::to_string(&json).expect("JSON serialization should succeed in test");
        let deserialized: InstrumentJson =
            serde_json::from_str(&serialized).expect("JSON deserialization should succeed in test");

        match deserialized {
            InstrumentJson::Bond(b) => {
                assert_eq!(b.id, bond.id);
                assert_eq!(b.notional, bond.notional);
            }
            _ => panic!("Expected Bond variant"),
        }
    }

    #[test]
    fn test_envelope_roundtrip() {
        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date"),
            Date::from_calendar_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let envelope = InstrumentEnvelope {
            schema: "finstack.instrument/1".to_string(),
            instrument: InstrumentJson::Bond(bond.clone()),
        };

        let serialized = serde_json::to_string_pretty(&envelope)
            .expect("JSON serialization should succeed in test");
        let deserialized: InstrumentEnvelope =
            serde_json::from_str(&serialized).expect("JSON deserialization should succeed in test");

        assert_eq!(deserialized.schema, envelope.schema);
        match deserialized.instrument {
            InstrumentJson::Bond(b) => {
                assert_eq!(b.id, bond.id);
            }
            _ => panic!("Expected Bond variant"),
        }
    }

    #[test]
    fn test_envelope_from_str() {
        // Use Bond which is simpler and fully tested
        let json = r#"{
            "schema": "finstack.instrument/1",
            "instrument": {
                "type": "bond",
                "spec": {
                    "id": "BOND-FROM-STR",
                    "notional": { "amount": "1000000", "currency": "USD" },
                    "issue": "2024-01-01",
                    "maturity": "2034-01-01",
                    "cashflow_spec": {
                        "Fixed": {
                            "coupon_type": "Cash",
                            "rate": 0.05,
                            "freq": { "count": 6, "unit": "months" },
                            "dc": "Thirty360",
                            "bdc": "following",
                            "calendar_id": "weekends_only",
                            "stub": "None",
                            "end_of_month": false,
                            "payment_lag_days": 0
                        }
                    },
                    "discount_curve_id": "USD-OIS",
                    "credit_curve_id": null,
                    "pricing_overrides": {
                        "quoted_clean_price": null,
                        "implied_volatility": null,
                        "quoted_spread_bp": null,
                        "upfront_payment": null,
                        "ytm_bump_decimal": null,
                        "theta_period": null,
                        "mc_seed_scenario": null,
                        "adaptive_bumps": false,
                        "spot_bump_pct": null,
                        "vol_bump_pct": null,
                        "rate_bump_bp": null
                    },
                    "call_put": null,
                    "accrual_method": "Linear",
                    "attributes": { "tags": [], "meta": {} },
                    "settlement_days": null,
                    "ex_coupon_days": null
                }
            }
        }"#;

        let instrument = InstrumentEnvelope::from_str(json)
            .expect("Instrument envelope parsing should succeed in test");
        assert_eq!(instrument.id(), "BOND-FROM-STR");
    }

    #[test]
    fn test_unknown_fields_rejected() {
        // Test with Bond and an extra unknown field
        let json = r#"{
            "schema": "finstack.instrument/1",
            "instrument": {
                "type": "bond",
                "spec": {
                    "id": "BOND-001",
                    "notional": { "amount": "1000000", "currency": "USD" },
                    "issue": "2024-01-01",
                    "maturity": "2034-01-01",
                    "cashflow_spec": {
                        "Fixed": {
                            "coupon_type": "Cash",
                            "rate": 0.05,
                            "freq": { "count": 6, "unit": "months" },
                            "dc": "Thirty360",
                            "bdc": "following",
                            "calendar_id": "weekends_only",
                            "stub": "None",
                            "end_of_month": false,
                            "payment_lag_days": 0
                        }
                    },
                    "discount_curve_id": "USD-OIS",
                    "credit_curve_id": null,
                    "pricing_overrides": {
                        "quoted_clean_price": null,
                        "implied_volatility": null,
                        "quoted_spread_bp": null,
                        "upfront_payment": null,
                        "ytm_bump_decimal": null,
                        "theta_period": null,
                        "mc_seed_scenario": null,
                        "adaptive_bumps": false,
                        "spot_bump_pct": null,
                        "vol_bump_pct": null,
                        "rate_bump_bp": null
                    },
                    "call_put": null,
                    "attributes": { "tags": [], "meta": {} },
                    "settlement_days": null,
                    "ex_coupon_days": null,
                    "unknown_field": "should_fail"
                }
            }
        }"#;

        let result = InstrumentEnvelope::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_schema_version() {
        let json = r#"{
            "schema": "finstack.instrument/999",
            "instrument": {
                "type": "bond",
                "spec": {
                    "id": "BOND-001",
                    "notional": { "amount": "1000000", "currency": "USD" },
                    "issue": "2024-01-01",
                    "maturity": "2034-01-01",
                    "cashflow_spec": {
                        "Fixed": {
                            "coupon_type": "Cash",
                            "rate": 0.05,
                            "freq": { "count": 6, "unit": "months" },
                            "dc": "Thirty360",
                            "bdc": "following",
                            "calendar_id": "weekends_only",
                            "stub": "None",
                            "end_of_month": false,
                            "payment_lag_days": 0
                        }
                    },
                    "discount_curve_id": "USD-OIS",
                    "credit_curve_id": null,
                    "pricing_overrides": {
                        "quoted_clean_price": null,
                        "implied_volatility": null,
                        "quoted_spread_bp": null,
                        "upfront_payment": null,
                        "ytm_bump_decimal": null,
                        "theta_period": null,
                        "mc_seed_scenario": null,
                        "adaptive_bumps": false,
                        "spot_bump_pct": null,
                        "vol_bump_pct": null,
                        "rate_bump_bp": null
                    },
                    "call_put": null,
                    "attributes": { "tags": [], "meta": {} },
                    "settlement_days": null,
                    "ex_coupon_days": null
                }
            }
        }"#;

        let result = InstrumentEnvelope::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_type_rejected() {
        let json = r#"{
            "schema": "finstack.instrument/1",
            "instrument": {
                "type": "totally_unknown_instrument",
                "spec": {}
            }
        }"#;

        let result = InstrumentEnvelope::from_str(json);
        assert!(result.is_err());
    }

    // Note: IRS and TermLoan tests skipped - complex builder patterns
    // The serialization/deserialization works but proper construction
    // requires detailed leg specifications beyond scope of simple unit tests

    #[test]
    fn test_cds_roundtrip() {
        let cds = test_utils::cds_buy_protection(
            "CDS-TEST",
            Money::new(10_000_000.0, Currency::USD),
            100.0,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date"),
            Date::from_calendar_date(2029, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
            "CORP-HAZARD",
        )
        .expect("CDS construction should succeed in test");

        let json = InstrumentJson::CreditDefaultSwap(cds.clone());
        let serialized =
            serde_json::to_string(&json).expect("JSON serialization should succeed in test");
        let deserialized: InstrumentJson =
            serde_json::from_str(&serialized).expect("JSON deserialization should succeed in test");

        match deserialized {
            InstrumentJson::CreditDefaultSwap(i) => {
                assert_eq!(i.id, cds.id);
                assert_eq!(i.notional, cds.notional);
            }
            _ => panic!("Expected CreditDefaultSwap variant"),
        }
    }

    #[test]
    fn test_fx_swap_roundtrip() {
        let fx_swap = FxSwap::builder()
            .id(InstrumentId::new("FXSWAP-TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .near_date(Date::from_calendar_date(2024, Month::January, 3).expect("Valid test date"))
            .far_date(Date::from_calendar_date(2024, Month::July, 3).expect("Valid test date"))
            .base_notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id("USD-OIS".into())
            .foreign_discount_curve_id("EUR-OIS".into())
            .near_rate_opt(Some(1.10))
            .far_rate_opt(Some(1.12))
            .attributes(Attributes::new())
            .build()
            .expect("FxSwap builder should succeed with valid test data");

        let json = InstrumentJson::FxSwap(fx_swap.clone());
        let serialized =
            serde_json::to_string(&json).expect("JSON serialization should succeed in test");
        let deserialized: InstrumentJson =
            serde_json::from_str(&serialized).expect("JSON deserialization should succeed in test");

        match deserialized {
            InstrumentJson::FxSwap(i) => {
                assert_eq!(i.id, fx_swap.id);
                assert_eq!(i.base_currency, fx_swap.base_currency);
            }
            _ => panic!("Expected FxSwap variant"),
        }
    }

    #[test]
    fn test_basis_swap_roundtrip() {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};

        let primary_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0005,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let reference_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let swap = BasisSwap::new(
            "BASIS-TEST",
            Money::new(10_000_000.0, Currency::USD),
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date"),
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
            primary_leg,
            reference_leg,
            CurveId::new("USD-OIS"),
        )
        .expect("BasisSwap construction should succeed in test")
        .with_calendar("USGS");

        let json = InstrumentJson::BasisSwap(swap.clone());
        let serialized =
            serde_json::to_string(&json).expect("JSON serialization should succeed in test");
        let deserialized: InstrumentJson =
            serde_json::from_str(&serialized).expect("JSON deserialization should succeed in test");

        match deserialized {
            InstrumentJson::BasisSwap(i) => {
                assert_eq!(i.id, swap.id);
                assert_eq!(i.discount_curve_id, swap.discount_curve_id);
                assert_eq!(i.calendar_id.as_deref(), Some("USGS"));
            }
            _ => panic!("Expected BasisSwap variant"),
        }
    }

    #[test]
    fn test_fx_spot_roundtrip() {
        let fx_spot = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .with_notional(Money::new(1_000_000.0, Currency::EUR))
            .expect("FxSpot notional should be valid")
            .with_rate(1.10)
            .expect("FxSpot rate should be valid")
            .with_settlement(
                Date::from_calendar_date(2024, Month::January, 15).expect("Valid test date"),
            )
            .with_base_calendar_id("TARGET")
            .with_quote_calendar_id("USNY");

        let json = InstrumentJson::FxSpot(fx_spot.clone());
        let serialized =
            serde_json::to_string(&json).expect("JSON serialization should succeed in test");
        let deserialized: InstrumentJson =
            serde_json::from_str(&serialized).expect("JSON deserialization should succeed in test");

        match deserialized {
            InstrumentJson::FxSpot(i) => {
                assert_eq!(i.id, fx_spot.id);
                assert_eq!(i.base, fx_spot.base);
                assert_eq!(i.quote, fx_spot.quote);
                assert_eq!(i.base_calendar_id.as_deref(), Some("TARGET"));
                assert_eq!(i.quote_calendar_id.as_deref(), Some("USNY"));
            }
            _ => panic!("Expected FxSpot variant"),
        }
    }

    /// Canonical list of all instrument type discriminators.
    ///
    /// This list MUST be kept in sync with:
    /// 1. The `InstrumentJson` enum variants (with `#[serde(rename_all = "snake_case")]`)
    /// 2. The JSON schema at `schemas/instruments/1/instrument.schema.json`
    ///
    /// When adding a new instrument type:
    /// 1. Add the variant to `InstrumentJson` enum
    /// 2. Add the snake_case name here (alphabetically sorted)
    /// 3. Update the JSON schema file
    /// 4. Run this test to verify parity
    const CANONICAL_INSTRUMENT_TYPES: &[&str] = &[
        "agency_cmo",
        "agency_mbs_passthrough",
        "agency_tba",
        "asian_option",
        "autocallable",
        "barrier_option",
        "basis_swap",
        "basket",
        "bond",
        "bond_future",
        "cds_index",
        "cds_option",
        "cds_tranche",
        "cliquet_option",
        "cms_option",
        "commodity_forward",
        "commodity_option",
        "commodity_swap",
        "convertible_bond",
        "credit_default_swap",
        "deposit",
        "dollar_roll",
        "equity",
        "equity_index_future",
        "equity_option",
        "forward_rate_agreement",
        "fx_barrier_option",
        "fx_forward",
        "fx_option",
        "fx_spot",
        "fx_swap",
        "fx_variance_swap",
        "inflation_cap_floor",
        "inflation_linked_bond",
        "inflation_swap",
        "interest_rate_future",
        "interest_rate_option",
        "interest_rate_swap",
        "lookback_option",
        "ndf",
        "private_markets_fund",
        "quanto_option",
        "range_accrual",
        "real_estate_asset",
        "repo",
        "revolving_credit",
        "structured_credit",
        "swaption",
        "term_loan",
        "trs_equity",
        "trs_fixed_income_index",
        "variance_swap",
        "volatility_index_future",
        "volatility_index_option",
        "xccy_swap",
        "yoy_inflation_swap",
    ];

    /// Verifies that the instrument.schema.json enum matches the canonical list.
    ///
    /// This test ensures that the JSON schema stays in sync with the Rust code.
    /// If this test fails, update the JSON schema file to match the canonical list.
    #[test]
    fn test_instrument_schema_enum_parity() {
        let schema_json = include_str!("../../schemas/instruments/1/instrument.schema.json");
        let schema: serde_json::Value =
            serde_json::from_str(schema_json).expect("Schema JSON should be valid");

        // Extract the enum array from the schema
        let schema_types: Vec<&str> = schema["properties"]["instrument"]["properties"]["type"]
            ["enum"]
            .as_array()
            .expect("Schema should have instrument.properties.type.enum array")
            .iter()
            .map(|v| v.as_str().expect("Enum values should be strings"))
            .collect();

        // Sort both lists for comparison
        let mut expected: Vec<&str> = CANONICAL_INSTRUMENT_TYPES.to_vec();
        expected.sort();
        let mut actual: Vec<&str> = schema_types.clone();
        actual.sort();

        // Find differences
        let missing_from_schema: Vec<&str> = expected
            .iter()
            .filter(|t| !actual.contains(t))
            .copied()
            .collect();
        let extra_in_schema: Vec<&str> = actual
            .iter()
            .filter(|t| !expected.contains(t))
            .copied()
            .collect();

        if !missing_from_schema.is_empty() || !extra_in_schema.is_empty() {
            let mut msg = String::from("instrument.schema.json is out of sync with Rust code!\n\n");
            if !missing_from_schema.is_empty() {
                msg.push_str(&format!(
                    "Missing from schema (add these):\n  {}\n\n",
                    missing_from_schema.join(", ")
                ));
            }
            if !extra_in_schema.is_empty() {
                msg.push_str(&format!(
                    "Extra in schema (remove these or add to CANONICAL_INSTRUMENT_TYPES):\n  {}\n",
                    extra_in_schema.join(", ")
                ));
            }
            panic!("{}", msg);
        }

        // Verify the schema enum is alphabetically sorted (for maintainability)
        assert_eq!(
            schema_types, actual,
            "Schema enum should be alphabetically sorted for maintainability"
        );
    }
}
