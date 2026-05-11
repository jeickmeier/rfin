# Calibration Envelope v3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the confusing `CalibrationEnvelope.initial_market` field with two
explicit sections (`market_data`, `prior_market`) using a flat
`(kind, id)`-addressable model, and turn `plan.quote_sets` into lists of quote
IDs that index into `market_data`.

**Architecture:** Hard cut to schema v3. New types `MarketDatum` and
`PriorMarketObject` are tagged enums with `kind` discriminators. The engine
materialises a `MarketContext` from the two lists before running steps; output
(`CalibrationResult.final_market`) stays a single merged `MarketContextState`.
Fixture migration is automated with a one-shot Python script.

**Tech Stack:** Rust (workspace), `serde`/`serde_json`, `schemars`, `ts-rs`,
Python 3 (one-shot migration script only), `jq` (smoke checks).

**Spec:** [docs/2026-05-10-calibration-envelope-cleanup-design.md](docs/2026-05-10-calibration-envelope-cleanup-design.md)

---

## File structure

### New files
| Path | Responsibility |
|---|---|
| `finstack/valuations/src/calibration/api/market_datum.rs` | `MarketDatum` enum + accessors + bucket-partition helper |
| `finstack/valuations/src/calibration/api/prior_market.rs` | `PriorMarketObject` enum + curve/surface dispatch |
| `finstack/valuations/src/calibration/api/context_builder.rs` | `build_initial_context(env) -> MarketContext` (was inline in engine.rs) |
| `tools/migrate_envelope_v2_to_v3.py` | One-shot v2→v3 JSON rewriter; deleted after fixtures land |
| `finstack/valuations/schemas/calibration/3/calibration.schema.json` | Regenerated schema |

### Modified files
| Path | Change |
|---|---|
| `finstack/valuations/src/calibration/api/schema.rs` | Replace `CalibrationEnvelope` fields; switch `quote_sets` to ID lists |
| `finstack/valuations/src/calibration/api/engine.rs` | Use `context_builder::build_initial_context`; rewrite quote resolution |
| `finstack/valuations/src/calibration/api/validate.rs` | Replace `collect_initial_ids`; add uniqueness + quote-set resolution checks |
| `finstack/valuations/src/calibration/api/errors.rs` | Two new error variants; doc fixes |
| `finstack/valuations/src/calibration/api/mod.rs` | Re-export new modules |
| `finstack/valuations/src/calibration/config.rs` | Add `CalibrationConfig.fx: FxConfig`, `CalibrationConfig.hierarchy: Option<...>` |
| `finstack/valuations/src/calibration/mod.rs` | Module-level doc fixes |
| `finstack/valuations/src/lib.rs` | Crate doc fixes |
| `finstack/valuations/src/instruments/rates/fra/metrics/dv01.rs` | Update one envelope construction |
| `finstack/valuations/src/instruments/rates/cap_floor/metrics/dv01.rs` | Update one envelope construction |
| `finstack/valuations/tests/support/test_utils.rs` | Convert builder to emit flat lists |
| `finstack/valuations/tests/calibration/*.rs` (13 files) | Update envelope-construction call sites |
| `finstack/valuations/benches/calibration.rs`, `benches/global_calibration.rs` | Same |
| `finstack/valuations/examples/market_bootstrap/*.json` (12 files) | Re-emitted via migration script |
| `finstack/valuations/tests/golden/data/pricing/**/*.json` (36 files) | Re-emitted via migration script |
| `finstack/valuations/examples/market_bootstrap/README.md` | Restate Track A/B for v3 |
| `finstack-wasm/src/api/valuations/calibration.rs` | Adjust binding shims |
| `finstack-wasm/index.d.ts`, `finstack-wasm/types/generated/CalibrationEnvelope.ts` | Regenerated TS exports |

### Deleted at end of migration
- `tools/migrate_envelope_v2_to_v3.py` (script removed after fixtures land)

---

## Phase 1 — Types and serialization

### Task 1: `MarketDatum` enum scaffold

**Files:**
- Create: `finstack/valuations/src/calibration/api/market_datum.rs`
- Modify: `finstack/valuations/src/calibration/api/mod.rs`
- Test: `finstack/valuations/src/calibration/api/market_datum.rs` (unit-level `#[cfg(test)]` module)

- [ ] **Step 1: Write failing test for round-tripping each `MarketDatum` variant**

Create the file with this test module (the enum doesn't exist yet — that's the point of the failing test):

```rust
// finstack/valuations/src/calibration/api/market_datum.rs
//! Flat-list market-data inputs for `CalibrationEnvelope` v3.

use crate::market::quotes::market_quote::{BondQuote, CdsQuote, CDSTrancheQuote, FxQuote, InflationQuote, RateQuote, VolQuote, XccyQuote};
use finstack_core::currency::Currency;
use finstack_core::market_data::context::CreditIndexState;
use finstack_core::market_data::dividends::DividendSchedule;
use finstack_core::market_data::scalars::{InflationIndex, MarketScalar, ScalarTimeSeries};
use finstack_core::market_data::surfaces::{FxDeltaVolSurface, VolCube};
use finstack_core::money::fx::FxRate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single id-addressable input to the calibrator.
///
/// All variants are tagged with `kind` for JSON serialisation. The `id()`
/// accessor returns the entry's identifier regardless of variant.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketDatum {
    // Quote kinds (mirror existing `MarketQuote` variants flattened in)
    RateQuote(RateQuote),
    CdsQuote(CdsQuote),
    CdsTrancheQuote(CDSTrancheQuote),
    FxQuote(FxQuote),
    InflationQuote(InflationQuote),
    VolQuote(VolQuote),
    XccyQuote(XccyQuote),
    BondQuote(BondQuote),

    // Snapshot-only kinds
    FxSpot(FxSpotDatum),
    Price(PriceDatum),
    DividendSchedule(DividendScheduleDatum),
    FixingSeries(ScalarTimeSeries),
    InflationFixings(InflationIndex),
    CreditIndex(CreditIndexState),
    FxVolSurface(FxDeltaVolSurface),
    VolCube(VolCube),
    Collateral(CollateralEntry),
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FxSpotDatum {
    pub id: String,
    pub from: Currency,
    pub to: Currency,
    pub rate: FxRate,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PriceDatum {
    pub id: String,
    pub scalar: MarketScalar,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DividendScheduleDatum {
    pub schedule: DividendSchedule,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CollateralEntry {
    /// The currency posting collateral (key).
    pub id: Currency,
    /// The CSA discount currency.
    pub csa_currency: Currency,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_price_datum() {
        let datum = MarketDatum::Price(PriceDatum {
            id: "AAPL".into(),
            scalar: MarketScalar::new("AAPL", 175.42, Currency::USD),
        });
        let json = serde_json::to_string(&datum).unwrap();
        assert!(json.contains(r#""kind":"price""#));
        let back: MarketDatum = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, MarketDatum::Price(_)));
    }
}
```

- [ ] **Step 2: Wire module into `mod.rs`**

```rust
// finstack/valuations/src/calibration/api/mod.rs   (add the line)
pub mod market_datum;
```

- [ ] **Step 3: Run test, expect compile errors**

Run: `cargo test -p finstack-valuations market_datum::tests::roundtrip_price_datum -- --nocapture`
Expected: compile error if any referenced item (e.g. `MarketScalar::new`) doesn't exist — fix the test to match the real constructor before claiming completion. Resolution: open `finstack/core/src/market_data/scalars/*.rs`, find the actual `MarketScalar` constructor, replace `MarketScalar::new("AAPL", 175.42, Currency::USD)` in the test with the correct expression.

- [ ] **Step 4: Run test again — must pass**

Run: `cargo test -p finstack-valuations market_datum::tests::roundtrip_price_datum`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/src/calibration/api/market_datum.rs \
        finstack/valuations/src/calibration/api/mod.rs
git commit -m "feat(calibration): add MarketDatum enum scaffold for v3 envelope"
```

---

### Task 2: `MarketDatum` accessors (`id()`, `as_quote()`, `kind_name()`)

**Files:**
- Modify: `finstack/valuations/src/calibration/api/market_datum.rs`

- [ ] **Step 1: Write failing test**

Append to the existing `mod tests` block:

```rust
#[test]
fn id_accessor_returns_variant_id() {
    use finstack_core::types::QuoteId;
    let rq = RateQuote::Deposit {
        id: QuoteId::new("USD-DEP-1M"),
        // ... use real RateQuote::Deposit constructor — open
        //     finstack/valuations/src/market/quotes/rates.rs for the exact field list
    };
    let datum = MarketDatum::RateQuote(rq);
    assert_eq!(datum.id(), "USD-DEP-1M");
    assert_eq!(datum.kind_name(), "rate_quote");
    assert!(datum.as_quote().is_some());
}

#[test]
fn price_is_not_a_quote() {
    let datum = MarketDatum::Price(PriceDatum {
        id: "AAPL".into(),
        scalar: /* same constructor as task 1 */,
    });
    assert!(datum.as_quote().is_none());
}
```

- [ ] **Step 2: Run, expect compile failure (no `id()`/`as_quote()`/`kind_name()`)**

Run: `cargo test -p finstack-valuations market_datum::tests::id_accessor_returns_variant_id`
Expected: error[E0599]: no method named `id` / `kind_name` / `as_quote`.

- [ ] **Step 3: Implement accessors**

Add to `market_datum.rs` (above the `#[cfg(test)] mod tests` block):

```rust
use crate::market::quotes::market_quote::MarketQuote;

impl MarketDatum {
    /// Return the entry's identifier as a string slice.
    pub fn id(&self) -> &str {
        match self {
            MarketDatum::RateQuote(q) => q.id().as_str(),
            MarketDatum::CdsQuote(q) => q.id().as_str(),
            MarketDatum::CdsTrancheQuote(q) => q.id().as_str(),
            MarketDatum::FxQuote(q) => q.id().as_str(),
            MarketDatum::InflationQuote(q) => q.id().as_str(),
            MarketDatum::VolQuote(q) => q.id().as_str(),
            MarketDatum::XccyQuote(q) => q.id().as_str(),
            MarketDatum::BondQuote(q) => q.id().as_str(),
            MarketDatum::FxSpot(d) => &d.id,
            MarketDatum::Price(d) => &d.id,
            MarketDatum::DividendSchedule(d) => &d.schedule.id,
            MarketDatum::FixingSeries(s) => s.id().as_str(),
            MarketDatum::InflationFixings(i) => &i.id,
            MarketDatum::CreditIndex(c) => &c.id,
            MarketDatum::FxVolSurface(s) => s.id(),
            MarketDatum::VolCube(c) => c.id(),
            MarketDatum::Collateral(c) => c.id.as_str(),
        }
    }

    /// `"rate_quote"`, `"price"`, etc. Mirrors the serde `kind` tag.
    pub fn kind_name(&self) -> &'static str {
        match self {
            MarketDatum::RateQuote(_) => "rate_quote",
            MarketDatum::CdsQuote(_) => "cds_quote",
            MarketDatum::CdsTrancheQuote(_) => "cds_tranche_quote",
            MarketDatum::FxQuote(_) => "fx_quote",
            MarketDatum::InflationQuote(_) => "inflation_quote",
            MarketDatum::VolQuote(_) => "vol_quote",
            MarketDatum::XccyQuote(_) => "xccy_quote",
            MarketDatum::BondQuote(_) => "bond_quote",
            MarketDatum::FxSpot(_) => "fx_spot",
            MarketDatum::Price(_) => "price",
            MarketDatum::DividendSchedule(_) => "dividend_schedule",
            MarketDatum::FixingSeries(_) => "fixing_series",
            MarketDatum::InflationFixings(_) => "inflation_fixings",
            MarketDatum::CreditIndex(_) => "credit_index",
            MarketDatum::FxVolSurface(_) => "fx_vol_surface",
            MarketDatum::VolCube(_) => "vol_cube",
            MarketDatum::Collateral(_) => "collateral",
        }
    }

    /// Returns `Some(MarketQuote)` for the eight quote variants, `None` otherwise.
    pub fn as_quote(&self) -> Option<MarketQuote> {
        Some(match self {
            MarketDatum::RateQuote(q) => MarketQuote::Rates(q.clone()),
            MarketDatum::CdsQuote(q) => MarketQuote::Cds(q.clone()),
            MarketDatum::CdsTrancheQuote(q) => MarketQuote::CDSTranche(q.clone()),
            MarketDatum::FxQuote(q) => MarketQuote::Fx(q.clone()),
            MarketDatum::InflationQuote(q) => MarketQuote::Inflation(q.clone()),
            MarketDatum::VolQuote(q) => MarketQuote::Vol(q.clone()),
            MarketDatum::XccyQuote(q) => MarketQuote::Xccy(q.clone()),
            MarketDatum::BondQuote(q) => MarketQuote::Bond(q.clone()),
            _ => return None,
        })
    }

    /// `true` if this variant is one of the eight `*_quote` kinds.
    pub fn is_quote(&self) -> bool {
        self.as_quote().is_some()
    }
}
```

If any quote-variant `id()` accessor doesn't exist yet, add it in
`finstack/valuations/src/market/quotes/<name>.rs` before continuing —
match the existing `FxQuote::id` style at
`finstack/valuations/src/market/quotes/fx.rs:72`.

- [ ] **Step 4: Run tests — must pass**

Run: `cargo test -p finstack-valuations market_datum::tests`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/src/calibration/api/market_datum.rs \
        finstack/valuations/src/market/quotes/
git commit -m "feat(calibration): MarketDatum accessors (id, kind_name, as_quote)"
```

---

### Task 3: `PriorMarketObject` enum

**Files:**
- Create: `finstack/valuations/src/calibration/api/prior_market.rs`
- Modify: `finstack/valuations/src/calibration/api/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
// finstack/valuations/src/calibration/api/prior_market.rs
//! Pre-built calibrated objects layered into the initial context before steps run.

use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::{
    BaseCorrelationCurve, BasisSpreadCurve, DiscountCurve, ForwardCurve,
    HazardCurve, InflationCurve, ParametricCurve, PriceCurve,
    VolatilityIndexCurve,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One pre-built object from a prior calibration to load into the initial
/// `MarketContext` before the new plan's steps execute.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PriorMarketObject {
    DiscountCurve(DiscountCurve),
    ForwardCurve(ForwardCurve),
    HazardCurve(HazardCurve),
    InflationCurve(InflationCurve),
    BaseCorrelationCurve(BaseCorrelationCurve),
    BasisSpreadCurve(BasisSpreadCurve),
    ParametricCurve(ParametricCurve),
    PriceCurve(PriceCurve),
    VolatilityIndexCurve(VolatilityIndexCurve),
    VolSurface(VolSurface),
}

impl PriorMarketObject {
    pub fn id(&self) -> &str {
        match self {
            PriorMarketObject::DiscountCurve(c) => c.id().as_str(),
            PriorMarketObject::ForwardCurve(c) => c.id().as_str(),
            PriorMarketObject::HazardCurve(c) => c.id().as_str(),
            PriorMarketObject::InflationCurve(c) => c.id().as_str(),
            PriorMarketObject::BaseCorrelationCurve(c) => c.id().as_str(),
            PriorMarketObject::BasisSpreadCurve(c) => c.id().as_str(),
            PriorMarketObject::ParametricCurve(c) => c.id().as_str(),
            PriorMarketObject::PriceCurve(c) => c.id().as_str(),
            PriorMarketObject::VolatilityIndexCurve(c) => c.id().as_str(),
            PriorMarketObject::VolSurface(s) => s.id(),
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            PriorMarketObject::DiscountCurve(_) => "discount_curve",
            PriorMarketObject::ForwardCurve(_) => "forward_curve",
            PriorMarketObject::HazardCurve(_) => "hazard_curve",
            PriorMarketObject::InflationCurve(_) => "inflation_curve",
            PriorMarketObject::BaseCorrelationCurve(_) => "base_correlation_curve",
            PriorMarketObject::BasisSpreadCurve(_) => "basis_spread_curve",
            PriorMarketObject::ParametricCurve(_) => "parametric_curve",
            PriorMarketObject::PriceCurve(_) => "price_curve",
            PriorMarketObject::VolatilityIndexCurve(_) => "volatility_index_curve",
            PriorMarketObject::VolSurface(_) => "vol_surface",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discount_curve_round_trips_with_kind_tag() {
        // Construct a minimal DiscountCurve — open
        //   finstack/core/src/market_data/term_structures/discount.rs
        // for the actual constructor and substitute below.
        let curve: DiscountCurve = /* use real test constructor used elsewhere in core */;
        let obj = PriorMarketObject::DiscountCurve(curve);
        let json = serde_json::to_string(&obj).unwrap();
        assert!(json.contains(r#""kind":"discount_curve""#));
        let back: PriorMarketObject = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind_name(), "discount_curve");
    }
}
```

Add to `mod.rs`:
```rust
pub mod prior_market;
```

- [ ] **Step 2: Run test, expect failures, fix the `DiscountCurve` constructor in the test to match the real API by grepping for `DiscountCurve::new` or similar in `finstack/core/src/`**

Run: `cargo test -p finstack-valuations prior_market::tests`

- [ ] **Step 3: Run test — must pass**

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/calibration/api/prior_market.rs \
        finstack/valuations/src/calibration/api/mod.rs
git commit -m "feat(calibration): add PriorMarketObject enum"
```

---

### Task 4: Move `FxConfig` and `hierarchy` into `CalibrationConfig`

**Files:**
- Modify: `finstack/valuations/src/calibration/config.rs`
- Test: `finstack/valuations/src/calibration/config.rs` (inline `#[cfg(test)]` module)

- [ ] **Step 1: Write failing test**

Append:

```rust
#[cfg(test)]
mod fx_and_hierarchy_settings_tests {
    use super::CalibrationConfig;
    use finstack_core::money::fx::FxConfig;

    #[test]
    fn config_defaults_carry_fx_subsection() {
        let cfg = CalibrationConfig::default();
        assert_eq!(cfg.fx, FxConfig::default());
        assert!(cfg.hierarchy.is_none());
    }

    #[test]
    fn fx_settings_round_trip_via_serde() {
        let json = r#"{ "fx": { "pivot_currency": "EUR", "enable_triangulation": false, "cache_capacity": 8 } }"#;
        let cfg: CalibrationConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.fx.pivot_currency, finstack_core::currency::Currency::EUR);
        assert!(!cfg.fx.enable_triangulation);
    }
}
```

- [ ] **Step 2: Implement**

In `config.rs` add the two new fields to `CalibrationConfig`:

```rust
use finstack_core::money::fx::FxConfig;
use finstack_core::market_data::hierarchy::MarketDataHierarchy;

#[derive(/* existing derives */, Default)]
#[serde(deny_unknown_fields)]
pub struct CalibrationConfig {
    // ... existing fields stay ...

    /// FX-matrix runtime config: pivot currency, triangulation, cache capacity.
    #[serde(default)]
    pub fx: FxConfig,

    /// Optional market-data hierarchy snapshot (was `initial_market.hierarchy`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<MarketDataHierarchy>,
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p finstack-valuations calibration::config::fx_and_hierarchy_settings_tests`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/calibration/config.rs
git commit -m "feat(calibration): hoist FxConfig+hierarchy into CalibrationConfig"
```

---

### Task 5: Replace `CalibrationEnvelope.initial_market` and shrink `quote_sets`

**Files:**
- Modify: `finstack/valuations/src/calibration/api/schema.rs`

This is a breaking type change. The next two tasks (validate.rs, engine.rs) will
not compile until we land them — that's intentional. We chain commits and keep
the test suite red between Task 5 and Task 8.

- [ ] **Step 1: Edit `CalibrationEnvelope` struct**

Replace the `initial_market` field at [schema.rs:101](finstack/valuations/src/calibration/api/schema.rs:101) with:

```rust
use crate::calibration::api::market_datum::MarketDatum;
use crate::calibration::api::prior_market::PriorMarketObject;

pub struct CalibrationEnvelope {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub schema_url: Option<String>,
    pub schema: String,
    pub plan: CalibrationPlan,

    /// Flat, id-addressable market data inputs (quotes + snapshot data).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub market_data: Vec<MarketDatum>,

    /// Pre-built calibrated objects from a prior run, layered in before steps execute.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prior_market: Vec<PriorMarketObject>,
}
```

Delete the `#[schemars(with = "Option<serde_json::Value>")]` and ts_export
`type = "unknown | null"` overrides that previously surrounded `initial_market`.

- [ ] **Step 2: Edit `CalibrationPlan.quote_sets`**

Replace `HashMap<String, Vec<MarketQuote>>` with `HashMap<String, Vec<QuoteId>>`.
Remove the now-unused `#[schemars(with = ...)]` and ts_export overrides that
referenced `Array<unknown>`. The new field is plain strings — schemars infers
correctly.

```rust
use finstack_core::types::QuoteId;

pub struct CalibrationPlan {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Named lists of quote IDs that step `quote_set` references resolve to.
    /// Each ID must exist in the envelope's `market_data` as a `*_quote` kind.
    #[serde(default)]
    pub quote_sets: HashMap<String, Vec<QuoteId>>,
    pub steps: Vec<CalibrationStep>,
    #[serde(default)]
    pub settings: CalibrationConfig,
}
```

- [ ] **Step 3: Build will fail across the crate — that's expected**

Run: `cargo check -p finstack-valuations 2>&1 | head -40`
Expected: many `no field 'initial_market'` errors and `expected Vec<QuoteId>,
found Vec<MarketQuote>` errors. These will be resolved in Tasks 6–10.

- [ ] **Step 4: Commit the partial state**

```bash
git add finstack/valuations/src/calibration/api/schema.rs
git commit -m "feat(calibration): cut CalibrationEnvelope to v3 (market_data + prior_market)"
```

---

### Task 6: Convert `From<MarketContextState>` helper

**Files:**
- Modify: `finstack/valuations/src/calibration/api/market_datum.rs`
- Modify: `finstack/valuations/src/calibration/api/prior_market.rs`

- [ ] **Step 1: Write failing test in `market_datum.rs`**

```rust
#[test]
fn market_context_state_splits_into_prior_and_data() {
    use crate::calibration::api::prior_market::PriorMarketObject;
    use finstack_core::market_data::context::{MarketContext, MarketContextState};

    let state: MarketContextState = (&MarketContext::new()).into();
    let (prior, data): (Vec<PriorMarketObject>, Vec<MarketDatum>) = state.into();
    assert!(prior.is_empty());
    assert!(data.is_empty());
}
```

- [ ] **Step 2: Implement the conversion**

Add to `market_datum.rs` (top-level, not in tests):

```rust
use crate::calibration::api::prior_market::PriorMarketObject;
use finstack_core::market_data::context::{CurveState, MarketContextState};

impl From<MarketContextState> for (Vec<PriorMarketObject>, Vec<MarketDatum>) {
    fn from(state: MarketContextState) -> Self {
        let mut prior = Vec::new();
        for curve in state.curves {
            prior.push(match curve {
                CurveState::DiscountCurve(c) => PriorMarketObject::DiscountCurve(c),
                CurveState::ForwardCurve(c) => PriorMarketObject::ForwardCurve(c),
                CurveState::HazardCurve(c) => PriorMarketObject::HazardCurve(c),
                CurveState::InflationCurve(c) => PriorMarketObject::InflationCurve(c),
                CurveState::BaseCorrelationCurve(c) => PriorMarketObject::BaseCorrelationCurve(c),
                CurveState::BasisSpreadCurve(c) => PriorMarketObject::BasisSpreadCurve(c),
                CurveState::ParametricCurve(c) => PriorMarketObject::ParametricCurve(c),
                CurveState::PriceCurve(c) => PriorMarketObject::PriceCurve(c),
                CurveState::VolatilityIndexCurve(c) => PriorMarketObject::VolatilityIndexCurve(c),
            });
        }
        for surface in state.surfaces {
            prior.push(PriorMarketObject::VolSurface(surface));
        }

        let mut data = Vec::new();
        if let Some(fx) = state.fx {
            for (from, to, rate) in fx.quotes {
                data.push(MarketDatum::FxSpot(FxSpotDatum {
                    id: format!("{from}/{to}"),
                    from,
                    to,
                    rate,
                }));
            }
        }
        for (id, scalar) in state.prices {
            data.push(MarketDatum::Price(PriceDatum { id, scalar }));
        }
        for s in state.series {
            data.push(MarketDatum::FixingSeries(s));
        }
        for i in state.inflation_indices {
            data.push(MarketDatum::InflationFixings(i));
        }
        for d in state.dividends {
            data.push(MarketDatum::DividendSchedule(DividendScheduleDatum { schedule: d }));
        }
        for c in state.credit_indices {
            data.push(MarketDatum::CreditIndex(c));
        }
        for s in state.fx_delta_vol_surfaces {
            data.push(MarketDatum::FxVolSurface(s));
        }
        for c in state.vol_cubes {
            data.push(MarketDatum::VolCube(c));
        }
        for (ccy, csa_ccy) in state.collateral {
            data.push(MarketDatum::Collateral(CollateralEntry {
                id: ccy.parse().expect("currency in collateral map"),
                csa_currency: csa_ccy.parse().expect("CSA currency"),
            }));
        }
        (prior, data)
    }
}
```

If `Currency::parse` (or `FromStr`) isn't available, open
`finstack/core/src/currency/mod.rs` for the actual API and substitute. If
`MarketContextState.collateral` already stores typed `Currency`, drop the
`.parse()`.

- [ ] **Step 3: Run test**

Run: `cargo test -p finstack-valuations market_datum::tests::market_context_state_splits_into_prior_and_data`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/calibration/api/market_datum.rs
git commit -m "feat(calibration): split MarketContextState into prior+market_data"
```

---

## Phase 2 — Engine integration

### Task 7: `build_initial_context` module

**Files:**
- Create: `finstack/valuations/src/calibration/api/context_builder.rs`
- Modify: `finstack/valuations/src/calibration/api/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
// finstack/valuations/src/calibration/api/context_builder.rs
//! Materialise a `MarketContext` from a v3 `CalibrationEnvelope`'s
//! `prior_market` + `market_data` lists.

use crate::calibration::api::market_datum::MarketDatum;
use crate::calibration::api::prior_market::PriorMarketObject;
use crate::calibration::config::CalibrationConfig;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::{FxMatrix, FxMatrixState};
use finstack_core::Result;

pub fn build_initial_context(
    prior: &[PriorMarketObject],
    data: &[MarketDatum],
    settings: &CalibrationConfig,
) -> Result<MarketContext> {
    use std::sync::Arc;
    let mut ctx = MarketContext::new();

    for obj in prior {
        insert_prior(&mut ctx, obj.clone())?;
    }

    // Bucket market data by kind so categorical writes are one pass per kind.
    let mut fx_quotes = Vec::new();
    for d in data {
        match d {
            MarketDatum::FxSpot(spot) => fx_quotes.push((spot.from, spot.to, spot.rate)),
            MarketDatum::Price(p) => {
                ctx.prices.insert(
                    finstack_core::types::CurveId::from(p.id.clone()),
                    p.scalar.clone(),
                );
            }
            MarketDatum::DividendSchedule(d) => {
                ctx.dividends.insert(d.schedule.id.clone(), Arc::new(d.schedule.clone()));
            }
            MarketDatum::FixingSeries(s) => {
                ctx.series.insert(s.id().clone(), s.clone());
            }
            MarketDatum::InflationFixings(i) => {
                let id = MarketContext::inflation_index_key_for_insert(i.id.clone(), i);
                ctx.inflation_indices.insert(id, Arc::new(i.clone()));
            }
            MarketDatum::CreditIndex(c) => apply_credit_index(&mut ctx, c)?,
            MarketDatum::FxVolSurface(s) => {
                ctx.fx_delta_vol_surfaces.insert(s.id().to_string(), Arc::new(s.clone()));
            }
            MarketDatum::VolCube(c) => {
                ctx.vol_cubes.insert(c.id().to_string(), Arc::new(c.clone()));
            }
            MarketDatum::Collateral(c) => {
                ctx.collateral.insert(c.id.to_string(), c.csa_currency.to_string());
            }
            MarketDatum::RateQuote(_)
            | MarketDatum::CdsQuote(_)
            | MarketDatum::CdsTrancheQuote(_)
            | MarketDatum::FxQuote(_)
            | MarketDatum::InflationQuote(_)
            | MarketDatum::VolQuote(_)
            | MarketDatum::XccyQuote(_)
            | MarketDatum::BondQuote(_) => {
                // Quotes are consumed by step execution, not loaded into ctx.
            }
        }
    }

    if !fx_quotes.is_empty() {
        let fx_state = FxMatrixState {
            config: settings.fx.clone(),
            quotes: fx_quotes,
        };
        // Reuse the existing MarketContextState→FxMatrix restore path to keep
        // SnapshotFxProvider behaviour identical to today.
        let mut state = finstack_core::market_data::context::MarketContextState::empty_with_fx(
            fx_state,
        );
        // Convert just the fx field; rebuild the matrix the same way state_serde does.
        ctx.fx = Some(Arc::new(FxMatrix::from_snapshot_state(&state.fx.take().unwrap(), settings.fx.clone())?));
    }

    if let Some(h) = settings.hierarchy.clone() {
        ctx.hierarchy = Some(h);
    }

    Ok(ctx)
}

fn insert_prior(ctx: &mut MarketContext, obj: PriorMarketObject) -> Result<()> {
    use finstack_core::market_data::context::{CurveState, curve_storage::CurveStorage};
    use std::sync::Arc;
    let state = match obj {
        PriorMarketObject::DiscountCurve(c) => CurveState::DiscountCurve(c),
        PriorMarketObject::ForwardCurve(c) => CurveState::ForwardCurve(c),
        PriorMarketObject::HazardCurve(c) => CurveState::HazardCurve(c),
        PriorMarketObject::InflationCurve(c) => CurveState::InflationCurve(c),
        PriorMarketObject::BaseCorrelationCurve(c) => CurveState::BaseCorrelationCurve(c),
        PriorMarketObject::BasisSpreadCurve(c) => CurveState::BasisSpreadCurve(c),
        PriorMarketObject::ParametricCurve(c) => CurveState::ParametricCurve(c),
        PriorMarketObject::PriceCurve(c) => CurveState::PriceCurve(c),
        PriorMarketObject::VolatilityIndexCurve(c) => CurveState::VolatilityIndexCurve(c),
        PriorMarketObject::VolSurface(s) => {
            ctx.surfaces.insert(s.id().clone(), Arc::new(s));
            return Ok(());
        }
    };
    let storage = CurveStorage::from_state(state);
    ctx.curves.insert(storage.id().clone(), storage);
    Ok(())
}

fn apply_credit_index(
    ctx: &mut MarketContext,
    s: &finstack_core::market_data::context::CreditIndexState,
) -> Result<()> {
    // Same logic as the credit_indices branch in
    //   finstack/core/src/market_data/context/state_serde.rs:447..489
    // — copy it here verbatim. Keep this helper colocated with the
    // context builder; do not refactor state_serde.rs in this task.
    todo!("port credit-index reconstruction from state_serde.rs:447..489 — keep semantics identical")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inputs_build_empty_context() {
        let ctx = build_initial_context(&[], &[], &CalibrationConfig::default()).unwrap();
        assert_eq!(ctx.curves.len(), 0);
        assert!(ctx.fx.is_none());
    }
}
```

The `todo!()` is a placeholder for the credit-index port in step 2 of this task,
*not* a long-term placeholder. Open `state_serde.rs` lines 447–489 and copy the
body, adapting `state.credit_indices` iteration to operate on a single `&CreditIndexState`.

Also: `MarketContextState::empty_with_fx` and
`FxMatrix::from_snapshot_state` don't exist yet — they're a refactor of the
current logic in `state_serde.rs`. If you'd rather not add helpers, inline the
existing `FxMatrix::try_with_config`+`load_from_state` calls from
`state_serde.rs:419..422` directly here.

- [ ] **Step 2: Port the credit-index logic, removing the `todo!`**

- [ ] **Step 3: Wire module into `mod.rs`**

```rust
pub mod context_builder;
```

- [ ] **Step 4: Run test**

Run: `cargo test -p finstack-valuations context_builder::tests::empty_inputs_build_empty_context`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add finstack/valuations/src/calibration/api/context_builder.rs \
        finstack/valuations/src/calibration/api/mod.rs
git commit -m "feat(calibration): build_initial_context for v3 envelope"
```

---

### Task 8: Engine quote resolution via `market_data`

**Files:**
- Modify: `finstack/valuations/src/calibration/api/engine.rs`

- [ ] **Step 1: Add `resolve_step_quotes`**

Above the existing `execute_sequential` function, add:

```rust
use crate::calibration::api::market_datum::MarketDatum;
use std::collections::HashMap;
use finstack_core::types::QuoteId;

fn resolve_step_quotes(
    plan: &CalibrationPlan,
    market_data: &[MarketDatum],
    step: &CalibrationStep,
) -> std::result::Result<Vec<MarketQuote>, ExecuteError> {
    let ids = plan.quote_sets.get(&step.quote_set).ok_or_else(|| {
        ExecuteError::Other(finstack_core::Error::Input(
            finstack_core::InputError::NotFound {
                id: format!("Quote set '{}' not found", step.quote_set),
            },
        ))
    })?;
    let by_id: HashMap<&str, MarketQuote> = market_data
        .iter()
        .filter_map(|d| d.as_quote().map(|q| (d.id(), q)))
        .collect();
    ids.iter()
        .map(|qid| {
            by_id.get(qid.as_str()).cloned().ok_or_else(|| {
                ExecuteError::Other(finstack_core::Error::Input(
                    finstack_core::InputError::NotFound {
                        id: format!(
                            "Quote ID '{}' (referenced by quote_set '{}') not in market_data",
                            qid, step.quote_set
                        ),
                    },
                ))
            })
        })
        .collect()
}
```

- [ ] **Step 2: Replace inline lookups in `execute_sequential` (engine.rs:379)**

Change:

```rust
let quotes = plan.quote_sets.get(&step.quote_set).ok_or_else(|| { ... })?;
preflight_step(step, quotes, context, &plan.settings)?;
// ...
let outcome = step_runtime::execute(step, quotes, context, &plan.settings)?;
```

to:

```rust
let quotes = resolve_step_quotes(plan, market_data, step)?;
preflight_step(step, &quotes, context, &plan.settings)?;
// ...
let outcome = step_runtime::execute(step, &quotes, context, &plan.settings)?;
```

Thread `market_data: &[MarketDatum]` into `execute_sequential`'s signature.

- [ ] **Step 3: Update `ParallelBatchBuilder::get_quotes` (engine.rs:144)**

Mirror the change: take a `&'a [MarketDatum]` reference, return `Vec<MarketQuote>` (not `&'a Vec<MarketQuote>` — quotes are now resolved on demand). Adjust `StepBatchItem` to own a `Vec<MarketQuote>` rather than `&'a [MarketQuote]`.

- [ ] **Step 4: Rewrite the entry point at engine.rs:461–465**

Replace:

```rust
let mut context: MarketContext = match &envelope.initial_market {
    Some(state) => MarketContext::try_from(state.clone())
        .map_err(|e| ExecuteError::Other(finstack_core::Error::Validation(e.to_string())))?,
    None => MarketContext::new(),
};
```

with:

```rust
use crate::calibration::api::context_builder;
let mut context = context_builder::build_initial_context(
    &envelope.prior_market,
    &envelope.market_data,
    &envelope.plan.settings,
)
.map_err(|e| ExecuteError::Other(finstack_core::Error::Validation(e.to_string())))?;
```

And pass `&envelope.market_data` into `execute_sequential` / `execute_parallel`.

- [ ] **Step 5: Smoke-build (the crate should now compile)**

Run: `cargo check -p finstack-valuations 2>&1 | tail -20`
Expected: 0 errors. Warnings about unused imports are OK and will be cleaned up
when we update tests.

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/src/calibration/api/engine.rs
git commit -m "feat(calibration): engine consumes v3 market_data + prior_market"
```

---

### Task 9: Validator update — IDs and resolution

**Files:**
- Modify: `finstack/valuations/src/calibration/api/validate.rs`
- Modify: `finstack/valuations/src/calibration/api/errors.rs`

- [ ] **Step 1: Add two new `EnvelopeError` variants**

In `errors.rs`:

```rust
DuplicateMarketDatumId {
    kind: String,
    id: String,
},
QuoteIdNotInMarketData {
    quote_set: String,
    id: String,
},
```

With matching `Display` arms and ts_export adjustments.

- [ ] **Step 2: Rewrite `collect_initial_ids` to read `prior_market`**

Replace the function body in `validate.rs:126`:

```rust
fn collect_initial_ids(envelope: &CalibrationEnvelope) -> HashSet<String> {
    let mut ids = HashSet::new();
    for obj in &envelope.prior_market {
        ids.insert(obj.id().to_string());
    }
    ids
}
```

Update call sites (validate.rs:71, validate.rs:103) — they pass `envelope`
directly now (no `initial_market.as_ref()`).

- [ ] **Step 3: Add `check_market_data_uniqueness` and `check_quote_sets_resolve`**

Append to `validate.rs`:

```rust
fn check_market_data_uniqueness(
    envelope: &CalibrationEnvelope,
    errors: &mut Vec<EnvelopeError>,
) {
    // Quote kinds share one namespace; every other kind has its own.
    use std::collections::HashMap;
    let mut seen: HashMap<&str, BTreeSet<String>> = HashMap::new();
    for datum in &envelope.market_data {
        let key = if datum.is_quote() { "quote" } else { datum.kind_name() };
        if !seen.entry(key).or_default().insert(datum.id().to_string()) {
            errors.push(EnvelopeError::DuplicateMarketDatumId {
                kind: key.to_string(),
                id: datum.id().to_string(),
            });
        }
    }
}

fn check_quote_sets_resolve(
    envelope: &CalibrationEnvelope,
    errors: &mut Vec<EnvelopeError>,
) {
    use std::collections::HashSet;
    let quote_ids: HashSet<&str> = envelope
        .market_data
        .iter()
        .filter(|d| d.is_quote())
        .map(|d| d.id())
        .collect();
    for (set_name, ids) in &envelope.plan.quote_sets {
        for qid in ids {
            if !quote_ids.contains(qid.as_str()) {
                errors.push(EnvelopeError::QuoteIdNotInMarketData {
                    quote_set: set_name.clone(),
                    id: qid.to_string(),
                });
            }
        }
    }
}
```

Call both from `validate(envelope)` after `check_quote_sets(envelope, &mut errors)`.

- [ ] **Step 4: Add unit tests**

Append to `validate.rs`:

```rust
#[cfg(test)]
mod v3_validation_tests {
    use super::*;
    use crate::calibration::api::market_datum::{MarketDatum, PriceDatum};
    use finstack_core::market_data::scalars::MarketScalar;

    #[test]
    fn duplicate_price_id_is_flagged() {
        let envelope = CalibrationEnvelope {
            schema_url: None,
            schema: "finstack.calibration".into(),
            plan: CalibrationPlan {
                id: "t".into(),
                description: None,
                quote_sets: Default::default(),
                steps: vec![],
                settings: Default::default(),
            },
            market_data: vec![
                MarketDatum::Price(PriceDatum { id: "AAPL".into(), scalar: /* same constructor as task 1 */ }),
                MarketDatum::Price(PriceDatum { id: "AAPL".into(), scalar: /* same constructor */ }),
            ],
            prior_market: vec![],
        };
        let report = validate(&envelope);
        assert!(report.errors.iter().any(|e| matches!(e, EnvelopeError::DuplicateMarketDatumId { .. })));
    }

    #[test]
    fn unresolved_quote_id_is_flagged() {
        let envelope = CalibrationEnvelope {
            schema_url: None,
            schema: "finstack.calibration".into(),
            plan: CalibrationPlan {
                id: "t".into(),
                description: None,
                quote_sets: [("usd".to_string(), vec![QuoteId::new("MISSING")])].into_iter().collect(),
                steps: vec![],
                settings: Default::default(),
            },
            market_data: vec![],
            prior_market: vec![],
        };
        let report = validate(&envelope);
        assert!(report.errors.iter().any(|e| matches!(e, EnvelopeError::QuoteIdNotInMarketData { .. })));
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p finstack-valuations validate::v3_validation_tests`
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/src/calibration/api/validate.rs \
        finstack/valuations/src/calibration/api/errors.rs
git commit -m "feat(calibration): v3 validation — id uniqueness + quote_set resolution"
```

---

### Task 10: Fix the two `dv01.rs` envelope-construction call sites

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/fra/metrics/dv01.rs`
- Modify: `finstack/valuations/src/instruments/rates/cap_floor/metrics/dv01.rs`

- [ ] **Step 1: Convert `initial_market: Some(MarketContextState::from(&initial_market))` (fra/metrics/dv01.rs:159)**

Replace with:

```rust
let (prior_market, market_data) =
    finstack_core::market_data::context::MarketContextState::from(&initial_market).into();

let envelope = CalibrationEnvelope {
    schema_url: None,
    schema: CALIBRATION_SCHEMA.to_string(),
    plan,
    market_data,
    prior_market,
};
```

Apply the identical change in `cap_floor/metrics/dv01.rs:157`.

- [ ] **Step 2: Build**

Run: `cargo check -p finstack-valuations 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 3: Run the two affected metrics tests**

Run: `cargo test -p finstack-valuations dv01 --lib`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/instruments/rates/fra/metrics/dv01.rs \
        finstack/valuations/src/instruments/rates/cap_floor/metrics/dv01.rs
git commit -m "refactor(dv01): construct v3 CalibrationEnvelope inline"
```

---

## Phase 3 — Migrate test code

### Task 11: Update `tests/support/test_utils.rs`

**Files:**
- Modify: `finstack/valuations/tests/support/test_utils.rs`

- [ ] **Step 1: Find the helper at line 564**

Open `finstack/valuations/tests/support/test_utils.rs` and locate the function
that produces `initial_market: Some(MarketContextState::from(context))`. It's
typically named `envelope_from_context` or similar.

- [ ] **Step 2: Convert to emit v3**

Replace:

```rust
initial_market: Some(MarketContextState::from(context)),
```

with:

```rust
let state = MarketContextState::from(context);
let (prior_market, market_data) = state.into();
```

And the returned struct literal becomes:

```rust
CalibrationEnvelope {
    schema_url: None,
    schema: CALIBRATION_SCHEMA.to_string(),
    plan,
    market_data,
    prior_market,
}
```

- [ ] **Step 3: Run `cargo check --tests -p finstack-valuations`**

Expected: many errors in `tests/calibration/*.rs` remaining (those are Task 12) —
but `test_utils.rs` itself compiles.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/tests/support/test_utils.rs
git commit -m "test(calibration): emit v3 envelopes from test_utils builder"
```

---

### Task 12: Mechanical migration of `tests/calibration/*.rs`

**Files (13 in total):**
- Modify: `finstack/valuations/tests/calibration/diagnostics.rs`
- Modify: `finstack/valuations/tests/calibration/reference_envelopes.rs`
- Modify: `finstack/valuations/tests/calibration/v2_engine_smoke.rs`
- Modify: `finstack/valuations/tests/calibration/inflation.rs`
- Modify: `finstack/valuations/tests/calibration/hazard_curve.rs`
- Modify: `finstack/valuations/tests/calibration/failure_modes.rs`
- Modify: `finstack/valuations/tests/calibration/bootstrap.rs`
- Modify: `finstack/valuations/tests/calibration/base_correlation.rs`
- Modify: `finstack/valuations/tests/calibration/serialization.rs`
- Modify: `finstack/valuations/tests/calibration/swaption_vol.rs`
- Modify: `finstack/valuations/tests/calibration/explainability.rs`
- Modify: `finstack/valuations/tests/calibration/repricing.rs`
- Modify: `finstack/valuations/tests/calibration/builder.rs`
- Modify: `finstack/valuations/benches/global_calibration.rs`
- Modify: `finstack/valuations/benches/calibration.rs`

There are exactly **three** patterns to rewrite. Use `grep` to enumerate sites
before editing:

```bash
grep -n "initial_market: None" finstack/valuations/tests/calibration/*.rs
grep -n "initial_market: Some" finstack/valuations/tests/calibration/*.rs
grep -n "initial_market:"      finstack/valuations/benches/*.rs
```

- [ ] **Step 1: Rewrite `initial_market: None`**

Replace with two fields:
```rust
market_data: vec![],
prior_market: vec![],
```

`sed` script (verify on one file first):

```bash
gsed -i 's/initial_market: None,/market_data: vec![],\n        prior_market: vec![],/' \
    finstack/valuations/tests/calibration/diagnostics.rs
```

Do every file by hand or with the script, but visually confirm indentation matches.

- [ ] **Step 2: Rewrite `initial_market: Some((&initial_market).into())`**

This requires the conversion helper. Replace the field with:

```rust
market_data: {
    let (_p, d): (Vec<PriorMarketObject>, Vec<MarketDatum>) =
        finstack_core::market_data::context::MarketContextState::from(&initial_market).into();
    d
},
prior_market: {
    let (p, _d): (Vec<PriorMarketObject>, Vec<MarketDatum>) =
        finstack_core::market_data::context::MarketContextState::from(&initial_market).into();
    p
},
```

That double-conversion is wasteful but unblocks compilation. If you want to be
clean, hoist the split above the struct literal:

```rust
let (prior_market, market_data): (Vec<PriorMarketObject>, Vec<MarketDatum>) =
    finstack_core::market_data::context::MarketContextState::from(&initial_market).into();
let envelope = CalibrationEnvelope { /* ..., market_data, prior_market */ };
```

- [ ] **Step 3: Rewrite `initial_market: Some(MarketContextState::from(&MarketContext::new()))`**

This pattern (used in `builder.rs:51` and `v2_engine_smoke.rs:124`) means "no
prior data" — collapse to `market_data: vec![], prior_market: vec![]`.

- [ ] **Step 4: Per-test-file `quote_sets` field updates**

Builders that today write `quote_sets: HashMap::from([("usd", vec![quote1, quote2])])`
must change to:
- The quotes (`quote1`, `quote2`) become entries in `market_data` (wrapped in their `MarketDatum::*Quote` variants).
- The map value becomes `vec![QuoteId::new("...")]`.

This is the most invasive rewrite. Convert one file at a time and run its tests
before moving on.

Suggested order (smallest → largest):

1. `diagnostics.rs` (smallest, one test) — pilot
2. `serialization.rs`
3. `builder.rs`
4. `failure_modes.rs`
5. `bootstrap.rs`
6. `repricing.rs` (largest — 8 envelope sites)
7. The rest

After each file:
```bash
cargo test -p finstack-valuations --test "<crate-name-of-file>" 2>&1 | tail -10
```

Expected per file: green.

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p finstack-valuations 2>&1 | tail -20`
Expected: all green except `reference_envelopes.rs` (covered in Task 14).

- [ ] **Step 6: Commit (one commit per file is fine, or batch by phase)**

```bash
git add finstack/valuations/tests/calibration/diagnostics.rs
git commit -m "test(calibration): migrate diagnostics.rs to v3 envelope"
# ... repeat for each file
```

---

## Phase 4 — Fixture migration

### Task 13: Write the migration script

**Files:**
- Create: `tools/migrate_envelope_v2_to_v3.py`

- [ ] **Step 1: Write the script**

```python
# tools/migrate_envelope_v2_to_v3.py
"""One-shot migration: CalibrationEnvelope v2 → v3.

Reads each v2 JSON envelope on stdin (or argv path), emits v3 on stdout.
Idempotent on already-v3 envelopes. Deleted after fixtures land.

Usage:
    python3 tools/migrate_envelope_v2_to_v3.py path/to/v2.json > path/to/v3.json
    python3 tools/migrate_envelope_v2_to_v3.py path/to/v2.json --in-place
"""
import json
import sys
from pathlib import Path

# Curve type → kind mapping for prior_market (matches CurveState serde tags).
CURVE_KIND = {
    "discount":              "discount_curve",
    "forward":               "forward_curve",
    "hazard":                "hazard_curve",
    "inflation":             "inflation_curve",
    "base_correlation":      "base_correlation_curve",
    "basis_spread":          "basis_spread_curve",
    "parametric":            "parametric_curve",
    "price":                 "price_curve",
    "volatility_index":      "volatility_index_curve",
}

def migrate(env: dict) -> dict:
    # No-op if already v3.
    if "market_data" in env or "prior_market" in env:
        return env

    initial = env.pop("initial_market", None) or {}
    quote_sets_v2 = env.get("plan", {}).get("quote_sets", {})

    # 1. Build a flat quote bag from quote_sets and turn quote_sets into id lists.
    quote_data = []
    seen_quote_ids = set()
    quote_sets_v3 = {}
    for set_name, quotes in quote_sets_v2.items():
        id_list = []
        for q in quotes:
            qid = _quote_id(q)
            if qid not in seen_quote_ids:
                seen_quote_ids.add(qid)
                quote_data.append({"kind": _kind_for_quote(q), **{k: v for k, v in q.items() if k != "class"}})
            id_list.append(qid)
        quote_sets_v3[set_name] = id_list
    env["plan"]["quote_sets"] = quote_sets_v3

    # 2. Translate initial_market into market_data + prior_market.
    market_data = list(quote_data)
    prior_market = []

    for curve in initial.get("curves", []) or []:
        ctype = curve.pop("type")
        prior_market.append({"kind": CURVE_KIND[ctype], **curve})

    for surface in initial.get("surfaces", []) or []:
        prior_market.append({"kind": "vol_surface", **surface})

    fx = initial.get("fx")
    if fx:
        # Move FxConfig into plan.settings.fx; quotes become fx_spot entries.
        env["plan"].setdefault("settings", {})["fx"] = fx.get("config", {})
        for (from_ccy, to_ccy, rate) in fx.get("quotes", []):
            market_data.append({
                "kind": "fx_spot",
                "id":   f"{from_ccy}/{to_ccy}",
                "from": from_ccy,
                "to":   to_ccy,
                "rate": rate,
            })

    for pid, scalar in (initial.get("prices") or {}).items():
        market_data.append({"kind": "price", "id": pid, "scalar": scalar})

    for series in initial.get("series", []) or []:
        market_data.append({"kind": "fixing_series", **series})

    for idx in initial.get("inflation_indices", []) or []:
        market_data.append({"kind": "inflation_fixings", **idx})

    for div in initial.get("dividends", []) or []:
        market_data.append({"kind": "dividend_schedule", "schedule": div})

    for ci in initial.get("credit_indices", []) or []:
        market_data.append({"kind": "credit_index", **ci})

    for fxs in initial.get("fx_delta_vol_surfaces", []) or []:
        market_data.append({"kind": "fx_vol_surface", **fxs})

    for cube in initial.get("vol_cubes", []) or []:
        market_data.append({"kind": "vol_cube", **cube})

    for ccy, csa in (initial.get("collateral") or {}).items():
        market_data.append({"kind": "collateral", "id": ccy, "csa_currency": csa})

    hierarchy = initial.get("hierarchy")
    if hierarchy is not None:
        env["plan"].setdefault("settings", {})["hierarchy"] = hierarchy

    env["market_data"] = market_data
    env["prior_market"] = prior_market

    # Bump $schema path if it points at v2.
    if "$schema" in env and "calibration/2/" in env["$schema"]:
        env["$schema"] = env["$schema"].replace("calibration/2/", "calibration/3/")

    return env

def _quote_id(q: dict) -> str:
    # Every MarketQuote variant carries `id` at the top level today.
    return q["id"]

def _kind_for_quote(q: dict) -> str:
    cls = q.get("class")
    return {
        "rates": "rate_quote", "cds": "cds_quote", "cds_tranche": "cds_tranche_quote",
        "fx": "fx_quote", "inflation": "inflation_quote", "vol": "vol_quote",
        "xccy": "xccy_quote", "bond": "bond_quote",
    }[cls]

def main():
    args = sys.argv[1:]
    in_place = "--in-place" in args
    args = [a for a in args if a != "--in-place"]
    if not args:
        sys.exit("usage: migrate_envelope_v2_to_v3.py <path> [--in-place]")
    path = Path(args[0])
    env = json.loads(path.read_text())
    out = migrate(env)
    text = json.dumps(out, indent=2)
    if in_place:
        path.write_text(text + "\n")
    else:
        sys.stdout.write(text + "\n")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Dry-run against one example**

Run:
```bash
python3 tools/migrate_envelope_v2_to_v3.py finstack/valuations/examples/market_bootstrap/01_usd_discount.json | jq .
```
Expected: a v3 envelope with `market_data: [...]` and `quote_sets` as ID lists.
Eyeball-verify each quote `kind` matches `class` from the v2 source.

- [ ] **Step 3: Commit the script (do NOT yet apply it)**

```bash
git add tools/migrate_envelope_v2_to_v3.py
git commit -m "tools: one-shot v2→v3 envelope migration script"
```

---

### Task 14: Run migration on all reference envelopes

**Files:**
- Modify (in place, 12 files): `finstack/valuations/examples/market_bootstrap/*.json`

- [ ] **Step 1: Run script in-place on every example**

```bash
for f in finstack/valuations/examples/market_bootstrap/*.json; do
    python3 tools/migrate_envelope_v2_to_v3.py "$f" --in-place
done
```

- [ ] **Step 2: Visual diff one file**

```bash
git diff -- finstack/valuations/examples/market_bootstrap/09_fx_matrix.json
```
Expected: `initial_market` block gone; `market_data` populated with `fx_spot` entries; `plan.settings.fx` carries the FX config.

- [ ] **Step 3: Run the reference-envelope integration tests**

Run: `cargo test -p finstack-valuations --test calibration reference_envelopes`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/examples/market_bootstrap/
git commit -m "examples: migrate market-bootstrap envelopes to v3 schema"
```

---

### Task 15: Run migration on all golden fixtures

**Files:**
- Modify: 36 files under `finstack/valuations/tests/golden/data/pricing/`

- [ ] **Step 1: Migrate**

```bash
for f in $(grep -rl '"initial_market"' finstack/valuations/tests/golden/data/); do
    python3 tools/migrate_envelope_v2_to_v3.py "$f" --in-place
done
```

- [ ] **Step 2: Run the golden suite**

Run: `cargo test -p finstack-valuations --release --test golden 2>&1 | tail -20`
Expected: green. If any golden's recorded result hash differs from the v2 hash,
investigate — the engine output must be byte-identical because no calibration
math changed.

- [ ] **Step 3: Commit**

```bash
git add finstack/valuations/tests/golden/data/
git commit -m "test(golden): migrate pricing fixtures to v3 envelope"
```

---

## Phase 5 — Schema, bindings, docs

### Task 16: Generate `schemas/calibration/3/calibration.schema.json`

**Files:**
- Create: `finstack/valuations/schemas/calibration/3/calibration.schema.json`

- [ ] **Step 1: Find the existing schema-generation entry point**

```bash
grep -rn "calibration.schema.json\|schema_for!" finstack/valuations/ --include="*.rs" | head
```
There should be a `build.rs`, `xtask`, or `cargo run --example gen-schema`
target that emits the v2 schema. Find it.

- [ ] **Step 2: Point the generator at v3**

Modify the path the generator writes to:

```rust
// in the schema-gen binary
let out = "finstack/valuations/schemas/calibration/3/calibration.schema.json";
```

If there is no such generator (the v2 schema was hand-written), create one:

```rust
// finstack/valuations/examples/gen_schema.rs
fn main() {
    let schema = schemars::schema_for!(finstack_valuations::calibration::api::schema::CalibrationEnvelope);
    let json = serde_json::to_string_pretty(&schema).unwrap();
    std::fs::write("finstack/valuations/schemas/calibration/3/calibration.schema.json", json).unwrap();
}
```

Run: `mkdir -p finstack/valuations/schemas/calibration/3 && cargo run --example gen_schema -p finstack-valuations`

- [ ] **Step 3: Sanity-check the schema with `jq`**

```bash
jq '.properties | keys' finstack/valuations/schemas/calibration/3/calibration.schema.json
```
Expected output includes: `["$schema", "market_data", "plan", "prior_market", "schema"]`.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/schemas/calibration/3/ finstack/valuations/examples/gen_schema.rs
git commit -m "schema(calibration): generate v3 JSON schema"
```

---

### Task 17: Regenerate TS types and update WASM binding

**Files:**
- Modify: `finstack-wasm/src/api/valuations/calibration.rs`
- Modify (generated): `finstack-wasm/types/generated/CalibrationEnvelope.ts`
- Modify (generated): `finstack-wasm/index.d.ts`

- [ ] **Step 1: Regenerate TS types**

```bash
cargo test -p finstack-valuations --features ts_export -- --nocapture export_bindings 2>&1 | tail
```

(Or whatever the project's `ts-rs` invocation is — check the `ts_export`
feature's tests for the exact command.)

- [ ] **Step 2: Fix the WASM binding's `initial_market` references**

Open `finstack-wasm/src/api/valuations/calibration.rs` and:
- Replace any reference to `envelope.initial_market` with `envelope.market_data` /
  `envelope.prior_market` as appropriate.
- If there's a TS-facing JSON sample in a doc comment, update it.

- [ ] **Step 3: Build the WASM crate**

Run: `cargo check -p finstack-wasm`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add finstack-wasm/
git commit -m "wasm: regenerate v3 envelope bindings"
```

---

### Task 18: Update `examples/market_bootstrap/README.md`

**Files:**
- Modify: `finstack/valuations/examples/market_bootstrap/README.md`

- [ ] **Step 1: Edit the catalog table's Track-B column language**

In the README's catalog table, replace "via `initial_market`" with the v3
phrasing:

| Old wording | New wording |
|---|---|
| ``via `initial_market.fx` `` | ``as `fx_spot` entries in `market_data` `` |
| ``via `initial_market.prices` `` | ``as `price` entries in `market_data` `` |
| ``in `initial_market` `` | ``in `market_data` `` |

Update the bottom-of-page worked example to read:

```rust
let envelope_json = std::fs::read_to_string("01_usd_discount.json")?;
let envelope: CalibrationEnvelope = serde_json::from_str(&envelope_json)?;
let result = engine::execute(&envelope)?;
let market = MarketContext::try_from(result.result.final_market)?;
```

(Identical — the consumer-side API hasn't changed.)

- [ ] **Step 2: Commit**

```bash
git add finstack/valuations/examples/market_bootstrap/README.md
git commit -m "docs(examples): describe Track B in v3 envelope terms"
```

---

### Task 19: Crate-level doc fixes

**Files:**
- Modify: `finstack/valuations/src/lib.rs`
- Modify: `finstack/valuations/src/calibration/mod.rs`
- Modify: `finstack/valuations/src/calibration/api/errors.rs`

- [ ] **Step 1: Update doc-example envelopes**

In `lib.rs:173` the snippet contains `"initial_market":null`. Change to omit
the field (v3 default-empty):

```rust
//! let envelope_json = r#"{"schema":"finstack.calibration","plan":{"id":"empty","description":null,"quote_sets":{},"steps":[],"settings":{}}}"#;
```

Apply the same edit at `calibration/mod.rs:12`.

- [ ] **Step 2: Update prose**

Replace `initial_market` with `market_data` / `prior_market` (per context) in:
- `calibration/mod.rs` lines 28, 41, 45, 47
- `calibration/api/errors.rs:37`

- [ ] **Step 3: Run doc-tests**

Run: `cargo test --doc -p finstack-valuations 2>&1 | tail`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/lib.rs \
        finstack/valuations/src/calibration/mod.rs \
        finstack/valuations/src/calibration/api/errors.rs
git commit -m "docs(calibration): update prose to v3 envelope vocabulary"
```

---

### Task 20: Pointer/superseded notes on prior design docs

**Files:**
- Modify (one-line addition each): `docs/2026-05-08-market-bootstrap-phase-{1,2,4,5}-*.md`,
  `docs/2026-05-09-golden-fixture-envelope-migration-plan.md`

- [ ] **Step 1: Add a top-of-file note**

At the very top of each historical document (after the H1 title), insert:

```markdown
> **Superseded** in v3 envelope shape: see [2026-05-10-calibration-envelope-cleanup-design.md](2026-05-10-calibration-envelope-cleanup-design.md). References to `initial_market` in this document predate the v3 cleanup.
```

(Do not rewrite the body — they're historical artifacts.)

- [ ] **Step 2: Commit**

```bash
git add docs/
git commit -m "docs: mark v2-era envelope design docs as superseded by v3"
```

---

### Task 21: Friendly error on v2 envelopes

**Files:**
- Modify: `finstack/valuations/src/calibration/api/schema.rs`

- [ ] **Step 1: Add custom `Deserialize` shim**

Wrap the derived `Deserialize` so that an `"initial_market"` key triggers a
clear message. Easiest path: a wrapper helper that pre-parses to
`serde_json::Value` and checks for the legacy field.

```rust
// in calibration/api/engine.rs or a new schema_helpers.rs
pub fn parse_envelope_v3(json: &str) -> std::result::Result<CalibrationEnvelope, EnvelopeError> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| EnvelopeError::JsonParse {
        message: e.to_string(),
        line: Some(e.line() as u32),
        col: Some(e.column() as u32),
    })?;
    if value.get("initial_market").is_some() {
        return Err(EnvelopeError::JsonParse {
            message: "envelope schema v2 is no longer supported; see docs/2026-05-10-calibration-envelope-cleanup-design.md for the v3 shape".to_string(),
            line: None,
            col: None,
        });
    }
    serde_json::from_value(value).map_err(|e| EnvelopeError::JsonParse {
        message: e.to_string(),
        line: None,
        col: None,
    })
}
```

Update `validate::parse_envelope` (validate.rs:118) and any other JSON-string
entry points to call `parse_envelope_v3`.

- [ ] **Step 2: Add a test**

```rust
#[test]
fn v2_envelope_yields_friendly_error() {
    let v2 = r#"{"schema":"finstack.calibration","plan":{"id":"x","description":null,"quote_sets":{},"steps":[],"settings":{}},"initial_market":null}"#;
    let err = parse_envelope_v3(v2).unwrap_err();
    assert!(matches!(err, EnvelopeError::JsonParse { ref message, .. } if message.contains("v3 shape")));
}
```

- [ ] **Step 3: Run**

Run: `cargo test -p finstack-valuations v2_envelope_yields_friendly_error`

- [ ] **Step 4: Commit**

```bash
git add finstack/valuations/src/calibration/api/
git commit -m "feat(calibration): clear error message for legacy v2 envelopes"
```

---

## Phase 6 — Cleanup

### Task 22: Full test sweep + lint

- [ ] **Step 1: Build everything**

Run: `cargo build --workspace --all-features 2>&1 | tail -10`
Expected: clean.

- [ ] **Step 2: Test everything**

Run: `cargo test --workspace --all-features 2>&1 | tail -20`
Expected: all green.

- [ ] **Step 3: Clippy + fmt**

Run: `make lint-rust` (or whatever the workspace's clippy/fmt invocation is — `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`).
Expected: clean.

- [ ] **Step 4: Doc tests**

Run: `cargo test --doc --workspace 2>&1 | tail`
Expected: clean.

- [ ] **Step 5: Commit any incidental fixes**

```bash
git add -p
git commit -m "chore(calibration): fix lint and doctest after v3 migration"
```

(If no changes, skip the commit.)

---

### Task 23: Delete the migration script

**Files:**
- Delete: `tools/migrate_envelope_v2_to_v3.py`

- [ ] **Step 1: Verify there are no remaining v2 envelopes**

Run: `grep -rln '"initial_market"' finstack/ docs/`
Expected: only the design-doc and superseded-note hits — no JSON fixtures.

- [ ] **Step 2: Delete the script**

```bash
git rm tools/migrate_envelope_v2_to_v3.py
```

If the `tools/` directory is now empty, delete it as well.

- [ ] **Step 3: Commit**

```bash
git commit -m "chore(tools): remove one-shot v2→v3 migration script"
```

---

### Task 24: Final smoke and pre-PR check

- [ ] **Step 1: Re-run the full suite one more time**

Run: `cargo test --workspace --all-features --release 2>&1 | tail -10`

- [ ] **Step 2: Confirm v2 schema directory is still in place (one-release grace)**

Run: `ls finstack/valuations/schemas/calibration/`
Expected: `2  3` — both directories present. The v2 directory will be removed
in a follow-up release per the design's "one release" grace window.

- [ ] **Step 3: Diff summary**

Run: `git log master.. --oneline`
Eyeball the commit list against the task list — every task should map to one or
more commits. Any missing? Go back and address before opening the PR.

- [ ] **Step 4: Open the PR**

```bash
gh pr create --title "feat(calibration): v3 envelope — replace initial_market" \
    --body "$(cat <<'EOF'
## Summary
- Replace `CalibrationEnvelope.initial_market` (`Option<MarketContextState>`) with two explicit sections: `market_data: Vec<MarketDatum>` and `prior_market: Vec<PriorMarketObject>`.
- `plan.quote_sets` now holds lists of `QuoteId`, indexing into the flat `market_data` quote pool.
- FX matrix config and hierarchy metadata move into `plan.settings`.
- Output (`CalibrationResult.final_market`) is unchanged.

See [docs/2026-05-10-calibration-envelope-cleanup-design.md](docs/2026-05-10-calibration-envelope-cleanup-design.md).

## Test plan
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo test --doc --workspace`
- [ ] All 12 reference envelopes parse and calibrate
- [ ] All 36 golden fixtures produce byte-identical results

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Self-review checklist

The plan author runs this before handing the plan to an executor.

1. **Spec coverage:**
   - [x] §"Envelope shape (v3)" → Tasks 1–6.
   - [x] §"Rust types" → Tasks 1–5.
   - [x] §"Engine integration" → Tasks 7–8, 10–11.
   - [x] §"Validation" → Task 9.
   - [x] §"Migration → File-system" → Task 16, Task 23.
   - [x] §"Migration → Code" → Tasks 5–10, 17.
   - [x] §"Migration → Test/fixture" → Tasks 11–15.
   - [x] §"Migration → Docs" → Tasks 18–20.
   - [x] §"Helper API" (`From<MarketContextState>`) → Task 6.
   - [x] §"Failure mode for v2 envelopes" → Task 21.

2. **Placeholder scan:**
   - Task 7's `apply_credit_index` body says `todo!("port credit-index reconstruction from state_serde.rs:447..489 — keep semantics identical")`. This is an explicit task-internal step ("Step 2: Port the credit-index logic, removing the `todo!`") and the source lines are named — acceptable, but reviewers should treat the function body as required work, not a placeholder.
   - Task 1 & 9 reference `MarketScalar::new("AAPL", 175.42, Currency::USD)` with a "substitute the real constructor" note. This is intentional — the test author is expected to consult the real type before committing. Not a placeholder hole.
   - No "implement later" / "TBD" entries elsewhere.

3. **Type consistency:**
   - `MarketDatum`, `PriorMarketObject`, `CalibrationEnvelope.market_data`, `CalibrationEnvelope.prior_market`, `CalibrationPlan.quote_sets: HashMap<String, Vec<QuoteId>>` — all consistent across Tasks 1–10.
   - `resolve_step_quotes` returns `std::result::Result<Vec<MarketQuote>, ExecuteError>` (Task 8). Consumers in `execute_sequential` adapt with `&quotes` — matches `preflight_step` signature in current code.
   - `From<MarketContextState>` produces `(Vec<PriorMarketObject>, Vec<MarketDatum>)` — same tuple order used in Tasks 10–12 destructure sites.
