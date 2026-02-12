// Integration tests for the real estate templates live under `tests/integration/`.
//
// Note: Cargo only discovers integration tests that are direct children of `tests/`.
// This file wires in the nested real estate test modules so they run in CI.

#[path = "integration/dated_cashflow_export_tests.rs"]
mod dated_cashflow_export_tests;

#[path = "integration/real_estate_template_tests.rs"]
mod real_estate_template_tests;
