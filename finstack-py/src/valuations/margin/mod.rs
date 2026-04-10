//! Margin type bindings.
//!
//! This module exposes the core margin types from `finstack-margin` as thin
//! Python wrappers. No Python-side margin logic is implemented here.

mod calculators;
mod classification;
mod csa;
mod helpers;
mod results;
mod specs;

pub(crate) use calculators::{PySimmCalculator, PySimmVersion, PyVmCalculator};
pub(crate) use classification::{
    PyClearingStatus, PyCollateralAssetClass, PyMarginCall, PyMarginCallType, PyRepoMarginType,
    PySimmRiskClass,
};
pub(crate) use csa::{
    PyCsaSpec, PyEligibleCollateralSchedule, PyImMethodology, PyImParameters, PyMarginCallTiming,
    PyMarginTenor, PyVmParameters,
};
pub(crate) use results::{PyImResult, PyInstrumentMarginResult, PySimmSensitivities, PyVmResult};
pub(crate) use specs::{PyOtcMarginSpec, PyRepoMarginSpec};

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Register margin type exports.
pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "margin")?;
    module.setattr(
        "__doc__",
        "Margin and collateral management types and calculators (CSA specs, VM/IM parameters, collateral schedules, margin calls, SIMM, repo margin).",
    )?;

    module.add_class::<PyMarginTenor>()?;
    module.add_class::<PyImMethodology>()?;
    module.add_class::<PyMarginCallTiming>()?;
    module.add_class::<PyVmParameters>()?;
    module.add_class::<PyImParameters>()?;
    module.add_class::<PyEligibleCollateralSchedule>()?;
    module.add_class::<PyCsaSpec>()?;
    module.add_class::<PyMarginCallType>()?;
    module.add_class::<PyCollateralAssetClass>()?;
    module.add_class::<PyClearingStatus>()?;
    module.add_class::<PySimmRiskClass>()?;
    module.add_class::<PyRepoMarginType>()?;
    module.add_class::<PyMarginCall>()?;
    module.add_class::<PyVmResult>()?;
    module.add_class::<PyImResult>()?;
    module.add_class::<PyInstrumentMarginResult>()?;
    module.add_class::<PySimmSensitivities>()?;
    module.add_class::<PyOtcMarginSpec>()?;
    module.add_class::<PyRepoMarginSpec>()?;
    module.add_class::<PyVmCalculator>()?;
    module.add_class::<PySimmVersion>()?;
    module.add_class::<PySimmCalculator>()?;

    let exports = [
        "MarginTenor",
        "ImMethodology",
        "MarginCallTiming",
        "VmParameters",
        "ImParameters",
        "EligibleCollateralSchedule",
        "CsaSpec",
        "MarginCallType",
        "CollateralAssetClass",
        "ClearingStatus",
        "SimmRiskClass",
        "RepoMarginType",
        "MarginCall",
        "VmResult",
        "ImResult",
        "InstrumentMarginResult",
        "SimmSensitivities",
        "OtcMarginSpec",
        "RepoMarginSpec",
        "VmCalculator",
        "SimmVersion",
        "SimmCalculator",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("margin", &module)?;
    Ok(exports.to_vec())
}
