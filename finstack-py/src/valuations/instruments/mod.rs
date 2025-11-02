mod asian_option;
mod autocallable;
mod barrier_option;
mod basis_swap;
mod basket;
mod bond;
mod cap_floor;
mod cds;
mod cds_index;
mod cds_option;
mod cds_tranche;
mod cliquet_option;
mod cms_option;
mod convertible;
mod deposit;
mod equity;
mod equity_option;
mod fra;
mod fx;
mod fx_barrier_option;
mod inflation_linked_bond;
mod inflation_swap;
mod ir_future;
mod irs;
mod lookback_option;
mod private_markets_fund;
mod quanto_option;
mod range_accrual;
mod repo;
mod revolving_credit;
mod structured_credit;
mod swaption;
mod term_loan;
mod trs;
mod variance_swap;

// Re-export only used wrappers to avoid unused import lints during clippy
use asian_option::PyAsianOption;
use autocallable::PyAutocallable;
use barrier_option::PyBarrierOption;
use basis_swap::PyBasisSwap;
use basket::PyBasket;
use bond::PyBond;
use cap_floor::PyInterestRateOption;
use cds::PyCreditDefaultSwap;
use cds_index::PyCdsIndex;
use cds_option::PyCdsOption;
use cds_tranche::PyCdsTranche;
use cliquet_option::PyCliquetOption;
use cms_option::PyCmsOption;
use convertible::PyConvertibleBond;
use deposit::PyDeposit;
use equity::PyEquity;
use equity_option::PyEquityOption;
use fra::PyForwardRateAgreement;
use fx::{PyFxOption, PyFxSpot, PyFxSwap};
use fx_barrier_option::PyFxBarrierOption;
use inflation_linked_bond::PyInflationLinkedBond;
use inflation_swap::PyInflationSwap;
use ir_future::PyInterestRateFuture;
use irs::PyInterestRateSwap;
use lookback_option::PyLookbackOption;
use private_markets_fund::PyPrivateMarketsFund;
use quanto_option::PyQuantoOption;
use range_accrual::PyRangeAccrual;
use repo::PyRepo;
use revolving_credit::PyRevolvingCredit;
use structured_credit::PyStructuredCredit;
use swaption::PySwaption;
use term_loan::PyTermLoan;
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
    if let Ok(obj) = value.extract::<PyRef<PyStructuredCredit>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::StructuredCredit,
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
    if let Ok(obj) = value.extract::<PyRef<PyAsianOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::AsianOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyAutocallable>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::Autocallable,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyBarrierOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::BarrierOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyCliquetOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CliquetOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyCmsOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::CmsOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyFxBarrierOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::FxBarrierOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyLookbackOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::LookbackOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyQuantoOption>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::QuantoOption,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyRangeAccrual>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::RangeAccrual,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyRevolvingCredit>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::RevolvingCredit,
        });
    }
    if let Ok(obj) = value.extract::<PyRef<PyTermLoan>>() {
        return Ok(InstrumentHandle {
            instrument: Box::new(obj.inner.clone()),
            instrument_type: InstrumentType::TermLoan,
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

    exports.sort_unstable();
    exports.dedup();
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
