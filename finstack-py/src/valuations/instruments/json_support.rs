//! Macro for adding JSON serialization/deserialization to Python instrument wrappers.
//!
//! Uses the `InstrumentEnvelope` format for consistent JSON representation.
//! `from_json()` accepts both the versioned envelope format and bare instrument JSON
//! for backward compatibility.

/// Try to parse as an envelope first, then fall back to bare tagged instrument JSON.
pub(super) fn parse_envelope_or_bare(
    json_str: &str,
) -> pyo3::PyResult<finstack_valuations::instruments::InstrumentJson> {
    use finstack_valuations::instruments::{InstrumentEnvelope, InstrumentJson};

    // Try envelope format first.
    if let Ok(envelope) = serde_json::from_str::<InstrumentEnvelope>(json_str) {
        return Ok(envelope.instrument);
    }
    // Fall back to bare tagged instrument JSON.
    if let Ok(instrument) = serde_json::from_str::<InstrumentJson>(json_str) {
        return Ok(instrument);
    }
    Err(pyo3::exceptions::PyValueError::new_err(
        "Invalid instrument JSON: expected envelope {\"schema\":..., \"instrument\":...} or tagged {\"type\":..., \"spec\":...} format",
    ))
}

/// Convert a Python `str` or `dict` into a JSON string for deserialization.
pub(super) fn extract_json_string(data: &pyo3::Bound<'_, pyo3::PyAny>) -> pyo3::PyResult<String> {
    use pyo3::prelude::*;

    if let Ok(s) = data.extract::<String>() {
        return Ok(s);
    }
    if let Ok(dict) = data.cast::<pyo3::types::PyDict>() {
        let py = dict.py();
        let json_mod = pyo3::types::PyModule::import(py, "json")?;
        return json_mod.call_method1("dumps", (dict,))?.extract::<String>();
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected JSON string or dict",
    ))
}

/// Add `to_json()` and `from_json()` methods to a Python instrument wrapper.
///
/// The generated `to_json()` produces the full envelope format:
/// ```json
/// {"schema": "finstack.instrument/1", "instrument": {"type": "bond", "spec": {...}}}
/// ```
///
/// `from_json()` accepts both the envelope format and bare tagged instrument JSON
/// for backward compatibility.
macro_rules! impl_instrument_json {
    ($py_type:ty, $rust_type:ty, $variant:ident) => {
        #[pyo3::pymethods]
        impl $py_type {
            /// Serialize this instrument to a JSON string in envelope format.
            ///
            /// Returns:
            ///     str: JSON string with schema version and tagged instrument spec.
            fn to_json(&self) -> pyo3::PyResult<String> {
                use finstack_valuations::instruments::{InstrumentEnvelope, InstrumentJson};
                let envelope = InstrumentEnvelope {
                    schema: "finstack.instrument/1".to_string(),
                    instrument: InstrumentJson::$variant((*self.inner).clone()),
                };
                serde_json::to_string_pretty(&envelope).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "JSON serialization failed: {e}"
                    ))
                })
            }

            /// Deserialize an instrument from a JSON string or Python dict.
            ///
            /// Accepts the versioned envelope format, bare tagged instrument JSON,
            /// or a Python dictionary.
            ///
            /// Args:
            ///     data: JSON string or dict in envelope or bare tagged format.
            ///
            /// Returns:
            ///     The deserialized instrument.
            ///
            /// Raises:
            ///     ValueError: If JSON is malformed or contains a different instrument type.
            ///     TypeError: If ``data`` is neither a string nor a dict.
            #[classmethod]
            #[pyo3(text_signature = "(cls, data)")]
            fn from_json(
                _cls: &pyo3::Bound<'_, pyo3::types::PyType>,
                data: pyo3::Bound<'_, pyo3::PyAny>,
            ) -> pyo3::PyResult<Self> {
                use finstack_valuations::instruments::InstrumentJson;
                let json_str = super::json_support::extract_json_string(&data)?;
                let instrument = super::json_support::parse_envelope_or_bare(&json_str)?;
                match instrument {
                    InstrumentJson::$variant(inner) => Ok(Self::new(inner)),
                    _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Expected instrument type '{}' in JSON",
                        stringify!($variant)
                    ))),
                }
            }
        }
    };
    // Boxed variant for types wrapped in Box<T> in InstrumentJson
    (boxed: $py_type:ty, $rust_type:ty, $variant:ident) => {
        #[pyo3::pymethods]
        impl $py_type {
            /// Serialize this instrument to a JSON string in envelope format.
            ///
            /// Returns:
            ///     str: JSON string with schema version and tagged instrument spec.
            fn to_json(&self) -> pyo3::PyResult<String> {
                use finstack_valuations::instruments::{InstrumentEnvelope, InstrumentJson};
                let envelope = InstrumentEnvelope {
                    schema: "finstack.instrument/1".to_string(),
                    instrument: InstrumentJson::$variant(Box::new((*self.inner).clone())),
                };
                serde_json::to_string_pretty(&envelope).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "JSON serialization failed: {e}"
                    ))
                })
            }

            /// Deserialize an instrument from a JSON string or Python dict.
            ///
            /// Accepts the versioned envelope format, bare tagged instrument JSON,
            /// or a Python dictionary.
            ///
            /// Args:
            ///     data: JSON string or dict in envelope or bare tagged format.
            ///
            /// Returns:
            ///     The deserialized instrument.
            ///
            /// Raises:
            ///     ValueError: If JSON is malformed or contains a different instrument type.
            ///     TypeError: If ``data`` is neither a string nor a dict.
            #[classmethod]
            #[pyo3(text_signature = "(cls, data)")]
            fn from_json(
                _cls: &pyo3::Bound<'_, pyo3::types::PyType>,
                data: pyo3::Bound<'_, pyo3::PyAny>,
            ) -> pyo3::PyResult<Self> {
                use finstack_valuations::instruments::InstrumentJson;
                let json_str = super::json_support::extract_json_string(&data)?;
                let instrument = super::json_support::parse_envelope_or_bare(&json_str)?;
                match instrument {
                    InstrumentJson::$variant(inner) => Ok(Self::new(*inner)),
                    _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Expected instrument type '{}' in JSON",
                        stringify!($variant)
                    ))),
                }
            }
        }
    };
}

// --- Fixed Income ---
impl_instrument_json!(super::fixed_income::bond::PyBond, Bond, Bond);
impl_instrument_json!(
    super::fixed_income::convertible::PyConvertibleBond,
    ConvertibleBond,
    ConvertibleBond
);
impl_instrument_json!(
    super::fixed_income::inflation_linked_bond::PyInflationLinkedBond,
    InflationLinkedBond,
    InflationLinkedBond
);
impl_instrument_json!(
    super::fixed_income::term_loan::PyTermLoan,
    TermLoan,
    TermLoan
);
impl_instrument_json!(
    super::fixed_income::revolving_credit::PyRevolvingCredit,
    RevolvingCredit,
    RevolvingCredit
);
impl_instrument_json!(boxed: super::fixed_income::bond_future::PyBondFuture, BondFuture, BondFuture);
impl_instrument_json!(
    super::fixed_income::mbs_passthrough::PyAgencyMbsPassthrough,
    AgencyMbsPassthrough,
    AgencyMbsPassthrough
);
impl_instrument_json!(super::fixed_income::tba::PyAgencyTba, AgencyTba, AgencyTba);
impl_instrument_json!(super::fixed_income::cmo::PyAgencyCmo, AgencyCmo, AgencyCmo);
impl_instrument_json!(
    super::fixed_income::dollar_roll::PyDollarRoll,
    DollarRoll,
    DollarRoll
);
impl_instrument_json!(boxed: super::fixed_income::structured_credit::PyStructuredCredit, StructuredCredit, StructuredCredit);
impl_instrument_json!(
    super::fixed_income::fi_trs::PyFiIndexTotalReturnSwap,
    FIIndexTotalReturnSwap,
    TrsFixedIncomeIndex
);

// --- Rates ---
impl_instrument_json!(
    super::rates::irs::PyInterestRateSwap,
    InterestRateSwap,
    InterestRateSwap
);
impl_instrument_json!(super::rates::basis_swap::PyBasisSwap, BasisSwap, BasisSwap);
impl_instrument_json!(
    super::rates::xccy_swap::PyCrossCurrencySwap,
    XccySwap,
    XccySwap
);
impl_instrument_json!(
    super::rates::inflation_swap::PyInflationSwap,
    InflationSwap,
    InflationSwap
);
impl_instrument_json!(
    super::rates::inflation_swap::PyYoYInflationSwap,
    YoYInflationSwap,
    YoYInflationSwap
);
impl_instrument_json!(
    super::rates::inflation_cap_floor::PyInflationCapFloor,
    InflationCapFloor,
    InflationCapFloor
);
impl_instrument_json!(
    super::rates::fra::PyForwardRateAgreement,
    ForwardRateAgreement,
    ForwardRateAgreement
);
impl_instrument_json!(super::rates::swaption::PySwaption, Swaption, Swaption);
impl_instrument_json!(
    super::rates::ir_future::PyInterestRateFuture,
    InterestRateFuture,
    InterestRateFuture
);
impl_instrument_json!(
    super::rates::cap_floor::PyInterestRateOption,
    InterestRateOption,
    InterestRateOption
);
impl_instrument_json!(super::rates::cms_swap::PyCmsSwap, CmsSwap, CmsSwap);
impl_instrument_json!(super::rates::cms_option::PyCmsOption, CmsOption, CmsOption);
impl_instrument_json!(
    super::rates::ir_future_option::PyIrFutureOption,
    IrFutureOption,
    IrFutureOption
);
impl_instrument_json!(super::rates::deposit::PyDeposit, Deposit, Deposit);
impl_instrument_json!(super::rates::repo::PyRepo, Repo, Repo);
impl_instrument_json!(
    super::rates::range_accrual::PyRangeAccrual,
    RangeAccrual,
    RangeAccrual
);

// --- Credit ---
impl_instrument_json!(
    super::credit_derivatives::cds::PyCreditDefaultSwap,
    CreditDefaultSwap,
    CreditDefaultSwap
);
impl_instrument_json!(
    super::credit_derivatives::cds_index::PyCdsIndex,
    CDSIndex,
    CDSIndex
);
impl_instrument_json!(
    super::credit_derivatives::cds_tranche::PyCDSTranche,
    CDSTranche,
    CDSTranche
);
impl_instrument_json!(
    super::credit_derivatives::cds_option::PyCDSOption,
    CDSOption,
    CDSOption
);

// --- Equity ---
impl_instrument_json!(super::equity::equity::PyEquity, Equity, Equity);
impl_instrument_json!(
    super::equity::equity_option::PyEquityOption,
    EquityOption,
    EquityOption
);
impl_instrument_json!(
    super::equity::variance_swap::PyVarianceSwap,
    VarianceSwap,
    VarianceSwap
);
impl_instrument_json!(
    super::equity::equity_index_future::PyEquityIndexFuture,
    EquityIndexFuture,
    EquityIndexFuture
);
impl_instrument_json!(
    super::equity::vol_index_future::PyVolatilityIndexFuture,
    VolatilityIndexFuture,
    VolatilityIndexFuture
);
impl_instrument_json!(
    super::equity::vol_index_option::PyVolatilityIndexOption,
    VolatilityIndexOption,
    VolatilityIndexOption
);
impl_instrument_json!(
    super::equity::trs::PyEquityTotalReturnSwap,
    EquityTotalReturnSwap,
    TrsEquity
);
impl_instrument_json!(
    super::equity::autocallable::PyAutocallable,
    Autocallable,
    Autocallable
);
impl_instrument_json!(
    super::equity::cliquet_option::PyCliquetOption,
    CliquetOption,
    CliquetOption
);
impl_instrument_json!(
    super::equity::private_markets_fund::PyPrivateMarketsFund,
    PrivateMarketsFund,
    PrivateMarketsFund
);
impl_instrument_json!(
    super::equity::real_estate::PyRealEstateAsset,
    RealEstateAsset,
    RealEstateAsset
);
impl_instrument_json!(boxed: super::equity::levered_real_estate_equity::PyLeveredRealEstateEquity, LeveredRealEstateEquity, LeveredRealEstateEquity);
impl_instrument_json!(
    super::equity::dcf::PyDiscountedCashFlow,
    DiscountedCashFlow,
    DiscountedCashFlow
);

// --- Exotics ---
impl_instrument_json!(
    super::exotics::asian_option::PyAsianOption,
    AsianOption,
    AsianOption
);
impl_instrument_json!(
    super::exotics::barrier_option::PyBarrierOption,
    BarrierOption,
    BarrierOption
);
impl_instrument_json!(
    super::exotics::lookback_option::PyLookbackOption,
    LookbackOption,
    LookbackOption
);
impl_instrument_json!(super::exotics::basket::PyBasket, Basket, Basket);

// --- FX ---
impl_instrument_json!(super::fx::fx::PyFxSpot, FxSpot, FxSpot);
impl_instrument_json!(super::fx::fx::PyFxSwap, FxSwap, FxSwap);
impl_instrument_json!(super::fx::fx::PyFxOption, FxOption, FxOption);
impl_instrument_json!(super::fx::fx_forward::PyFxForward, FxForward, FxForward);
impl_instrument_json!(super::fx::ndf::PyNdf, Ndf, Ndf);
impl_instrument_json!(
    super::fx::fx_digital_option::PyFxDigitalOption,
    FxDigitalOption,
    FxDigitalOption
);
impl_instrument_json!(
    super::fx::fx_touch_option::PyFxTouchOption,
    FxTouchOption,
    FxTouchOption
);
impl_instrument_json!(
    super::fx::fx_barrier_option::PyFxBarrierOption,
    FxBarrierOption,
    FxBarrierOption
);
impl_instrument_json!(
    super::fx::fx_variance_swap::PyFxVarianceSwap,
    FxVarianceSwap,
    FxVarianceSwap
);
impl_instrument_json!(
    super::fx::quanto_option::PyQuantoOption,
    QuantoOption,
    QuantoOption
);

// --- Commodity ---
impl_instrument_json!(
    super::commodity::commodity_option::PyCommodityOption,
    CommodityOption,
    CommodityOption
);
impl_instrument_json!(
    super::commodity::commodity_asian_option::PyCommodityAsianOption,
    CommodityAsianOption,
    CommodityAsianOption
);
impl_instrument_json!(
    super::commodity::commodity_forward::PyCommodityForward,
    CommodityForward,
    CommodityForward
);
impl_instrument_json!(
    super::commodity::commodity_swap::PyCommoditySwap,
    CommoditySwap,
    CommoditySwap
);
impl_instrument_json!(
    super::commodity::commodity_swaption::PyCommoditySwaption,
    CommoditySwaption,
    CommoditySwaption
);
impl_instrument_json!(
    super::commodity::commodity_spread_option::PyCommoditySpreadOption,
    CommoditySpreadOption,
    CommoditySpreadOption
);
