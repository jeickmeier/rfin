use crate::core::config::PyResultsMeta;
use finstack_core::expr::ExecutionPlan;
use finstack_core::expr::{
    BinOp, CompiledExpr as CoreCompiledExpr, EvalOpts, EvaluationResult, Expr as CoreExpr,
    Function, SimpleContext, UnaryOp,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

#[pyclass(name = "Function", module = "finstack.core.expr", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyFunction {
    pub(crate) inner: Function,
}

impl PyFunction {
    pub(crate) const fn new(inner: Function) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFunction {
    #[classattr]
    const LAG: Self = Self::new(Function::Lag);
    #[classattr]
    const LEAD: Self = Self::new(Function::Lead);
    #[classattr]
    const DIFF: Self = Self::new(Function::Diff);
    #[classattr]
    const PCT_CHANGE: Self = Self::new(Function::PctChange);
    #[classattr]
    const CUM_SUM: Self = Self::new(Function::CumSum);
    #[classattr]
    const CUM_PROD: Self = Self::new(Function::CumProd);
    #[classattr]
    const CUM_MIN: Self = Self::new(Function::CumMin);
    #[classattr]
    const CUM_MAX: Self = Self::new(Function::CumMax);
    #[classattr]
    const ROLLING_MEAN: Self = Self::new(Function::RollingMean);
    #[classattr]
    const ROLLING_SUM: Self = Self::new(Function::RollingSum);
    #[classattr]
    const EWM_MEAN: Self = Self::new(Function::EwmMean);
    #[classattr]
    const STD: Self = Self::new(Function::Std);
    #[classattr]
    const VAR: Self = Self::new(Function::Var);
    #[classattr]
    const MEDIAN: Self = Self::new(Function::Median);
    #[classattr]
    const ROLLING_STD: Self = Self::new(Function::RollingStd);
    #[classattr]
    const ROLLING_VAR: Self = Self::new(Function::RollingVar);
    #[classattr]
    const ROLLING_MEDIAN: Self = Self::new(Function::RollingMedian);
    #[classattr]
    const SHIFT: Self = Self::new(Function::Shift);
    #[classattr]
    const RANK: Self = Self::new(Function::Rank);
    #[classattr]
    const QUANTILE: Self = Self::new(Function::Quantile);
    #[classattr]
    const ROLLING_MIN: Self = Self::new(Function::RollingMin);
    #[classattr]
    const ROLLING_MAX: Self = Self::new(Function::RollingMax);
    #[classattr]
    const ROLLING_COUNT: Self = Self::new(Function::RollingCount);
    #[classattr]
    const EWM_STD: Self = Self::new(Function::EwmStd);
    #[classattr]
    const EWM_VAR: Self = Self::new(Function::EwmVar);
    #[classattr]
    const SUM: Self = Self::new(Function::Sum);
    #[classattr]
    const MEAN: Self = Self::new(Function::Mean);
    #[classattr]
    const ANNUALIZE: Self = Self::new(Function::Annualize);
    #[classattr]
    const ANNUALIZE_RATE: Self = Self::new(Function::AnnualizeRate);
    #[classattr]
    const TTM: Self = Self::new(Function::Ttm);
    #[classattr]
    const YTD: Self = Self::new(Function::Ytd);
    #[classattr]
    const QTD: Self = Self::new(Function::Qtd);
    #[classattr]
    const FISCAL_YTD: Self = Self::new(Function::FiscalYtd);
    #[classattr]
    const COALESCE: Self = Self::new(Function::Coalesce);
    #[classattr]
    const ABS: Self = Self::new(Function::Abs);
    #[classattr]
    const SIGN: Self = Self::new(Function::Sign);
    #[classattr]
    const GROWTH_RATE: Self = Self::new(Function::GrowthRate);

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            Function::Lag => "lag",
            Function::Lead => "lead",
            Function::Diff => "diff",
            Function::PctChange => "pct_change",
            Function::CumSum => "cumsum",
            Function::CumProd => "cumprod",
            Function::CumMin => "cummin",
            Function::CumMax => "cummax",
            Function::RollingMean => "rolling_mean",
            Function::RollingSum => "rolling_sum",
            Function::EwmMean => "ewm_mean",
            Function::Std => "std",
            Function::Var => "var",
            Function::Median => "median",
            Function::RollingStd => "rolling_std",
            Function::RollingVar => "rolling_var",
            Function::RollingMedian => "rolling_median",
            Function::Shift => "shift",
            Function::Rank => "rank",
            Function::Quantile => "quantile",
            Function::RollingMin => "rolling_min",
            Function::RollingMax => "rolling_max",
            Function::RollingCount => "rolling_count",
            Function::EwmStd => "ewm_std",
            Function::EwmVar => "ewm_var",
            Function::Sum => "sum",
            Function::Mean => "mean",
            Function::Annualize => "annualize",
            Function::AnnualizeRate => "annualize_rate",
            Function::Ttm => "ttm",
            Function::Ytd => "ytd",
            Function::Qtd => "qtd",
            Function::FiscalYtd => "fiscal_ytd",
            Function::Coalesce => "coalesce",
            Function::Abs => "abs",
            Function::Sign => "sign",
            Function::GrowthRate => "growth_rate",
        }
    }

    fn __repr__(&self) -> String {
        format!("Function('{}')", self.name())
    }
}

#[pyclass(name = "BinOp", module = "finstack.core.expr", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBinOp {
    pub(crate) inner: BinOp,
}

impl PyBinOp {
    pub(crate) const fn new(inner: BinOp) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBinOp {
    #[classattr]
    const ADD: Self = Self::new(BinOp::Add);
    #[classattr]
    const SUB: Self = Self::new(BinOp::Sub);
    #[classattr]
    const MUL: Self = Self::new(BinOp::Mul);
    #[classattr]
    const DIV: Self = Self::new(BinOp::Div);
    #[classattr]
    const MOD: Self = Self::new(BinOp::Mod);
    #[classattr]
    const EQ: Self = Self::new(BinOp::Eq);
    #[classattr]
    const NE: Self = Self::new(BinOp::Ne);
    #[classattr]
    const LT: Self = Self::new(BinOp::Lt);
    #[classattr]
    const LE: Self = Self::new(BinOp::Le);
    #[classattr]
    const GT: Self = Self::new(BinOp::Gt);
    #[classattr]
    const GE: Self = Self::new(BinOp::Ge);
    #[classattr]
    const AND: Self = Self::new(BinOp::And);
    #[classattr]
    const OR: Self = Self::new(BinOp::Or);

    fn __repr__(&self) -> String {
        format!("BinOp::{:?}", self.inner)
    }
}

#[pyclass(name = "UnaryOp", module = "finstack.core.expr", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyUnaryOp {
    pub(crate) inner: UnaryOp,
}

impl PyUnaryOp {
    pub(crate) const fn new(inner: UnaryOp) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyUnaryOp {
    #[classattr]
    const NEG: Self = Self::new(UnaryOp::Neg);
    #[classattr]
    const NOT: Self = Self::new(UnaryOp::Not);

    fn __repr__(&self) -> String {
        format!("UnaryOp::{:?}", self.inner)
    }
}

#[pyclass(name = "Expr", module = "finstack.core.expr")]
#[derive(Clone, Debug)]
pub struct PyExpr {
    pub(crate) inner: CoreExpr,
}

impl PyExpr {
    pub(crate) fn new(inner: CoreExpr) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExpr {
    #[staticmethod]
    #[pyo3(text_signature = "(name)")]
    fn column(name: &str) -> Self {
        Self::new(CoreExpr::column(name))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(value)")]
    fn literal(value: f64) -> Self {
        Self::new(CoreExpr::literal(value))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(func, args)")]
    fn call(func: PyRef<PyFunction>, args: Vec<PyRef<PyExpr>>) -> Self {
        let inner_args = args.into_iter().map(|a| a.inner.clone()).collect();
        Self::new(CoreExpr::call(func.inner, inner_args))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(op, left, right)")]
    fn bin_op(op: PyRef<PyBinOp>, left: PyRef<PyExpr>, right: PyRef<PyExpr>) -> Self {
        Self::new(CoreExpr::bin_op(
            op.inner,
            left.inner.clone(),
            right.inner.clone(),
        ))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(op, operand)")]
    fn unary_op(op: PyRef<PyUnaryOp>, operand: PyRef<PyExpr>) -> Self {
        Self::new(CoreExpr::unary_op(op.inner, operand.inner.clone()))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(condition, then_expr, else_expr)")]
    fn if_then_else(
        condition: PyRef<PyExpr>,
        then_expr: PyRef<PyExpr>,
        else_expr: PyRef<PyExpr>,
    ) -> Self {
        Self::new(CoreExpr::if_then_else(
            condition.inner.clone(),
            then_expr.inner.clone(),
            else_expr.inner.clone(),
        ))
    }

    #[pyo3(text_signature = "(self, id)")]
    fn with_id(&self, id: u64) -> Self {
        Self::new(self.inner.clone().with_id(id))
    }

    fn __repr__(&self) -> String {
        format!("Expr<{:?}>", self.inner.node)
    }
}

#[pyclass(
    name = "ExecutionPlan",
    module = "finstack.core.expr",
    frozen,
    unsendable
)]
#[derive(Clone, Debug)]
pub struct PyExecutionPlan {
    pub(crate) inner: ExecutionPlan,
}

impl PyExecutionPlan {
    pub(crate) fn new(inner: ExecutionPlan) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExecutionPlan {
    #[getter]
    fn roots(&self) -> Vec<u64> {
        self.inner.roots.clone()
    }

    #[getter]
    fn node_count(&self) -> usize {
        self.inner.nodes.len()
    }

    #[getter]
    fn metadata(&self) -> PyResultsMeta {
        PyResultsMeta::new(self.inner.meta.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionPlan(nodes={}, roots={})",
            self.inner.nodes.len(),
            self.inner.roots.len()
        )
    }
}

#[pyclass(name = "EvaluationResult", module = "finstack.core.expr", frozen)]
#[derive(Clone, Debug)]
pub struct PyEvaluationResult {
    pub(crate) inner: EvaluationResult,
}

impl PyEvaluationResult {
    pub(crate) fn new(inner: EvaluationResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEvaluationResult {
    #[getter]
    fn values(&self) -> Vec<f64> {
        self.inner.values.clone()
    }

    #[getter]
    fn metadata(&self) -> PyResultsMeta {
        PyResultsMeta::new(self.inner.metadata.clone())
    }
}

#[pyclass(name = "EvalOpts", module = "finstack.core.expr")]
#[derive(Clone, Debug)]
pub struct PyEvalOpts {
    pub(crate) inner: EvalOpts,
}

impl PyEvalOpts {
    pub(crate) fn new(inner: EvalOpts) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEvalOpts {
    #[new]
    #[pyo3(signature = (*, plan=None, cache_budget_mb=None))]
    #[pyo3(text_signature = "(*, plan=None, cache_budget_mb=None)")]
    fn ctor(plan: Option<PyRef<PyExecutionPlan>>, cache_budget_mb: Option<usize>) -> Self {
        let opts = EvalOpts {
            plan: plan.map(|p| p.inner.clone()),
            cache_budget_mb,
        };
        Self::new(opts)
    }

    #[getter]
    fn cache_budget_mb(&self) -> Option<usize> {
        self.inner.cache_budget_mb
    }

    #[setter]
    fn set_cache_budget_mb(&mut self, budget: Option<usize>) {
        self.inner.cache_budget_mb = budget;
    }

    #[getter]
    fn plan(&self) -> Option<PyExecutionPlan> {
        self.inner.clone().plan.map(PyExecutionPlan::new)
    }
}

#[pyclass(name = "CompiledExpr", module = "finstack.core.expr", unsendable)]
#[derive(Clone, Debug)]
pub struct PyCompiledExpr {
    pub(crate) inner: CoreCompiledExpr,
}

impl PyCompiledExpr {
    pub(crate) fn new(inner: CoreCompiledExpr) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCompiledExpr {
    #[new]
    #[pyo3(text_signature = "(expr)")]
    fn ctor(expr: PyRef<PyExpr>) -> Self {
        Self::new(CoreCompiledExpr::new(expr.inner.clone()))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, expr, results_meta)")]
    fn with_planning(
        _cls: &Bound<'_, PyType>,
        expr: PyRef<PyExpr>,
        results_meta: PyRef<PyResultsMeta>,
    ) -> Self {
        Self::new(CoreCompiledExpr::with_planning(
            expr.inner.clone(),
            results_meta.inner.clone(),
        ))
    }

    #[pyo3(text_signature = "(self, budget_mb)")]
    fn with_cache(&self, budget_mb: usize) -> Self {
        Self::new(self.inner.clone().with_cache(budget_mb))
    }

    #[getter]
    fn plan(&self) -> Option<PyExecutionPlan> {
        self.inner.plan.clone().map(PyExecutionPlan::new)
    }

    #[pyo3(signature = (columns, data, opts=None), text_signature = "(self, columns, data, opts=None)")]
    fn eval(
        &self,
        columns: Vec<String>,
        data: Vec<Vec<f64>>,
        opts: Option<PyRef<PyEvalOpts>>,
    ) -> PyResult<PyEvaluationResult> {
        if columns.len() != data.len() {
            return Err(PyValueError::new_err("columns and data length must match"));
        }
        let expected_len = data.first().map(|c| c.len()).unwrap_or(0);
        if data
            .iter()
            .any(|col| col.len() != expected_len && !col.is_empty())
        {
            return Err(PyValueError::new_err(
                "all data series must have the same length",
            ));
        }

        let ctx = SimpleContext::new(columns);
        let slices: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();
        let eval_opts = opts.map(|o| o.inner.clone()).unwrap_or_default();

        let result = self.inner.eval(&ctx, &slices, eval_opts);
        Ok(PyEvaluationResult::new(result))
    }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "expr")?;
    module.setattr(
        "__doc__",
        "Expression engine bindings (AST construction, planning, evaluation).",
    )?;

    module.add_class::<PyFunction>()?;
    module.add_class::<PyBinOp>()?;
    module.add_class::<PyUnaryOp>()?;
    module.add_class::<PyExpr>()?;
    module.add_class::<PyExecutionPlan>()?;
    module.add_class::<PyEvalOpts>()?;
    module.add_class::<PyCompiledExpr>()?;
    module.add_class::<PyEvaluationResult>()?;

    let exports = [
        "Function",
        "BinOp",
        "UnaryOp",
        "Expr",
        "ExecutionPlan",
        "EvalOpts",
        "CompiledExpr",
        "EvaluationResult",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(())
}
