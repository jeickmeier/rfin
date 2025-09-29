mod abs;
mod basis_swap;
mod basket;
mod bond;
mod cap_floor;
mod cds;
mod cds_index;
mod cds_option;
mod cds_tranche;
mod clo;
mod cmbs;
mod convertible;
mod deposit;
mod equity;
mod equity_option;
mod fra;
mod fx;
mod inflation_linked_bond;
mod inflation_swap;
mod ir_future;
mod irs;
mod private_markets_fund;
mod repo;
mod rmbs;
mod swaption;
mod trs;
mod variance_swap;

// Re-export only used wrappers to avoid unused import lints during clippy
use abs::PyAbs;
use basis_swap::PyBasisSwap;
use basket::PyBasket;
use bond::PyBond;
use cap_floor::PyInterestRateOption;
use cds::PyCreditDefaultSwap;
use cds_index::PyCdsIndex;
use cds_option::PyCdsOption;
use cds_tranche::PyCdsTranche;
use clo::PyClo;
use cmbs::PyCmbs;
use convertible::PyConvertibleBond;
use deposit::PyDeposit;
use equity::PyEquity;
use equity_option::PyEquityOption;
use fra::PyForwardRateAgreement;
use fx::{PyFxOption, PyFxSpot, PyFxSwap};
use inflation_linked_bond::PyInflationLinkedBond;
use inflation_swap::PyInflationSwap;
use ir_future::PyInterestRateFuture;
use irs::PyInterestRateSwap;
use private_markets_fund::PyPrivateMarketsFund;
use repo::PyRepo;
use rmbs::PyRmbs;
use swaption::PySwaption;
use trs::{PyEquityTotalReturnSwap, PyFiIndexTotalReturnSwap};
use variance_swap::PyVarianceSwap;

use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::pricer::InstrumentType;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::{Bound, PyRef};

/// Borrowed reference to an instrument along with its dispatch key.
pub(crate) struct InstrumentHandle {
    pub instrument: Box<dyn Instrument>,
    pub instrument_type: InstrumentType,
}

// No generic constructor; map each wrapper explicitly to its inner instrument

/// Downcast a Python instrument wrapper into a core instrument reference.
pub(crate) fn extract_instrument<'py>(value: &Bound<'py, PyAny>) -> PyResult<InstrumentHandle> {
    if let Ok(obj) = value.extract::<PyRef<PyBond>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Bond,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyDeposit>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Deposit,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyBasisSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::BasisSwap,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyForwardRateAgreement>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::FRA,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyInterestRateOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CapFloor,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyInterestRateFuture>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::InterestRateFuture,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyInterestRateSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::IRS,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyFxSpot>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::FxSpot,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyFxOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::FxOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyFxSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::FxSwap,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyEquity>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Equity,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyEquityOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::EquityOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyConvertibleBond>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Convertible,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PySwaption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Swaption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyEquityTotalReturnSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::TRS,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyFiIndexTotalReturnSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::TRS,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyVarianceSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::VarianceSwap,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyCreditDefaultSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CDS,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyCdsIndex>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CDSIndex,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyCdsOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CDSOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyCdsTranche>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CDSTranche,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyRepo>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Repo,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyInflationLinkedBond>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::InflationLinkedBond,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyInflationSwap>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::InflationSwap,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyAbs>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::ABS,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyClo>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CLO,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyCmbs>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CMBS,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyRmbs>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::RMBS,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyPrivateMarketsFund>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::PrivateMarketsFund,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyBasket>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Basket,
        });
    }
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

    let bond_exports = bond::register(py, &module)?;
    exports.extend(bond_exports.iter().copied());

    let basis_exports = basis_swap::register(py, &module)?;
    exports.extend(basis_exports.iter().copied());

    let deposit_exports = deposit::register(py, &module)?;
    exports.extend(deposit_exports.iter().copied());

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

    let equity_exports = equity::register(py, &module)?;
    exports.extend(equity_exports.iter().copied());

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

    let basket_exports = basket::register(py, &module)?;
    exports.extend(basket_exports.iter().copied());

    let abs_exports = abs::register(py, &module)?;
    exports.extend(abs_exports.iter().copied());

    let clo_exports = clo::register(py, &module)?;
    exports.extend(clo_exports.iter().copied());

    let cmbs_exports = cmbs::register(py, &module)?;
    exports.extend(cmbs_exports.iter().copied());

    let rmbs_exports = rmbs::register(py, &module)?;
    exports.extend(rmbs_exports.iter().copied());

    let pmf_exports = private_markets_fund::register(py, &module)?;
    exports.extend(pmf_exports.iter().copied());

    exports.sort_unstable();
    exports.dedup();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
