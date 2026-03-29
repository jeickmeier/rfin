//! WASM bindings for the finstack expression engine.
//!
//! This mirrors the Python bindings (`finstack.core.expr`) and exposes:
//! - Function/BinOp/UnaryOp enums
//! - Expr builders (column, literal, call, binOp, unaryOp, ifThenElse)
//! - ExecutionPlan / EvalOpts
//! - CompiledExpr evaluation against columnar data

use crate::core::error::js_error;
use crate::utils::json::to_js_value;
use crate::valuations::results::JsResultsMeta;
use finstack_core::expr::{
    BinOp, CompiledExpr as CoreCompiledExpr, EvalOpts, EvaluationResult, ExecutionPlan,
    Expr as CoreExpr, Function, SimpleContext, UnaryOp,
};
use js_sys::Array;
use wasm_bindgen::prelude::*;

// ======================================================================
// Enum wrappers
// ======================================================================

/// Built-in expression functions.
#[wasm_bindgen(js_name = Function)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsFunction {
    Lag,
    Lead,
    Diff,
    PctChange,
    CumSum,
    CumProd,
    CumMin,
    CumMax,
    RollingMean,
    RollingSum,
    EwmMean,
    Std,
    Var,
    Median,
    RollingStd,
    RollingVar,
    RollingMedian,
    Shift,
    Rank,
    Quantile,
    RollingMin,
    RollingMax,
    RollingCount,
    EwmStd,
    EwmVar,
    Sum,
    Mean,
    Annualize,
    AnnualizeRate,
    Ttm,
    Ytd,
    Qtd,
    FiscalYtd,
    Coalesce,
    Abs,
    Sign,
    GrowthRate,
}

impl From<JsFunction> for Function {
    fn from(value: JsFunction) -> Self {
        use JsFunction::*;
        match value {
            Lag => Function::Lag,
            Lead => Function::Lead,
            Diff => Function::Diff,
            PctChange => Function::PctChange,
            CumSum => Function::CumSum,
            CumProd => Function::CumProd,
            CumMin => Function::CumMin,
            CumMax => Function::CumMax,
            RollingMean => Function::RollingMean,
            RollingSum => Function::RollingSum,
            EwmMean => Function::EwmMean,
            Std => Function::Std,
            Var => Function::Var,
            Median => Function::Median,
            RollingStd => Function::RollingStd,
            RollingVar => Function::RollingVar,
            RollingMedian => Function::RollingMedian,
            Shift => Function::Shift,
            Rank => Function::Rank,
            Quantile => Function::Quantile,
            RollingMin => Function::RollingMin,
            RollingMax => Function::RollingMax,
            RollingCount => Function::RollingCount,
            EwmStd => Function::EwmStd,
            EwmVar => Function::EwmVar,
            Sum => Function::Sum,
            Mean => Function::Mean,
            Annualize => Function::Annualize,
            AnnualizeRate => Function::AnnualizeRate,
            Ttm => Function::Ttm,
            Ytd => Function::Ytd,
            Qtd => Function::Qtd,
            FiscalYtd => Function::FiscalYtd,
            Coalesce => Function::Coalesce,
            Abs => Function::Abs,
            Sign => Function::Sign,
            GrowthRate => Function::GrowthRate,
        }
    }
}

impl From<Function> for JsFunction {
    fn from(value: Function) -> Self {
        use Function::*;
        match value {
            Lag => JsFunction::Lag,
            Lead => JsFunction::Lead,
            Diff => JsFunction::Diff,
            PctChange => JsFunction::PctChange,
            CumSum => JsFunction::CumSum,
            CumProd => JsFunction::CumProd,
            CumMin => JsFunction::CumMin,
            CumMax => JsFunction::CumMax,
            RollingMean => JsFunction::RollingMean,
            RollingSum => JsFunction::RollingSum,
            EwmMean => JsFunction::EwmMean,
            Std => JsFunction::Std,
            Var => JsFunction::Var,
            Median => JsFunction::Median,
            RollingStd => JsFunction::RollingStd,
            RollingVar => JsFunction::RollingVar,
            RollingMedian => JsFunction::RollingMedian,
            Shift => JsFunction::Shift,
            Rank => JsFunction::Rank,
            Quantile => JsFunction::Quantile,
            RollingMin => JsFunction::RollingMin,
            RollingMax => JsFunction::RollingMax,
            RollingCount => JsFunction::RollingCount,
            EwmStd => JsFunction::EwmStd,
            EwmVar => JsFunction::EwmVar,
            Sum => JsFunction::Sum,
            Mean => JsFunction::Mean,
            Annualize => JsFunction::Annualize,
            AnnualizeRate => JsFunction::AnnualizeRate,
            Ttm => JsFunction::Ttm,
            Ytd => JsFunction::Ytd,
            Qtd => JsFunction::Qtd,
            FiscalYtd => JsFunction::FiscalYtd,
            Coalesce => JsFunction::Coalesce,
            Abs => JsFunction::Abs,
            Sign => JsFunction::Sign,
            GrowthRate => JsFunction::GrowthRate,
        }
    }
}

/// Binary operators.
#[wasm_bindgen(js_name = BinOp)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl From<JsBinOp> for BinOp {
    fn from(value: JsBinOp) -> Self {
        use JsBinOp::*;
        match value {
            Add => BinOp::Add,
            Sub => BinOp::Sub,
            Mul => BinOp::Mul,
            Div => BinOp::Div,
            Mod => BinOp::Mod,
            Eq => BinOp::Eq,
            Ne => BinOp::Ne,
            Lt => BinOp::Lt,
            Le => BinOp::Le,
            Gt => BinOp::Gt,
            Ge => BinOp::Ge,
            And => BinOp::And,
            Or => BinOp::Or,
        }
    }
}

impl From<BinOp> for JsBinOp {
    fn from(value: BinOp) -> Self {
        use BinOp::*;
        match value {
            Add => JsBinOp::Add,
            Sub => JsBinOp::Sub,
            Mul => JsBinOp::Mul,
            Div => JsBinOp::Div,
            Mod => JsBinOp::Mod,
            Eq => JsBinOp::Eq,
            Ne => JsBinOp::Ne,
            Lt => JsBinOp::Lt,
            Le => JsBinOp::Le,
            Gt => JsBinOp::Gt,
            Ge => JsBinOp::Ge,
            And => JsBinOp::And,
            Or => JsBinOp::Or,
        }
    }
}

/// Unary operators.
#[wasm_bindgen(js_name = UnaryOp)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsUnaryOp {
    Neg,
    Not,
}

impl From<JsUnaryOp> for UnaryOp {
    fn from(value: JsUnaryOp) -> Self {
        match value {
            JsUnaryOp::Neg => UnaryOp::Neg,
            JsUnaryOp::Not => UnaryOp::Not,
        }
    }
}

impl From<UnaryOp> for JsUnaryOp {
    fn from(value: UnaryOp) -> Self {
        match value {
            UnaryOp::Neg => JsUnaryOp::Neg,
            UnaryOp::Not => JsUnaryOp::Not,
        }
    }
}

// ======================================================================
// Expr and planning
// ======================================================================

#[wasm_bindgen(js_name = Expr)]
#[derive(Clone, Debug)]
pub struct JsExpr {
    pub(crate) inner: CoreExpr,
}

#[wasm_bindgen(js_class = Expr)]
impl JsExpr {
    /// Column reference.
    #[wasm_bindgen(js_name = column)]
    pub fn column(name: &str) -> JsExpr {
        JsExpr {
            inner: CoreExpr::column(name),
        }
    }

    /// Literal numeric value.
    #[wasm_bindgen(js_name = literal)]
    pub fn literal(value: f64) -> JsExpr {
        JsExpr {
            inner: CoreExpr::literal(value),
        }
    }

    /// Function call.
    #[wasm_bindgen(js_name = call)]
    pub fn call(func: JsFunction, args: Vec<JsExpr>) -> JsExpr {
        let inner_args = args.iter().map(|a| a.inner.clone()).collect();
        JsExpr {
            inner: CoreExpr::call(func.into(), inner_args),
        }
    }

    /// Binary operator.
    #[wasm_bindgen(js_name = binOp)]
    pub fn bin_op(op: JsBinOp, left: &JsExpr, right: &JsExpr) -> JsExpr {
        JsExpr {
            inner: CoreExpr::bin_op(op.into(), left.inner.clone(), right.inner.clone()),
        }
    }

    /// Unary operator.
    #[wasm_bindgen(js_name = unaryOp)]
    pub fn unary_op(op: JsUnaryOp, operand: &JsExpr) -> JsExpr {
        JsExpr {
            inner: CoreExpr::unary_op(op.into(), operand.inner.clone()),
        }
    }

    /// Conditional expression.
    #[wasm_bindgen(js_name = ifThenElse)]
    pub fn if_then_else(condition: &JsExpr, then_expr: &JsExpr, else_expr: &JsExpr) -> JsExpr {
        JsExpr {
            inner: CoreExpr::if_then_else(
                condition.inner.clone(),
                then_expr.inner.clone(),
                else_expr.inner.clone(),
            ),
        }
    }

    /// Attach a stable ID to this expression.
    #[wasm_bindgen(js_name = withId)]
    pub fn with_id(&self, id: u64) -> JsExpr {
        JsExpr {
            inner: self.inner.clone().with_id(id),
        }
    }

    /// Serialize to JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize to JSON string (feature-compatible with Rust/Python bindings).
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Deserialize from JSON.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsExpr, JsValue> {
        serde_json::from_str(json)
            .map(|inner| JsExpr { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        format!("Expr<{:?}>", self.inner.node)
    }
}

#[wasm_bindgen(js_name = ExecutionPlan)]
#[derive(Clone, Debug)]
pub struct JsExecutionPlan {
    inner: ExecutionPlan,
}

impl JsExecutionPlan {
    pub(crate) fn new(inner: ExecutionPlan) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> ExecutionPlan {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = ExecutionPlan)]
impl JsExecutionPlan {
    /// Root node IDs.
    #[wasm_bindgen(getter)]
    pub fn roots(&self) -> Vec<u64> {
        self.inner.roots.clone()
    }

    /// Number of nodes in the plan.
    #[wasm_bindgen(getter, js_name = nodeCount)]
    pub fn node_count(&self) -> usize {
        self.inner.nodes.len()
    }

    /// Execution metadata.
    #[wasm_bindgen(getter, js_name = metadata)]
    pub fn metadata(&self) -> JsResultsMeta {
        JsResultsMeta::new(self.inner.meta.clone())
    }
}

#[wasm_bindgen(js_name = EvalOpts)]
#[derive(Clone, Debug, Default)]
pub struct JsEvalOpts {
    inner: EvalOpts,
}

#[wasm_bindgen(js_class = EvalOpts)]
impl JsEvalOpts {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsEvalOpts {
        JsEvalOpts {
            inner: EvalOpts::default(),
        }
    }

    /// Attach a pre-built execution plan.
    #[wasm_bindgen(js_name = withPlan)]
    pub fn with_plan(&self, plan: &JsExecutionPlan) -> JsEvalOpts {
        let mut opts = self.inner.clone();
        opts.plan = Some(plan.inner());
        JsEvalOpts { inner: opts }
    }

    /// Set cache budget in MB for evaluation.
    #[wasm_bindgen(js_name = withCacheBudgetMb)]
    pub fn with_cache_budget_mb(&self, budget: usize) -> JsEvalOpts {
        let mut opts = self.inner.clone();
        opts.cache_budget_mb = Some(budget);
        JsEvalOpts { inner: opts }
    }
}

// ======================================================================
// Evaluation results
// ======================================================================

#[wasm_bindgen(js_name = EvaluationResult)]
#[derive(Clone, Debug)]
pub struct JsEvaluationResult {
    inner: EvaluationResult,
}

impl JsEvaluationResult {
    pub(crate) fn new(inner: EvaluationResult) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = EvaluationResult)]
impl JsEvaluationResult {
    #[wasm_bindgen(getter)]
    pub fn values(&self) -> Vec<f64> {
        self.inner.values.clone()
    }

    #[wasm_bindgen(getter, js_name = metadata)]
    pub fn metadata(&self) -> JsResultsMeta {
        JsResultsMeta::new(self.inner.metadata.clone())
    }
}

// ======================================================================
// Compiled expression
// ======================================================================

#[wasm_bindgen(js_name = CompiledExpr)]
#[derive(Clone, Debug)]
pub struct JsCompiledExpr {
    inner: CoreCompiledExpr,
}

impl JsCompiledExpr {
    pub(crate) fn from_inner(inner: CoreCompiledExpr) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = CompiledExpr)]
impl JsCompiledExpr {
    /// Compile an expression (optional plan/metadata handled lazily).
    #[wasm_bindgen(constructor)]
    pub fn new(expr: &JsExpr) -> JsCompiledExpr {
        JsCompiledExpr::from_inner(CoreCompiledExpr::new(expr.inner.clone()))
    }

    /// Compile with pre-computed planning metadata.
    #[wasm_bindgen(js_name = withPlanning)]
    pub fn with_planning(
        expr: &JsExpr,
        results_meta: &JsResultsMeta,
    ) -> Result<JsCompiledExpr, JsValue> {
        CoreCompiledExpr::with_planning(expr.inner.clone(), results_meta.inner().clone())
            .map(JsCompiledExpr::from_inner)
            .map_err(|err| js_error(err.to_string()))
    }

    /// Enable evaluation cache.
    #[wasm_bindgen(js_name = withCache)]
    pub fn with_cache(&self, budget_mb: usize) -> JsCompiledExpr {
        JsCompiledExpr::from_inner(self.inner.clone().with_cache(budget_mb))
    }

    /// Underlying execution plan (if available).
    #[wasm_bindgen(getter)]
    pub fn plan(&self) -> Option<JsExecutionPlan> {
        self.inner.plan.clone().map(JsExecutionPlan::new)
    }

    /// Evaluate against columnar data.
    ///
    /// @param {string[]} columns - column names
    /// @param {number[][]} data - array-of-arrays matching `columns`
    /// @param {EvalOpts=} opts - optional evaluation options
    #[wasm_bindgen(js_name = eval)]
    pub fn eval(
        &self,
        columns: Array,
        data: Array,
        opts: Option<JsEvalOpts>,
    ) -> Result<JsEvaluationResult, JsValue> {
        let col_names: Vec<String> = columns
            .iter()
            .map(|v| {
                v.as_string()
                    .ok_or_else(|| js_error("columns must be strings"))
            })
            .collect::<Result<_, _>>()?;

        let mut series: Vec<Vec<f64>> = Vec::with_capacity(data.length() as usize);
        for entry in data.iter() {
            let arr = Array::from(&entry);
            let mut col = Vec::with_capacity(arr.length() as usize);
            for v in arr.iter() {
                let num = v
                    .as_f64()
                    .ok_or_else(|| js_error("data must be numeric arrays"))?;
                col.push(num);
            }
            series.push(col);
        }

        if col_names.len() != series.len() {
            return Err(js_error("columns and data length must match"));
        }
        let expected_len = series.first().map(|c| c.len()).unwrap_or(0);
        if series
            .iter()
            .any(|col| !col.is_empty() && col.len() != expected_len)
        {
            return Err(js_error(
                "all data series must have the same length (or be empty)",
            ));
        }

        let ctx = SimpleContext::new(col_names).map_err(|e| js_error(e.to_string()))?;
        let slices: Vec<&[f64]> = series.iter().map(|v| v.as_slice()).collect();
        let eval_opts = opts.map(|o| o.inner).unwrap_or_default();
        let result = self
            .inner
            .eval(&ctx, &slices, eval_opts)
            .map_err(|err| js_error(err.to_string()))?;

        Ok(JsEvaluationResult::new(result))
    }
}
