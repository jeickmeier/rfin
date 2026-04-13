# Historical Scenario Replay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `replay` module to `finstack-portfolio` that steps a static portfolio through a sequence of dated market snapshots, producing configurable P&L and attribution output.

**Architecture:** New `replay.rs` in portfolio crate alongside `scenarios.rs`. Thin orchestration loop calling existing `value_portfolio()` and `attribute_portfolio_pnl()`. Python and WASM bindings follow existing `pipeline.rs` / `portfolio/mod.rs` patterns (JSON in, JSON out).

**Tech Stack:** Rust (finstack-portfolio, finstack-core, finstack-valuations), pyo3 (Python), wasm-bindgen (WASM), serde_json

**Spec:** `docs/superpowers/specs/2026-04-12-historical-replay-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `finstack/portfolio/src/replay.rs` | Core types (`ReplayTimeline`, `ReplayConfig`, `ReplayMode`, `ReplayStep`, `ReplayResult`, `ReplaySummary`) and `replay_portfolio()` engine function |
| `finstack/portfolio/src/lib.rs` | Module declaration + re-exports |
| `finstack/portfolio/tests/replay.rs` | Integration tests for replay engine |
| `finstack-py/src/bindings/portfolio/replay.rs` | Python binding: `replay_portfolio()` |
| `finstack-py/src/bindings/portfolio/mod.rs` | Register replay submodule + update `__all__` |
| `finstack-wasm/src/api/portfolio/mod.rs` | WASM binding: `replayPortfolio()` |

---

### Task 1: Core Types

**Files:**
- Create: `finstack/portfolio/src/replay.rs`
- Modify: `finstack/portfolio/src/lib.rs`

- [ ] **Step 1: Create `replay.rs` with types only**

```rust
// finstack/portfolio/src/replay.rs

//! Historical scenario replay for portfolios.
//!
//! Replays a static portfolio through a sequence of dated market snapshots,
//! producing configurable P&L and attribution output at each step.
//!
//! This module is only available when the `scenarios` feature is enabled.

use crate::attribution::PortfolioAttribution;
use crate::error::{Error, Result};
use crate::valuation::PortfolioValuation;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

/// What to compute at each replay step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReplayMode {
    /// Just portfolio PV at each date.
    PvOnly,
    /// PV + daily/cumulative P&L.
    PvAndPnl,
    /// PV + P&L + per-position factor decomposition.
    FullAttribution,
}

/// Configuration for a replay run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// What to compute at each step.
    pub mode: ReplayMode,
    /// Attribution method (only used in `FullAttribution` mode).
    #[serde(default)]
    pub attribution_method: finstack_valuations::attribution::AttributionMethod,
    /// Valuation options forwarded to `value_portfolio`.
    #[serde(default)]
    pub valuation_options: crate::valuation::PortfolioValuationOptions,
}

/// A dated sequence of market snapshots.
///
/// Invariants enforced by [`ReplayTimeline::new`]:
/// - Non-empty
/// - Sorted by date ascending
/// - No duplicate dates
pub struct ReplayTimeline {
    snapshots: Vec<(Date, MarketContext)>,
}

impl ReplayTimeline {
    /// Create a new timeline from a vector of `(date, market)` pairs.
    ///
    /// Returns an error if the vector is empty, not sorted by date, or
    /// contains duplicate dates.
    pub fn new(snapshots: Vec<(Date, MarketContext)>) -> Result<Self> {
        if snapshots.is_empty() {
            return Err(Error::InvalidInput("ReplayTimeline must be non-empty".into()));
        }
        for window in snapshots.windows(2) {
            let (d0, _) = &window[0];
            let (d1, _) = &window[1];
            if d1 <= d0 {
                return Err(Error::InvalidInput(format!(
                    "ReplayTimeline dates must be strictly ascending, found {d0} >= {d1}"
                )));
            }
        }
        Ok(Self { snapshots })
    }

    /// Number of snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Whether the timeline is empty (always false after construction).
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// First and last dates in the timeline.
    pub fn date_range(&self) -> (Date, Date) {
        // Indexing is safe: new() enforces non-empty.
        (self.snapshots[0].0, self.snapshots[self.snapshots.len() - 1].0)
    }

    /// Iterate over `(date, market)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = &(Date, MarketContext)> {
        self.snapshots.iter()
    }
}

/// Output for a single replay step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayStep {
    /// Valuation date.
    pub date: Date,
    /// Full portfolio valuation at this date.
    pub valuation: PortfolioValuation,
    /// Daily P&L (this step minus prior step). `None` at step 0.
    pub daily_pnl: Option<Money>,
    /// Cumulative P&L (this step minus step 0). `None` at step 0.
    pub cumulative_pnl: Option<Money>,
    /// Factor attribution between prior step and this step. `None` at step 0
    /// and in non-attribution modes.
    pub attribution: Option<PortfolioAttribution>,
}

/// Aggregate statistics across the full replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplaySummary {
    /// First date in the timeline.
    pub start_date: Date,
    /// Last date in the timeline.
    pub end_date: Date,
    /// Number of steps (including step 0).
    pub num_steps: usize,
    /// Portfolio value at step 0.
    pub start_value: Money,
    /// Portfolio value at the last step.
    pub end_value: Money,
    /// Total P&L (end value minus start value).
    pub total_pnl: Money,
    /// Maximum drawdown from peak to trough.
    pub max_drawdown: Money,
    /// Maximum drawdown as a percentage of peak value.
    pub max_drawdown_pct: f64,
    /// Date of the peak before the maximum drawdown.
    pub max_drawdown_peak_date: Date,
    /// Date of the trough of the maximum drawdown.
    pub max_drawdown_trough_date: Date,
}

/// Full output of a replay run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Per-step output.
    pub steps: Vec<ReplayStep>,
    /// Aggregate statistics.
    pub summary: ReplaySummary,
}
```

- [ ] **Step 2: Add module declaration and re-exports to `lib.rs`**

In `finstack/portfolio/src/lib.rs`, add below the existing `pub mod scenarios;` line (line 111):

```rust
#[cfg(feature = "scenarios")]
/// Historical scenario replay for portfolios.
pub mod replay;
```

And add below the existing scenarios re-export (line 148):

```rust
#[cfg(feature = "scenarios")]
pub use replay::{
    replay_portfolio, ReplayConfig, ReplayMode, ReplayResult, ReplayStep, ReplaySummary,
    ReplayTimeline,
};
```

- [ ] **Step 3: Check that it compiles**

Run: `cargo check -p finstack-portfolio --features scenarios`

Expected: Warning about unused imports (the engine function doesn't exist yet) but no errors. If `replay_portfolio` causes an unresolved import, temporarily comment out that re-export line until Task 2.

- [ ] **Step 4: Commit**

```bash
git add finstack/portfolio/src/replay.rs finstack/portfolio/src/lib.rs
git commit -m "feat(portfolio): add replay module types (ReplayTimeline, ReplayConfig, ReplayResult)"
```

---

### Task 2: Timeline Validation Tests

**Files:**
- Create: `finstack/portfolio/tests/replay.rs`

- [ ] **Step 1: Write failing tests for `ReplayTimeline::new` validation**

```rust
// finstack/portfolio/tests/replay.rs

#[cfg(feature = "scenarios")]
mod replay_tests {
    use finstack_core::market_data::context::MarketContext;
    use finstack_portfolio::ReplayTimeline;
    use time::macros::date;

    fn empty_market() -> MarketContext {
        MarketContext::new()
    }

    #[test]
    fn timeline_rejects_empty() {
        let result = ReplayTimeline::new(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn timeline_accepts_single_snapshot() {
        let result = ReplayTimeline::new(vec![(date!(2024 - 01 - 01), empty_market())]);
        assert!(result.is_ok());
        let tl = result.unwrap();
        assert_eq!(tl.len(), 1);
        assert!(!tl.is_empty());
        let (start, end) = tl.date_range();
        assert_eq!(start, date!(2024 - 01 - 01));
        assert_eq!(end, date!(2024 - 01 - 01));
    }

    #[test]
    fn timeline_accepts_sorted_dates() {
        let result = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), empty_market()),
            (date!(2024 - 01 - 02), empty_market()),
            (date!(2024 - 01 - 03), empty_market()),
        ]);
        assert!(result.is_ok());
        let tl = result.unwrap();
        assert_eq!(tl.len(), 3);
        let (start, end) = tl.date_range();
        assert_eq!(start, date!(2024 - 01 - 01));
        assert_eq!(end, date!(2024 - 01 - 03));
    }

    #[test]
    fn timeline_rejects_unsorted_dates() {
        let result = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 02), empty_market()),
            (date!(2024 - 01 - 01), empty_market()),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn timeline_rejects_duplicate_dates() {
        let result = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), empty_market()),
            (date!(2024 - 01 - 01), empty_market()),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn timeline_iter_yields_all_snapshots() {
        let tl = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), empty_market()),
            (date!(2024 - 01 - 02), empty_market()),
        ])
        .unwrap();
        let dates: Vec<_> = tl.iter().map(|(d, _)| *d).collect();
        assert_eq!(dates, vec![date!(2024 - 01 - 01), date!(2024 - 01 - 02)]);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p finstack-portfolio --features scenarios --test replay`

Expected: All 6 tests pass. If the `Error::InvalidInput` variant doesn't exist, check the error module and use the appropriate existing variant (likely `Error::PortfolioError` or similar with a String payload).

- [ ] **Step 3: Commit**

```bash
git add finstack/portfolio/tests/replay.rs
git commit -m "test(portfolio): add ReplayTimeline validation tests"
```

---

### Task 3: Replay Engine — PvOnly Mode

**Files:**
- Modify: `finstack/portfolio/src/replay.rs`
- Modify: `finstack/portfolio/tests/replay.rs`

- [ ] **Step 1: Write failing test for `replay_portfolio` in PvOnly mode**

Add to `finstack/portfolio/tests/replay.rs` inside the `replay_tests` module:

```rust
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_portfolio::{
        Entity, Portfolio, Position, PositionUnit, ReplayConfig, ReplayMode,
    };
    use finstack_valuations::instruments::rates::deposit::Deposit;
    use std::sync::Arc;

    fn build_test_portfolio() -> Portfolio {
        let as_of = date!(2024 - 01 - 01);
        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start_date(as_of)
            .maturity(date!(2024 - 02 - 01))
            .day_count(DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .unwrap();

        Portfolio::builder("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .unwrap()
    }

    fn market_at_rate(as_of: time::Date, rate_bp: f64) -> MarketContext {
        let rate = rate_bp / 10_000.0;
        let curve = DiscountCurve::builder("USD")
            .base_date(as_of)
            .knots(vec![
                (0.0, 1.0),
                (1.0, (-rate * 1.0_f64).exp()),
                (5.0, (-rate * 5.0_f64).exp()),
            ])
            .interp(InterpStyle::Linear)
            .allow_non_monotonic()
            .build()
            .unwrap();
        MarketContext::new().insert(curve)
    }

    #[test]
    fn replay_pv_only_produces_steps_for_each_date() {
        let portfolio = build_test_portfolio();
        let timeline = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), market_at_rate(date!(2024 - 01 - 01), 0.0)),
            (date!(2024 - 01 - 02), market_at_rate(date!(2024 - 01 - 02), 50.0)),
            (date!(2024 - 01 - 03), market_at_rate(date!(2024 - 01 - 03), 100.0)),
        ])
        .unwrap();

        let config = ReplayConfig {
            mode: ReplayMode::PvOnly,
            attribution_method: Default::default(),
            valuation_options: Default::default(),
        };

        let result = finstack_portfolio::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &FinstackConfig::default(),
        )
        .unwrap();

        // One step per snapshot
        assert_eq!(result.steps.len(), 3);

        // Step 0 has no P&L
        assert!(result.steps[0].daily_pnl.is_none());
        assert!(result.steps[0].cumulative_pnl.is_none());
        assert!(result.steps[0].attribution.is_none());

        // All steps in PvOnly have no P&L fields (not just step 0)
        for step in &result.steps {
            assert!(step.daily_pnl.is_none());
            assert!(step.cumulative_pnl.is_none());
            assert!(step.attribution.is_none());
        }

        // Dates match timeline
        assert_eq!(result.steps[0].date, date!(2024 - 01 - 01));
        assert_eq!(result.steps[1].date, date!(2024 - 01 - 02));
        assert_eq!(result.steps[2].date, date!(2024 - 01 - 03));

        // Summary
        assert_eq!(result.summary.num_steps, 3);
        assert_eq!(result.summary.start_date, date!(2024 - 01 - 01));
        assert_eq!(result.summary.end_date, date!(2024 - 01 - 03));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p finstack-portfolio --features scenarios --test replay replay_pv_only`

Expected: FAIL — `replay_portfolio` not found.

- [ ] **Step 3: Implement `replay_portfolio`**

Add to the bottom of `finstack/portfolio/src/replay.rs`:

```rust
use crate::portfolio::Portfolio;
use crate::valuation::value_portfolio;
use finstack_core::config::FinstackConfig;

/// Replay a portfolio through a sequence of dated market snapshots.
///
/// Produces a [`ReplayResult`] with per-step valuations, optional P&L,
/// and optional factor attribution depending on [`ReplayConfig::mode`].
///
/// # Arguments
///
/// * `portfolio` - Static portfolio replayed across all dates.
/// * `timeline` - Dated sequence of market snapshots (must be non-empty and sorted).
/// * `config` - Controls what to compute at each step.
/// * `finstack_config` - Forwarded to [`value_portfolio`](crate::valuation::value_portfolio).
///
/// # Errors
///
/// Propagates errors from [`value_portfolio`](crate::valuation::value_portfolio)
/// and [`attribute_portfolio_pnl`](crate::attribution::attribute_portfolio_pnl).
pub fn replay_portfolio(
    portfolio: &Portfolio,
    timeline: &ReplayTimeline,
    config: &ReplayConfig,
    finstack_config: &FinstackConfig,
) -> Result<ReplayResult> {
    let compute_pnl = matches!(config.mode, ReplayMode::PvAndPnl | ReplayMode::FullAttribution);
    let compute_attribution = matches!(config.mode, ReplayMode::FullAttribution);

    let mut steps = Vec::with_capacity(timeline.len());

    // Step 0: anchor valuation
    let (first_date, first_market) = &timeline.snapshots[0];
    let val_0 = value_portfolio(portfolio, first_market, finstack_config, &config.valuation_options)?;

    steps.push(ReplayStep {
        date: *first_date,
        valuation: val_0,
        daily_pnl: None,
        cumulative_pnl: None,
        attribution: None,
    });

    // Steps 1..N
    for (date, market) in timeline.snapshots.iter().skip(1) {
        let val_i = value_portfolio(portfolio, market, finstack_config, &config.valuation_options)?;

        let prev_step = steps.last().ok_or_else(|| {
            Error::InvalidInput("internal: steps should never be empty here".into())
        })?;

        let daily_pnl = if compute_pnl {
            Some(val_i.total_base_ccy - prev_step.valuation.total_base_ccy)
        } else {
            None
        };

        let cumulative_pnl = if compute_pnl {
            Some(val_i.total_base_ccy - steps[0].valuation.total_base_ccy)
        } else {
            None
        };

        let attribution = if compute_attribution {
            let attr = crate::attribution::attribute_portfolio_pnl(
                portfolio,
                &timeline.snapshots[steps.len() - 1].1,
                market,
                prev_step.date,
                *date,
                finstack_config,
                config.attribution_method.clone(),
            )?;
            Some(attr)
        } else {
            None
        };

        steps.push(ReplayStep {
            date: *date,
            valuation: val_i,
            daily_pnl,
            cumulative_pnl,
            attribution,
        });
    }

    let summary = compute_summary(&steps);

    Ok(ReplayResult { steps, summary })
}

fn compute_summary(steps: &[ReplayStep]) -> ReplaySummary {
    let start_value = steps[0].valuation.total_base_ccy;
    let end_value = steps[steps.len() - 1].valuation.total_base_ccy;
    let total_pnl = end_value - start_value;

    // Max drawdown via high-water mark
    let mut peak_value = start_value.amount();
    let mut peak_date = steps[0].date;
    let mut max_dd = 0.0_f64;
    let mut max_dd_peak_date = steps[0].date;
    let mut max_dd_trough_date = steps[0].date;

    for step in steps {
        let val = step.valuation.total_base_ccy.amount();
        if val > peak_value {
            peak_value = val;
            peak_date = step.date;
        }
        let dd = peak_value - val;
        if dd > max_dd {
            max_dd = dd;
            max_dd_peak_date = peak_date;
            max_dd_trough_date = step.date;
        }
    }

    let max_drawdown_pct = if peak_value.abs() > f64::EPSILON {
        max_dd / peak_value
    } else {
        0.0
    };

    ReplaySummary {
        start_date: steps[0].date,
        end_date: steps[steps.len() - 1].date,
        num_steps: steps.len(),
        start_value,
        end_value,
        total_pnl,
        max_drawdown: Money::new(max_dd, start_value.currency()),
        max_drawdown_pct,
        max_drawdown_peak_date: max_dd_peak_date,
        max_drawdown_trough_date: max_dd_trough_date,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p finstack-portfolio --features scenarios --test replay replay_pv_only`

Expected: PASS. If `Money` does not implement `Sub`, use `Money::new(val_i.total_base_ccy.amount() - prev.total_base_ccy.amount(), val_i.total_base_ccy.currency())` instead.

- [ ] **Step 5: Commit**

```bash
git add finstack/portfolio/src/replay.rs finstack/portfolio/tests/replay.rs
git commit -m "feat(portfolio): implement replay_portfolio engine with PvOnly mode"
```

---

### Task 4: Replay Engine — PvAndPnl and FullAttribution Modes

**Files:**
- Modify: `finstack/portfolio/tests/replay.rs`

- [ ] **Step 1: Write test for PvAndPnl mode**

Add to `replay_tests`:

```rust
    #[test]
    fn replay_pv_and_pnl_computes_daily_and_cumulative() {
        let portfolio = build_test_portfolio();
        let timeline = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), market_at_rate(date!(2024 - 01 - 01), 0.0)),
            (date!(2024 - 01 - 02), market_at_rate(date!(2024 - 01 - 02), 50.0)),
            (date!(2024 - 01 - 03), market_at_rate(date!(2024 - 01 - 03), 100.0)),
        ])
        .unwrap();

        let config = ReplayConfig {
            mode: ReplayMode::PvAndPnl,
            attribution_method: Default::default(),
            valuation_options: Default::default(),
        };

        let result = finstack_portfolio::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &FinstackConfig::default(),
        )
        .unwrap();

        // Step 0: no P&L
        assert!(result.steps[0].daily_pnl.is_none());
        assert!(result.steps[0].cumulative_pnl.is_none());

        // Step 1+: has P&L, no attribution
        for step in &result.steps[1..] {
            assert!(step.daily_pnl.is_some());
            assert!(step.cumulative_pnl.is_some());
            assert!(step.attribution.is_none());
        }

        // Cumulative at last step equals total_pnl in summary
        let last_cum = result.steps.last().unwrap().cumulative_pnl.unwrap();
        let diff = (last_cum.amount() - result.summary.total_pnl.amount()).abs();
        assert!(diff < 1e-6, "cumulative P&L should match summary total_pnl");
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p finstack-portfolio --features scenarios --test replay replay_pv_and_pnl`

Expected: PASS (PvAndPnl logic is already implemented in Task 3).

- [ ] **Step 3: Write test for FullAttribution mode**

Add to `replay_tests`:

```rust
    use finstack_valuations::attribution::AttributionMethod;

    #[test]
    fn replay_full_attribution_produces_attribution_at_each_step() {
        let portfolio = build_test_portfolio();
        let timeline = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), market_at_rate(date!(2024 - 01 - 01), 0.0)),
            (date!(2024 - 01 - 02), market_at_rate(date!(2024 - 01 - 02), 100.0)),
        ])
        .unwrap();

        let config = ReplayConfig {
            mode: ReplayMode::FullAttribution,
            attribution_method: AttributionMethod::Parallel,
            valuation_options: Default::default(),
        };

        let result = finstack_portfolio::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &FinstackConfig::default(),
        )
        .unwrap();

        // Step 0: no attribution
        assert!(result.steps[0].attribution.is_none());

        // Step 1: has attribution with P&L decomposition
        let attr = result.steps[1].attribution.as_ref().expect("step 1 should have attribution");
        // The attribution total should be close to the daily P&L
        let daily = result.steps[1].daily_pnl.unwrap();
        let diff = (attr.total_pnl.amount() - daily.amount()).abs();
        assert!(
            diff < 1.0, // within $1 tolerance for rounding
            "attribution total ({}) should be close to daily P&L ({})",
            attr.total_pnl.amount(),
            daily.amount()
        );
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p finstack-portfolio --features scenarios --test replay replay_full_attribution`

Expected: PASS.

- [ ] **Step 5: Write test for max drawdown summary**

Add to `replay_tests`:

```rust
    #[test]
    fn replay_summary_tracks_max_drawdown() {
        let portfolio = build_test_portfolio();
        // Rates: 0bp → 200bp (value drops) → 100bp (partial recovery)
        let timeline = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), market_at_rate(date!(2024 - 01 - 01), 0.0)),
            (date!(2024 - 01 - 02), market_at_rate(date!(2024 - 01 - 02), 200.0)),
            (date!(2024 - 01 - 03), market_at_rate(date!(2024 - 01 - 03), 100.0)),
        ])
        .unwrap();

        let config = ReplayConfig {
            mode: ReplayMode::PvAndPnl,
            attribution_method: Default::default(),
            valuation_options: Default::default(),
        };

        let result = finstack_portfolio::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &FinstackConfig::default(),
        )
        .unwrap();

        // Max drawdown should be positive (a loss amount)
        assert!(result.summary.max_drawdown.amount() >= 0.0);
        // Peak should be at step 0 (rates started at 0)
        assert_eq!(result.summary.max_drawdown_peak_date, date!(2024 - 01 - 01));
        // Trough should be at step 1 (highest rates)
        assert_eq!(result.summary.max_drawdown_trough_date, date!(2024 - 01 - 02));
    }
```

- [ ] **Step 6: Run all replay tests**

Run: `cargo test -p finstack-portfolio --features scenarios --test replay`

Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add finstack/portfolio/tests/replay.rs
git commit -m "test(portfolio): add PvAndPnl, FullAttribution, and drawdown replay tests"
```

---

### Task 5: Replay Serde Round-Trip Test

**Files:**
- Modify: `finstack/portfolio/tests/replay.rs`

- [ ] **Step 1: Write serde round-trip test for ReplayConfig and ReplayResult**

Add to `replay_tests`:

```rust
    #[test]
    fn replay_config_roundtrips_via_json() {
        let config = ReplayConfig {
            mode: ReplayMode::FullAttribution,
            attribution_method: AttributionMethod::Parallel,
            valuation_options: Default::default(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ReplayConfig = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.mode, ReplayMode::FullAttribution));
    }

    #[test]
    fn replay_result_serializes_to_json() {
        let portfolio = build_test_portfolio();
        let timeline = ReplayTimeline::new(vec![
            (date!(2024 - 01 - 01), market_at_rate(date!(2024 - 01 - 01), 0.0)),
            (date!(2024 - 01 - 02), market_at_rate(date!(2024 - 01 - 02), 50.0)),
        ])
        .unwrap();

        let config = ReplayConfig {
            mode: ReplayMode::PvAndPnl,
            attribution_method: Default::default(),
            valuation_options: Default::default(),
        };

        let result = finstack_portfolio::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &FinstackConfig::default(),
        )
        .unwrap();

        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.is_empty());

        // Deserialize back
        let deserialized: finstack_portfolio::ReplayResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.steps.len(), 2);
        assert_eq!(deserialized.summary.num_steps, 2);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p finstack-portfolio --features scenarios --test replay replay_config_roundtrips && cargo test -p finstack-portfolio --features scenarios --test replay replay_result_serializes`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add finstack/portfolio/tests/replay.rs
git commit -m "test(portfolio): add replay serde round-trip tests"
```

---

### Task 6: Python Binding

**Files:**
- Create: `finstack-py/src/bindings/portfolio/replay.rs`
- Modify: `finstack-py/src/bindings/portfolio/mod.rs`

- [ ] **Step 1: Create the Python binding module**

```rust
// finstack-py/src/bindings/portfolio/replay.rs

//! Python binding for portfolio historical replay.

use crate::bindings::extract::extract_market;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn replay_to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Replay a portfolio through dated market snapshots.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// snapshots_json : str
///     JSON array of ``{"date": "YYYY-MM-DD", "market": {...}}`` objects.
///     Markets use the standard ``MarketContextState`` JSON format.
/// config_json : str
///     JSON-serialized ``ReplayConfig``.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ReplayResult``.
#[pyfunction]
fn replay_portfolio(spec_json: &str, snapshots_json: &str, config_json: &str) -> PyResult<String> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(replay_to_py)?;
    let portfolio =
        finstack_portfolio::Portfolio::from_spec(spec).map_err(replay_to_py)?;

    let config: finstack_portfolio::ReplayConfig =
        serde_json::from_str(config_json).map_err(replay_to_py)?;

    // Parse snapshots: [{"date": "YYYY-MM-DD", "market": {...}}, ...]
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(snapshots_json).map_err(replay_to_py)?;

    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let mut snapshots = Vec::with_capacity(raw.len());
    for entry in &raw {
        let date_str = entry["date"]
            .as_str()
            .ok_or_else(|| replay_to_py("each snapshot must have a 'date' string field"))?;
        let date = time::Date::parse(date_str, &format).map_err(replay_to_py)?;
        let market: finstack_core::market_data::context::MarketContext =
            serde_json::from_value(entry["market"].clone()).map_err(replay_to_py)?;
        snapshots.push((date, market));
    }

    let timeline = finstack_portfolio::ReplayTimeline::new(snapshots).map_err(replay_to_py)?;
    let finstack_config = finstack_core::config::FinstackConfig::default();

    let result =
        finstack_portfolio::replay_portfolio(&portfolio, &timeline, &config, &finstack_config)
            .map_err(replay_to_py)?;

    serde_json::to_string(&result).map_err(replay_to_py)
}

/// Register replay functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(replay_portfolio, m)?)?;
    Ok(())
}
```

- [ ] **Step 2: Wire up in `mod.rs`**

In `finstack-py/src/bindings/portfolio/mod.rs`, add the module declaration with the other sub-modules (after line 10 `mod spec;`):

```rust
mod replay;
```

Add the `register` call in the `register` function body (after `optimization::register(py, &m)?;`):

```rust
    replay::register(py, &m)?;
```

Add `"replay_portfolio"` to the `exports` vec.

- [ ] **Step 3: Check that it compiles**

Run: `cargo check -p finstack-py`

Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add finstack-py/src/bindings/portfolio/replay.rs finstack-py/src/bindings/portfolio/mod.rs
git commit -m "feat(python): add replay_portfolio binding"
```

---

### Task 7: WASM Binding

**Files:**
- Modify: `finstack-wasm/src/api/portfolio/mod.rs`

- [ ] **Step 1: Add `replayPortfolio` function**

Add to the bottom of `finstack-wasm/src/api/portfolio/mod.rs`, before the `#[cfg(test)]` block:

```rust
/// Replay a portfolio through dated market snapshots.
///
/// Accepts a portfolio spec, an array of dated market snapshots, and a
/// replay configuration. Returns a JSON-serialized `ReplayResult`.
#[wasm_bindgen(js_name = replayPortfolio)]
pub fn replay_portfolio(
    spec_json: &str,
    snapshots_json: &str,
    config_json: &str,
) -> Result<String, JsValue> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;

    let config: finstack_portfolio::ReplayConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;

    // Parse snapshots: [{"date": "YYYY-MM-DD", "market": {...}}, ...]
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(snapshots_json).map_err(to_js_err)?;

    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let mut snapshots = Vec::with_capacity(raw.len());
    for entry in &raw {
        let date_str = entry["date"]
            .as_str()
            .ok_or_else(|| to_js_err("each snapshot must have a 'date' string field"))?;
        let date = time::Date::parse(date_str, &format).map_err(to_js_err)?;
        let market: finstack_core::market_data::context::MarketContext =
            serde_json::from_value(entry["market"].clone()).map_err(to_js_err)?;
        snapshots.push((date, market));
    }

    let timeline = finstack_portfolio::ReplayTimeline::new(snapshots).map_err(to_js_err)?;
    let finstack_config = finstack_core::config::FinstackConfig::default();

    let result =
        finstack_portfolio::replay_portfolio(&portfolio, &timeline, &config, &finstack_config)
            .map_err(to_js_err)?;

    serde_json::to_string(&result).map_err(to_js_err)
}
```

- [ ] **Step 2: Add WASM test**

Add inside the existing `#[cfg(test)]` block at the bottom of the same file:

```rust
    #[test]
    fn replay_portfolio_empty_portfolio() {
        let spec = minimal_portfolio_spec_json();
        let market = empty_market_json();
        let snapshots = serde_json::json!([
            {"date": "2024-01-15", "market": serde_json::from_str::<serde_json::Value>(&market).unwrap()},
            {"date": "2024-01-16", "market": serde_json::from_str::<serde_json::Value>(&market).unwrap()}
        ]).to_string();
        let config = serde_json::json!({"mode": "PvOnly", "attribution_method": "Parallel", "valuation_options": {}}).to_string();
        let result = replay_portfolio(&spec, &snapshots, &config).expect("replay");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(parsed["steps"].is_array());
        assert_eq!(parsed["steps"].as_array().unwrap().len(), 2);
    }
```

- [ ] **Step 3: Check that it compiles**

Run: `cargo check -p finstack-wasm`

Expected: Compiles without errors.

- [ ] **Step 4: Run WASM tests**

Run: `cargo test -p finstack-wasm replay_portfolio`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack-wasm/src/api/portfolio/mod.rs
git commit -m "feat(wasm): add replayPortfolio binding"
```

---

### Task 8: Full Build Verification

**Files:** None (verification only)

- [ ] **Step 1: Run all portfolio tests**

Run: `cargo test -p finstack-portfolio --features scenarios`

Expected: All existing + new replay tests pass.

- [ ] **Step 2: Run Python binding tests**

Run: `cargo test -p finstack-py`

Expected: All tests pass.

- [ ] **Step 3: Run WASM binding tests**

Run: `cargo test -p finstack-wasm`

Expected: All tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p finstack-portfolio --features scenarios -- -D warnings`

Expected: No warnings. If clippy flags `unwrap_or_default()` on Date or the `unwrap()` in `compute_summary`, refactor to avoid them (the function is only called with non-empty steps, but clippy doesn't know that).

- [ ] **Step 5: Commit any clippy fixes if needed**

```bash
git add -u
git commit -m "fix(portfolio): address clippy warnings in replay module"
```
