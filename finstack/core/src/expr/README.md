## `core::expr` — Deterministic Scalar Expression Engine

The `core::expr` module is a small, deterministic expression engine used throughout
Finstack for **time–series style** computations (lags, diffs, rolling windows, EWMs,
etc.) over plain `f64` slices. It is designed to be:

- **Deterministic**: stable results across platforms and runs.
- **Allocation‑aware**: scratch arenas and an arena‑style executor minimise per‑node `Vec` allocations.
- **DAG‑optimized**: shared sub‑expressions across many formulas are evaluated once.
- **Cache‑friendly**: intermediate node results can be cached with an LRU cache and explicit memory budget.
- **Embedding‑friendly**: no Polars dependency, `ExpressionContext` makes the caller responsible for column resolution.

At a high level, you:

- **Build an AST** with `Expr`, `ExprNode`, `BinOp`, `UnaryOp`, and `Function`.
- **Compile** it into a `CompiledExpr` (optionally with a DAG `ExecutionPlan` and cache).
- **Evaluate** it against a column context (`ExpressionContext`) and a slice of numeric columns.

---

### Public Surface

The `mod.rs` re‑exports the small public API:

- **AST / operations**
  - `Expr`, `ExprNode`
  - `BinOp`, `UnaryOp`
  - `Function`
  - `EvaluationResult`
- **Context**
  - `ExpressionContext` (trait)
  - `SimpleContext`
- **Evaluator**
  - `CompiledExpr`
  - `EvalOpts`

The Polars `Series` API is intentionally **not** exposed here; callers work with simple slices (`&[f64]`).

---

## Module Structure

- **`ast.rs`**: expression data model and function registry
  - `Expr`: top‑level expression with optional `id: Option<u64>` for DAG/caching identification.
  - `ExprNode`: core variants:
    - **Columns**: `Column(String)`
    - **Literals**: `Literal(f64)`
    - **Function calls**: `Call(Function, Vec<Expr>)`
    - **Operators**: `BinOp`, `UnaryOp`
    - **Conditionals**: `IfThenElse { condition, then_expr, else_expr }`
  - `Function`: enum of all supported scalar functions (lags, diffs, rolling, EWMs, ranking, and a few financial utilities).
  - `EvaluationResult`: `{ values: Vec<f64>, metadata: config::ResultsMeta }`.
  - **Hash/eq semantics**: `Expr` implements `Hash` / `Eq` **ignoring** `id` so structurally identical trees deduplicate in the DAG.

- **`context.rs`**: column resolution
  - `SimpleContext`: name→index map for small, in‑memory frames.
  - `ExpressionContext`: trait used by evaluators to resolve column names (`fn resolve_index(&self, name: &str) -> Option<usize>`).

- **`dag.rs`**: DAG planning and execution plans
  - `DagNode { id, expr, dependencies, ref_count, cost }`.
  - `ExecutionPlan { nodes, roots, meta, cache_strategy }`.
  - `CacheStrategy { cache_nodes, expected_hit_rate, memory_budget }`.
  - `DagBuilder`: walks one or more roots, deduplicates identical sub‑trees, assigns IDs, computes ref counts, topological order, and a cache strategy.

- **`cache.rs`**: LRU cache for intermediate node results
  - `CachedResult::Scalar(Arc<[f64]>)`.
  - `ExpressionCache`: LRU + explicit **memory budget in bytes**, hit/miss/eviction statistics.
  - `CacheManager`: thin `Arc<Mutex<ExpressionCache>>` wrapper used by `CompiledExpr`.

- **`eval.rs`**: compiled evaluator and scalar implementations
  - `EvalOpts { plan: Option<ExecutionPlan>, cache_budget_mb: Option<usize> }`.
  - `CompiledExpr`:
    - `ast: Expr`
    - `plan: Option<ExecutionPlan>`
    - `cache: Option<CacheManager>`
    - internal `ScratchArena { tmp: Vec<f64>, window: Vec<f64> }` for allocations.
  - Evaluation entrypoint:  
    `fn eval<C: ExpressionContext>(&self, ctx: &C, cols: &[&[f64]], opts: EvalOpts) -> EvaluationResult`.
  - Core responsibilities:
    - Decide execution plan (external `EvalOpts.plan` → internal `self.plan` → auto‑build).
    - Choose a cache (external budget or internal `self.cache`).
    - Execute DAG nodes in **topological order** into a single arena `Vec<f64>`.
    - Use `eval_node_into` and scalar helpers (`eval_lag`, `eval_rolling_mean`, etc.) to write each node’s values into a slice of the arena.

- **`mod.rs`**: module docs and public re‑exports
  - High‑level description, supported functions list, and a simple example (see below for expanded usage).

---

## Supported Functions

The `Function` enum in `ast.rs` is the authoritative list. Broadly, functions fall into:

- **Shifts / differences**
  - `Lag`, `Lead`
  - `Diff`, `PctChange`
  - `Shift` (signed shift, positive = down, negative = up)

- **Cumulative operations**
  - `CumSum`, `CumProd`
  - `CumMin`, `CumMax`

- **Rolling window operations** (row‑based windows)
  - `RollingMean`, `RollingSum`
  - `RollingStd`, `RollingVar`, `RollingMedian`
  - `RollingMin`, `RollingMax`, `RollingCount`

- **Exponentially weighted moving stats**
  - `EwmMean`
  - `EwmStd`, `EwmVar`

- **Reducers over the entire series** (broadcast scalar result)
  - `Std`, `Var`, `Median`
  - `Rank` (dense ranking)
  - `Quantile` (global percentile, **not** rolling)

- **Financial utilities** (statement‑layer only)
  - `Sum`, `Mean`
  - `Annualize`, `AnnualizeRate`
  - `Ttm`, `Ytd`, `Qtd`, `FiscalYtd`
  - `Coalesce`

> **Important**: financial utilities (`Sum`, `Mean`, `Annualize*`, `Ttm`, `Ytd`, `Qtd`, `FiscalYtd`, `Coalesce`) are meant to be evaluated at the
> **statements** layer. The scalar evaluator in `eval.rs` will `panic!` if they are invoked from `core::expr`.

---

## Basic Usage

### Building and Evaluating a Simple Expression

Below is a minimal end‑to‑end example using `SimpleContext` and direct evaluation:

```rust
use finstack_core::expr::{
    BinOp, CompiledExpr, EvalOpts, Expr, Function, SimpleContext, UnaryOp,
};

// Columns in input frame: ["x", "y"]
let ctx = SimpleContext::new(["x", "y"]);
let x = vec![1.0, 2.0, 3.0, 4.0];
let y = vec![0.5, 1.5, 2.5, 3.5];
let cols: Vec<&[f64]> = vec![x.as_slice(), y.as_slice()];

// if x > y { x } else { -y }
let condition = Expr::bin_op(BinOp::Gt, Expr::column("x"), Expr::column("y"));
let then_branch = Expr::column("x");
let else_branch = Expr::unary_op(UnaryOp::Neg, Expr::column("y"));
let expr = Expr::if_then_else(condition, then_branch, else_branch);

let compiled = CompiledExpr::new(expr);
let out = compiled.eval(&ctx, &cols, EvalOpts::default());

assert_eq!(out.values.len(), 4);
// out.values ≈ [-0.5, -1.5, 3.0, 4.0]
```

### Rolling Example: `rolling_mean`

```rust
use finstack_core::expr::{CompiledExpr, EvalOpts, Expr, Function, SimpleContext};

// Single column ["x"]
let ctx = SimpleContext::new(["x"]);
let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let cols: Vec<&[f64]> = vec![x.as_slice()];

// rolling_mean(x, 3)
let expr = Expr::call(
    Function::RollingMean,
    vec![Expr::column("x"), Expr::literal(3.0)],
);

let compiled = CompiledExpr::new(expr);
let out = compiled.eval(&ctx, &cols, EvalOpts::default());

assert!(out.values[0].is_nan());
assert!(out.values[1].is_nan());
assert!((out.values[2] - 2.0).abs() < 1e-12);
assert!((out.values[3] - 3.0).abs() < 1e-12);
assert!((out.values[4] - 4.0).abs() < 1e-12);
```

### Using DAG Planning and Caching

For large model graphs (e.g., financial statements), using a pre‑built plan with a cache
significantly reduces recomputation of shared sub‑expressions:

```rust
use finstack_core::config::{results_meta, FinstackConfig};
use finstack_core::expr::{CompiledExpr, EvalOpts, Expr, Function, SimpleContext};

let ctx = SimpleContext::new(["x"]);
let x = vec![0.2, 0.5, 3.0, 4.0];
let cols: Vec<&[f64]> = vec![x.as_slice()];

let expr = Expr::call(
    Function::RollingSum,
    vec![Expr::column("x"), Expr::literal(2.0)],
);

// Build a plan and attach a cache sized to that plan.
let meta = results_meta(&FinstackConfig::default());
let compiled = CompiledExpr::with_planning(expr, meta).with_cache(1); // 1 MB cache

let result = compiled.eval(
    &ctx,
    &cols,
    EvalOpts {
        plan: None,
        cache_budget_mb: Some(1),
    },
);

// Access both values and minimal metadata.
let values = result.values;
let meta = result.metadata;
```

### Overriding the Execution Plan

Advanced callers (e.g., the statements engine) can build a plan once and reuse it:

```rust
use finstack_core::config::{results_meta, FinstackConfig};
use finstack_core::expr::{CompiledExpr, EvalOpts, Expr, Function, SimpleContext};

let ctx = SimpleContext::new(["x"]);
let x = vec![0.2, 0.5, 3.0, 4.0];
let cols: Vec<&[f64]> = vec![x.as_slice()];

let expr = Expr::call(Function::Diff, vec![Expr::column("x"), Expr::literal(1.0)]);
let meta = results_meta(&FinstackConfig::default());
let compiled = CompiledExpr::with_planning(expr, meta);
let external_plan = compiled.plan.clone();

let result = compiled.eval(
    &ctx,
    &cols,
    EvalOpts {
        plan: external_plan,
        cache_budget_mb: None,
    },
);
```

---

## Execution Model and Determinism

- **Scalar only**: all functions operate on `&[f64]` slices; there is no dynamic dispatch to external DataFrame libraries.
- **Arena‑style execution**: the evaluator allocates a single `Vec<f64>` arena sized to
  `len × number_of_nodes`, and each node writes directly into a slice of that arena.
- **Topological order**: DAG nodes are executed in dependency order, ensuring all inputs are
  available before a node is computed.
- **NaN conventions**:
  - Rolling windows that are not yet full return `NaN`.
  - Divisions by zero return `NaN`.
  - Many reducers ignore `NaN` inputs when computing aggregates but emit `NaN` if there are no valid values.
- **Metadata**: `EvaluationResult.metadata` is stamped via `config::results_meta` and is designed
  to be consistent with other Finstack result envelopes. The evaluator itself does not track timings or parallelism.

---

## Extending the Expression Engine

This section describes how to add **new functions** or **new contexts** safely.

### Adding a New Function

1. **Add a variant to `Function` in `ast.rs`**
   - Add a new enum case with a clear doc comment.
   - If the function is meant for `core::expr` evaluation, keep it in the **scalar** family (no cross‑currency math, no external IO).
   - If the function is *statements‑only*, document that clearly and ensure the core evaluator never tries to execute it.

2. **Implement scalar evaluation in `eval.rs`**
   - Add a helper like `fn eval_my_func(&self, arg_results: &[Vec<f64>]) -> Vec<f64>` or an `_into` variant that writes into a buffer.
   - Update `eval_function_core` to dispatch your new `Function` variant.
   - Prefer writing into an existing output slice when possible (see `eval_rolling_*_into`, `eval_std_into`, etc.) to minimise allocations:
     - Reuse `ScratchArena` buffers (`tmp`, `window`) for sort‑heavy or windowed logic.
     - Avoid per‑row allocations inside hot loops.

3. **Update cost model in `dag.rs`**
   - In `DagBuilder::estimate_cost`, add an appropriate cost estimate for the new `Function`:
     - Cheap element‑wise ops: single‑digit cost.
     - Multi‑pass or sorting operations: higher costs (similar to `RollingMedian`, `Rank`, `Quantile`).
   - This influences which nodes the cache strategy recommends for caching.

4. **Add tests**
   - Add focused scalar tests in `eval.rs` (or nearby) that:
     - Check both “normal” values and edge cases (`NaN`, zeros, small windows, etc.).
   - If the function interacts heavily with DAG behaviour or caching, add a test in `dag.rs` or a dedicated integration‑style test that:
     - Builds a small expression graph with shared sub‑expressions.
     - Asserts that the new node is deduplicated and/or appears in the cache strategy as expected.

5. **Update docs**
   - Update this `README.md` and the module docs in `mod.rs` / `ast.rs` to mention the new function and its semantics.

### Adding a New `ExpressionContext`

The default `SimpleContext` is intentionally small. For more complex environments (e.g., a large
statements model with many frames or partitioned data), you can implement your own context:

```rust
use finstack_core::expr::ExpressionContext;

struct MyContext {
    // your own lookup structures
}

impl ExpressionContext for MyContext {
    fn resolve_index(&self, name: &str) -> Option<usize> {
        // map name → index in your input frame representation
        unimplemented!()
    }
}
```

Guidelines:

- **Resolve only by name**: the evaluator assumes column indices are stable and consistent with the `cols` slice passed at evaluation time.
- **No side effects**: `resolve_index` should be cheap and side‑effect‑free; it may be called many times.

### Working with Caching

If you need to tune caching behaviour beyond the defaults:

- Use `CompiledExpr::with_planning(ast, meta)` to get an `ExecutionPlan` with a `CacheStrategy`.
- Attach a cache sized for your workload with `.with_cache(budget_mb)` or per‑call via `EvalOpts.cache_budget_mb`.
- Inspect cache statistics via `CacheManager::stats()` / `hit_ratio()` to size budgets in benchmarks.

---

## When *Not* to Use `core::expr`

`core::expr` is intentionally minimal and **scalar**:

- For currency‑aware arithmetic, FX, or anything that must respect `Amount`/`Currency` invariants,
  use the higher‑level **statements**, **valuations**, or **portfolio** layers.
- For heavy DataFrame‑level pushdown, use Polars via the re‑exports in `core` and the statement/valuation engines built on top.
- For cross‑currency or policy‑aware math, use the dedicated primitives in `core::money`, `core::dates`, and `core::market_data`.

Use `core::expr` when you need **fast, deterministic, scalar expressions over `f64` slices** with
good caching and DAG planning, typically as a building block inside larger engines.


