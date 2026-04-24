//! Python bindings for `finstack_core::credit::pd` (calibration subset).

use finstack_core::credit::pd::{central_tendency, pit_to_ttc, ttc_to_pit, PdCycleParams};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

use crate::errors::display_to_py;

// ---------------------------------------------------------------------------
// PiT / TtC conversion
// ---------------------------------------------------------------------------

/// Convert a Point-in-Time PD to a Through-the-Cycle PD.
///
/// Uses the Merton-Vasicek single-factor model (Basel II IRB):
///
///   PD_TtC = Phi( Phi^{-1}(PD_PiT) * sqrt(1 - rho) + sqrt(rho) * z )
///
/// Arguments:
///     pit_pd: Point-in-Time PD in (0, 1).
///     asset_correlation: Asset correlation rho in (0, 1). Basel uses 0.12 - 0.24 for corporates.
///     cycle_index: Systematic risk factor z. 0 = average, < 0 = downturn, > 0 = benign.
#[pyfunction]
#[pyo3(text_signature = "(pit_pd, asset_correlation, cycle_index)")]
fn pit_to_ttc_py(pit_pd: f64, asset_correlation: f64, cycle_index: f64) -> PyResult<f64> {
    let params = PdCycleParams {
        asset_correlation,
        cycle_index,
    };
    pit_to_ttc(pit_pd, &params).map_err(display_to_py)
}

/// Convert a Through-the-Cycle PD to a Point-in-Time PD.
///
/// Uses the Merton-Vasicek single-factor model (Basel II IRB):
///
///   PD_PiT = Phi( (Phi^{-1}(PD_TtC) - sqrt(rho) * z) / sqrt(1 - rho) )
///
/// Arguments:
///     ttc_pd: Through-the-Cycle PD in (0, 1).
///     asset_correlation: Asset correlation rho in (0, 1).
///     cycle_index: Systematic risk factor z. 0 = average, < 0 = downturn, > 0 = benign.
#[pyfunction]
#[pyo3(text_signature = "(ttc_pd, asset_correlation, cycle_index)")]
fn ttc_to_pit_py(ttc_pd: f64, asset_correlation: f64, cycle_index: f64) -> PyResult<f64> {
    let params = PdCycleParams {
        asset_correlation,
        cycle_index,
    };
    ttc_to_pit(ttc_pd, &params).map_err(display_to_py)
}

/// Calibrate a central tendency (long-run average PD) from annual default rates
/// using the geometric mean (the standard regulatory TtC approach).
///
/// Zero annual default rates are rejected; callers should apply an explicit
/// smoothing policy before calibration when zero-default years are present.
///
/// Returns the geometric mean in [0, 1].
#[pyfunction]
#[pyo3(text_signature = "(annual_default_rates)")]
fn central_tendency_py(annual_default_rates: Vec<f64>) -> PyResult<f64> {
    central_tendency(&annual_default_rates).map_err(display_to_py)
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

/// Build the `finstack.core.credit.pd` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "pd")?;
    m.setattr(
        "__doc__",
        "Probability of default: PiT/TtC conversion (Merton-Vasicek) and central-tendency calibration.",
    )?;

    m.add_function(wrap_pyfunction!(pit_to_ttc_py, &m)?)?;
    m.add_function(wrap_pyfunction!(ttc_to_pit_py, &m)?)?;
    m.add_function(wrap_pyfunction!(central_tendency_py, &m)?)?;

    // Expose under the unsuffixed public names.
    m.setattr("pit_to_ttc", m.getattr("pit_to_ttc_py")?)?;
    m.setattr("ttc_to_pit", m.getattr("ttc_to_pit_py")?)?;
    m.setattr("central_tendency", m.getattr("central_tendency_py")?)?;

    let all = PyList::new(py, ["pit_to_ttc", "ttc_to_pit", "central_tendency"])?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core.credit".to_string(),
        },
        Err(_) => "finstack.core.credit".to_string(),
    };
    let qual = format!("{pkg}.pd");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
