//! Shared utilities for registering Python submodules.
//!
//! Every binding submodule needs to:
//!
//! 1. Call `parent.add_submodule(&m)?` so attribute access works.
//! 2. Set `m.__package__` to the fully-qualified dotted path.
//! 3. Insert `m` into `sys.modules` under the qualified name so `import
//!    finstack.x.y` resolves correctly (matters for re-export shims, the
//!    importlib machinery, and tools like `inspect.getmodule`).
//!
//! Two flavors are provided. They differ only in how the parent's qualified
//! name is obtained — kept separate to preserve the historical behavior at
//! each call site without changing observable semantics.

use pyo3::prelude::*;

/// Register `submodule` under `parent`, deriving the qualified path from the
/// parent's `__package__` attribute and falling back to
/// `parent_default_pkg` when the attribute is missing or unreadable.
///
/// Used by submodules nested several layers deep (e.g. `finstack.core.math`,
/// `finstack.core.market_data`) where the parent already has a stable
/// `__package__` set by its own `register`.
pub(crate) fn register_submodule_by_package(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    submodule: &Bound<'_, PyModule>,
    submod_name: &str,
    parent_default_pkg: &str,
) -> PyResult<()> {
    let pkg: String = parent
        .getattr("__package__")
        .ok()
        .and_then(|v| v.extract::<String>().ok())
        .unwrap_or_else(|| parent_default_pkg.to_string());
    let qual = format!("{pkg}.{submod_name}");
    register_at(py, parent, submodule, &qual)
}

/// Register `submodule` under `parent`, deriving the qualified path from the
/// parent's `__name__` attribute and falling back to `parent_default_name`
/// when the attribute is missing or unreadable.
///
/// Used by top-level domain modules (e.g. `finstack.analytics`,
/// `finstack.portfolio`) where parent is the root `finstack` module.
pub(crate) fn register_submodule_by_parent_name(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    submodule: &Bound<'_, PyModule>,
    submod_name: &str,
    parent_default_name: &str,
) -> PyResult<()> {
    let parent_name: String = parent
        .getattr("__name__")
        .ok()
        .and_then(|v| v.extract::<String>().ok())
        .unwrap_or_else(|| parent_default_name.to_string());
    let qual = format!("{parent_name}.{submod_name}");
    register_at(py, parent, submodule, &qual)
}

fn register_at(
    py: Python<'_>,
    parent: &Bound<'_, PyModule>,
    submodule: &Bound<'_, PyModule>,
    qual: &str,
) -> PyResult<()> {
    parent.add_submodule(submodule)?;
    submodule.setattr("__package__", qual)?;
    let sys = PyModule::import(py, "sys")?;
    sys.getattr("modules")?.set_item(qual, submodule)?;
    Ok(())
}
