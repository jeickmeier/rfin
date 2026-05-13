# Hard-Coded Values Configuration Registry - Design Spec

**Status:** Draft
**Date:** 2026-04-26
**Owner:** finstack/core + finstack/valuations + finstack/margin + finstack/statements-analytics + bindings
**Schemas:** `finstack.credit_assumptions/1`, `finstack.structured_credit_assumptions/1`, `finstack.market_contract_specs/1`, `finstack.accounting_policy/1`

## 1. Motivation

The workspace still contains hard-coded values that are not stable algorithmic constants. Several are external market conventions, rating-agency studies, regulatory/accounting policies, exchange contract specs, or business assumptions that can change over time. The highest-risk examples are credit calibration tables:

- Moody's WARF / idealized default-rate factors.
- Moody's and S&P historical recovery distributions.
- S&P and Moody's empirical PD master scales.
- Structured-credit CDR, CPR, recovery, correlation, volatility, seasoning, fee, and concentration assumptions.

This spec moves those values into versioned registries while preserving today's behavior by making the embedded registry defaults exactly match the current literals.

## 2. Goals

- Replace mutable external assumptions with embedded, versioned JSON registry data plus optional `FinstackConfig.extensions` overlays.
- Keep Rust as the canonical API surface; Python and WASM must reuse Rust defaults rather than duplicate literals.
- Preserve current numerical behavior in the default registry snapshot unless a value is explicitly identified as inconsistent and changed in a separate review.
- Make every external assumption traceable to a registry id, source label, study period or effective date, and schema version.
- Fail fast on malformed registry data during tests and at explicit registry construction time.
- Leave true algorithmic constants, validation clamps, array indices, and numerical tolerances in code.

## 3. Non-Goals

- Do not fetch Moody's, S&P, exchange, regulator, or vendor data from the network at runtime.
- Do not change current pricing/risk outputs during the migration except where tests prove that an existing value is internally inconsistent and a separate migration note approves the fix.
- Do not move every numeric literal. Test fixtures, examples, validation bounds, published formula coefficients, and pure math constants stay code-side unless they are used as mutable policy.
- Do not introduce global mutable state. Registries follow the existing embedded-data plus config-overlay pattern.

## 4. Classification Rules

| Class | Examples | Target |
|---|---|---|
| External source data | Moody's WARF, S&P PD bands, agency recovery studies, exchange contract specs, MBS agency delays | JSON registry |
| Business assumptions | structured-credit standard CDR/CPR/recovery, fees, trustee fees, seasonality, stress grids | JSON registry |
| Regulatory/accounting policy | IFRS staging, CECL horizon, margin CCP parameters, collateral haircuts, Basel floors | JSON registry |
| Host-language defaults | Python/WASM annualization, basis name, currency default, liquidity thresholds | Rust default object or shared constant exported through bindings |
| Published model definitions | Altman/Ohlson/Zmijewski coefficients | named constants or model-definition registry if governance/versioning is needed |
| Algorithmic constants | clamps, tolerances, interpolation weights, test data | code, with a named constant only if reused |

## 5. Registry Architecture

Use the pattern already present in `finstack/margin/src/registry`:

- Registry JSON is embedded with `include_str!`.
- Each owning crate exposes a parsed registry type and `embedded_registry() -> Result<&'static Registry>`.
- Each owning crate exposes `registry_from_config(cfg: &FinstackConfig) -> Result<Registry>` for overlays.
- Extension keys are versioned and namespaced, for example `core.credit_assumptions.v1` or `valuations.structured_credit_assumptions.v1`.
- Registry records use strict serde types and validation helpers. Unknown fields should fail parsing unless there is a clear forward-compatibility reason.
- Public constructors that need defaults should call registry-backed constructors. If a registry lookup fails, return an error unless the function is explicitly documented as infallible and only reads compile-time embedded data.

### 5.1 Shared Registry Envelope

All new registries should use an envelope with source metadata:

```json
{
  "schema_version": "finstack.credit_assumptions/1",
  "default_id": "moodys_idealized_default_rates_2024_09",
  "entries": [
    {
      "ids": ["moodys_idealized_default_rates_2024_09", "moodys_standard"],
      "source": "Moody's Approach to Rating Collateralized Loan Obligations",
      "source_version": "2024-09",
      "effective_date": "2024-09-01",
      "study_period": null,
      "values": {}
    }
  ]
}
```

Use `study_period` for historical studies and `effective_date` / `source_version` for rulebooks, exchange specs, regulatory tables, or methodology snapshots.

### 5.2 Overlay Semantics

- Overlay entries are additive by id.
- If an overlay reuses an existing id, it replaces the full entry and must pass the same validation.
- Defaults can be overridden only by a top-level `default_id` field.
- Validation errors must report the registry key, entry id, field path, and invalid value.
- Serialization order should be deterministic (`BTreeMap` or sorted vectors) wherever observable.

## 6. Registry Inventory

### 6.1 Core Credit Assumptions Registry

Create `finstack/core/data/credit/credit_assumptions.v1.json` and `finstack/core/src/credit/registry`.

Move these current literals:

- `finstack/core/src/types/ratings.rs`
  - `RatingFactorTable::moodys_standard()`: Moody's WARF / idealized default-rate factor table, `default_factor = 3650`.
- `finstack/valuations/src/instruments/fixed_income/structured_credit/metrics/pool/warf.rs`
  - Missing-rating fallback `3650.0`; must read the active rating-factor table default.
- `finstack/core/src/credit/lgd/seniority.rs`
  - Moody's 1982-2023 recovery means/std devs by seniority.
  - S&P historical recovery means/std devs by seniority.
  - Agency aliases and default agency.
- `finstack/core/src/credit/pd/master_scale.rs`
  - S&P 1981-2023 empirical one-year PD master scale.
  - Moody's 1983-2023 empirical one-year PD master scale.
- `finstack/core/src/credit/lgd/downturn.rs`
  - Basel secured/unsecured LGD floors and add-ons.
- `finstack/core/src/credit/lgd/workout.rs`
  - Default workout costs, default workout duration, and default discount rate.

Public APIs should gain registry-aware constructors, while existing short constructors continue to use embedded defaults:

```rust
RatingFactorTable::from_registry_id("moodys_standard")?;
RatingFactorTable::moodys_standard()?; // registry-backed compatibility constructor
SeniorityCalibration::from_registry_id("moodys_1982_2023")?;
MasterScale::from_registry_id("sp_1981_2023")?;
```

### 6.2 Rating Scale Registry

Unify duplicate rating-scale JSON currently present under both `finstack/statements-analytics/data/rating_scales/` and `finstack/statements/data/rating_scales/`.

- Keep one canonical registry, preferably in `finstack/core/data/rating_scales/rating_scales.v1.json` if multiple crates consume it.
- `statements-analytics` should load through the shared registry rather than `include_str!` local copies.
- Move `DEFAULT_SCORECARD_SCORE = 50.0`, default scale `"S&P"`, Fitch aliasing to S&P notation, and unknown-scale fallback behavior into a scorecard policy record.
- Unknown rating scale should be configurable as either `error`, `fallback_to_default`, or `warn_and_fallback`; default behavior should match today's fallback unless a migration explicitly changes it.

### 6.3 Structured Credit Assumption Registry

Create `finstack/valuations/data/assumptions/structured_credit_assumptions.v1.json` and `finstack/valuations/src/instruments/fixed_income/structured_credit/assumptions`.

Move these groups:

- `types/constants.rs`
  - Mortgage and credit-card seasonality arrays.
  - Baseline unemployment rate.
  - Standard PSA speed grid, CDR rate grid, severity grid.
  - CLO/ABS/CMBS/RMBS servicing, management, and trustee fees.
  - Cleanup threshold and resolution lag.
  - PSA/SDA ramp, peak, terminal, and burnout values.
  - CLO concentration limits.
  - Standard CLO/RMBS/ABS auto/CMBS CDR, CPR, recovery, PSA/SDA, and ABS speed assumptions.
- `types/setup.rs`
  - `DefaultAssumptions` product defaults.
  - Asset-type maps for CPR, CDR, and recovery.
- `types/constructors.rs`
  - ABS/CLO/CMBS/RMBS default first-payment dates, payment frequencies, lockout terms, recovery lags, LTV, and model specs.
- `types/mod.rs`
  - `MarketConditions::default()` refi rate and seasonal factor.
  - `CreditModelConfig` default prepayment/default/recovery specs.
- `pricing/stochastic/calibrations.rs`
  - RMBS/CLO/CMBS stochastic CDR, CPR, correlation, volatility, mean-reversion, factor-loading, refi sensitivity, and burnout profiles.
- `pricing/stochastic/default/*`
  - default correlations, volatility defaults, hazard-curve adapter profiles, RMBS/CLO stochastic constructors, and SDA curve values.
- `pricing/stochastic/prepayment/*`
  - Richard-Roll agency/non-agency profiles, factor loading, CPR volatility, refi slope, ramp months, and burnout.
- `pricing/stochastic/tree/*`
  - tree profile branch counts, seeds, pool coupon defaults, recovery specs, and factor specs.
- `pricing/stochastic/metrics/calculator.rs`
  - fallback LGD when probability mass is effectively zero.

The registry should be organized by `deal_type`, `collateral_type`, `model_family`, `profile_id`, and `source`. It must allow multiple profiles for the same deal type, for example `rmbs_agency_standard`, `rmbs_non_agency_standard`, `clo_bsl_standard`, and `cmbs_conduit_standard`.

### 6.4 Cashflow Prepayment and Default Curve Registry

Create either a small `finstack/cashflows/data/credit_curves/credit_curves.v1.json` registry or consume the structured-credit registry for product-specific curves.

Move or centralize:

- `DefaultModelSpec::sda`: peak month, peak CDR, terminal CDR, terminal month/decline period, default terminal CDR.
- `DefaultModelSpec::cdr_2pct`: baseline 2% CDR convenience value.
- `PrepaymentModelSpec::psa`: ramp months and terminal CPR.
- `PrepaymentModelSpec::psa_100`: 100% PSA multiplier.

The base PSA/SDA definitions can remain named model definitions, but their numeric values must not be duplicated across cashflows and structured-credit code.

### 6.5 Agency MBS and TBA Registry

Create `finstack/valuations/data/assumptions/agency_mbs_conventions.v1.json`.

Move:

- Agency program payment delays and payment-day rules.
- FNMA/FHLMC/GNMA servicing and guarantee-fee defaults.
- TBA assumed-pool defaults: pool factor, servicing fee, guarantee fee, WAC construction, PSA baseline.
- TBA good-delivery heuristics: WAC spread range, low-factor cutoff, loan-size thresholds, seasoning/burnout multipliers, SIFMA face variance tolerance.

Existing agency enum names remain code. Values that come from SIFMA, agency program conventions, or product heuristics move to registry records.

### 6.6 Market Convention Registry Completion

The valuations convention JSON already exists under `finstack/valuations/data/conventions/`. Complete migration by removing code duplicates and routing defaults through the existing `ConventionRegistry`.

Move or route through registry:

- Bond settlement days, ex-coupon days, default discount curves, and calendar ids.
- IRS observation shifts, payment lags, reset lags, business-day conventions, calendars, discount curves, and forward curves.
- Commodity settlement days, business-day conventions, calendars, currencies, units, and exchanges.
- IRS cashflow fallback RFR calendars and `weekends_only` fallback policy.

If a value is required before registry access is possible, expose a named constant in one module and document why it cannot be registry-backed.

### 6.7 Exchange Contract Specs Registry

Create `finstack/valuations/data/contract_specs/contract_specs.v1.json`.

Move:

- Interest-rate future specs: face value, tick size, tick value, delivery months.
- Bond future specs: contract size, tick size/value, standard coupon, standard maturity, settlement days, calendar, repo day-count basis.
- Equity index futures: multiplier, tick size, tick value, settlement method for ES/NQ and other listed products.
- Volatility index futures/options: multipliers, tick sizes, tick values, index ids.
- Commodity futures/options specs if any equivalent values are added or already exist outside the audited snippets.

Registry keys should be exchange/product codes, for example `cme.es`, `cboe.vix_future`, `cme.sofr_3m`, `cme.ust_10y`.

### 6.8 Margin and Regulatory Registry Cleanup

Margin already has a registry. The work here is to remove fallback literals that bypass it:

- CCP `mpor_days` and conservative-rate fallbacks in `clearing.rs`.
- Generic VaR defaults in `clearing.rs` and `metrics/instrument.rs`.
- XVA default grid/recovery should either move to `margin/data/margin/defaults.v1.json` or a new `xva_defaults` section.
- Repo margin defaults and any collateral haircut defaults should route through `margin/data/margin/*.json`.
- Existing SIMM fallback helpers must remain only for legacy registry compatibility and should be covered by tests proving current SIMM versions are fully registry-backed.

### 6.9 Accounting Policy Registry

Create `finstack/statements-analytics/data/accounting/ecl_policy.v1.json`.

Move:

- IFRS 9 staging thresholds: absolute PD delta, relative PD multiplier, rating-downgrade notches, DPD thresholds, qualitative trigger flags, cure periods.
- ECL engine defaults: bucket width, base scenario id/weight, LGD type.
- CECL defaults: bucket width, forecast horizon, reversion method, historical annual PD, scenario default, methodology.

Policy records should be keyed by accounting framework and policy id, for example `ifrs9_default`, `cecl_default`.

### 6.10 Binding and API Default Centralization

Python and WASM must stop duplicating Rust-side defaults.

Move or centralize:

- Portfolio liquidity thresholds and liquidity config defaults.
- Almgren-Chriss wrapper constants and calibration profile defaults.
- Analytics annualization defaults such as `252.0`, default MAR/risk-free values, and `act365_25`.
- Monte Carlo defaults such as `num_steps = 252`, LSMC steps `50`, basis degree `3`, basis name `"laguerre"`, default currency `"USD"`, and `parallel = false`.
- Margin base currency `"USD"` and other binding-level default strings.

Bindings should expose helper constructors or default config JSON generated from Rust types rather than retyping arrays and scalar defaults.

### 6.11 Published Scoring Model Definitions

Altman, Ohlson, and Zmijewski coefficients are published model definitions. They may stay code-side if represented as named model constants with source docs. If governance requires model versioning, create `finstack/core/data/credit/scoring_models.v1.json`.

At minimum:

- Replace anonymous coefficient literals with named structs or constants.
- Separate published coefficients from local implied-PD mappings.
- Move local implied-PD mappings to a configurable model-policy record if they are used for production credit decisions.

## 7. Public API Shape

Each registry-backed domain should expose three layers:

1. Strict parsed registry types.
2. Registry-backed constructors by id.
3. Compatibility constructors preserving current names and defaults.

Example:

```rust
pub fn embedded_credit_assumptions() -> Result<&'static CreditAssumptionRegistry>;
pub fn credit_assumptions_from_config(cfg: &FinstackConfig) -> Result<CreditAssumptionRegistry>;

impl RatingFactorTable {
    pub fn from_registry_id(id: &str) -> Result<Self>;
    pub fn from_registry(registry: &CreditAssumptionRegistry, id: &str) -> Result<Self>;
    pub fn moodys_standard() -> Result<Self>;
}
```

For existing APIs that currently return `Self` infallibly, choose one of:

- Keep the infallible function only if it reads a compile-time embedded registry and panics only for an invalid build asset.
- Add a fallible sibling (`try_moodys_standard`) and mark the infallible function as compatibility-only.
- Prefer fallible constructors for new public APIs.

## 8. Validation Requirements

Every registry loader must validate:

- Percent/rate fields are finite and in the documented range.
- Probabilities and recoveries are in `[0, 1]`.
- Standard deviations and volatilities are non-negative.
- Rating scales are strictly ordered where required.
- WARF/default-factor maps contain every supported rating or define explicit fallback behavior.
- Correlation matrices are symmetric, diagonal is one, values are in `[-1, 1]`, and PSD checks pass where applicable.
- Default ids point to existing entries.
- Aliases do not collide across incompatible entries.
- Study periods and effective dates parse and are included for external studies.

## 9. Migration Plan

### PR 1 - Inventory Guardrails

- Add an audit document or script under `tools/` that searches for external-assumption literals using terms from this audit: Moody's, S&P, recovery, default rate, WARF, CDR, CPR, PSA, SDA, fee, haircut, settlement, tick, multiplier, staging, CECL, ECL, Basel, XVA.
- Add an allowlist for known algorithmic/test literals.
- CI does not need to fail on this immediately, but the script should produce a stable report.

### PR 2 - Core Credit Assumptions

- Add the core credit registry JSON and loader.
- Move WARF, seniority recovery, PD master scales, LGD downturn/workout defaults.
- Update Python LGD/PD bindings to call Rust defaults.
- Add golden tests asserting migrated defaults match current values exactly.

### PR 3 - Rating Scale De-Dupe

- Create the canonical rating-scale registry.
- Remove one duplicate data copy.
- Update statements/statements-analytics loaders.
- Add alias/fallback policy tests.

### PR 4 - Structured Credit Assumptions

- Add structured-credit assumption registry and typed profiles.
- Replace constants and scattered constructors with registry-backed profiles.
- Add tests for current CLO/RMBS/ABS/CMBS default equivalence.
- Add a test highlighting known internal inconsistencies before changing values.

### PR 5 - Cashflow Curve Defaults

- Centralize PSA/SDA definitions.
- Remove duplicate PSA/SDA literals in cashflows and structured-credit stochastic modules.
- Add curve shape tests for 100% PSA and 100% SDA.

### PR 6 - Agency MBS, TBA, and Contract Specs

- Add agency MBS/TBA convention registry.
- Add exchange contract spec registry.
- Route futures/options/MBS defaults through the registries.
- Add tests for all existing default constructors.

### PR 7 - Market Convention Completion

- Remove remaining code duplicates where convention JSON already exists.
- Route IRS, bond, commodity, and cashflow fallback calendars through `ConventionRegistry`.
- Add tests that current enum convenience constructors match registry entries.

### PR 8 - Margin, XVA, and Accounting Policy

- Remove CCP and generic VaR fallback literals where registry data exists.
- Move XVA defaults and accounting policy defaults into registries.
- Add overlay tests for margin/XVA/ECL/CECL.

### PR 9 - Binding Default Centralization

- Replace Python/WASM duplicate literals with Rust default config calls.
- Update `.pyi`, TypeScript declarations, parity contract, and docs.
- Add parity tests that Rust/Python/WASM defaults agree.

### PR 10 - Documentation and Release Hygiene

- Document all registry extension keys and override examples.
- Add migration notes.
- Run full formatting, linting, Rust tests, Python build/parity, and WASM contract tests.

## 10. Testing Strategy

- Golden-default tests: each migrated constructor must equal the pre-migration literal behavior.
- Registry parse tests: embedded registries load and validate in every owning crate.
- Overlay tests: overriding one value changes only that value and leaves unrelated defaults intact.
- Missing-id tests: missing registry ids produce clear errors.
- Binding parity tests: Python and WASM defaults match Rust registry-backed defaults.
- Serialization tests: registry records round-trip and preserve deterministic order.
- Negative tests: invalid probabilities, invalid dates, missing defaults, invalid correlation matrices, and duplicate aliases fail validation.

## 11. Acceptance Criteria

- No hard-coded external-assumption literal from the audit remains outside an approved registry, named model definition, or documented allowlist.
- All current default constructors either read registry defaults or delegate to one canonical Rust default object.
- Python and WASM contain no duplicated default arrays/scalars for migrated settings.
- Existing JSON registries are not duplicated across crates unless there is a documented packaging reason.
- Current behavior is preserved by default and verified with golden tests.
- Every registry has schema/version metadata, source metadata, and overlay support through `FinstackConfig.extensions`.

## 12. Initial File Checklist

Highest-priority source files to change during implementation:

- `finstack/core/src/types/ratings.rs`
- `finstack/core/src/credit/lgd/seniority.rs`
- `finstack/core/src/credit/pd/master_scale.rs`
- `finstack/core/src/credit/lgd/downturn.rs`
- `finstack/core/src/credit/lgd/workout.rs`
- `finstack/statements-analytics/src/extensions/scorecards/mod.rs`
- `finstack/valuations/src/instruments/fixed_income/structured_credit/types/constants.rs`
- `finstack/valuations/src/instruments/fixed_income/structured_credit/types/setup.rs`
- `finstack/valuations/src/instruments/fixed_income/structured_credit/types/constructors.rs`
- `finstack/valuations/src/instruments/fixed_income/structured_credit/pricing/stochastic/calibrations.rs`
- `finstack/cashflows/src/builder/specs/default.rs`
- `finstack/cashflows/src/builder/specs/prepayment.rs`
- `finstack/valuations/src/instruments/fixed_income/mbs_passthrough/types.rs`
- `finstack/valuations/src/instruments/fixed_income/mbs_passthrough/servicing.rs`
- `finstack/valuations/src/instruments/fixed_income/tba/pricer.rs`
- `finstack/valuations/src/instruments/fixed_income/tba/allocation.rs`
- `finstack/valuations/src/instruments/common/parameters/conventions.rs`
- `finstack/valuations/src/instruments/rates/irs/cashflow.rs`
- `finstack/valuations/src/instruments/rates/ir_future/types.rs`
- `finstack/valuations/src/instruments/fixed_income/bond_future/types.rs`
- `finstack/valuations/src/instruments/equity/equity_index_future/types.rs`
- `finstack/valuations/src/instruments/equity/vol_index_future/types.rs`
- `finstack/valuations/src/instruments/equity/vol_index_option/types.rs`
- `finstack/margin/src/calculators/im/clearing.rs`
- `finstack/margin/src/metrics/instrument.rs`
- `finstack/margin/src/xva/types.rs`
- `finstack/statements-analytics/src/analysis/ecl/staging.rs`
- `finstack/statements-analytics/src/analysis/ecl/engine.rs`
- `finstack/statements-analytics/src/analysis/ecl/cecl.rs`
- `finstack-py/src/bindings/**`
- `finstack-wasm/src/api/**`

## 13. Open Questions

- Should rating scales live in `core` or remain owned by `statements-analytics` with re-exported loaders?
- Should published scoring models be JSON-governed now, or is named-constant cleanup enough for this release?
- Should market-convention registries become one cross-crate registry or remain owned by `valuations`?
- Should default constructor behavior become fallible everywhere registry data is involved, or should compatibility constructors continue to panic only on invalid embedded assets?
- Which structured-credit profiles are official defaults versus examples? This must be decided before changing any inconsistent CDR/CPR/recovery values.
