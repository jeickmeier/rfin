//! Metrics integration tests

#[path = "metrics/graceful_metrics_test.rs"]
mod graceful_metrics_test;

#[path = "metrics/greek_relationships.rs"]
mod greek_relationships;

#[path = "metrics/sign_conventions.rs"]
mod sign_conventions;

#[path = "metrics/determinism.rs"]
mod determinism;

#[path = "metrics/edge_cases.rs"]
mod edge_cases;

#[path = "metrics/convergence.rs"]
mod convergence;

#[path = "metrics/fd_greeks.rs"]
mod fd_greeks;
