# Calibration Envelope Cleanup — v3 Schema

**Status:** Draft
**Date:** 2026-05-10
**Author:** jeickmeier

## Motivation

The current `CalibrationEnvelope` carries inputs in a field named `initial_market`,
typed as `MarketContextState`. The name conflates two distinct concerns:

1. **Snapshot data inputs** that the calibrator passes through but does not produce
   (FX rates, bond/equity prices, dividend schedules, fixings, inflation history,
   credit-index aggregates, FX-vol surfaces, vol cubes, CSA mappings).
2. **Pre-built calibrated objects** from a prior run (curves, surfaces) layered in
   to chain a new calibration on top.

The `initial_market` name suggests a temporal baseline; in practice it is a bag of
heterogeneous inputs, and authoring the envelope requires writing `"version": 2`
boilerplate plus a dozen empty arrays for unused categories. The type
`MarketContextState` is also the output type, which makes input ergonomics suffer
to keep round-tripping cheap.

End-users learning the library should be able to read an envelope and
immediately see what data is being supplied, what is being calibrated, and what
came from elsewhere. The current shape obscures that.

## Goals

- Replace `initial_market` with named, intent-revealing sections.
- Make every market datum addressable by `(kind, id)`, with one consistent shape
  across quotes, prices, dividends, fixings, indices, surfaces, and cubes.
- Allow a single quote to participate in multiple `quote_sets` without
  duplication.
- Keep `CalibrationResult.final_market` as a single merged `MarketContextState`
  so downstream pricing code is unchanged.
- Hard cut to v3 — no back-compat shim. Library is early enough that a clean
  break is worth more than two parallel deserialization paths.

## Non-goals

- Splitting the output (`CalibrationResult.final_market`) into mirrored sections.
- Adding new calibration capabilities. The change is API/schema-only; behavior
  of every existing step kind is preserved.
- Reworking `MarketContext` or `MarketContextState` themselves. Both keep their
  shape; only the envelope-input contract changes.

## Envelope shape (v3)

```jsonc
{
  "$schema": "../../schemas/calibration/3/calibration.schema.json",
  "schema":  "finstack.calibration",

  "plan": {
    "id":          "usd_full",
    "description": "USD discount + 3M forward",
    "steps": [
      { "id": "USD-OIS",     "kind": "discount", "quote_set": "usd_ois",     /* ... */ },
      { "id": "USD-SOFR-3M", "kind": "forward",  "quote_set": "usd_sofr_3m", /* ... */ }
    ],
    "quote_sets": {
      "usd_ois":     ["USD-SOFR-DEP-1M", "USD-OIS-SWAP-1Y", "USD-OIS-SWAP-5Y"],
      "usd_sofr_3m": ["USD-SOFR-DEP-1M", "USD-SOFR-3M-FRA-1Y"]
    },
    "settings": {
      "fx": { "pivot_currency": "USD", "enable_triangulation": true, "cache_capacity": 256 }
    }
  },

  "market_data": [
    { "kind": "rate_quote",        "id": "USD-SOFR-DEP-1M", "type": "deposit", "rate": 0.0525 /* ... */ },
    { "kind": "rate_quote",        "id": "USD-OIS-SWAP-1Y", "type": "swap",    "rate": 0.0510 /* ... */ },
    { "kind": "fx_quote",          "id": "EUR/USD",         "from": "EUR", "to": "USD", "rate": 1.085 },
    { "kind": "price",             "id": "AAPL",            "value": 175.42, "currency": "USD" },
    { "kind": "dividend_schedule", "id": "AAPL",            "events": [/* ... */] },
    { "kind": "fixing_series",     "id": "USD-SOFR",        "observations": [/* ... */] },
    { "kind": "inflation_fixings", "id": "USA-CPI-U",       "observations": [/* ... */] },
    { "kind": "credit_index",      "id": "CDX-IG-46",       /* ... */ },
    { "kind": "fx_vol_surface",    "id": "EURUSD-1M",       /* ... */ },
    { "kind": "vol_cube",          "id": "USD-SWPTN-CUBE",  /* ... */ },
    { "kind": "collateral",        "id": "USD",             "csa_currency": "USD" }
  ],

  "prior_market": [
    { "kind": "discount_curve", "id": "USD-OIS",  /* ... */ },
    { "kind": "vol_surface",    "id": "AAPL-SABR", /* ... */ }
  ]
}
```

### Mental model

| Section        | Role                                                                |
|----------------|---------------------------------------------------------------------|
| `plan`         | Execution recipe — steps to run, in what order, and which named quote IDs each step consumes. |
| `market_data`  | Flat, id-addressable list of all data the calibrator can read.       |
| `prior_market` | Flat, id-addressable list of pre-built calibrated objects to chain on. |

`plan.quote_sets` are *named ID lists*, not quote bundles. A given quote
ID can appear in any number of sets without being duplicated in the data.

### Section omissions

Every section is optional and every collection is empty-by-default. A
Track-B envelope supplying only FX rates reads:

```jsonc
{
  "schema": "finstack.calibration",
  "plan":   { "id": "fx_only", "steps": [], "quote_sets": {} },
  "market_data": [
    { "kind": "fx_quote", "id": "EUR/USD", "from": "EUR", "to": "USD", "rate": 1.085 }
  ]
}
```

No `version`, no empty arrays, no `prior_market`.

## Rust types

```rust
// finstack/valuations/src/calibration/api/schema.rs

pub struct CalibrationEnvelope {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema_url: Option<String>,
    pub schema:     String,
    pub plan:       CalibrationPlan,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub market_data: Vec<MarketDatum>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prior_market: Vec<PriorMarketObject>,
}

pub struct CalibrationPlan {
    pub id:          String,
    #[serde(default)]
    pub description: Option<String>,
    pub steps:       Vec<CalibrationStep>,
    #[serde(default)]
    pub quote_sets:  HashMap<String, Vec<QuoteId>>,
    #[serde(default)]
    pub settings:    CalibrationConfig,
}

#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketDatum {
    // Quotes (currently the MarketQuote variants, flattened in)
    RateQuote(RateQuote),
    CdsQuote(CdsQuote),
    CdsTrancheQuote(CDSTrancheQuote),
    FxQuote(FxQuote),
    InflationQuote(InflationQuote),
    VolQuote(VolQuote),
    XccyQuote(XccyQuote),
    BondQuote(BondQuote),

    // Snapshot-only inputs
    Price(PriceDatum),
    DividendSchedule(DividendSchedule),
    FixingSeries(ScalarTimeSeries),
    InflationFixings(InflationIndex),
    CreditIndex(CreditIndexState),
    FxVolSurface(FxDeltaVolSurface),
    VolCube(VolCube),
    Collateral(CollateralEntry),       // { id: currency, csa_currency: String }
}

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
```

`MarketDatum` exposes accessors:

```rust
impl MarketDatum {
    pub fn id(&self) -> &str { /* match arm per variant */ }
    pub fn as_quote(&self) -> Option<MarketQuote> { /* Some for the 8 quote variants */ }
}
```

### Reallocated fields

| Today (`MarketContextState`)            | v3 destination                                |
|-----------------------------------------|-----------------------------------------------|
| `version: u32`                          | Dropped — schema version is envelope-level    |
| `curves: Vec<CurveState>`               | `prior_market` entries                        |
| `surfaces: Vec<VolSurface>`             | `prior_market` entries                        |
| `fx.config` (`FxMatrixState`)           | `plan.settings.fx`                            |
| `fx.quotes` (Vec of pair tuples)        | `market_data` entries `kind: "fx_quote"`      |
| `prices`                                | `market_data` entries `kind: "price"`         |
| `series`                                | `market_data` entries `kind: "fixing_series"` |
| `inflation_indices`                     | `market_data` entries `kind: "inflation_fixings"` |
| `dividends`                             | `market_data` entries `kind: "dividend_schedule"` |
| `credit_indices`                        | `market_data` entries `kind: "credit_index"` |
| `fx_delta_vol_surfaces`                 | `market_data` entries `kind: "fx_vol_surface"` (renamed) |
| `vol_cubes`                             | `market_data` entries `kind: "vol_cube"`      |
| `collateral` (map ccy→csa-ccy)          | `market_data` entries `kind: "collateral"`    |
| `hierarchy`                             | `plan.settings.hierarchy`                     |

### Renames

| Old name                  | New name             |
|---------------------------|----------------------|
| `series`                  | `fixing_series`      |
| `inflation_indices`       | `inflation_fixings`  |
| `fx_delta_vol_surfaces`   | `fx_vol_surface`     |

## Engine integration

`engine::execute` builds the initial `MarketContext` from the two flat lists,
then runs steps unchanged.

```rust
fn build_initial_context(env: &CalibrationEnvelope) -> Result<MarketContext> {
    let mut ctx = MarketContext::new();
    for obj in &env.prior_market { ctx.insert_prior(obj.clone())?; }

    let bk = MarketDataBuckets::partition(&env.market_data);
    if !bk.fx_quotes.is_empty() {
        ctx.set_fx_matrix(FxMatrix::from_quotes(&bk.fx_quotes, &env.plan.settings.fx)?);
    }
    ctx.extend_prices(bk.prices);
    ctx.extend_dividends(bk.dividends);
    ctx.extend_fixings(bk.fixings);
    ctx.extend_inflation_fixings(bk.inflation_fixings);
    ctx.extend_credit_indices(bk.credit_indices);
    ctx.extend_fx_vol_surfaces(bk.fx_vol_surfaces);
    ctx.extend_vol_cubes(bk.vol_cubes);
    ctx.set_collateral(bk.collateral.into_iter().collect());
    Ok(ctx)
}
```

Step quote resolution:

```rust
fn resolve_step_quotes(
    plan: &CalibrationPlan,
    market_data: &[MarketDatum],
    step: &CalibrationStep,
) -> Result<Vec<MarketQuote>> {
    let ids = plan.quote_sets.get(&step.quote_set)
        .ok_or_else(|| Error::UnknownQuoteSet(step.quote_set.clone()))?;
    let by_id: HashMap<&QuoteId, MarketQuote> = market_data.iter()
        .filter_map(|d| d.as_quote().map(|q| (q.id(), q)))
        .collect();
    ids.iter()
        .map(|id| by_id.get(id).cloned().ok_or_else(|| Error::UnknownQuoteId(id.clone())))
        .collect()
}
```

`CalibrationResult.final_market` keeps its current `MarketContextState`
type and produces a single merged snapshot ready to be loaded back into a
`MarketContext` by downstream pricers.

## Validation

In `calibration/api/validate.rs`:

1. **ID uniqueness.** Uniqueness is **per-kind** for non-quote kinds — e.g.
   `AAPL` as both a `price` and a `dividend_schedule` is allowed. The eight
   quote kinds (`rate_quote`, `cds_quote`, `cds_tranche_quote`, `fx_quote`,
   `inflation_quote`, `vol_quote`, `xccy_quote`, `bond_quote`) **share one ID
   namespace**, because `plan.quote_sets` references quotes by ID without
   carrying a kind discriminator. A `rate_quote` and a `cds_quote` with the
   same ID is an error.
2. **Quote-set resolution.** Every ID in `plan.quote_sets[name]` must
   resolve to exactly one `market_data` entry whose `kind` is one of the
   eight `*_quote` kinds.
3. **Step quote-set reference.** Every `step.quote_set` must name an entry in
   `plan.quote_sets`.
4. **Dependency satisfaction.** Every curve/surface ID a step depends on must
   either appear in `prior_market` or be produced by an earlier step in plan
   order. (Existing logic; rewritten against the new list shape.)

## Migration

Hard cut to v3 in one commit-series. No deserializer-level v2 fallback.

### File-system changes

- Add `schemas/calibration/3/calibration.schema.json` (auto-generated via
  `schemars`).
- Keep `schemas/calibration/2/` on disk for one release as historical
  reference; the code emits and accepts only v3.

### Code changes

| File                                                                | Change                                                                  |
|---------------------------------------------------------------------|-------------------------------------------------------------------------|
| `finstack/valuations/src/calibration/api/schema.rs`                 | Replace `CalibrationEnvelope`, `CalibrationPlan`; add `MarketDatum`, `PriorMarketObject`. |
| `finstack/valuations/src/calibration/api/engine.rs`                 | `build_initial_context`, `resolve_step_quotes`; rewrite `execute_with_diagnostics` entry. |
| `finstack/valuations/src/calibration/api/validate.rs`               | New uniqueness/resolution checks; existing dependency check adapted to consult `prior_market`. |
| `finstack/valuations/src/lib.rs`, `calibration/mod.rs`              | Replace `initial_market` references in module docs.                     |
| `finstack/valuations/src/instruments/rates/fra/metrics/dv01.rs`     | Call site constructing envelopes — switch to `prior_market`/`market_data`. |
| `finstack/valuations/src/instruments/rates/cap_floor/metrics/dv01.rs` | Same.                                                                 |
| `finstack-wasm/src/api/valuations/calibration.rs`                   | TS-export updates and any direct schema references.                     |

### Test/fixture changes

- Rewrite all 12 `finstack/valuations/examples/market_bootstrap/*.json` files.
- Migrate every `tests/calibration/*.rs` builder currently producing
  `initial_market: Some(...)` to the new flat-list shape (~35 sites per the
  earlier audit).
- `tests/support/test_utils.rs:564` — convert the helper from emitting
  `MarketContextState` into emitting `(Vec<PriorMarketObject>, Vec<MarketDatum>)`.
- Touch ~30+ golden envelopes under `tests/golden/data/pricing/**/*.json`.

### Docs changes

- `examples/market_bootstrap/README.md` — restate Track A/B against the v3
  shape; update the worked example.
- `docs/2026-05-08-market-bootstrap-phase-*-design.md`,
  `docs/2026-05-09-golden-fixture-envelope-migration-plan.md` — add a
  superseded-by pointer to this design rather than rewriting historical text.
- `finstack/valuations/src/lib.rs` and `calibration/mod.rs` rustdoc.

### Helper API

For users holding a `MarketContextState` from a previous calibration who want
to feed v3 directly:

```rust
impl From<MarketContextState> for (Vec<PriorMarketObject>, Vec<MarketDatum>) {
    fn from(state: MarketContextState) -> Self { /* ... */ }
}
```

So:

```rust
let (prior, data) = previous_result.final_market.into();
let envelope = CalibrationEnvelope { plan, market_data: data, prior_market: prior, .. };
```

### Failure mode for v2 envelopes

Deserializing a v2 envelope (which has `"initial_market"`) fails at the
serde layer with `unknown field 'initial_market'`. The
`CalibrationEnvelope::deserialize` impl wraps this with a custom error that
prints a one-line pointer:

> envelope schema v2 is no longer supported; see
> `docs/2026-05-10-calibration-envelope-cleanup-design.md`
> for the v3 shape

## Open questions

None. Decisions captured:

- ID uniqueness is **per-kind**, except the eight quote kinds share one ID
  namespace so `plan.quote_sets` lookups are unambiguous.
- `FxMatrix.config` and `MarketDataHierarchy` move into `plan.settings`.
- `CalibrationResult.final_market` stays a single merged `MarketContextState`.
- Migration is a hard cut; v2 envelopes hard-error.

## Risk

The largest blast-radius change is the golden envelope rewrite (~30+ files).
These are checked-in fixtures the test suite reads verbatim. A scripted
migration (Python or `jq`) that walks each file and rewrites
`initial_market.{field}` into the matching v3 lists, run once and committed,
is the safest path. The implementation plan should sequence:

1. Land the new types and engine wiring with v2 fixtures temporarily routed
   through an in-test compatibility adapter.
2. Run the scripted migration on all checked-in JSON.
3. Remove the in-test adapter and the v2 schema directory references.
