//! Instrument bindings for Python.
//!
//! ## WASM Parity Note
//!
//! All logic must stay in Rust to ensure WASM bindings can share the same functionality.
//! These modules only handle type conversion and builder ergonomics - no business logic
//! or financial calculations belong here. Argument parsing has been centralized in
//! `crate::core::common::args` to ensure consistent behavior across all instruments.

mod agency_mbs;
mod asian_option;
mod autocallable;
mod barrier_option;
mod basis_swap;
mod basket;
mod bond;
mod bond_future;
mod cap_floor;
mod cds;
mod cds_index;
mod cds_option;
mod cds_tranche;
mod cliquet_option;
mod cms_option;
mod commodity_forward;
mod commodity_option;
mod commodity_swap;
mod convertible;
mod dcf;
mod deposit;
mod equity;
mod equity_index_future;
mod equity_option;
mod fra;
mod fx;
mod fx_barrier_option;
mod fx_variance_swap;
mod inflation_cap_floor;
mod inflation_linked_bond;
mod inflation_swap;
mod ir_future;
mod irs;
mod levered_real_estate_equity;
mod lookback_option;
mod ndf;
mod private_markets_fund;
mod quanto_option;
mod range_accrual;
mod real_estate;
mod repo;
mod revolving_credit;
mod structured_credit;
mod swaption;
mod term_loan;
mod trs;
mod variance_swap;
mod vol_index_future;
mod vol_index_option;
mod xccy_swap;

// Re-export only used wrappers to avoid unused import lints during clippy
use agency_mbs::{PyAgencyCmo, PyAgencyMbsPassthrough, PyAgencyTba, PyDollarRoll};
use asian_option::PyAsianOption;
use autocallable::PyAutocallable;
use barrier_option::PyBarrierOption;
use basis_swap::PyBasisSwap;
use basket::PyBasket;
use bond::PyBond;
use bond_future::PyBondFuture;
use cap_floor::PyInterestRateOption;
use cds::PyCreditDefaultSwap;
use cds_index::PyCdsIndex;
use cds_option::PyCDSOption;
use cds_tranche::PyCDSTranche;
use cliquet_option::PyCliquetOption;
use cms_option::PyCmsOption;
use commodity_forward::PyCommodityForward;
use commodity_option::PyCommodityOption;
use commodity_swap::PyCommoditySwap;
use convertible::PyConvertibleBond;
use deposit::PyDeposit;
use equity::PyEquity;
use equity_index_future::PyEquityIndexFuture;
use equity_option::PyEquityOption;
use fra::PyForwardRateAgreement;
use fx::{PyFxOption, PyFxSpot, PyFxSwap};
use fx_barrier_option::PyFxBarrierOption;
use fx_variance_swap::PyFxVarianceSwap;
use inflation_cap_floor::PyInflationCapFloor;
use inflation_linked_bond::PyInflationLinkedBond;
use inflation_swap::PyInflationSwap;
use ir_future::PyInterestRateFuture;
use irs::PyInterestRateSwap;
use levered_real_estate_equity::PyLeveredRealEstateEquity;
use lookback_option::PyLookbackOption;
use ndf::PyNdf;
use private_markets_fund::PyPrivateMarketsFund;
use quanto_option::PyQuantoOption;
use range_accrual::PyRangeAccrual;
use real_estate::PyRealEstateAsset;
use repo::PyRepo;
use revolving_credit::PyRevolvingCredit;
use structured_credit::PyStructuredCredit;
use swaption::PySwaption;
use term_loan::PyTermLoan;
use trs::{PyEquityTotalReturnSwap, PyFiIndexTotalReturnSwap};
use variance_swap::PyVarianceSwap;
use vol_index_future::PyVolatilityIndexFuture;
use vol_index_option::PyVolatilityIndexOption;
use xccy_swap::PyCrossCurrencySwap;

use finstack_valuations::instruments::Instrument;
use finstack_valuations::pricer::InstrumentType;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::{Bound, PyRef};
use std::sync::Arc;

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

    let agency_mbs_exports = agency_mbs::register(py, &module)?;
    exports.extend(agency_mbs_exports.iter().copied());

    let bond_exports = bond::register(py, &module)?;
    exports.extend(bond_exports.iter().copied());

    let bond_future_exports = bond_future::register(py, &module)?;
    exports.extend(bond_future_exports.iter().copied());

    let basis_exports = basis_swap::register(py, &module)?;
    exports.extend(basis_exports.iter().copied());

    let deposit_exports = deposit::register(py, &module)?;
    exports.extend(deposit_exports.iter().copied());

    let dcf_exports = dcf::register(py, &module)?;
    exports.extend(dcf_exports.iter().copied());

    let irs_exports = irs::register(py, &module)?;
    exports.extend(irs_exports.iter().copied());

    let fra_exports = fra::register(py, &module)?;
    exports.extend(fra_exports.iter().copied());

    let cap_floor_exports = cap_floor::register(py, &module)?;
    exports.extend(cap_floor_exports.iter().copied());

    let ir_future_exports = ir_future::register(py, &module)?;
    exports.extend(ir_future_exports.iter().copied());

    let swaption_exports = swaption::register(py, &module)?;
    exports.extend(swaption_exports.iter().copied());

    let fx_exports = fx::register(py, &module)?;
    exports.extend(fx_exports.iter().copied());

    let ndf_exports = ndf::register(py, &module)?;
    exports.extend(ndf_exports.iter().copied());

    let fx_variance_swap_exports = fx_variance_swap::register(py, &module)?;
    exports.extend(fx_variance_swap_exports.iter().copied());

    let equity_exports = equity::register(py, &module)?;
    exports.extend(equity_exports.iter().copied());

    let equity_index_future_exports = equity_index_future::register(py, &module)?;
    exports.extend(equity_index_future_exports.iter().copied());

    let equity_option_exports = equity_option::register(py, &module)?;
    exports.extend(equity_option_exports.iter().copied());

    let convertible_exports = convertible::register(py, &module)?;
    exports.extend(convertible_exports.iter().copied());

    let cds_exports = cds::register(py, &module)?;
    exports.extend(cds_exports.iter().copied());

    let cds_index_exports = cds_index::register(py, &module)?;
    exports.extend(cds_index_exports.iter().copied());

    let cds_option_exports = cds_option::register(py, &module)?;
    exports.extend(cds_option_exports.iter().copied());

    let cds_tranche_exports = cds_tranche::register(py, &module)?;
    exports.extend(cds_tranche_exports.iter().copied());

    let repo_exports = repo::register(py, &module)?;
    exports.extend(repo_exports.iter().copied());

    let trs_exports = trs::register(py, &module)?;
    exports.extend(trs_exports.iter().copied());

    let variance_exports = variance_swap::register(py, &module)?;
    exports.extend(variance_exports.iter().copied());

    let ilb_exports = inflation_linked_bond::register(py, &module)?;
    exports.extend(ilb_exports.iter().copied());

    let inflation_swap_exports = inflation_swap::register(py, &module)?;
    exports.extend(inflation_swap_exports.iter().copied());

    let inflation_cap_floor_exports = inflation_cap_floor::register(py, &module)?;
    exports.extend(inflation_cap_floor_exports.iter().copied());

    let xccy_swap_exports = xccy_swap::register(py, &module)?;
    exports.extend(xccy_swap_exports.iter().copied());

    let basket_exports = basket::register(py, &module)?;
    exports.extend(basket_exports.iter().copied());

    let structured_credit_exports = structured_credit::register(py, &module)?;
    exports.extend(structured_credit_exports.iter().copied());

    let pmf_exports = private_markets_fund::register(py, &module)?;
    exports.extend(pmf_exports.iter().copied());

    let asian_option_exports = asian_option::register(py, &module)?;
    exports.extend(asian_option_exports.iter().copied());

    let autocallable_exports = autocallable::register(py, &module)?;
    exports.extend(autocallable_exports.iter().copied());

    let barrier_option_exports = barrier_option::register(py, &module)?;
    exports.extend(barrier_option_exports.iter().copied());

    let cliquet_option_exports = cliquet_option::register(py, &module)?;
    exports.extend(cliquet_option_exports.iter().copied());

    let cms_option_exports = cms_option::register(py, &module)?;
    exports.extend(cms_option_exports.iter().copied());

    commodity_forward::register_module(&module)?;
    exports.push("CommodityForward");
    exports.push("CommodityForwardBuilder");

    commodity_option::register_module(&module)?;
    exports.push("CommodityOption");
    exports.push("CommodityOptionBuilder");

    commodity_swap::register_module(&module)?;
    exports.push("CommoditySwap");
    exports.push("CommoditySwapBuilder");

    real_estate::register_module(&module)?;
    exports.push("RealEstateAsset");

    levered_real_estate_equity::register_module(&module)?;
    exports.push("LeveredRealEstateEquity");

    let fx_barrier_option_exports = fx_barrier_option::register(py, &module)?;
    exports.extend(fx_barrier_option_exports.iter().copied());

    let lookback_option_exports = lookback_option::register(py, &module)?;
    exports.extend(lookback_option_exports.iter().copied());

    let quanto_option_exports = quanto_option::register(py, &module)?;
    exports.extend(quanto_option_exports.iter().copied());

    let range_accrual_exports = range_accrual::register(py, &module)?;
    exports.extend(range_accrual_exports.iter().copied());

    let revolving_credit_exports = revolving_credit::register(py, &module)?;
    exports.extend(revolving_credit_exports.iter().copied());

    let term_loan_exports = term_loan::register(py, &module)?;
    exports.extend(term_loan_exports.iter().copied());

    let vol_index_future_exports = vol_index_future::register(py, &module)?;
    exports.extend(vol_index_future_exports.iter().copied());

    let vol_index_option_exports = vol_index_option::register(py, &module)?;
    exports.extend(vol_index_option_exports.iter().copied());

    exports.sort_unstable();
    exports.dedup();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
