# Historical Scenario Replay

## Problem

The scenario framework is bump-based (deterministic stress). It applies predefined shocks to the current market state. Even the built-in historical templates (GFC 2008, COVID 2020, etc.) are pre-packaged bump magnitudes, not actual market data.

There is no ability to replay actual historical market moves -- e.g., "show me my portfolio P&L through March 2020" by stepping through real curve/spread/vol snapshots from a time series database.

## Design Decisions

- **Location:** New `replay` module in `finstack/portfolio/`, alongside existing `scenarios.rs`
- **Data source:** User-provided market snapshots. The engine is agnostic to storage -- it accepts a `Vec<(Date, MarketContext)>` and does not load data itself.
- **Portfolio:** Static. Same positions replayed across all dates. Isolates pure market P&L from trading activity.
- **Output:** Configurable per run via `ReplayMode` -- PV-only for speed, full attribution for analysis.
- **Bindings:** Rust core + Python + WASM from day one, following existing scenario binding patterns.

## Core Types

All types live in `finstack/portfolio/src/replay.rs`.

### ReplayTimeline (input)

Dated sequence of market snapshots with validation invariants:

```rust
pub struct ReplayTimeline {
    snapshots: Vec<(Date, MarketContext)>,
    // Invariant: sorted by date ascending, non-empty, no duplicate dates
}

impl ReplayTimeline {
    pub fn new(snapshots: Vec<(Date, MarketContext)>) -> Result<Self>;
    pub fn len(&self) -> usize;
    pub fn date_range(&self) -> (Date, Date);
    pub fn iter(&self) -> impl Iterator<Item = &(Date, MarketContext)>;
}
```

### ReplayConfig (configuration)

```rust
pub enum ReplayMode {
    PvOnly,          // Just portfolio PV at each date
    PvAndPnl,        // PV + daily/cumulative P&L
    FullAttribution, // PV + P&L + per-position factor decomposition
}

pub struct ReplayConfig {
    pub mode: ReplayMode,
    pub attribution_method: AttributionMethod,  // Default: Parallel
    pub valuation_options: PortfolioValuationOptions,
}
```

Uses existing `AttributionMethod` from `finstack-valuations` and `PortfolioValuationOptions` from `finstack-portfolio`. No new configuration types.

### ReplayStep (per-step output)

```rust
pub struct ReplayStep {
    pub date: Date,
    pub valuation: PortfolioValuation,
    pub daily_pnl: Option<Money>,
    pub cumulative_pnl: Option<Money>,
    pub attribution: Option<PortfolioAttribution>,
}
```

- `daily_pnl`: Present in `PvAndPnl` and `FullAttribution` modes. `None` at step 0.
- `cumulative_pnl`: Running total relative to step 0. `None` at step 0.
- `attribution`: Present only in `FullAttribution` mode. `None` at step 0.

### ReplayResult (aggregate output)

```rust
pub struct ReplayResult {
    pub steps: Vec<ReplayStep>,
    pub summary: ReplaySummary,
}

pub struct ReplaySummary {
    pub start_date: Date,
    pub end_date: Date,
    pub num_steps: usize,
    pub start_value: Money,
    pub end_value: Money,
    pub total_pnl: Money,
    pub max_drawdown: Money,
    pub max_drawdown_pct: f64,
    pub max_drawdown_peak_date: Date,
    pub max_drawdown_trough_date: Date,
}
```

All types derive `Serialize, Deserialize` for JSON round-tripping through bindings.

## Engine

### Public API

```rust
pub fn replay_portfolio(
    portfolio: &Portfolio,
    timeline: &ReplayTimeline,
    config: &ReplayConfig,
    finstack_config: &FinstackConfig,
) -> Result<ReplayResult>
```

### Algorithm

1. **Validate** -- timeline non-empty (enforced by `ReplayTimeline::new`).
2. **Step 0** -- value portfolio against `timeline[0].market` via existing `value_portfolio()`. This is the anchor. No P&L or attribution at step 0.
3. **Steps 1..N** -- for each subsequent `(date, market)`:
   - Call `value_portfolio(portfolio, &market, finstack_config, &config.valuation_options)`.
   - If `PvAndPnl` or `FullAttribution`: compute `daily_pnl = val_i.total_base_ccy - val_{i-1}.total_base_ccy` and `cumulative_pnl = val_i.total_base_ccy - val_0.total_base_ccy`.
   - If `FullAttribution`: call existing `attribute_portfolio_pnl()` with `market_{i-1}`, `market_i`, `val_{i-1}`, `val_i`, and the configured `AttributionMethod`.
4. **Summary** -- single pass over steps to compute max drawdown via high-water mark tracking.
5. **Return** `ReplayResult`.

### Sequencing

Steps are sequential -- each depends on the prior step for P&L delta. Within each step, `value_portfolio` already uses rayon for position-level parallelism when the `parallel` feature is enabled.

### Error Handling

Follows existing `PortfolioValuationOptions::strict_risk` behavior. Positions that fail to price are marked as `degraded_positions` in `PortfolioValuation` rather than aborting the entire replay. Attribution skips degraded positions.

## Bindings

### Snapshot Wire Format (shared)

```json
[
  {
    "date": "2020-03-01",
    "market": { "version": 2, "curves": [...], "fx": {...}, "surfaces": [...], ... }
  },
  {
    "date": "2020-03-02",
    "market": { "version": 2, "curves": [...], "fx": {...}, "surfaces": [...], ... }
  }
]
```

The `market` field is the existing `MarketContextState` JSON -- no new serialization format.

### Python (`finstack-py/src/bindings/portfolio/replay.rs`)

```rust
#[pyfunction]
pub fn replay_portfolio(
    portfolio_json: &str,
    snapshots_json: &str,
    config_json: &str,
    finstack_config_json: Option<&str>,
) -> PyResult<String>
```

Follows the same `extract_market()` / `serde_json` pattern used in `apply_scenario()`.

### WASM (`finstack-wasm/src/api/portfolio/replay.rs`)

```rust
#[wasm_bindgen]
pub fn replayPortfolio(
    portfolio_json: &str,
    snapshots_json: &str,
    config_json: &str,
    finstack_config_json: Option<String>,
) -> Result<JsValue, JsValue>
```

Same pattern as existing WASM scenario bindings.

## Reuse Map

### Reused as-is (zero changes)

| Component | Crate | Role in replay |
|-----------|-------|----------------|
| `MarketContext` + `MarketContextState` serde | finstack-core | Market state at each step, JSON round-trip |
| `value_portfolio()` | finstack-portfolio | Valuation at each step (with rayon parallelism) |
| `PortfolioValuation`, `PositionValue` | finstack-portfolio | Per-step valuation output |
| `attribute_portfolio_pnl()` | finstack-portfolio | Factor decomposition between steps |
| `PnlAttribution`, `AttributionMethod` | finstack-valuations | Attribution results and configuration |
| `PortfolioValuationOptions` | finstack-portfolio | Controls strict_risk, additional_metrics |
| `Money`, `Currency` | finstack-core | P&L arithmetic |
| `FinstackConfig` | finstack-core | Pricing configuration |
| `Portfolio`, `Position` | finstack-portfolio | Static portfolio passed through |

### New code

| File | Contents | Est. size |
|------|----------|-----------|
| `finstack/portfolio/src/replay.rs` | Types + `replay_portfolio()` | ~250 lines |
| `finstack-py/src/bindings/portfolio/replay.rs` | Python binding | ~60 lines |
| `finstack-wasm/src/api/portfolio/replay.rs` | WASM binding | ~60 lines |
| Tests | Timeline validation, replay modes, summary stats | ~200 lines |

Total: ~570 lines. The replay engine is a thin orchestration loop with no new valuation, attribution, or serialization logic.
