//! WASM bindings for statements DSL parse/compile helpers.
//!
//! Exposes the statements DSL so that users can validate and inspect
//! formulas without building a full financial model.

use crate::core::error::js_error;
use crate::core::expr::JsExpr;
use finstack_statements::dsl::StmtExpr;
use wasm_bindgen::prelude::*;

/// A parsed statements DSL expression (AST).
///
/// Wraps `finstack_statements::dsl::StmtExpr`.
#[wasm_bindgen(js_name = StmtExpr)]
pub struct JsStmtExpr {
    inner: StmtExpr,
}

#[wasm_bindgen(js_class = StmtExpr)]
impl JsStmtExpr {
    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("StmtExpr({:?})", self.inner)
    }
}

impl JsStmtExpr {
    pub(crate) fn new(inner: StmtExpr) -> Self {
        Self { inner }
    }
}

/// Parse a DSL formula into an AST.
///
/// # Arguments
/// * `formula` - DSL formula string (e.g., "revenue - cogs")
///
/// # Returns
/// Parsed statement expression AST
#[wasm_bindgen(js_name = parseFormula)]
pub fn parse_formula(formula: &str) -> Result<JsStmtExpr, JsValue> {
    let ast = finstack_statements::dsl::parse_formula(formula)
        .map_err(|e| js_error(format!("Failed to parse formula: {e}")))?;
    Ok(JsStmtExpr::new(ast))
}

/// Compile a parsed AST into a core `Expr`.
///
/// # Arguments
/// * `ast` - Parsed statement expression
///
/// # Returns
/// Core expression ready for evaluation
#[wasm_bindgen(js_name = compileFormula)]
pub fn compile_formula(ast: &JsStmtExpr) -> Result<JsExpr, JsValue> {
    let expr = finstack_statements::dsl::compile(&ast.inner)
        .map_err(|e| js_error(format!("Failed to compile formula: {e}")))?;
    Ok(JsExpr { inner: expr })
}

/// Parse and compile a DSL formula into a core `Expr` in one step.
///
/// # Arguments
/// * `formula` - DSL formula string (e.g., "revenue - cogs")
///
/// # Returns
/// Core expression ready for evaluation
#[wasm_bindgen(js_name = parseAndCompile)]
pub fn parse_and_compile(formula: &str) -> Result<JsExpr, JsValue> {
    let expr = finstack_statements::dsl::parse_and_compile(formula)
        .map_err(|e| js_error(format!("Failed to parse and compile formula: {e}")))?;
    Ok(JsExpr { inner: expr })
}
