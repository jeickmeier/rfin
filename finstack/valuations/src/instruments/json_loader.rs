//! JSON import/export for financial instruments.
//!
//! This module provides a tagged union for all instrument types and helpers
//! for loading instruments from JSON files with strict validation.

use super::*;
use finstack_core::Result;
use serde::{
    de::{DeserializeOwned, Deserializer, Error as DeError},
    Deserialize, Serialize,
};
use std::io::Read;

/// Versioned envelope for JSON instrument definitions.
///
/// This wrapper allows for future schema evolution while maintaining
/// compatibility with existing JSON files.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
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
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
#[serde(tag = "type", content = "spec", rename_all = "snake_case")]
#[non_exhaustive]
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
    BondFuture(Box<BondFuture>),
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
    /// Constant maturity swap (CMS) swap
    CmsSwap(CmsSwap),
    /// Constant maturity swap (CMS) option
    CmsOption(CmsOption),
    /// Interest rate future option
    IrFutureOption(IrFutureOption),
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
    #[serde(rename = "cds_tranche")]
    CDSTranche(CDSTranche),
    /// CDS option
    #[serde(rename = "cds_option")]
    CDSOption(CDSOption),

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
    /// FX digital (binary) option
    FxDigitalOption(FxDigitalOption),
    /// FX touch/no-touch option
    FxTouchOption(FxTouchOption),
    /// FX barrier option
    FxBarrierOption(FxBarrierOption),
    /// FX variance swap
    FxVarianceSwap(FxVarianceSwap),
    /// Quanto option (cross-currency)
    QuantoOption(QuantoOption),

    // Commodity
    /// Commodity option
    CommodityOption(CommodityOption),
    /// Commodity Asian option
    CommodityAsianOption(CommodityAsianOption),
    /// Commodity forward
    CommodityForward(CommodityForward),
    /// Commodity swap
    CommoditySwap(CommoditySwap),
    /// Commodity swaption
    CommoditySwaption(CommoditySwaption),
    /// Commodity spread option
    CommoditySpreadOption(CommoditySpreadOption),

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
    /// Levered real estate equity
    LeveredRealEstateEquity(Box<crate::instruments::equity::real_estate::LeveredRealEstateEquity>),
    /// Discounted cash flow (DCF) valuation
    DiscountedCashFlow(DiscountedCashFlow),
}

macro_rules! with_instrument_json_registry {
    ($callback:ident $(, $extra:expr )* $(,)?) => {
        $callback!(
            [$($extra),*]
            plain: Bond(Bond) => "bond";
            plain: ConvertibleBond(ConvertibleBond) => "convertible_bond";
            plain: InflationLinkedBond(InflationLinkedBond) => "inflation_linked_bond";
            plain: TermLoan(TermLoan) => "term_loan";
            plain: RevolvingCredit(RevolvingCredit) => "revolving_credit";
            plain: AgencyMbsPassthrough(AgencyMbsPassthrough) => "agency_mbs_passthrough";
            plain: AgencyTba(AgencyTba) => "agency_tba";
            plain: AgencyCmo(AgencyCmo) => "agency_cmo";
            plain: DollarRoll(DollarRoll) => "dollar_roll";
            plain: InterestRateSwap(InterestRateSwap) => "interest_rate_swap";
            plain: BasisSwap(BasisSwap) => "basis_swap";
            plain: XccySwap(XccySwap) => "xccy_swap";
            plain: InflationSwap(InflationSwap) => "inflation_swap";
            plain: YoYInflationSwap(YoYInflationSwap) => "yoy_inflation_swap", "yo_y_inflation_swap";
            plain: InflationCapFloor(InflationCapFloor) => "inflation_cap_floor";
            plain: ForwardRateAgreement(ForwardRateAgreement) => "forward_rate_agreement";
            plain: Swaption(Swaption) => "swaption";
            plain: InterestRateFuture(InterestRateFuture) => "interest_rate_future";
            plain: InterestRateOption(InterestRateOption) => "interest_rate_option";
            plain: CmsSwap(CmsSwap) => "cms_swap";
            plain: CmsOption(CmsOption) => "cms_option";
            plain: IrFutureOption(IrFutureOption) => "ir_future_option";
            plain: Deposit(Deposit) => "deposit";
            plain: Repo(Repo) => "repo";
            plain: CreditDefaultSwap(CreditDefaultSwap) => "credit_default_swap";
            plain: CDSIndex(CDSIndex) => "cds_index";
            plain: CDSTranche(CDSTranche) => "cds_tranche";
            plain: CDSOption(CDSOption) => "cds_option";
            plain: Equity(Equity) => "equity";
            plain: EquityOption(EquityOption) => "equity_option";
            plain: AsianOption(AsianOption) => "asian_option";
            plain: BarrierOption(BarrierOption) => "barrier_option";
            plain: LookbackOption(LookbackOption) => "lookback_option";
            plain: VarianceSwap(VarianceSwap) => "variance_swap";
            plain: EquityIndexFuture(EquityIndexFuture) => "equity_index_future";
            plain: VolatilityIndexFuture(VolatilityIndexFuture) => "volatility_index_future";
            plain: VolatilityIndexOption(VolatilityIndexOption) => "volatility_index_option";
            plain: FxSpot(FxSpot) => "fx_spot";
            plain: FxSwap(FxSwap) => "fx_swap";
            plain: FxForward(FxForward) => "fx_forward";
            plain: Ndf(Ndf) => "ndf";
            plain: FxOption(FxOption) => "fx_option";
            plain: FxDigitalOption(FxDigitalOption) => "fx_digital_option";
            plain: FxTouchOption(FxTouchOption) => "fx_touch_option";
            plain: FxBarrierOption(FxBarrierOption) => "fx_barrier_option";
            plain: FxVarianceSwap(FxVarianceSwap) => "fx_variance_swap";
            plain: QuantoOption(QuantoOption) => "quanto_option";
            plain: CommodityOption(CommodityOption) => "commodity_option";
            plain: CommodityAsianOption(CommodityAsianOption) => "commodity_asian_option";
            plain: CommodityForward(CommodityForward) => "commodity_forward";
            plain: CommoditySwap(CommoditySwap) => "commodity_swap";
            plain: CommoditySwaption(CommoditySwaption) => "commodity_swaption";
            plain: CommoditySpreadOption(CommoditySpreadOption) => "commodity_spread_option";
            plain: Autocallable(Autocallable) => "autocallable";
            plain: CliquetOption(CliquetOption) => "cliquet_option";
            plain: RangeAccrual(RangeAccrual) => "range_accrual";
            plain: TrsEquity(EquityTotalReturnSwap) => "trs_equity", "equity_trs";
            plain: TrsFixedIncomeIndex(FIIndexTotalReturnSwap) => "trs_fixed_income_index", "fi_trs", "fixed_income_trs";
            plain: Basket(Basket) => "basket";
            plain: PrivateMarketsFund(PrivateMarketsFund) => "private_markets_fund";
            plain: RealEstateAsset(RealEstateAsset) => "real_estate_asset";
            plain: DiscountedCashFlow(DiscountedCashFlow) => "discounted_cash_flow";
            boxed: BondFuture(BondFuture) => "bond_future";
            boxed: StructuredCredit(StructuredCredit) => "structured_credit";
            boxed: LeveredRealEstateEquity(crate::instruments::equity::real_estate::LeveredRealEstateEquity) => "levered_real_estate_equity";
        )
    };
}

macro_rules! instrument_json_into_boxed_match {
    (
        [$instrument_json:expr]
        $(plain: $variant:ident($ty:ty) => $tag:literal $(, $alias:literal)*;)*
        $(boxed: $boxed_variant:ident($boxed_ty:ty) => $boxed_tag:literal $(, $boxed_alias:literal)*;)*
    ) => {
        match $instrument_json {
            $(InstrumentJson::$variant(instrument) => Ok(Box::new(instrument)),)*
            $(InstrumentJson::$boxed_variant(instrument) => Ok(Box::new(*instrument)),)*
        }
    };
}

macro_rules! instrument_json_from_dyn_match {
    (
        [$instrument:expr]
        $(plain: $variant:ident($ty:ty) => $tag:literal $(, $alias:literal)*;)*
        $(boxed: $boxed_variant:ident($boxed_ty:ty) => $boxed_tag:literal $(, $boxed_alias:literal)*;)*
    ) => {{
        let instrument = $instrument;
        'match_instrument: {
            $(
                if let Some(concrete) = instrument.as_any().downcast_ref::<$ty>() {
                    break 'match_instrument Some(InstrumentJson::$variant(concrete.clone()));
                }
            )*
            $(
                if let Some(concrete) = instrument.as_any().downcast_ref::<$boxed_ty>() {
                    break 'match_instrument Some(InstrumentJson::$boxed_variant(Box::new(
                        concrete.clone(),
                    )));
                }
            )*
            None
        }
    }};
}

#[cfg(test)]
macro_rules! instrument_json_canonical_types {
    (
        []
        $(plain: $variant:ident($ty:ty) => $tag:literal $(, $alias:literal)*;)*
        $(boxed: $boxed_variant:ident($boxed_ty:ty) => $boxed_tag:literal $(, $boxed_alias:literal)*;)*
    ) => {
        &[
            $($tag,)*
            $($boxed_tag,)*
        ]
    };
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
    pub fn into_boxed(self) -> Result<Box<DynInstrument>> {
        with_instrument_json_registry!(instrument_json_into_boxed_match, self)
    }

    /// Convert a concrete instrument trait object into its tagged JSON form.
    ///
    /// Returns `None` when the instrument is not part of the JSON registry.
    pub fn from_instrument(
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
    ) -> Option<Self> {
        with_instrument_json_registry!(instrument_json_from_dyn_match, instrument)
    }
}

#[derive(Deserialize)]
struct TaggedInstrumentValue {
    #[serde(rename = "type")]
    ty: String,
    spec: serde_json::Value,
}

fn parse_spec<T>(
    spec: serde_json::Value,
    wrap: impl FnOnce(T) -> InstrumentJson,
) -> serde_json::Result<InstrumentJson>
where
    T: DeserializeOwned,
{
    serde_json::from_value(spec).map(wrap)
}

fn parse_tagged_value(value: serde_json::Value) -> serde_json::Result<InstrumentJson> {
    let TaggedInstrumentValue { ty, spec } = serde_json::from_value(value)?;

    match ty.as_str() {
        // Fixed Income
        "bond" => parse_spec(spec, InstrumentJson::Bond),
        "convertible_bond" => parse_spec(spec, InstrumentJson::ConvertibleBond),
        "inflation_linked_bond" => parse_spec(spec, InstrumentJson::InflationLinkedBond),
        "term_loan" => parse_spec(spec, InstrumentJson::TermLoan),
        "bond_future" => {
            parse_spec::<BondFuture>(spec, |i| InstrumentJson::BondFuture(Box::new(i)))
        }
        "agency_mbs_passthrough" => parse_spec(spec, InstrumentJson::AgencyMbsPassthrough),
        "agency_tba" => parse_spec(spec, InstrumentJson::AgencyTba),
        "agency_cmo" => parse_spec(spec, InstrumentJson::AgencyCmo),
        "dollar_roll" => parse_spec(spec, InstrumentJson::DollarRoll),

        // Swaps
        "interest_rate_swap" => parse_spec(spec, InstrumentJson::InterestRateSwap),
        "basis_swap" => parse_spec(spec, InstrumentJson::BasisSwap),
        "xccy_swap" => parse_spec(spec, InstrumentJson::XccySwap),
        "inflation_swap" => parse_spec(spec, InstrumentJson::InflationSwap),
        "yoy_inflation_swap" | "yo_y_inflation_swap" => {
            parse_spec(spec, InstrumentJson::YoYInflationSwap)
        }
        "inflation_cap_floor" => parse_spec(spec, InstrumentJson::InflationCapFloor),
        "fx_swap" => parse_spec(spec, InstrumentJson::FxSwap),
        "variance_swap" => parse_spec(spec, InstrumentJson::VarianceSwap),

        // Rates Derivatives
        "forward_rate_agreement" => parse_spec(spec, InstrumentJson::ForwardRateAgreement),
        "swaption" => parse_spec(spec, InstrumentJson::Swaption),
        "interest_rate_future" => parse_spec(spec, InstrumentJson::InterestRateFuture),
        "interest_rate_option" => parse_spec(spec, InstrumentJson::InterestRateOption),
        "cms_swap" => parse_spec(spec, InstrumentJson::CmsSwap),
        "cms_option" => parse_spec(spec, InstrumentJson::CmsOption),
        "ir_future_option" => parse_spec(spec, InstrumentJson::IrFutureOption),

        // Credit
        "credit_default_swap" => parse_spec(spec, InstrumentJson::CreditDefaultSwap),
        "cds_index" => parse_spec(spec, InstrumentJson::CDSIndex),
        "cds_tranche" => parse_spec(spec, InstrumentJson::CDSTranche),
        "cds_option" => parse_spec(spec, InstrumentJson::CDSOption),

        // Equity
        "equity" => parse_spec(spec, InstrumentJson::Equity),
        "equity_option" => parse_spec(spec, InstrumentJson::EquityOption),
        "asian_option" => parse_spec(spec, InstrumentJson::AsianOption),
        "barrier_option" => parse_spec(spec, InstrumentJson::BarrierOption),
        "lookback_option" => parse_spec(spec, InstrumentJson::LookbackOption),
        "equity_index_future" => parse_spec(spec, InstrumentJson::EquityIndexFuture),
        "volatility_index_future" => parse_spec(spec, InstrumentJson::VolatilityIndexFuture),
        "volatility_index_option" => parse_spec(spec, InstrumentJson::VolatilityIndexOption),

        // FX
        "fx_spot" => parse_spec(spec, InstrumentJson::FxSpot),
        "fx_forward" => parse_spec(spec, InstrumentJson::FxForward),
        "ndf" => parse_spec(spec, InstrumentJson::Ndf),
        "fx_option" => parse_spec(spec, InstrumentJson::FxOption),
        "fx_digital_option" => parse_spec(spec, InstrumentJson::FxDigitalOption),
        "fx_touch_option" => parse_spec(spec, InstrumentJson::FxTouchOption),
        "fx_barrier_option" => parse_spec(spec, InstrumentJson::FxBarrierOption),
        "fx_variance_swap" => parse_spec(spec, InstrumentJson::FxVarianceSwap),
        "quanto_option" => parse_spec(spec, InstrumentJson::QuantoOption),

        // Commodity
        "commodity_option" => parse_spec(spec, InstrumentJson::CommodityOption),
        "commodity_asian_option" => parse_spec(spec, InstrumentJson::CommodityAsianOption),
        "commodity_forward" => parse_spec(spec, InstrumentJson::CommodityForward),
        "commodity_swap" => parse_spec(spec, InstrumentJson::CommoditySwap),
        "commodity_swaption" => parse_spec(spec, InstrumentJson::CommoditySwaption),
        "commodity_spread_option" => parse_spec(spec, InstrumentJson::CommoditySpreadOption),

        // Exotic Options
        "autocallable" => parse_spec(spec, InstrumentJson::Autocallable),
        "cliquet_option" => parse_spec(spec, InstrumentJson::CliquetOption),
        "range_accrual" => parse_spec(spec, InstrumentJson::RangeAccrual),

        // Total Return Swaps
        "trs_equity" | "equity_trs" => parse_spec(spec, InstrumentJson::TrsEquity),
        "trs_fixed_income_index" | "fi_trs" | "fixed_income_trs" => {
            parse_spec(spec, InstrumentJson::TrsFixedIncomeIndex)
        }

        // Structured Credit
        "structured_credit" => parse_spec::<StructuredCredit>(spec, |sc| {
            InstrumentJson::StructuredCredit(Box::new(sc))
        }),

        // Other
        "basket" => parse_spec(spec, InstrumentJson::Basket),
        "deposit" => parse_spec(spec, InstrumentJson::Deposit),
        "repo" => parse_spec(spec, InstrumentJson::Repo),
        "private_markets_fund" => parse_spec(spec, InstrumentJson::PrivateMarketsFund),
        "real_estate_asset" => parse_spec(spec, InstrumentJson::RealEstateAsset),
        "levered_real_estate_equity" => parse_spec::<
            crate::instruments::equity::real_estate::LeveredRealEstateEquity,
        >(spec, |i| {
            InstrumentJson::LeveredRealEstateEquity(Box::new(i))
        }),
        "revolving_credit" => parse_spec(spec, InstrumentJson::RevolvingCredit),
        "discounted_cash_flow" => parse_spec(spec, InstrumentJson::DiscountedCashFlow),

        other => Err(<serde_json::Error as serde::de::Error>::unknown_variant(
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
                "cms_swap",
                "cms_option",
                "ir_future_option",
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
                "fx_digital_option",
                "fx_touch_option",
                "fx_barrier_option",
                "fx_variance_swap",
                "quanto_option",
                // Commodity
                "commodity_option",
                "commodity_asian_option",
                "commodity_forward",
                "commodity_swap",
                "commodity_swaption",
                "commodity_spread_option",
                // Exotics
                "autocallable",
                "cliquet_option",
                "range_accrual",
                // TRS
                "trs_equity",
                "equity_trs",
                "trs_fixed_income_index",
                "fi_trs",
                "fixed_income_trs",
                // Structured
                "structured_credit",
                // Other
                "basket",
                "deposit",
                "repo",
                "private_markets_fund",
                "real_estate_asset",
                "levered_real_estate_equity",
                "revolving_credit",
                "discounted_cash_flow",
            ],
        )),
    }
}

// Manual Deserialize implementation keeps the explicit tag/alias map while
// avoiding Value -> String -> from_str round-trips in bulk ingestion paths.
impl<'de> Deserialize<'de> for InstrumentJson {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        parse_tagged_value(value).map_err(D::Error::custom)
    }
}

impl InstrumentEnvelope {
    /// Current schema version emitted by Finstack instrument envelopes.
    pub const CURRENT_SCHEMA: &'static str = "finstack.instrument/1";

    /// Create a versioned envelope from an instrument JSON payload.
    pub fn new(instrument: InstrumentJson) -> Self {
        Self {
            schema: Self::CURRENT_SCHEMA.to_string(),
            instrument,
        }
    }

    fn validate_schema(&self) -> Result<()> {
        if self.schema != Self::CURRENT_SCHEMA {
            return Err(finstack_core::InputError::Invalid.into());
        }
        Ok(())
    }

    fn finalize_loaded_instrument(instrument: Box<DynInstrument>) -> Result<Box<DynInstrument>> {
        if let Some(overrides) = instrument.scenario_overrides() {
            overrides.validate()?;
        }
        Ok(instrument)
    }

    /// Convert a concrete instrument into a versioned JSON envelope.
    ///
    /// Returns `None` when the instrument does not participate in the JSON
    /// registry.
    pub fn from_instrument(
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
    ) -> Option<Self> {
        InstrumentJson::from_instrument(instrument).map(Self::new)
    }

    /// Load an instrument from a JSON value.
    ///
    /// Accepts either the versioned envelope form:
    ///
    /// ```json
    /// { "schema": "finstack.instrument/1", "instrument": { ... } }
    /// ```
    ///
    /// or the bare tagged instrument form:
    ///
    /// ```json
    /// { "type": "bond", "spec": { ... } }
    /// ```
    pub fn from_value(value: serde_json::Value) -> Result<Box<DynInstrument>> {
        if let Ok(envelope) = serde_json::from_value::<Self>(value.clone()) {
            envelope.validate_schema()?;
            return Self::finalize_loaded_instrument(envelope.instrument.into_boxed()?);
        }

        let instrument_json: InstrumentJson = serde_json::from_value(value).map_err(|e| {
            finstack_core::Error::Validation(format!("Failed to parse instrument JSON: {e}"))
        })?;
        Self::finalize_loaded_instrument(instrument_json.into_boxed()?)
    }

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
    pub fn from_reader<R: Read>(reader: R) -> Result<Box<DynInstrument>> {
        let value = serde_json::from_reader(reader)
            .map_err(|e| finstack_core::Error::Validation(format!("Failed to parse JSON: {e}")))?;
        Self::from_value(value)
    }

    /// Load an instrument from a JSON string.
    ///
    /// Convenience wrapper around `from_reader`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Box<DynInstrument>> {
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
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Box<DynInstrument>> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "Failed to open instrument JSON file '{}': {e}",
                path.display()
            ))
        })?;
        Self::from_reader(file)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use rust_decimal::Decimal;

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
                    "issue_date": "2024-01-01",
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
    fn test_envelope_from_value_accepts_bare_tagged_instrument() {
        let bond = Bond::fixed(
            "TEST-BARE",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date"),
            Date::from_calendar_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let value =
            serde_json::to_value(InstrumentJson::Bond(bond)).expect("serialize tagged instrument");
        let instrument = InstrumentEnvelope::from_value(value)
            .expect("Tagged instrument payload should deserialize without envelope");

        assert_eq!(instrument.id(), "TEST-BARE");
    }

    #[test]
    fn test_envelope_from_instrument_serializes_supported_types() {
        let bond = Bond::fixed(
            "TEST-ENVELOPE",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date"),
            Date::from_calendar_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .expect("Bond::fixed should succeed with valid parameters");

        let envelope = InstrumentEnvelope::from_instrument(&bond)
            .expect("Bond should participate in the instrument JSON registry");

        assert_eq!(envelope.schema, InstrumentEnvelope::CURRENT_SCHEMA);
        match envelope.instrument {
            InstrumentJson::Bond(serialized) => assert_eq!(serialized.id, bond.id),
            _ => panic!("Expected Bond variant"),
        }
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
                    "issue_date": "2024-01-01",
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
                    "issue_date": "2024-01-01",
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

    #[test]
    fn test_unknown_variant_error_lists_all_supported_dispatch_tags() {
        let err = serde_json::from_value::<InstrumentJson>(serde_json::json!({
            "type": "totally_unknown_instrument",
            "spec": {}
        }))
        .expect_err("unknown type should fail");
        let message = err.to_string();

        assert!(
            message.contains("commodity_swaption"),
            "unknown variant message should list commodity_swaption, got: {message}"
        );
        assert!(
            message.contains("commodity_spread_option"),
            "unknown variant message should list commodity_spread_option, got: {message}"
        );
        assert!(
            message.contains("equity_trs"),
            "unknown variant message should list equity_trs alias, got: {message}"
        );
        assert!(
            message.contains("fi_trs"),
            "unknown variant message should list fi_trs alias, got: {message}"
        );
        assert!(
            message.contains("fixed_income_trs"),
            "unknown variant message should list fixed_income_trs alias, got: {message}"
        );
    }

    #[test]
    fn test_trs_alias_tags_deserialize_to_expected_variants() {
        let equity_trs =
            EquityTotalReturnSwap::example().expect("Equity TRS example should be valid");
        let equity_trs_id = equity_trs.id.clone();
        let mut equity_trs_json = serde_json::to_value(InstrumentJson::TrsEquity(equity_trs))
            .expect("Equity TRS JSON serialization should succeed");
        equity_trs_json["type"] = serde_json::Value::String("equity_trs".to_string());

        match serde_json::from_value::<InstrumentJson>(equity_trs_json)
            .expect("equity_trs alias should deserialize")
        {
            InstrumentJson::TrsEquity(instrument) => assert_eq!(instrument.id, equity_trs_id),
            _ => panic!("Expected TrsEquity variant"),
        }

        let fi_trs = FIIndexTotalReturnSwap::example().expect("FI TRS example should be valid");
        let fi_trs_id = fi_trs.id.clone();
        let fi_trs_json = serde_json::to_value(InstrumentJson::TrsFixedIncomeIndex(fi_trs))
            .expect("FI TRS JSON serialization should succeed");

        for alias in ["fi_trs", "fixed_income_trs"] {
            let mut alias_json = fi_trs_json.clone();
            alias_json["type"] = serde_json::Value::String(alias.to_string());

            match serde_json::from_value::<InstrumentJson>(alias_json)
                .expect("FI TRS alias should deserialize")
            {
                InstrumentJson::TrsFixedIncomeIndex(instrument) => {
                    assert_eq!(instrument.id, fi_trs_id)
                }
                _ => panic!("Expected TrsFixedIncomeIndex variant"),
            }
        }
    }

    #[test]
    fn test_structured_credit_deserializes_into_boxed_variant() {
        let structured_credit = StructuredCredit::example();
        let structured_credit_id = structured_credit.id.clone();
        let json = serde_json::to_value(InstrumentJson::StructuredCredit(Box::new(
            structured_credit,
        )))
        .expect("StructuredCredit JSON serialization should succeed");

        match serde_json::from_value::<InstrumentJson>(json)
            .expect("StructuredCredit JSON deserialization should succeed")
        {
            InstrumentJson::StructuredCredit(instrument) => {
                assert_eq!(instrument.id, structured_credit_id)
            }
            _ => panic!("Expected StructuredCredit variant"),
        }
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
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};

        let start = Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let primary_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            discount_curve_id: CurveId::new("USD-OIS"),
            start,
            end,
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USGS".to_string()),
            stub: StubKind::ShortFront,
            spread_bp: Decimal::from(5),
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let reference_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            discount_curve_id: CurveId::new("USD-OIS"),
            start,
            end,
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USGS".to_string()),
            stub: StubKind::ShortFront,
            spread_bp: Decimal::ZERO,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let swap = BasisSwap::new(
            "BASIS-TEST",
            Money::new(10_000_000.0, Currency::USD),
            primary_leg,
            reference_leg,
        )
        .expect("BasisSwap construction should succeed in test");

        let json = InstrumentJson::BasisSwap(swap.clone());
        let serialized =
            serde_json::to_string(&json).expect("JSON serialization should succeed in test");
        let deserialized: InstrumentJson =
            serde_json::from_str(&serialized).expect("JSON deserialization should succeed in test");

        match deserialized {
            InstrumentJson::BasisSwap(i) => {
                assert_eq!(i.id, swap.id);
                assert_eq!(
                    i.primary_leg.discount_curve_id,
                    swap.primary_leg.discount_curve_id
                );
                assert_eq!(i.primary_leg.calendar_id.as_deref(), Some("USGS"));
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
                assert_eq!(i.base_currency, fx_spot.base_currency);
                assert_eq!(i.quote_currency, fx_spot.quote_currency);
                assert_eq!(i.base_calendar_id.as_deref(), Some("TARGET"));
                assert_eq!(i.quote_calendar_id.as_deref(), Some("USNY"));
            }
            _ => panic!("Expected FxSpot variant"),
        }
    }

    fn remove_spec_key(value: &mut serde_json::Value, key: &str) {
        value
            .get_mut("spec")
            .and_then(serde_json::Value::as_object_mut)
            .expect("InstrumentJson should serialize with an object spec")
            .remove(key);
    }

    #[test]
    fn test_parse_tagged_value_accepts_yoy_alias() {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};

        let instrument = YoYInflationSwap::builder()
            .id("YOY-ALIAS".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid test date"))
            .maturity(Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date"))
            .fixed_rate(rust_decimal::Decimal::new(250, 4))
            .frequency(Tenor::annual())
            .inflation_index_id("US-CPI".into())
            .discount_curve_id("USD-OIS".into())
            .day_count(DayCount::Act365F)
            .side(PayReceive::Pay)
            .bdc(BusinessDayConvention::ModifiedFollowing)
            .attributes(Attributes::new())
            .build()
            .expect("YoYInflationSwap builder should succeed");
        let value = serde_json::json!({
            "type": "yo_y_inflation_swap",
            "spec": serde_json::to_value(&instrument)
                .expect("YoYInflationSwap JSON serialization should succeed"),
        });

        let parsed = parse_tagged_value(value).expect("alias should deserialize");
        match parsed {
            InstrumentJson::YoYInflationSwap(i) => {
                assert_eq!(i.id, instrument.id);
            }
            _ => panic!("Expected YoYInflationSwap variant"),
        }
    }

    #[test]
    fn test_parse_tagged_value_rejects_unknown_type() {
        let err = parse_tagged_value(serde_json::json!({
            "type": "definitely_not_real",
            "spec": {}
        }))
        .expect_err("unknown type should fail");

        assert!(err.is_data(), "expected data error, got {err}");
    }

    #[test]
    fn test_basis_swap_defaults_when_optional_fields_omitted() {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};

        let start = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");

        let primary_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            discount_curve_id: CurveId::new("USD-OIS"),
            start,
            end,
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::ShortFront,
            spread_bp: Decimal::from(5),
            payment_lag_days: 0,
            reset_lag_days: 0,
        };
        let reference_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            discount_curve_id: CurveId::new("USD-OIS"),
            start,
            end,
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::ShortFront,
            spread_bp: Decimal::ZERO,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };
        let swap = BasisSwap::new(
            "BASIS-DEFAULTS",
            Money::new(10_000_000.0, Currency::USD),
            primary_leg,
            reference_leg,
        )
        .expect("BasisSwap construction should succeed in test");
        let mut json = serde_json::to_value(InstrumentJson::BasisSwap(swap))
            .expect("BasisSwap JSON serialization should succeed");
        remove_spec_key(&mut json, "allow_calendar_fallback");
        remove_spec_key(&mut json, "allow_same_curve");

        let deserialized: InstrumentJson =
            serde_json::from_value(json).expect("BasisSwap JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::BasisSwap(i) => {
                assert!(!i.allow_calendar_fallback);
                assert!(!i.allow_same_curve);
                assert_eq!(i.primary_leg.bdc, BusinessDayConvention::ModifiedFollowing);
                assert_eq!(i.primary_leg.stub, StubKind::ShortFront);
            }
            _ => panic!("Expected BasisSwap variant"),
        }
    }

    #[test]
    fn test_interest_rate_option_defaults_when_optional_fields_omitted() {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};

        let option = InterestRateOption::new_cap(
            InstrumentId::new("IROPT-DEFAULTS"),
            Money::new(1_000_000.0, Currency::USD),
            0.03,
            Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date"),
            Date::from_calendar_date(2028, Month::January, 1).expect("Valid test date"),
            Tenor::quarterly(),
            DayCount::Act360,
            CurveId::new("USD-OIS"),
            CurveId::new("USD-SOFR-3M"),
            CurveId::new("USD-CAPFLOOR-VOL"),
        )
        .expect("valid strike");
        let mut json = serde_json::to_value(InstrumentJson::InterestRateOption(option))
            .expect("InterestRateOption JSON serialization should succeed");
        remove_spec_key(&mut json, "stub");
        remove_spec_key(&mut json, "bdc");

        let deserialized: InstrumentJson = serde_json::from_value(json)
            .expect("InterestRateOption JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::InterestRateOption(i) => {
                assert_eq!(i.stub, StubKind::ShortFront);
                assert_eq!(i.bdc, BusinessDayConvention::ModifiedFollowing);
            }
            _ => panic!("Expected InterestRateOption variant"),
        }
    }

    #[test]
    fn test_repo_default_bdc_when_omitted() {
        use finstack_core::dates::BusinessDayConvention;

        let repo = Repo::term(
            "REPO-DEFAULTS",
            Money::new(10_000_000.0, Currency::USD),
            CollateralSpec::new("UST-10Y", 1000.0, "UST_10Y_PRICE"),
            0.0525,
            Date::from_calendar_date(2025, Month::January, 2).expect("Valid test date"),
            Date::from_calendar_date(2025, Month::January, 9).expect("Valid test date"),
            CurveId::new("USD-OIS"),
        )
        .expect("Repo::term should succeed for test setup");
        let mut json = serde_json::to_value(InstrumentJson::Repo(repo))
            .expect("Repo JSON serialization should succeed");
        remove_spec_key(&mut json, "bdc");

        let deserialized: InstrumentJson =
            serde_json::from_value(json).expect("Repo JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::Repo(i) => {
                assert_eq!(i.bdc, BusinessDayConvention::ModifiedFollowing);
            }
            _ => panic!("Expected Repo variant"),
        }
    }

    #[test]
    fn test_inflation_linked_bond_defaults_when_optional_fields_omitted() {
        use finstack_core::dates::{BusinessDayConvention, StubKind};

        let bond = InflationLinkedBond::example();
        let mut json = serde_json::to_value(InstrumentJson::InflationLinkedBond(bond))
            .expect("InflationLinkedBond JSON serialization should succeed");
        remove_spec_key(&mut json, "bdc");
        remove_spec_key(&mut json, "stub");

        let deserialized: InstrumentJson = serde_json::from_value(json)
            .expect("InflationLinkedBond JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::InflationLinkedBond(i) => {
                assert_eq!(i.bdc, BusinessDayConvention::ModifiedFollowing);
                assert_eq!(i.stub, StubKind::ShortFront);
            }
            _ => panic!("Expected InflationLinkedBond variant"),
        }
    }

    #[test]
    fn test_cds_tranche_default_bdc_when_omitted() {
        use finstack_core::dates::BusinessDayConvention;

        let tranche = CDSTranche::example();
        let mut json = serde_json::to_value(InstrumentJson::CDSTranche(tranche))
            .expect("CDSTranche JSON serialization should succeed");
        remove_spec_key(&mut json, "bdc");

        let deserialized: InstrumentJson =
            serde_json::from_value(json).expect("CDSTranche JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::CDSTranche(i) => {
                assert_eq!(i.bdc, BusinessDayConvention::ModifiedFollowing);
            }
            _ => panic!("Expected CDSTranche variant"),
        }
    }

    #[test]
    fn test_fx_spot_default_bdc_when_omitted() {
        use finstack_core::dates::BusinessDayConvention;

        let spot = FxSpot::new(
            InstrumentId::new("EURUSD-DEFAULTS"),
            Currency::EUR,
            Currency::USD,
        );
        let mut json = serde_json::to_value(InstrumentJson::FxSpot(spot))
            .expect("FxSpot JSON serialization should succeed");
        remove_spec_key(&mut json, "bdc");

        let deserialized: InstrumentJson =
            serde_json::from_value(json).expect("FxSpot JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::FxSpot(i) => {
                assert_eq!(i.bdc, BusinessDayConvention::ModifiedFollowing);
            }
            _ => panic!("Expected FxSpot variant"),
        }
    }

    #[test]
    fn test_cds_option_defaults_when_optional_fields_omitted() {
        let option = CDSOption::example().expect("CDSOption example is valid");
        let mut json = serde_json::to_value(InstrumentJson::CDSOption(option))
            .expect("CDSOption JSON serialization should succeed");
        remove_spec_key(&mut json, "underlying_is_index");
        remove_spec_key(&mut json, "forward_spread_adjust");

        let deserialized: InstrumentJson =
            serde_json::from_value(json).expect("CDSOption JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::CDSOption(i) => {
                assert!(!i.underlying_is_index);
                assert_eq!(i.forward_spread_adjust, rust_decimal::Decimal::ZERO);
            }
            _ => panic!("Expected CDSOption variant"),
        }
    }

    #[test]
    fn test_equity_option_default_discrete_dividends_when_omitted() {
        let option = EquityOption::example().expect("EquityOption example is valid");
        let mut json = serde_json::to_value(InstrumentJson::EquityOption(option))
            .expect("EquityOption JSON serialization should succeed");
        remove_spec_key(&mut json, "discrete_dividends");

        let deserialized: InstrumentJson =
            serde_json::from_value(json).expect("EquityOption JSON deserialization should succeed");
        match deserialized {
            InstrumentJson::EquityOption(i) => {
                assert!(i.discrete_dividends.is_empty());
            }
            _ => panic!("Expected EquityOption variant"),
        }
    }

    /// Canonical list of primary instrument type discriminators.
    ///
    /// This list is derived from the same registry that powers manual
    /// `InstrumentJson` deserialization, so the schema parity test validates the
    /// public primary tags without duplicating that inventory again.
    const CANONICAL_INSTRUMENT_TYPES: &[&str] =
        with_instrument_json_registry!(instrument_json_canonical_types);

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
