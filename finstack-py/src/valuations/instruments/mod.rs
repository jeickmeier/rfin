//! Instrument bindings for Python.
//!
//! ## WASM Parity Note
//!
//! All logic must stay in Rust to ensure WASM bindings can share the same functionality.
//! These modules only handle type conversion and builder ergonomics - no business logic
//! or financial calculations belong here. Argument parsing has been centralized in
//! `crate::core::common::args` to ensure consistent behavior across all instruments.

mod commodity;
mod credit_derivatives;
pub(crate) mod equity;
mod exotics;
mod fixed_income;
mod fx;
mod rates;

use commodity::commodity_asian_option::PyCommodityAsianOption;
use commodity::commodity_forward::PyCommodityForward;
use commodity::commodity_option::PyCommodityOption;
use commodity::commodity_swap::PyCommoditySwap;
use credit_derivatives::cds::PyCreditDefaultSwap;
use credit_derivatives::cds_index::PyCdsIndex;
use credit_derivatives::cds_option::PyCDSOption;
use credit_derivatives::cds_tranche::PyCDSTranche;
use equity::autocallable::PyAutocallable;
use equity::cliquet_option::PyCliquetOption;
use equity::equity::PyEquity;
use equity::equity_index_future::PyEquityIndexFuture;
use equity::equity_option::PyEquityOption;
use equity::levered_real_estate_equity::PyLeveredRealEstateEquity;
use equity::private_markets_fund::PyPrivateMarketsFund;
use equity::real_estate::PyRealEstateAsset;
use equity::trs::PyEquityTotalReturnSwap;
use equity::variance_swap::PyVarianceSwap;
use equity::vol_index_future::PyVolatilityIndexFuture;
use equity::vol_index_option::PyVolatilityIndexOption;
use exotics::asian_option::PyAsianOption;
use exotics::barrier_option::PyBarrierOption;
use exotics::basket::PyBasket;
use exotics::lookback_option::PyLookbackOption;
use fixed_income::bond::PyBond;
use fixed_income::bond_future::PyBondFuture;
use fixed_income::cmo::PyAgencyCmo;
use fixed_income::convertible::PyConvertibleBond;
use fixed_income::dollar_roll::PyDollarRoll;
use fixed_income::fi_trs::PyFiIndexTotalReturnSwap;
use fixed_income::inflation_linked_bond::PyInflationLinkedBond;
use fixed_income::mbs_passthrough::PyAgencyMbsPassthrough;
use fixed_income::revolving_credit::PyRevolvingCredit;
use fixed_income::structured_credit::PyStructuredCredit;
use fixed_income::tba::PyAgencyTba;
use fixed_income::term_loan::PyTermLoan;
use fx::fx::{PyFxOption, PyFxSpot, PyFxSwap};
use fx::fx_barrier_option::PyFxBarrierOption;
use fx::fx_digital_option::PyFxDigitalOption;
use fx::fx_forward::PyFxForward;
use fx::fx_touch_option::PyFxTouchOption;
use fx::fx_variance_swap::PyFxVarianceSwap;
use fx::ndf::PyNdf;
use fx::quanto_option::PyQuantoOption;
use rates::basis_swap::PyBasisSwap;
use rates::cap_floor::PyInterestRateOption;
use rates::cms_option::PyCmsOption;
use rates::deposit::PyDeposit;
use rates::fra::PyForwardRateAgreement;
use rates::inflation_cap_floor::PyInflationCapFloor;
use rates::inflation_swap::PyInflationSwap;
use rates::ir_future::PyInterestRateFuture;
use rates::irs::PyInterestRateSwap;
use rates::range_accrual::PyRangeAccrual;
use rates::repo::PyRepo;
use rates::swaption::{PyBermudanSwaption, PySwaption};
use rates::xccy_swap::PyCrossCurrencySwap;

use finstack_valuations::instruments::Instrument;
use finstack_valuations::pricer::InstrumentType;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::{Bound, PyRef};
use std::sync::Arc;

macro_rules! try_downcast_to_py {
    ($inst:expr, $py:expr, $rust_type:ty, $py_type:ident) => {
        if let Some(concrete) = $inst.as_any().downcast_ref::<$rust_type>() {
            let wrapper = $py_type {
                inner: Arc::new(concrete.clone()),
            };
            return Ok(wrapper.into_pyobject($py)?.into_any().unbind());
        }
    };
}

/// Convert an `Arc<dyn Instrument>` back to the appropriate Python wrapper.
pub(crate) fn instrument_to_py(py: Python<'_>, inst: &Arc<dyn Instrument>) -> PyResult<Py<PyAny>> {
    use finstack_valuations::instruments::rates::swaption::BermudanSwaption;
    use finstack_valuations::instruments::*;

    // Fixed income
    try_downcast_to_py!(inst, py, AgencyMbsPassthrough, PyAgencyMbsPassthrough);
    try_downcast_to_py!(inst, py, AgencyTba, PyAgencyTba);
    try_downcast_to_py!(inst, py, DollarRoll, PyDollarRoll);
    try_downcast_to_py!(inst, py, AgencyCmo, PyAgencyCmo);
    try_downcast_to_py!(inst, py, Bond, PyBond);
    try_downcast_to_py!(inst, py, BondFuture, PyBondFuture);
    try_downcast_to_py!(inst, py, ConvertibleBond, PyConvertibleBond);
    try_downcast_to_py!(inst, py, FIIndexTotalReturnSwap, PyFiIndexTotalReturnSwap);
    try_downcast_to_py!(inst, py, InflationLinkedBond, PyInflationLinkedBond);
    try_downcast_to_py!(inst, py, RevolvingCredit, PyRevolvingCredit);
    try_downcast_to_py!(inst, py, StructuredCredit, PyStructuredCredit);
    try_downcast_to_py!(inst, py, TermLoan, PyTermLoan);

    // Rates
    try_downcast_to_py!(inst, py, Deposit, PyDeposit);
    try_downcast_to_py!(inst, py, BasisSwap, PyBasisSwap);
    try_downcast_to_py!(inst, py, ForwardRateAgreement, PyForwardRateAgreement);
    try_downcast_to_py!(inst, py, InterestRateOption, PyInterestRateOption);
    try_downcast_to_py!(inst, py, InterestRateFuture, PyInterestRateFuture);
    try_downcast_to_py!(inst, py, InterestRateSwap, PyInterestRateSwap);
    try_downcast_to_py!(inst, py, Swaption, PySwaption);
    try_downcast_to_py!(inst, py, BermudanSwaption, PyBermudanSwaption);
    try_downcast_to_py!(inst, py, Repo, PyRepo);
    try_downcast_to_py!(inst, py, InflationSwap, PyInflationSwap);
    try_downcast_to_py!(inst, py, InflationCapFloor, PyInflationCapFloor);
    try_downcast_to_py!(inst, py, XccySwap, PyCrossCurrencySwap);
    try_downcast_to_py!(inst, py, CmsOption, PyCmsOption);
    try_downcast_to_py!(inst, py, RangeAccrual, PyRangeAccrual);

    // FX
    try_downcast_to_py!(inst, py, FxSpot, PyFxSpot);
    try_downcast_to_py!(inst, py, FxOption, PyFxOption);
    try_downcast_to_py!(inst, py, FxSwap, PyFxSwap);
    try_downcast_to_py!(inst, py, Ndf, PyNdf);
    try_downcast_to_py!(inst, py, FxVarianceSwap, PyFxVarianceSwap);
    try_downcast_to_py!(inst, py, FxBarrierOption, PyFxBarrierOption);
    try_downcast_to_py!(inst, py, FxDigitalOption, PyFxDigitalOption);
    try_downcast_to_py!(inst, py, FxForward, PyFxForward);
    try_downcast_to_py!(inst, py, FxTouchOption, PyFxTouchOption);
    try_downcast_to_py!(inst, py, QuantoOption, PyQuantoOption);

    // Equity
    try_downcast_to_py!(inst, py, Equity, PyEquity);
    try_downcast_to_py!(inst, py, EquityIndexFuture, PyEquityIndexFuture);
    try_downcast_to_py!(inst, py, EquityOption, PyEquityOption);
    try_downcast_to_py!(inst, py, EquityTotalReturnSwap, PyEquityTotalReturnSwap);
    try_downcast_to_py!(inst, py, VarianceSwap, PyVarianceSwap);
    try_downcast_to_py!(inst, py, PrivateMarketsFund, PyPrivateMarketsFund);
    try_downcast_to_py!(inst, py, RealEstateAsset, PyRealEstateAsset);
    try_downcast_to_py!(inst, py, LeveredRealEstateEquity, PyLeveredRealEstateEquity);
    try_downcast_to_py!(inst, py, Autocallable, PyAutocallable);
    try_downcast_to_py!(inst, py, CliquetOption, PyCliquetOption);
    try_downcast_to_py!(inst, py, VolatilityIndexFuture, PyVolatilityIndexFuture);
    try_downcast_to_py!(inst, py, VolatilityIndexOption, PyVolatilityIndexOption);

    // Exotics
    try_downcast_to_py!(inst, py, Basket, PyBasket);
    try_downcast_to_py!(inst, py, AsianOption, PyAsianOption);
    try_downcast_to_py!(inst, py, BarrierOption, PyBarrierOption);
    try_downcast_to_py!(inst, py, LookbackOption, PyLookbackOption);

    // Commodity
    try_downcast_to_py!(inst, py, CommodityForward, PyCommodityForward);
    try_downcast_to_py!(inst, py, CommodityOption, PyCommodityOption);
    try_downcast_to_py!(inst, py, CommoditySwap, PyCommoditySwap);
    try_downcast_to_py!(inst, py, CommodityAsianOption, PyCommodityAsianOption);

    // Credit derivatives
    try_downcast_to_py!(inst, py, CreditDefaultSwap, PyCreditDefaultSwap);
    try_downcast_to_py!(inst, py, CDSIndex, PyCdsIndex);
    try_downcast_to_py!(inst, py, CDSOption, PyCDSOption);
    try_downcast_to_py!(inst, py, CDSTranche, PyCDSTranche);

    Err(PyTypeError::new_err(format!(
        "Cannot convert instrument '{}' back to Python type",
        inst.id()
    )))
}

/// Borrowed reference to an instrument along with its dispatch key.
pub(crate) struct InstrumentHandle {
    pub instrument: Arc<dyn Instrument>,
    pub instrument_type: InstrumentType,
}

macro_rules! try_extract_arc {
    ($value:expr, $py_type:ty, $inst_type:expr) => {
        if let Ok(obj) = $value.extract::<PyRef<$py_type>>() {
            let inst: Arc<dyn Instrument> = obj.inner.clone();
            return Ok(InstrumentHandle {
                instrument: inst,
                instrument_type: $inst_type,
            });
        }
    };
}

/// Downcast a Python instrument wrapper into a core instrument reference.
pub(crate) fn extract_instrument<'py>(value: &Bound<'py, PyAny>) -> PyResult<InstrumentHandle> {
    // Generic instruments produced by `finstack.valuations.market` builders.
    if let Ok(obj) = value.extract::<PyRef<crate::valuations::market::PyBuiltInstrument>>() {
        return Ok(InstrumentHandle {
            instrument: Arc::from(obj.inner.clone_box()),
            instrument_type: obj.inner.key(),
        });
    }

    try_extract_arc!(
        value,
        PyAgencyMbsPassthrough,
        InstrumentType::AgencyMbsPassthrough
    );
    try_extract_arc!(value, PyAgencyTba, InstrumentType::AgencyTba);
    try_extract_arc!(value, PyDollarRoll, InstrumentType::DollarRoll);
    try_extract_arc!(value, PyAgencyCmo, InstrumentType::AgencyCmo);
    try_extract_arc!(value, PyBond, InstrumentType::Bond);
    try_extract_arc!(value, PyBondFuture, InstrumentType::BondFuture);
    try_extract_arc!(value, PyDeposit, InstrumentType::Deposit);
    try_extract_arc!(value, PyBasisSwap, InstrumentType::BasisSwap);
    try_extract_arc!(value, PyForwardRateAgreement, InstrumentType::FRA);
    try_extract_arc!(value, PyInterestRateOption, InstrumentType::CapFloor);
    try_extract_arc!(
        value,
        PyInterestRateFuture,
        InstrumentType::InterestRateFuture
    );
    try_extract_arc!(value, PyInterestRateSwap, InstrumentType::IRS);
    try_extract_arc!(value, PyFxSpot, InstrumentType::FxSpot);
    try_extract_arc!(value, PyFxOption, InstrumentType::FxOption);
    try_extract_arc!(value, PyFxSwap, InstrumentType::FxSwap);
    try_extract_arc!(value, PyNdf, InstrumentType::Ndf);
    try_extract_arc!(value, PyFxVarianceSwap, InstrumentType::FxVarianceSwap);
    try_extract_arc!(value, PyEquity, InstrumentType::Equity);
    try_extract_arc!(
        value,
        PyEquityIndexFuture,
        InstrumentType::EquityIndexFuture
    );
    try_extract_arc!(value, PyEquityOption, InstrumentType::EquityOption);
    try_extract_arc!(value, PyConvertibleBond, InstrumentType::Convertible);
    try_extract_arc!(value, PySwaption, InstrumentType::Swaption);
    try_extract_arc!(value, PyBermudanSwaption, InstrumentType::BermudanSwaption);
    try_extract_arc!(
        value,
        PyEquityTotalReturnSwap,
        InstrumentType::EquityTotalReturnSwap
    );
    try_extract_arc!(
        value,
        PyFiIndexTotalReturnSwap,
        InstrumentType::FIIndexTotalReturnSwap
    );
    try_extract_arc!(value, PyVarianceSwap, InstrumentType::VarianceSwap);
    try_extract_arc!(value, PyCreditDefaultSwap, InstrumentType::CDS);
    try_extract_arc!(value, PyCdsIndex, InstrumentType::CDSIndex);
    try_extract_arc!(value, PyCDSOption, InstrumentType::CDSOption);
    try_extract_arc!(value, PyCDSTranche, InstrumentType::CDSTranche);
    try_extract_arc!(value, PyCommodityForward, InstrumentType::CommodityForward);
    try_extract_arc!(value, PyCommodityOption, InstrumentType::CommodityOption);
    try_extract_arc!(value, PyCommoditySwap, InstrumentType::CommoditySwap);
    try_extract_arc!(
        value,
        PyCommodityAsianOption,
        InstrumentType::CommodityAsianOption
    );
    try_extract_arc!(value, PyRepo, InstrumentType::Repo);
    try_extract_arc!(
        value,
        PyInflationLinkedBond,
        InstrumentType::InflationLinkedBond
    );
    try_extract_arc!(value, PyInflationSwap, InstrumentType::InflationSwap);
    try_extract_arc!(
        value,
        PyInflationCapFloor,
        InstrumentType::InflationCapFloor
    );
    try_extract_arc!(value, PyCrossCurrencySwap, InstrumentType::XccySwap);
    try_extract_arc!(value, PyStructuredCredit, InstrumentType::StructuredCredit);
    try_extract_arc!(
        value,
        PyPrivateMarketsFund,
        InstrumentType::PrivateMarketsFund
    );
    try_extract_arc!(value, PyRealEstateAsset, InstrumentType::RealEstateAsset);
    try_extract_arc!(
        value,
        PyLeveredRealEstateEquity,
        InstrumentType::LeveredRealEstateEquity
    );
    try_extract_arc!(value, PyBasket, InstrumentType::Basket);
    try_extract_arc!(value, PyAsianOption, InstrumentType::AsianOption);
    try_extract_arc!(value, PyAutocallable, InstrumentType::Autocallable);
    try_extract_arc!(value, PyBarrierOption, InstrumentType::BarrierOption);
    try_extract_arc!(value, PyCliquetOption, InstrumentType::CliquetOption);
    try_extract_arc!(value, PyCmsOption, InstrumentType::CmsOption);
    try_extract_arc!(value, PyFxBarrierOption, InstrumentType::FxBarrierOption);
    try_extract_arc!(value, PyFxDigitalOption, InstrumentType::FxDigitalOption);
    try_extract_arc!(value, PyFxForward, InstrumentType::FxForward);
    try_extract_arc!(value, PyFxTouchOption, InstrumentType::FxTouchOption);
    try_extract_arc!(value, PyLookbackOption, InstrumentType::LookbackOption);
    try_extract_arc!(value, PyQuantoOption, InstrumentType::QuantoOption);
    try_extract_arc!(value, PyRangeAccrual, InstrumentType::RangeAccrual);
    try_extract_arc!(value, PyRevolvingCredit, InstrumentType::RevolvingCredit);
    try_extract_arc!(value, PyTermLoan, InstrumentType::TermLoan);
    try_extract_arc!(
        value,
        PyVolatilityIndexFuture,
        InstrumentType::VolatilityIndexFuture
    );
    try_extract_arc!(
        value,
        PyVolatilityIndexOption,
        InstrumentType::VolatilityIndexOption
    );

    Err(PyTypeError::new_err(
        "Unsupported instrument type; construct instruments from finstack.valuations",
    ))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "instruments")?;
    module.setattr(
        "__doc__",
        "Instrument wrappers for finstack-valuations (rates, FX, credit, equity).",
    )?;

    let mut exports: Vec<&str> = Vec::new();

    let fixed_income_exports = fixed_income::register(py, &module)?;
    exports.extend(fixed_income_exports.iter().copied());

    let rates_exports = rates::register(py, &module)?;
    exports.extend(rates_exports.iter().copied());

    let fx_exports = fx::register(py, &module)?;
    exports.extend(fx_exports.iter().copied());

    let equity_exports = equity::register(py, &module)?;
    exports.extend(equity_exports.iter().copied());

    let exotics_exports = exotics::register(py, &module)?;
    exports.extend(exotics_exports.iter().copied());

    let commodity_exports = commodity::register(py, &module)?;
    exports.extend(commodity_exports.iter().copied());

    let credit_exports = credit_derivatives::register(py, &module)?;
    exports.extend(credit_exports.iter().copied());

    exports.sort_unstable();
    exports.dedup();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
