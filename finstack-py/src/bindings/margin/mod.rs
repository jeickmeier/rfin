//! Python bindings for the `finstack-margin` crate.
//!
//! Exposes variation/initial margin calculators, CSA specifications,
//! collateral types, XVA configuration/results, and margin metrics.

mod calculators;
mod metrics;
mod regulatory;
mod types;
mod xva;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `margin` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "margin")?;
    m.setattr(
        "__doc__",
        "Margin and collateral: VM/IM calculators, CSA specifications, XVA, metrics.",
    )?;

    types::register(py, &m)?;
    calculators::register(py, &m)?;
    xva::register(py, &m)?;
    metrics::register(py, &m)?;
    regulatory::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            // Types & enums
            "ImMethodology",
            "MarginTenor",
            "MarginCallType",
            "ClearingStatus",
            "CollateralAssetClass",
            "NettingSetId",
            "CsaSpec",
            "EligibleCollateralSchedule",
            "CONSTANTS",
            // Calculators
            "VmResult",
            "VmCalculator",
            "ImResult",
            // XVA
            "FundingConfig",
            "XvaConfig",
            "ExposureDiagnostics",
            "ExposureProfile",
            "XvaResult",
            "CsaTerms",
            "XvaNettingSet",
            // Metrics
            "MarginUtilization",
            "ExcessCollateral",
            "MarginFundingCost",
            "Haircut01",
            // Regulatory (FRTB SBA + SA-CCR)
            "FrtbSensitivities",
            "SaCcrTrade",
            "frtb_sba_charge",
            "saccr_ead",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let parent_name: String = match parent.getattr("__name__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.finstack".to_string(),
        },
        Err(_) => "finstack.finstack".to_string(),
    };
    let qual = format!("{parent_name}.margin");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
