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
//! ```rust,ignore
//! use finstack_statements::explain::{DependencyTracer, FormulaExplainer};
//! use finstack_statements::evaluator::DependencyGraph;
//!
//! // Trace dependencies
//! let graph = DependencyGraph::from_model(&model)?;
//! let tracer = DependencyTracer::new(&model, &graph);
//! let tree = tracer.dependency_tree("gross_profit")?;
//! println!("{}", tree.to_string_ascii());
//!
//! // Explain formula calculation
//! let explainer = FormulaExplainer::new(&model, &results);
//! let explanation = explainer.explain("gross_profit", &period)?;
//! println!("{}", explanation.to_string_detailed());
//! ```

pub mod dependency_trace;
pub mod formula_explain;
pub mod visualization;

pub use dependency_trace::{DependencyTracer, DependencyTree};
pub use formula_explain::{Explanation, ExplanationStep, FormulaExplainer};
pub use visualization::{render_tree_ascii, render_tree_detailed};

