//! DSL helper bindings.
//!
//! This module exposes the statements DSL parse/compile helpers directly so that users can
//! validate and inspect formulas without building a full financial model.

use crate::core::expr::PyExpr;
use crate::statements::error::stmt_to_py;
use finstack_statements::dsl::StmtExpr;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// A parsed statements DSL expression (AST).
///
/// This wraps `finstack_statements::dsl::StmtExpr`.
#[pyclass(name = "StmtExpr", module = "finstack.statements.dsl")]
#[derive(Clone, Debug)]
pub struct PyStmtExpr {
    pub(crate) inner: StmtExpr,
}

impl PyStmtExpr {
    pub(crate) fn new(inner: StmtExpr) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStmtExpr {
    fn __repr__(&self) -> String {
        format!("StmtExpr({:?})", self.inner)
    }
}

/// Parse a DSL formula into an AST.
#[pyfunction(name = "parse_formula", text_signature = "(formula)")]
fn parse_formula_py(formula: &str) -> PyResult<PyStmtExpr> {
    let ast = finstack_statements::dsl::parse_formula(formula).map_err(stmt_to_py)?;
    Ok(PyStmtExpr::new(ast))
}

/// Compile a parsed AST into a core `Expr`.
#[pyfunction(name = "compile_formula", text_signature = "(ast)")]
fn compile_formula_py(ast: &PyStmtExpr) -> PyResult<PyExpr> {
    let expr = finstack_statements::dsl::compile(&ast.inner).map_err(stmt_to_py)?;
    Ok(PyExpr::new(expr))
}

/// Parse and compile a DSL formula into a core `Expr` in one step.
#[pyfunction(name = "parse_and_compile", text_signature = "(formula)")]
fn parse_and_compile_py(formula: &str) -> PyResult<PyExpr> {
    let expr = finstack_statements::dsl::parse_and_compile(formula).map_err(stmt_to_py)?;
    Ok(PyExpr::new(expr))
}

/// Register DSL helper exports.
pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "dsl")?;
    module.setattr(
        "__doc__",
        "Statements DSL helpers (parse/compile formulas into inspectable AST and core Expr).",
    )?;

    module.add_class::<PyStmtExpr>()?;
    module.add_function(wrap_pyfunction!(parse_formula_py, &module)?)?;
    module.add_function(wrap_pyfunction!(compile_formula_py, &module)?)?;
    module.add_function(wrap_pyfunction!(parse_and_compile_py, &module)?)?;

    let exports = [
        "StmtExpr",
        "parse_formula",
        "compile_formula",
        "parse_and_compile",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("dsl", &module)?;
    Ok(exports.to_vec())
}
