//! Node explanation and dependency tracing.
//!
//! This module provides tools for understanding how financial statement nodes
//! are calculated and what dependencies they have.
//!
//! # Features
//!
//! - **Dependency Tracing** - Identify direct and transitive dependencies
//! - **Formula Explanation** - Break down calculations step-by-step
//! - **Tree Visualization** - ASCII tree rendering of dependency graphs
//!
//! # Examples
//!
//! ```rust
//! use finstack_statements::prelude::*;
//! use finstack_statements::explain::{DependencyTracer, FormulaExplainer};
//! use finstack_statements::evaluator::DependencyGraph;
//!
//! # fn main() -> Result<()> {
//! // Build a model
//! let model = ModelBuilder::new("explain_demo")
//!     .periods("2025Q1..Q1", None)?
//!     .value("revenue", &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0))])
//!     .compute("cogs", "revenue * 0.6")?
//!     .compute("gross_profit", "revenue - cogs")?
//!     .build()?;
//!
//! // Trace dependencies
//! let graph = DependencyGraph::from_model(&model)?;
//! let tracer = DependencyTracer::new(&model, &graph);
//! let tree = tracer.dependency_tree("gross_profit")?;
//! let tree_str = tree.to_string_ascii();
//! assert!(tree_str.contains("gross_profit"));
//!
//! // Explain formula calculation
//! let mut evaluator = Evaluator::new();
//! let results = evaluator.evaluate(&model)?;
//! let explainer = FormulaExplainer::new(&model, &results);
//! let period = PeriodId::quarter(2025, 1);
//! let explanation = explainer.explain("gross_profit", &period)?;
//! assert_eq!(explanation.node_id, "gross_profit");
//! # Ok(())
//! # }
//! ```

pub mod dependency_trace;
pub mod formula_explain;
pub mod visualization;

pub use dependency_trace::{DependencyTracer, DependencyTree};
pub use formula_explain::{Explanation, ExplanationStep, FormulaExplainer};
pub use visualization::{render_tree_ascii, render_tree_detailed};

