# Golden Tests Framework — Design Spec

**Status:** Draft
**Date:** 2026-04-30
**Owner:** finstack/valuations + finstack/analytics + finstack-py + scripts/goldens
**Schema:** `finstack.golden/1`

## 1. Motivation

The library prices instruments, calibrates curves and surfaces, computes performance analytics, and decomposes returns via attribution. None of this is currently locked in against industry-benchmark reference values. Existing `tests/quantlib_parity/` files mostly assert internal consistency (par-rate self-consistency, pay/receive symmetry) rather than agreement with QuantLib or Bloomberg outputs. `INVARIANTS.md` §6 already declares the convention ("Golden-value tests vs QuantLib, ISDA SIMM, GIPS, etc. live in `<crate>/tests/` with explicit provenance in the test header") — this spec gives that convention a framework, a fixture format, and a v1 coverage list.

Golden tests serve two purposes:

1. **Regression detection.** A model change that drifts pricing 5bp from QuantLib should fail CI before merge, not surface as a client question three weeks later.
2. **Integration validation.** A curve calibration that bootstraps cleanly in isolation but produces wrong forwards when paired with a swap pricer is exactly the kind of multi-component bug that single-unit tests miss. The "calibrate-then-price" cross-crate fixtures catch this.

## 2. Goals

- Single source of truth for reference data: one JSON fixture per scenario, consumed by both Rust and Python test layers.
- Fixtures are committed, versioned, and reviewable in PR diffs — no live Bloomberg / QuantLib calls at test time.
- Each fixture carries enough provenance (source, as-of date, captured-by, last-reviewed-by, last-reviewed-on, screenshots) to be auditable years later.
- Tolerances are declared per-fixture, with workspace-default tables published in this spec.
- Adding a new fixture is a 3-file change: one JSON, one Rust test wrapper (5 lines), one Python test wrapper (5 lines).
- Stale fixtures surface in a periodic review report; drift between frozen reference and today's source is observable on demand.

## 3. Non-Goals

- Do not run QuantLib-Python or BLPAPI at `cargo test` / `pytest` time — both layers must run offline against committed JSON.
- Do not gate CI on the drift report — drift is a diagnostic for humans, not a build break.
- Do not cover stress/scenario, ISDA SIMM, SA-CCR, MC determinism, exotics, MBS, CMS, or commodity in v1 — those are listed in §13 with their deferral rationale.
- Do not introduce a new fixture format that competes with the existing `tests/golden/credit_factor_model_v1.json`. The new format is a strict superset.

## 4. Architecture

Three layers, one source of truth:

```
┌─────────────────────────────────────────────────────────────┐
│  Reference data (committed JSON fixtures)                   │
│  finstack/<crate>/tests/golden/data/<domain>/<scenario>.json│
│  Each = inputs + expected_outputs + tolerances + provenance │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┴───────────────┐
        ▼                              ▼
┌────────────────────┐         ┌──────────────────────────┐
│ Rust runner        │         │ Python runner            │
│ run_golden! macro  │         │ run_golden() helper      │
│ in tests/golden/   │         │ in finstack-py/tests/    │
│ Calls canonical    │         │ Calls bindings, asserts  │
│ Rust API, asserts. │         │ same fixture, same tol.  │
└────────────────────┘         └──────────────────────────┘
        ▲                              ▲
        └──────────────┬───────────────┘
                       │
┌──────────────────────┴──────────────────────────────────────┐
│  Generator + drift + review tooling                         │
│  scripts/goldens/                                           │
│   regen.py            — capture from QL-Python / BLPAPI /   │
│                         manual screen entry                 │
│   drift_report.py     — compare today's pull vs frozen ref  │
│   stale_report.py     — flag fixtures past review window    │
└─────────────────────────────────────────────────────────────┘
```

Rust is the canonical API. Python tests exercise the PyO3 bindings using the same JSON fixtures, catching binding/serialization drift that Rust-only tests would miss.

## 5. Fixture Schema

Schema version: `finstack.golden/1`. Stored as JSON. Fields are normative — the test runner rejects fixtures that omit required fields.

```json
{
  "schema_version": "finstack.golden/1",
  "name": "irs_sofr_5y_par",
  "domain": "rates.irs",
  "description": "5Y USD SOFR par swap, flat curve at 4%",
  "provenance": {
    "as_of": "2026-04-30",
    "source": "quantlib",
    "source_detail": "QuantLib-Python 1.34, VanillaSwap, OISCurve from FlatForward",
    "captured_by": "jeickmeier",
    "captured_on": "2026-04-30",
    "last_reviewed_by": "jeickmeier",
    "last_reviewed_on": "2026-04-30",
    "review_interval_months": 6,
    "regen_command": "uv run scripts/goldens/regen.py --kind irs-par --tenor 5Y --rate 0.04",
    "screenshots": []
  },
  "inputs": {
    "as_of_date": "2026-04-30",
    "curves": [
      {"id": "USD-OIS", "kind": "discount", "knots": [[0.0, 1.0], [5.0, 0.8187307]]}
    ],
    "instrument": {
      "kind": "irs",
      "notional": 10000000.0,
      "currency": "USD",
      "fixed_rate": 0.04,
      "tenor": "5Y",
      "side": "pay_fixed"
    }
  },
  "expected_outputs": {
    "npv": -2.41,
    "par_rate": 0.040,
    "dv01": 4523.18
  },
  "tolerances": {
    "npv": {"abs": 0.01, "rel": 1e-6},
    "par_rate": {"abs": 1e-7},
    "dv01": {"abs": 0.5, "rel": 1e-4}
  }
}
```

### 5.1 Required fields

| Field | Type | Notes |
|---|---|---|
| `schema_version` | string | Must equal `"finstack.golden/1"`. Enforced by runner walk-test. |
| `name` | string | Snake-case, unique within domain. |
| `domain` | string | Dotted path: `rates.irs`, `analytics.performance`, `attribution.fi_risk_based`. Used by the drift/stale reports for filtering. |
| `description` | string | One sentence. |
| `provenance.as_of` | date | YYYY-MM-DD. The market date the reference values represent. |
| `provenance.source` | enum | One of `quantlib`, `bloomberg-api`, `bloomberg-screen`, `intex`, `formula`, `textbook`. |
| `provenance.source_detail` | string | Free-form: QL version + engine, Bloomberg screen + ticker, textbook page, etc. |
| `provenance.captured_by` | string | Username at capture time. |
| `provenance.captured_on` | date | When the fixture was first written. |
| `provenance.last_reviewed_by` | string | Username at last review. |
| `provenance.last_reviewed_on` | date | When the fixture was last reviewed. Equals `captured_on` initially. |
| `provenance.review_interval_months` | int | Default 6. Drives the stale-report threshold. |
| `provenance.regen_command` | string | The exact command to regenerate this fixture. Empty string allowed for `formula` / `textbook` sources where the values are derived by hand. |
| `provenance.screenshots` | array | See §5.2. Empty array allowed for `quantlib` / `bloomberg-api` / `formula`. Required (≥1) for `bloomberg-screen` and `intex`. |
| `inputs` | object | Domain-specific. The runner's adapter for this `domain` knows how to construct a canonical-API input from this object. |
| `expected_outputs` | object | Map of metric name → reference value. All keys must have a tolerance entry. |
| `tolerances` | object | Map of metric name → `{abs?: number, rel?: number}`. Must cover every key in `expected_outputs`. |

### 5.2 Screenshots

Stored next to the fixture JSON, in a `screenshots/` subdirectory:

```
finstack/valuations/tests/golden/data/pricing/irs/
├── usd_sofr_5y_par.json
└── screenshots/
    └── usd_sofr_5y_par__swpm_2026-04-30.png
```

Each entry in `provenance.screenshots`:

```json
{
  "path": "screenshots/usd_sofr_5y_par__swpm_2026-04-30.png",
  "screen": "SWPM",
  "captured_on": "2026-04-30",
  "description": "Pricing screen with default USD-OIS curve and 5Y par swap"
}
```

Path is relative to the fixture JSON. Multiple screenshots allowed (e.g. one for input curve, one for pricing tab). Image format PNG or JPG. The walk-test asserts every referenced file exists on disk and is tracked by `git ls-files`.

### 5.3 Tolerance semantics

A metric matches if **either** the absolute tolerance OR the relative tolerance is satisfied:

```
match := |actual - expected| ≤ abs  OR  |actual - expected| / max(|expected|, ε) ≤ rel
```

where ε = `1e-12`. This handles small numbers (where rel blows up) and large numbers (where abs is meaningless) without per-fixture special-casing.

## 6. Tolerance Defaults

Fixtures may declare any tolerance they want. Authors should start from the metric-type defaults below and override only with a `tolerance_reason` field next to the override:

| Metric class | Examples | Default abs | Default rel |
|---|---|---|---|
| Money / NPV / dirty/clean price | `npv`, `clean_price`, `dirty_price`, `accrued`, `upfront` | 0.01 | 1e-6 |
| Sensitivities (Greeks) | `dv01`, `cs01`, `delta`, `gamma`, `vega`, `theta`, `rho` | 0.5 | 1e-4 |
| Rates / spreads | `par_rate`, `ytm`, `z_spread`, `oas`, `discount_margin`, `breakeven_inflation` | 1e-4 (1bp) | — |
| Implied vol | `implied_vol`, `atm_vol`, `wing_vol` | 1e-4 | — |
| Discount factors | `discount_factor` | 1e-8 | — |
| Hazard / probabilities | `survival_probability`, `hazard_rate` | 1e-6 | — |
| Calibration residual | `calibration_rmse`, `repricing_error` | 1e-4 (1bp) | — |
| Pure analytics math | `arith_return`, `log_return`, `sharpe`, `vol`, `max_dd`, `var`, `es` | 1e-8 | — |
| Attribution residual | `residual` after carry+rates+credit | 5e-5 (0.5bp of total return) | — |

Override example:

```json
"tolerances": {
  "ytm": {"abs": 2e-6, "tolerance_reason": "QL uses Brent on YTM, finstack uses Newton; ~1e-6 disagreement expected"}
}
```

This table also lives in `docs/golden-tolerances.md` (created in Phase 1) so it stays callable as a reference outside this spec.

## 7. Test Runners

### 7.1 Rust runner

Located at `finstack/<crate>/tests/golden/mod.rs`. Provides:

- `GoldenFixture` struct deserialized from JSON.
- `run_golden!(path)` macro that takes a path relative to the crate's `tests/golden/data/`.
- Per-domain dispatch: the macro reads `inputs.instrument.kind` (or analytics/attribution analog) and routes to a domain-specific runner under `tests/golden/runners/`.
- Tolerance comparison helper using the abs-OR-rel semantics from §5.3.
- A failure message format that includes fixture path, metric, actual, expected, abs-diff, rel-diff, and tolerance.

Example test:

```rust
// finstack/valuations/tests/golden/pricing/irs.rs
use crate::run_golden;

#[test]
fn golden_irs_usd_sofr_5y_par() {
    run_golden!("pricing/irs/usd_sofr_5y_par.json");
}
```

`tests/golden/mod.rs` also contains the **walk-test** that asserts every fixture under `tests/golden/data/` is well-formed:

```rust
#[test]
fn all_fixtures_well_formed() {
    for fixture in walk_fixtures("tests/golden/data") {
        fixture.validate_schema_version();
        fixture.validate_provenance_complete();
        fixture.validate_tolerances_cover_outputs();
        fixture.validate_screenshots_exist_on_disk();
    }
}
```

### 7.2 Python runner

Located at `finstack-py/tests/golden/conftest.py`. Provides:

- `run_golden(path)` function with the same semantics as the Rust macro, but consuming the same JSON files in the Rust crate's `tests/golden/data/` directories (paths constructed via a shared helper in `conftest.py`).
- pytest parametrization that auto-discovers fixtures by glob, so a single test file can yield N tests:

```python
# finstack-py/tests/golden/test_pricing_irs.py
import pytest
from .conftest import run_golden, discover_fixtures

@pytest.mark.parametrize("fixture", discover_fixtures("pricing/irs"))
def test_pricing_irs(fixture):
    run_golden(fixture)
```

The walk-test from §7.1 is mirrored as a Python test that runs the same checks on the same files — catches drift if someone hand-edits fixtures and forgets to update review fields.

### 7.3 Shared dispatch contract

The runner must call the **canonical public API** — no test-only constructors that bypass validation. For each domain, the runner specifies:

- How to build the input objects from `inputs` (e.g. construct `MarketContext` from `inputs.curves`, build `InterestRateSwap` from `inputs.instrument`).
- Which canonical entry point to call (e.g. `swap.price_with_metrics(market, as_of, &metrics, options)`).
- How to extract each `expected_outputs` metric from the result.

Domain dispatch lives in:

```
finstack/valuations/tests/golden/runners/
├── pricing_irs.rs
├── pricing_bond.rs
├── pricing_equity_option.rs
├── ...
finstack-py/tests/golden/runners/
├── pricing_irs.py
├── pricing_bond.py
├── ...
```

Each runner is small (≤ 100 LOC) and only knows one domain. Adding a new instrument type = adding a new runner.

## 8. Generator & Workflow Tooling

All under `scripts/goldens/`. Python because every source mode either uses QL-Python or BLPAPI or interactive CLI for manual entry.

### 8.1 `regen.py` — capture or refresh a fixture

```
uv run scripts/goldens/regen.py \
    --kind irs-par \
    --tenor 5Y --rate 0.04 --currency USD \
    --as-of 2026-04-30 \
    --reviewer jeickmeier \
    --source quantlib \
    --out finstack/valuations/tests/golden/data/pricing/irs/usd_sofr_5y_par.json
```

- `--source quantlib`: runs through QL-Python adapter, emits JSON automatically.
- `--source bloomberg-api`: runs through BLPAPI adapter, requires terminal access.
- `--source bloomberg-screen` / `--source intex`: generates a YAML stub with input/output keys, opens `$EDITOR` for the analyst to fill in values from the screen, copies referenced screenshots into the fixture's `screenshots/` directory, writes the JSON.
- `--refresh <existing-fixture-path>`: re-pulls *the same scenario* (same inputs, new outputs); for manual sources, pre-fills the YAML stub with previous values so the analyst sees diffs as they re-type. Updates `last_reviewed_by` / `last_reviewed_on`.

Adapters live under `scripts/goldens/adapters/<kind>.py`. Each adapter:

- Knows how to build inputs from the `--kind` arg signature.
- Runs the reference computation (QL engine call / BLPAPI request / YAML capture).
- Emits a JSON conforming to schema `finstack.golden/1`.

New `kind` = new adapter file. Adapters share a common `BaseAdapter` interface for the JSON envelope.

### 8.2 `drift_report.py` — opt-in diagnostic

```
uv run scripts/goldens/drift_report.py \
    [--source quantlib|bloomberg-api] \
    [--filter pricing/irs/*]
```

For each fixture matched by `--filter`:

- Reads the frozen JSON's `inputs` and `expected_outputs`.
- Re-runs the same source adapter to get today's outputs.
- Prints a table: fixture | metric | frozen | today | abs-diff | rel-diff | within-tolerance?

Diagnostic only — does not gate CI. Manual sources (`bloomberg-screen`, `intex`) are skipped automatically since they need analyst interaction.

### 8.3 `stale_report.py` — review-window check

```
uv run scripts/goldens/stale_report.py [--threshold-months 6]
```

Walks all fixtures, reads `provenance.last_reviewed_on` and `provenance.review_interval_months`, prints fixtures past their review window sorted oldest-first. Exit code 1 if any fixture is stale.

### 8.4 mise tasks

```toml
[tasks.goldens-regen]
run = "uv run scripts/goldens/regen.py"

[tasks.goldens-drift]
run = "uv run scripts/goldens/drift_report.py"

[tasks.goldens-stale]
run = "uv run scripts/goldens/stale_report.py"

[tasks.goldens-test]
run = """
cargo test -p finstack-valuations --test golden &&
cargo test -p finstack-analytics --test golden &&
uv run pytest finstack-py/tests/golden -v
"""
```

### 8.5 CI integration

- `cargo test --workspace` runs Rust goldens on every commit. Fast, no Python deps.
- `pytest finstack-py/tests/golden` runs in the existing Python CI matrix (depends on `mise run python-build`).
- `goldens-stale` runs in a separate scheduled CI job (weekly) and creates a GitHub issue listing stale fixtures. Does **not** block PRs.
- `goldens-drift` is **not** wired into CI by default — it requires QL/BLPAPI access and is opt-in for humans diagnosing.

## 9. Layout

### 9.1 Per-crate goldens directory

```
finstack/<crate>/tests/golden/
├── mod.rs                         # macros, walk-test, common helpers
├── runners/
│   ├── pricing_irs.rs
│   ├── pricing_bond.rs
│   ├── ...
├── pricing.rs                     # delegating mod (cargo test discovery)
├── calibration.rs
├── attribution.rs                 # if present in this crate
└── data/
    ├── pricing/
    │   ├── irs/*.json + screenshots/
    │   ├── bond/*.json
    │   ├── ...
    ├── calibration/
    │   ├── curves/*.json
    │   ├── vol/*.json
    │   ├── hazard/*.json
    └── integration/                # cross-crate scenarios live here
        └── *.json
```

### 9.2 Python mirror

```
finstack-py/tests/golden/
├── conftest.py                    # run_golden(), discover_fixtures()
├── runners/
│   ├── pricing_irs.py
│   ├── ...
├── test_pricing_irs.py            # parametrized over data/pricing/irs/*.json
├── test_pricing_bond.py
├── test_calibration_curves.py
├── test_analytics_returns.py
├── ...
└── (no JSON files — all paths point into Rust crate dirs via shared resolver)
```

### 9.3 Cross-crate placement rule

Fixtures that span multiple crates live in the **consuming** crate's `tests/golden/data/integration/`. A "calibrate OIS curve, then price IRS" scenario lives in `finstack/valuations/tests/golden/data/integration/` because valuations consumes the curve.

## 10. v1 Coverage — Comprehensive List

89 fixtures total. Every row below is one JSON file, one Rust test, one Python test. Where source = `bloomberg-screen` or `intex`, screenshots are mandatory.

### 10.1 Pricing — single instrument (48 fixtures)

#### Rates building blocks (10)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/rates/usd_deposit_3m.json` | quantlib + bloomberg-screen ICVS | npv, dv01 |
| `pricing/rates/usd_fra_3x6.json` | quantlib + bloomberg-screen FRA | npv, par_fwd_rate, dv01 |
| `pricing/rates/eurusd_fx_swap_3m.json` | quantlib + bloomberg-screen FXIP | npv, fwd_points, dv01 |
| `pricing/rates/usd_ois_swap_5y.json` | quantlib + bloomberg-screen SWPM | npv, par_rate, dv01 (overnight compounding) |
| `pricing/rates/usd_irs_sofr_5y_par.json` | quantlib + bloomberg-screen SWPM | npv, par_rate, dv01 |
| `pricing/rates/usd_irs_sofr_5y_off_par.json` | quantlib + bloomberg-screen SWPM | npv, dv01, modified_duration |
| `pricing/rates/usd_irs_sofr_10y.json` | quantlib + bloomberg-screen SWPM | npv, par_rate, dv01, bucketed_dv01 (1Y/2Y/5Y/10Y) |
| `pricing/rates/usd_irs_sofr_2y.json` | quantlib + bloomberg-screen SWPM | npv, par_rate, dv01 |
| `pricing/rates/eur_irs_estr_5y.json` | quantlib + bloomberg-screen SWPM | npv, par_rate, dv01 |
| `pricing/rates/gbp_irs_sonia_5y.json` | quantlib + bloomberg-screen SWPM | npv, par_rate, dv01 |

#### Fixed Income — bonds (8)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/bond/ust_2y_bullet.json` | quantlib + bloomberg-screen YA | clean_price, dirty_price, accrued, ytm, modified_duration |
| `pricing/bond/ust_10y_bullet.json` | quantlib + bloomberg-screen YA | clean_price, ytm, mod_dur, dv01, key_rate_dv01 |
| `pricing/bond/ust_30y_long_duration.json` | quantlib + bloomberg-screen YA | clean_price, ytm, mod_dur, convexity |
| `pricing/bond/corp_ig_5y_zspread.json` | quantlib + bloomberg-screen YAS | clean_price, z_spread, oas, cs01_zspread |
| `pricing/bond/corp_hy_5y_ytm_recovery.json` | quantlib + bloomberg-screen YAS | clean_price, ytm, recovery_adjusted_spread |
| `pricing/bond/bond_with_accrued_midperiod.json` | quantlib + bloomberg-screen YA | clean, dirty, accrued (day-count stress) |
| `pricing/bond/corp_callable_7nc3.json` | bloomberg-screen OAS1 | clean_price, oas, effective_duration, effective_convexity (flag if API gap) |
| `pricing/bond/amortizing_bond_known_schedule.json` | quantlib + bloomberg-screen YA | clean_price, ytm, mod_dur, weighted_avg_life |

#### Convertible bonds (2)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/convertible/conv_bond_atm_3y.json` | bloomberg-screen OVCV | clean_price, parity, conversion_premium, delta, vega, rho |
| `pricing/convertible/conv_bond_distressed.json` | bloomberg-screen OVCV | clean_price, delta (low-equity stress) |

#### Term loans (1)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/term_loan/term_loan_b_5y_floating.json` | bloomberg-screen LOAN/WCDS | npv, discount_margin, ytm, weighted_avg_life |

#### Equity options — Black-Scholes (5)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/equity_option/bs_atm_call_1y.json` | quantlib + bloomberg-screen OVME | npv, delta, gamma, vega, theta, rho |
| `pricing/equity_option/bs_otm_call_25d.json` | quantlib + bloomberg-screen OVME | npv, delta, gamma, vega |
| `pricing/equity_option/bs_itm_put.json` | quantlib + bloomberg-screen OVME | npv, delta, gamma, vega |
| `pricing/equity_option/bs_short_dated_1m.json` | quantlib + bloomberg-screen OVME | npv, gamma, theta |
| `pricing/equity_option/bs_with_dividend_yield.json` | quantlib + bloomberg-screen OVME | npv, greeks |

#### FX options — Garman-Kohlhagen (4)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/fx_option/gk_eurusd_atm_3m.json` | quantlib + bloomberg-screen OVML | npv, delta_spot, vega |
| `pricing/fx_option/gk_eurusd_25d_call.json` | quantlib + bloomberg-screen OVML | npv, greeks (delta convention stress) |
| `pricing/fx_option/gk_usdjpy_atm_1y.json` | quantlib + bloomberg-screen OVML | npv, greeks (JPY notional convention) |
| `pricing/fx_option/gk_eurusd_otm_call_6m.json` | formula | npv, delta, vega; premium-adjusted delta remains planned until exposed as a metric |

#### Credit — CDS / CDS option / CDS tranche (6)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/cds/cds_5y_par_spread.json` | quantlib + bloomberg-screen CDSW | npv, par_spread, dv01_spread |
| `pricing/cds/cds_5y_running_upfront.json` | quantlib + bloomberg-screen CDSW | npv, upfront, running_spread, jtd |
| `pricing/cds/cds_off_par_hazard.json` | quantlib + bloomberg-screen CDSW | npv, cs01, jtd, recovery01 |
| `pricing/cds/cds_high_yield_recovery.json` | quantlib + bloomberg-screen CDSW | npv, cs01 (recovery sensitivity) |
| `pricing/cds_option/cds_option_payer_atm_3m.json` | quantlib + bloomberg-screen CDSO | npv, delta_spread, vega, gamma_spread |
| `pricing/cds_tranche/cdx_ig_5y_3_7_mezz.json` | quantlib + bloomberg-screen CDXT | upfront, par_spread, cs01 (Gaussian copula, base correlation) |

#### CLO / ABS (2)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/structured_credit/clo_mezzanine_base_case.json` | intex + bloomberg-screen DDIS | tranche cashflow waterfall (interest + principal per period, 5–10 periods), tranche_irr, weighted_avg_life, expected_loss |
| `pricing/structured_credit/abs_credit_card_senior.json` | intex + bloomberg-screen DDIS | waterfall outputs, irr, wal |

#### Bond / IR / equity-index futures (4)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/bond_future/ust_ty_10y_front_month.json` | formula | npv, dv01; conversion factor, basis, implied repo, and CTD identifiers remain source-reference fields until exposed as metrics |
| `pricing/ir_future/sofr_3m_quarterly.json` | quantlib + bloomberg-screen SR3 | futures_price, convexity_adjustment, implied_forward, dv01 |
| `pricing/ir_future/sofr_1m_serial.json` | quantlib + bloomberg-screen | futures_price (avg vs compounded mechanics) |
| `pricing/equity_index_future/spx_es_3m.json` | formula | npv, delta, dv01; futures price and basis remain source-reference fields until exposed as metrics |

#### Inflation (2)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/inflation/inflation_linked_bond_5y.json` | quantlib + bloomberg-screen YA / TIPS | clean_price, real_yield, breakeven_inflation, inflation_adjusted_principal, mod_duration |
| `pricing/inflation/inflation_zc_swap_5y.json` | quantlib + bloomberg-screen ZCIS | npv, par_breakeven_rate, dv01_real, inflation_dv01 |

#### Rates options — cap/floor + swaption (4)

| Fixture | Source | Key assertions |
|---|---|---|
| `pricing/cap_floor/usd_cap_5y_atm_black.json` | quantlib + bloomberg-screen OVCF | npv, vega, dv01, per-caplet decomposition |
| `pricing/cap_floor/usd_floor_5y_atm_normal.json` | quantlib + bloomberg-screen OVCF | npv, vega, dv01 (Bachelier/Normal model) |
| `pricing/swaption/usd_swaption_5y_into_5y_payer_atm.json` | quantlib + bloomberg-screen OVSW | npv, delta, vega, gamma, vanna |
| `pricing/swaption/usd_swaption_5y_into_5y_receiver_25_otm.json` | quantlib + bloomberg-screen OVSW | npv, vega (smile-OTM stress) |

### 10.2 Calibration (11 fixtures)

| Fixture | Source | Key assertions |
|---|---|---|
| `calibration/curves/usd_ois_bootstrap.json` | quantlib + bloomberg-screen ICVS 134 | discount_factors at standard pillars, zero_rates |
| `calibration/curves/usd_sofr_3m_bootstrap.json` | quantlib + bloomberg-screen ICVS 490 | discount, forward 3M at pillars |
| `calibration/curves/eur_estr_bootstrap.json` | quantlib + bloomberg-screen ICVS | DFs, zeros |
| `calibration/curves/gbp_sonia_bootstrap.json` | quantlib + bloomberg-screen ICVS | DFs, zeros |
| `calibration/curves/jpy_tona_bootstrap.json` | quantlib + bloomberg-screen ICVS | DFs, zeros |
| `calibration/curves/usd_cpi_zc_inflation_bootstrap.json` | quantlib + bloomberg-screen ZCIS | inflation zero rates at pillars, inflation index forecast |
| `calibration/vol/usd_swaption_sabr_cube.json` | quantlib + bloomberg-screen VCUB | sabr params (alpha/beta/rho/nu) at expiry/tenor pairs, repriced ATM vol RMSE |
| `calibration/vol/spx_equity_vol_smile.json` | quantlib + bloomberg-screen OMON/OVDV | smile params, repriced strikes |
| `calibration/vol/eurusd_fx_vol_smile.json` | quantlib + bloomberg-screen OVML smile | 25d/10d wing vols, ATM, butterfly/risk-reversal |
| `calibration/hazard/cdx_ig_hazard.json` | quantlib + bloomberg-screen CDX | hazard rates at pillars, survival probabilities |
| `calibration/hazard/single_name_hazard_5y.json` | quantlib + bloomberg-screen CDSD | hazard at 1Y/3Y/5Y/7Y/10Y, survival probs |

### 10.3 Cross-crate — calibrate-then-price (3 fixtures)

Live in `finstack/valuations/tests/golden/data/integration/`.

| Fixture | Source | Key assertions |
|---|---|---|
| `integration/usd_ois_calib_then_price_5y_irs.json` | quantlib + bloomberg-screen ICVS+SWPM | calibrated DFs at pillars **and** IRS NPV/DV01 |
| `integration/swaption_calib_then_price_atm.json` | quantlib + bloomberg-screen VCUB+OVSW | SABR params **and** swaption NPV+vega |
| `integration/cds_hazard_calib_then_price_off_par.json` | quantlib + bloomberg-screen CDSW | hazard at pillars **and** CDS NPV/CS01 on a different (off-par) instrument |

### 10.4 Analytics (19 fixtures)

References for analytics come from synthetic series with closed-form expected values, or from textbook formulas, or from `quantstats` / `empyrical` cross-checks. Source = `formula` or `textbook` (see provenance enum). Lives in `finstack/analytics/tests/golden/data/`.

| Fixture | Source | Key assertions |
|---|---|---|
| **Returns & period stats** | | |
| `analytics/returns/log_vs_arith_roundtrip.json` | formula | log_ret, arith_ret, cumulative |
| `analytics/returns/period_stats_monthly_quarterly_annual.json` | formula | grouped means, vols, geometric returns by period |
| `analytics/returns/period_stats_weekly_iso.json` | formula | weekly returns at year-end transition |
| `analytics/returns/returns_with_missing_data.json` | formula | NaN propagation, partial-period handling |
| `analytics/returns/cumulative_returns.json` | formula | cumulative wealth, ytd_return |
| **Performance ratios** | | |
| `analytics/performance/sharpe_known_series.json` | formula + quantstats | sharpe at rf=0 and rf=3% |
| `analytics/performance/sortino_known_series.json` | formula + quantstats | sortino, downside_deviation |
| `analytics/performance/calmar_ratio.json` | formula | calmar, max_dd |
| `analytics/performance/information_ratio.json` | formula | tracking_error, info_ratio vs benchmark |
| `analytics/performance/treynor_m2_modsharpe.json` | formula | treynor, m2, modified_sharpe (Cornish-Fisher) |
| **Volatility models** | | |
| `analytics/vol/rolling_volatility.json` | formula | rolling_vol(window=21,63,252) |
| `analytics/vol/ewma_riskmetrics_lambda_94.json` | textbook (RiskMetrics 1996) | ewma_vol, ewma_correlation pair |
| `analytics/vol/garch_11_known_series.json` | textbook (Bollerslev 1986) + arch package | GARCH(1,1) params, fitted vol path |
| **Drawdown & risk** | | |
| `analytics/drawdown/maxdd_calmar_ulcer.json` | formula | max_dd, ulcer_index, martin_ratio |
| `analytics/drawdown/cdar_chekhlov.json` | textbook (Chekhlov 2005) | conditional_drawdown_at_risk(α=0.05) |
| `analytics/risk/parametric_var_es.json` | textbook (RiskMetrics) | parametric VaR/ES at 95%, 99% |
| `analytics/risk/historical_var_es.json` | formula | historical VaR/ES at 95%, 99% |
| `analytics/risk/cornish_fisher_var.json` | textbook (Cornish-Fisher) | CF-VaR vs parametric (skew/kurt adj) |
| **Benchmark relative** | | |
| `analytics/benchmark/beta_alpha_regression.json` | formula + scipy.stats.linregress | beta, alpha, r_squared |

### 10.5 Attribution (8 fixtures)

References from textbook examples. Lives in `finstack/valuations/tests/golden/data/attribution/` (valuations consumes attribution APIs).

| Fixture | Source | Key assertions |
|---|---|---|
| **Equity attribution (5)** | | |
| `attribution/brinson_fachler_2period.json` | textbook (Bacon ch. 5) | allocation, selection, interaction at segment level + total |
| `attribution/brinson_hood_beebower.json` | textbook (Brinson 1986) | allocation, selection (no interaction) variant |
| `attribution/multi_factor_ff3_attribution.json` | formula + statsmodels OLS | factor_loadings, factor_contribs, residual |
| `attribution/currency_local_decomposition.json` | textbook (Bacon ch. 7) | local_return, currency_return, total |
| `attribution/contribution_to_return.json` | formula | weighted_contribution per holding, sum to total |
| **Fixed-income risk-based attribution (3)** | | |
| `attribution/fi_carry_decomposition.json` | textbook (Campisi 2000) | total_return = carry (coupon + roll-down + roll-up) + price_change; components sum exactly |
| `attribution/fi_curve_attribution_parallel_slope_twist.json` | textbook (Bacon ch. 9) | parallel_shift, slope, twist, butterfly contributions; residual |
| `attribution/fi_risk_based_carry_rates_credit_residual.json` | textbook (Campisi + finstack canonical) | full decomposition: carry + rates (parallel + slope + twist) + credit (spread × duration) + residual; sums to total |

References Campisi 2000 and Bacon (Practical Portfolio Performance Measurement and Attribution) need to be added to `docs/REFERENCES.md` under a new "Performance attribution" section.

### 10.6 v1 totals

| Bucket | Count |
|---|---|
| Pricing — rates building blocks | 10 |
| Pricing — fixed income (bonds + callable + amortizing) | 8 |
| Pricing — convertible bonds | 2 |
| Pricing — term loans | 1 |
| Pricing — equity options | 5 |
| Pricing — FX options | 4 |
| Pricing — CDS / CDS option / CDS tranche | 6 |
| Pricing — CLO / ABS | 2 |
| Pricing — bond / IR / equity-index futures | 4 |
| Pricing — inflation | 2 |
| Pricing — rates options | 4 |
| Calibration — curves + vol + hazard + inflation | 11 |
| Cross-crate — calibrate-then-price | 3 |
| Analytics | 19 |
| Attribution (5 equity + 3 FI risk-based) | 8 |
| **Total v1** | **89** |

## 11. Error Handling

- **Fixture parsing failures** (missing required fields, unknown schema version): runner returns a clear error pointing at the fixture path and the offending field. Walk-test catches these at `cargo test` time before any individual fixture runs.
- **Adapter failures during regen** (QL throws, BLPAPI returns no data): adapter raises with a message that tells the analyst which input was the problem; no JSON is written.
- **Missing screenshots referenced from JSON**: walk-test fails with the specific fixture path and screenshot path.
- **Tolerance violation**: failure message format includes fixture path, metric, actual, expected, abs-diff, rel-diff, and the tolerance used. No truncation — full precision.
- **Drift report**: shows tolerance violations as a warning column, not a failure exit code (drift report is diagnostic, not gate).
- **Stale report**: exit 1 if any fixture is past its review window. Used by the weekly scheduled CI job to file an issue.

## 12. Testing the framework itself

The framework needs its own meta-tests to avoid the embarrassment of "the golden test framework had a bug that made all goldens silently pass":

- **Tolerance comparator unit tests**: verify abs-OR-rel semantics, including edge cases (expected = 0, expected very large, both abs and rel given, only one given).
- **Walk-test self-test**: a deliberately malformed fixture committed under `tests/golden/data/_meta/` that the walk-test must reject. (Not run as a regular test — gated by a feature flag so it doesn't pollute the normal suite.)
- **Schema deserialization round-trip**: parse every committed fixture, re-serialize, parse again, assert byte-identical (modulo whitespace). Catches schema drift.
- **Cross-language fixture identity**: a single test that parses one fixture in Rust and one in Python, prints both as canonical JSON, and asserts equal. Catches "Python sees a different fixture than Rust" failures.

These live alongside the runners in `tests/golden/mod.rs` (Rust) and `tests/golden/conftest.py` (Python).

## 13. Existing tests — cleanup plan

Before the new framework lands, the existing tree contains several misnamed or misplaced tests. The word "parity" currently means three different things across the codebase: sometimes "internal consistency," sometimes "construction smoke test," sometimes "actual external reference comparison." Phase 0 (see §15) renames these so that after this work, "parity" means only the third — and lives only inside the new `tests/golden/` framework.

### 13.1 Renames and moves

| Existing path | What the name implies | What it actually does | Action |
|---|---|---|---|
| `valuations/tests/quantlib_parity/` (5 files) + `tests/quantlib_parity.rs` entry point | Parity vs QuantLib | Asserts internal consistency only — par-rate self-consistency, pay/receive symmetry, DV01 magnitude bands. No actual QL reference values. | **Rename** directory and entry-point to `tests/sanity_invariants/` and `tests/sanity_invariants.rs`. Tests stay; the misleading name goes away. |
| `valuations/tests/calibration/parity_comprehensive.rs` | Comprehensive parity test | "Comprehensive quote-to-instrument construction test" per the doc comment — purely tests serde/builder construction across quote types. | **Rename** to `calibration/quote_construction.rs`. Update `calibration/mod.rs` and any super-references. |
| `valuations/tests/calibration/v2_parity.rs` | Parity for v2 engine | Tests that the v2 calibration engine works on a simple USD setup. Smoke test, not parity. | **Rename** to `calibration/v2_engine_smoke.rs`. Update `calibration/mod.rs`. |
| `valuations/tests/calibration/bloomberg_accuracy.rs` | Bloomberg parity | Honest — pulls Bloomberg-derived quotes and validates calibrated curves against Bloomberg reference data. | **Keep file as-is in Phase 0.** In Phase 4 (calibration goldens), migrate its embedded literal arrays into `finstack.golden/1` JSON fixtures so the data becomes refreshable through `regen.py` like everything else. |
| `valuations/tests/golden/credit_factor_model_v1.json` | Numerical golden fixture | Schema-validation fixture for the `finstack.credit_factor_model/1` schema (per INVARIANTS.md §8). Different purpose. | **Move** to `valuations/tests/schema_fixtures/credit_factor_model_v1.json`. Update consumers `credit_calibration.rs` and `integration/schema/credit_factor_model.rs`. After this, `tests/golden/data/` has one consistent purpose (numerical reference data). |
| `analytics/tests/fixtures/analytics_parity.json` | Analytics parity reference | Used by `api_invariants.rs` — tests API invariants (shape, NaN propagation), not numerical parity. | **Rename** to `fixtures/api_invariants_data.json`. Update `api_invariants.rs`. |

### 13.2 Why bundle this into the same PR

These cleanups are integral to the framework's clarity, not separate work:

- The new framework introduces `tests/golden/data/` as a single-purpose namespace. Leaving `credit_factor_model_v1.json` (a schema fixture) inside that namespace creates exactly the category confusion the spec is trying to remove.
- The new framework gives "parity" a precise meaning. Leaving three different misuses in place defeats the new precision before it's even shipped.
- Bundle = one PR, one clean diff, one commit message that explains the rationale once. Splitting = two PRs, two reviews, the cleanup PR's rationale ("we want to free these names for a future framework") is harder to defend out of context.

The cleanup is purely renames + path moves + import updates. Zero behavioral change. Risk is low and isolated to test-only files.

### 13.3 Migration paths once the framework is in place

These don't happen in Phase 0 — they're follow-ups in later phases:

- **Phase 3 (pricing goldens land):** the renamed `tests/sanity_invariants/` tests stay where they are. They keep their internal-consistency assertions (which are useful regardless). New `tests/golden/data/pricing/irs/*.json` fixtures add the actual parity assertions on top.
- **Phase 4 (calibration goldens land):** `bloomberg_accuracy.rs`'s embedded literal arrays move into `tests/golden/data/calibration/curves/*.json`. The `.rs` test file shrinks to a thin runner, and the data becomes refreshable through `regen.py` like every other golden. This is the only existing test whose data graduates into the new framework — everything else stays on the sanity/smoke side.

## 14. Out of Scope (v2+)

| Domain | Why deferred |
|---|---|
| **Stress / scenario testing** | Builds on calibration v1; needs its own scenario-authoring design pass |
| **ISDA SIMM** | Whole subsystem (CRIF mapping, bucket structure, risk-class aggregation); deserves its own design |
| **SA-CCR** | Replacement-cost + PFE goldens against BCBS 279 worked examples; same scope rationale |
| **Monte Carlo determinism** | Bit-exact path goldens for HW1F / QE-Heston; partially covered by INVARIANTS.md §2; v2 formalizes |
| **Exotics** | Autocallable, barrier (touch/knock-out), basket, lookback, range accrual, snowball, TARN, variance/quanto — each needs its own pricer-vs-reference design (often multi-method: PDE vs MC vs Fourier) |
| **Inflation YoY swap, inflation cap/floor** | YoY adds convexity adjustment; inflation cap/floor adds vol surface |
| **Commodity option, commodity forward** | Uses futures-curve calibration not in v1 |
| **MBS / TBA / dollar roll** | Prepayment-model-driven goldens — prepay model needs reference alignment first |
| **CMS / CMS spread / CMS option** | Uses convexity-adjusted swap-rate replication; build after v1 vol-cube fixture is solid |
| **Repo, NDF, FX barrier, FX touch, FX variance, FX digital, quanto** | Pricers exist; reference-value parity is a separate question per instrument |
| **Cross-currency basis curves** | Builds on FX-swap fixture but needs its own basis-curve design |
| **Statements / portfolio analytics** | Financial-statement-derived metrics, capital structure waterfalls — not the core "pricing/risk" question this design targets |

## 15. Implementation Phasing

Coarse phasing for the implementation plan (writing-plans skill will turn this into ordered tasks):

0. **Existing-tests cleanup** (per §13) — rename `quantlib_parity/` → `sanity_invariants/`; rename `parity_comprehensive.rs` → `quote_construction.rs`; rename `v2_parity.rs` → `v2_engine_smoke.rs`; move `tests/golden/credit_factor_model_v1.json` → `tests/schema_fixtures/`; rename `analytics_parity.json` → `api_invariants_data.json`. Update all import / include / consumer references. Zero behavioral change. Verify with `mise run all-fmt && mise run all-lint && mise run all-test`.
1. **Framework foundation** — fixture schema crate (Rust + Python), tolerance comparator, walk-test, run_golden runner skeletons. No instruments yet. Validates the plumbing.
2. **First domain end-to-end** — pricing/irs/usd_sofr_5y_par.json with QL adapter, Rust runner, Python runner, walk-test, generator script. One fixture, one generator command, both layers green.
3. **Pricing — rates expanding to fixed-income, equity options, FX options** — adapter per instrument family. Existing `sanity_invariants/` tests stay alongside; new goldens add real reference assertions on top.
4. **Calibration — curve bootstrap + vol cube + hazard** — calibration runners, calibration adapters. **Includes** migrating `bloomberg_accuracy.rs`'s embedded literal arrays into `finstack.golden/1` JSON fixtures.
5. **Cross-crate integration fixtures**.
6. **Analytics + attribution** — different runner shapes (timeseries inputs, no market context).
7. **Manual-source instruments (callable, CLO/ABS, convertibles)** — bloomberg-screen + intex regen flows.
8. **Drift + stale tooling, CI integration**.
9. **Documentation**: `docs/golden-tolerances.md`, README updates, REFERENCES.md additions for Campisi / Bacon.

## 16. References

- This design will add anchors to `docs/REFERENCES.md`:
  - Campisi 2000 (FI risk-based attribution)
  - Bacon — *Practical Portfolio Performance Measurement and Attribution*
  - Tsiveriotis-Fernandes 1998 (convertible bond pricing)
- Existing references already cited:
  - QuantLib — primary parity source for vanilla instruments
  - Bloomberg screens (SWPM, OVME, OVML, OVCF, OVSW, OVCV, OAS1, YA, YAS, ICVS, VCUB, CDSW, CDSO, CDXT, ZCIS, DDIS, DLV, CT, FRA, FXIP, SR3) — practitioner reference values
  - Intex — CLO/ABS cashflow waterfall reference
  - RiskMetrics 1996 — parametric VaR / EWMA
  - quantstats / empyrical — analytics cross-checks
  - INVARIANTS.md §6 — golden-test convention this spec implements
