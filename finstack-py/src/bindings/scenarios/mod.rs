//! Python bindings for the `finstack-scenarios` crate.
//!
//! Scenarios are spec-based (serde), so this module exposes JSON round-trip
//! functions for [`ScenarioSpec`] construction, validation, template
//! registry discovery, and scenario engine application.

mod engine;
mod horizon;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Parse an ISO 8601 date string into a `time::Date`.
fn parse_date(s: &str) -> PyResult<time::Date> {
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    time::Date::parse(s, &format)
        .map_err(|e| PyValueError::new_err(format!("Invalid date '{s}': {e}")))
}

// ---------------------------------------------------------------------------
// ScenarioSpec JSON round-trip
// ---------------------------------------------------------------------------

#[pyfunction]
fn parse_scenario_spec(json_str: &str) -> PyResult<String> {
    let spec: finstack_scenarios::ScenarioSpec = serde_json::from_str(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse ScenarioSpec JSON: {e}")))?;
    spec.validate()
        .map_err(|e| PyValueError::new_err(format!("ScenarioSpec validation failed: {e}")))?;
    serde_json::to_string(&spec)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize ScenarioSpec: {e}")))
}

#[pyfunction]
#[pyo3(signature = (id, operations_json, name=None, description=None, priority=0))]
fn build_scenario_spec(
    id: &str,
    operations_json: &str,
    name: Option<&str>,
    description: Option<&str>,
    priority: i32,
) -> PyResult<String> {
    let operations: Vec<finstack_scenarios::OperationSpec> = serde_json::from_str(operations_json)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse operations JSON: {e}")))?;
    let spec = finstack_scenarios::ScenarioSpec {
        id: id.to_string(),
        name: name.map(str::to_string),
        description: description.map(str::to_string),
        operations,
        priority,
        resolution_mode: Default::default(),
    };
    spec.validate()
        .map_err(|e| PyValueError::new_err(format!("ScenarioSpec validation failed: {e}")))?;
    serde_json::to_string(&spec)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize ScenarioSpec: {e}")))
}

#[pyfunction]
fn compose_scenarios(specs_json: &str) -> PyResult<String> {
    let specs: Vec<finstack_scenarios::ScenarioSpec> = serde_json::from_str(specs_json)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse specs JSON: {e}")))?;
    let engine = finstack_scenarios::ScenarioEngine::new();
    let composed = engine
        .try_compose(specs)
        .map_err(|e| PyValueError::new_err(format!("Scenario composition failed: {e}")))?;
    serde_json::to_string(&composed)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize composed spec: {e}")))
}

#[pyfunction]
fn validate_scenario_spec(json_str: &str) -> PyResult<bool> {
    let spec: finstack_scenarios::ScenarioSpec = serde_json::from_str(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse ScenarioSpec JSON: {e}")))?;
    spec.validate()
        .map_err(|e| PyValueError::new_err(format!("ScenarioSpec validation failed: {e}")))?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Template registry
// ---------------------------------------------------------------------------

#[pyfunction]
fn list_builtin_templates() -> PyResult<Vec<String>> {
    let registry = finstack_scenarios::TemplateRegistry::with_embedded_builtins()
        .map_err(|e| PyValueError::new_err(format!("Failed to load embedded templates: {e}")))?;
    Ok(registry.list().iter().map(|m| m.id.clone()).collect())
}

#[pyfunction]
fn list_builtin_template_metadata() -> PyResult<String> {
    let registry = finstack_scenarios::TemplateRegistry::with_embedded_builtins()
        .map_err(|e| PyValueError::new_err(format!("Failed to load embedded templates: {e}")))?;
    let metadata: Vec<&finstack_scenarios::TemplateMetadata> = registry.list();
    serde_json::to_string(&metadata)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize template metadata: {e}")))
}

#[pyfunction]
fn build_from_template(template_id: &str) -> PyResult<String> {
    let registry = finstack_scenarios::TemplateRegistry::with_embedded_builtins()
        .map_err(|e| PyValueError::new_err(format!("Failed to load embedded templates: {e}")))?;
    let entry = registry
        .get(template_id)
        .ok_or_else(|| PyValueError::new_err(format!("Unknown template: '{template_id}'")))?;
    let spec = entry
        .builder()
        .build()
        .map_err(|e| PyValueError::new_err(format!("Failed to build template spec: {e}")))?;
    serde_json::to_string(&spec)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize spec: {e}")))
}

#[pyfunction]
fn list_template_components(template_id: &str) -> PyResult<Vec<String>> {
    let registry = finstack_scenarios::TemplateRegistry::with_embedded_builtins()
        .map_err(|e| PyValueError::new_err(format!("Failed to load embedded templates: {e}")))?;
    let entry = registry
        .get(template_id)
        .ok_or_else(|| PyValueError::new_err(format!("Unknown template: '{template_id}'")))?;
    Ok(entry
        .component_ids()
        .into_iter()
        .map(str::to_string)
        .collect())
}

#[pyfunction]
fn build_template_component(template_id: &str, component_id: &str) -> PyResult<String> {
    let registry = finstack_scenarios::TemplateRegistry::with_embedded_builtins()
        .map_err(|e| PyValueError::new_err(format!("Failed to load embedded templates: {e}")))?;
    let entry = registry
        .get(template_id)
        .ok_or_else(|| PyValueError::new_err(format!("Unknown template: '{template_id}'")))?;
    let builder = entry.component(component_id).ok_or_else(|| {
        PyValueError::new_err(format!(
            "Unknown component '{component_id}' in template '{template_id}'"
        ))
    })?;
    let spec = builder
        .build()
        .map_err(|e| PyValueError::new_err(format!("Failed to build component spec: {e}")))?;
    serde_json::to_string(&spec)
        .map_err(|e| PyValueError::new_err(format!("Failed to serialize component spec: {e}")))
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "scenarios")?;
    m.setattr(
        "__doc__",
        "Scenario specification, validation, composition, application, and built-in templates.",
    )?;

    m.add_function(wrap_pyfunction!(parse_scenario_spec, &m)?)?;
    m.add_function(wrap_pyfunction!(build_scenario_spec, &m)?)?;
    m.add_function(wrap_pyfunction!(compose_scenarios, &m)?)?;
    m.add_function(wrap_pyfunction!(validate_scenario_spec, &m)?)?;
    m.add_function(wrap_pyfunction!(list_builtin_templates, &m)?)?;
    m.add_function(wrap_pyfunction!(list_builtin_template_metadata, &m)?)?;
    m.add_function(wrap_pyfunction!(build_from_template, &m)?)?;
    m.add_function(wrap_pyfunction!(list_template_components, &m)?)?;
    m.add_function(wrap_pyfunction!(build_template_component, &m)?)?;
    engine::register(py, &m)?;
    horizon::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "parse_scenario_spec",
            "build_scenario_spec",
            "compose_scenarios",
            "validate_scenario_spec",
            "list_builtin_templates",
            "list_builtin_template_metadata",
            "build_from_template",
            "list_template_components",
            "build_template_component",
            "apply_scenario",
            "apply_scenario_to_market",
            "compute_horizon_return",
            "HorizonResult",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_by_parent_name(
        py,
        parent,
        &m,
        "scenarios",
        "finstack.finstack",
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duplicate_time_roll_specs_json() -> String {
        use finstack_core::market_data::hierarchy::ResolutionMode;
        use finstack_scenarios::{OperationSpec, ScenarioSpec, TimeRollMode};

        let specs = vec![
            ScenarioSpec {
                id: "roll_1m".into(),
                name: None,
                description: None,
                operations: vec![OperationSpec::TimeRollForward {
                    period: "1M".into(),
                    apply_shocks: true,
                    roll_mode: TimeRollMode::BusinessDays,
                }],
                priority: 0,
                resolution_mode: ResolutionMode::Cumulative,
            },
            ScenarioSpec {
                id: "roll_3m".into(),
                name: None,
                description: None,
                operations: vec![OperationSpec::TimeRollForward {
                    period: "3M".into(),
                    apply_shocks: true,
                    roll_mode: TimeRollMode::BusinessDays,
                }],
                priority: 1,
                resolution_mode: ResolutionMode::Cumulative,
            },
        ];
        serde_json::to_string(&specs).expect("serialize specs")
    }

    #[test]
    fn compose_scenarios_rejects_duplicate_time_rolls() {
        let err = compose_scenarios(&duplicate_time_roll_specs_json())
            .expect_err("duplicate time rolls should be rejected");
        assert!(
            err.to_string().contains("TimeRollForward"),
            "unexpected error: {err}"
        );
    }
}
