# PRD: CDS Family ISDA Compliance Remediation

## Document Status

- Status: Draft
- Scope: `finstack/valuations` credit-derivatives stack, schemas, market-build flow, calibration, tests, and docs
- Products in scope:
  - Single-name CDS
  - CDS Index
  - CDS Tranche
  - CDS Option
  - Hazard rate curve construction and calibration
- Primary code paths:
  - `finstack/valuations/src/instruments/credit_derivatives/cds/*`
  - `finstack/valuations/src/instruments/credit_derivatives/cds_index/*`
  - `finstack/valuations/src/instruments/credit_derivatives/cds_tranche/*`
  - `finstack/valuations/src/instruments/credit_derivatives/cds_option/*`
  - `finstack/valuations/src/calibration/targets/hazard.rs`
  - `finstack/valuations/src/calibration/solver/bootstrap.rs`
  - `finstack/core/src/market_data/term_structures/hazard_curve.rs`
  - `finstack/valuations/src/market/build/cds.rs`

## Executive Summary

This review concludes that the current CDS stack is only partially ISDA compliant.

What is already directionally sound:

- `HazardCurve` is piecewise-flat and produces monotone survival probabilities.
- `HazardCurveTarget` plus `SequentialBootstrapper` is the strongest existing calibration path.
- Single-name CDS is the closest product to a recognizable ISDA-style implementation.

What prevents a full ISDA compliance claim today:

1. Trade lifecycle semantics are incomplete. The implementation does not model `step_in_date`, `cash_settlement_date`, or front-end protection explicitly across the CDS family.
2. Single-name CDS still has material valuation defects:
   - accrual-on-default is discounted to coupon payment date instead of default settlement timing;
   - `IsdaStandardModel` is still grid-based rather than exact on actual breakpoint intervals.
3. Quote normalization is inconsistent across single-name CDS, index CDS, and tranche products, especially for upfront, clean-vs-dirty settlement cash, and standard-coupon enforcement.
4. CDS tranche scheduling is not contract-anchored when `effective_date` is absent, which can restart seasoned trades at `as_of`.
5. CDS option exposes settlement and exercise fields that are not implemented in pricing, so the public surface overstates capability.
6. Validation and parity coverage are too weak to certify compliance. The current parity suite does not contain official or frozen vendor-style lifecycle fixtures.

This PRD expands the original single-name remediation effort into a full CDS-family compliance program. The target outcome is not "mostly ISDA-like"; it is a codebase that either:

- implements the relevant standard conventions and algorithms correctly, or
- narrows the public product surface so unsupported behaviors are rejected rather than implied.

## Compliance Verdict

| Area | Current State | Verdict | Main Gaps |
|------|---------------|---------|-----------|
| Single-name CDS | Closest to ISDA-style | Partial | AoD timing, analytic integration, lifecycle semantics |
| CDS Index | Tradable, but convention surface drifts | Partial | calendar defaults, settlement lifecycle, quote normalization |
| CDS Tranche | Market-style copula engine present | Partial | contract anchoring, settlement/accrued handling, schema/example drift |
| CDS Option | Black-on-spread engine present | Not compliant as exposed | exercise/settlement fields not implemented, product scope ambiguous |
| Hazard curve representation | Sound core design | Partial | canonical path not fully enforced, duplicate bootstrap logic |
| Hazard curve calibration | Mostly robust | Partial | parity/fixture quality, quote validation, helper drift |

## Review Findings

### Blockers

#### 1. CDS lifecycle dates are not modeled in an ISDA-separable way

Evidence:

- `CDSConvention::settlement_delay()` currently represents trade-date-to-settlement only.
- `ProtectionLegSpec::settlement_delay` is reused for valuation logic that should distinguish trade settlement from default settlement.
- No explicit CDS-family support exists for:
  - `trade_date`
  - `step_in_date`
  - `cash_settlement_date`
  - front-end protection flag

Impact:

- Clean and dirty upfront settlement cannot be modeled correctly.
- Default between trade date and step-in date cannot be handled explicitly.
- Product-specific differences between single-name, index, and tranche settlement flows are blurred.

Required fix:

- Introduce an explicit lifecycle model shared across CDS-family instruments or pricing context:
  - `trade_date`
  - `step_in_date`
  - `cash_settlement_date`
  - `default_settlement_lag`
  - `front_end_protection`
- Use this lifecycle model consistently in premium leg, protection leg, upfront settlement, and option expiry logic.

#### 2. Accrual-on-default timing is economically wrong in production single-name CDS pricing

Evidence:

- `CDSPricer::pv_premium_leg_raw()` uses `accrual_on_default_isda_midpoint()`.
- `accrual_on_default_isda_midpoint()` discounts accrued premium to the scheduled coupon `payment_date`, not default settlement timing.

Impact:

- Premium leg PV is understated.
- Full-premium par spread denominator is understated.
- Par spreads are biased high.
- Distortion grows with distress, longer accrual periods, and settlement lag.

Required fix:

- Replace coupon-date discounting with discounting to default settlement date.
- If midpoint approximation is retained, midpoint must determine both:
  - accrued fraction within the coupon period;
  - settlement discount date.
- Reuse the same corrected AoD logic in:
  - premium leg PV,
  - full-premium par spread denominator,
  - PV-per-bp.

#### 3. `IsdaStandardModel` is mislabeled today

Evidence:

- `protection_leg_isda_standard_model_cond()` still integrates on equal `dt` slices driven by step controls.
- The path is documented as analytical over piecewise-constant inputs but remains grid-dependent.

Impact:

- Results depend on tuning parameters.
- The method name and docs overstate compliance.
- Piecewise-flat curves are not exploited exactly.

Required fix:

- Build an integration grid from the union of:
  - hazard-curve breakpoints,
  - discount-curve breakpoints relevant to settlement discounting,
  - protection interval boundaries,
  - settlement-date transition boundaries.
- Integrate analytically on each locally constant interval.
- Remove step-size dependence from the `IsdaStandardModel` path.
- If exact interval integration is not delivered, rename the method to an explicitly approximate label.

#### 4. CDS option exposes unsupported behavior

Evidence:

- `CDSOption` stores `exercise_style` and `settlement`.
- `CDSOption::new()` hardcodes `ExerciseStyle::European` and `SettlementType::Cash`.
- `CDSOptionPricer` does not branch on `exercise_style` or `settlement`.

Impact:

- The schema and API imply capabilities that do not exist.
- Users can reasonably infer physical settlement or broader exercise support even though pricing does not implement them.

Required fix:

- Choose one of two paths:
  - narrow the product definition to "European, cash-settled Black-on-spread CDS option only" and reject any unsupported values;
  - or implement settlement-style and exercise-style behavior explicitly.
- If index-option support remains approximate, document that it is a simplified index-option model rather than full market-standard index-option coverage.

#### 5. Standard tranche pricing can restart the contract at valuation date

Evidence:

- `cds_tranche/pricer.rs` uses `effective_date.unwrap_or(as_of)` for schedule generation and first-period accrual.
- `CDSTranche::standard()` leaves `effective_date` as `None`.

Impact:

- Seasoned tranche valuation can lose contractual history.
- Accrued premium and dirty upfront can be miscomputed.
- Standardized tranche contracts are not anchored to their actual roll/effective dates.

Required fix:

- Make contractual start/roll anchoring mandatory for standard tranches.
- Separate contract effective date from valuation settlement date.
- Compute accrued premium from the previous coupon date on the actual schedule.

### Major Findings

#### 6. Runtime conventions, schemas, and examples drift from intended market defaults

Evidence:

- Example schemas for single-name CDS, index CDS, and tranche show `bdc: "following"` and `calendar_id: null`.
- Runtime docs and constructors otherwise imply Modified Following plus regional calendars.
- Index examples do not consistently inherit default regional calendars.

Impact:

- Users copying examples instantiate non-standard conventions.
- Schema documentation undermines the runtime compliance story.

Required fix:

- Update schema examples to match real constructors and market defaults.
- Populate `calendar_id` from convention defaults where applicable.
- Keep Modified Following where that is the supported standard.

#### 7. Tranche schema example uses the wrong attachment/detachment units

Evidence:

- Tranche code uses percentage points, e.g. `3.0` for 3%.
- `cds_tranche.schema.json` example uses `detach_pct: 0.03`.

Impact:

- Example payload economically describes 0.03% instead of 3%.
- Users can create materially wrong tranches from schema examples.

Required fix:

- Change example values to percentage-point units.
- Document units explicitly in schema descriptions.
- Add example roundtrip tests that assert the deserialized economic meaning.

#### 8. Quote support is inconsistent across CDS products

Evidence:

- Single-name CDS supports both dated upfront cashflows and raw PV-level overrides.
- Index CDS supports raw PV-level adjustments.
- Tranche uses a dated upfront concept.
- Shared quote override fields such as `quoted_spread_bp` and `quoted_clean_price` are not consistently exercised.

Impact:

- Standard-coupon + upfront workflows are not normalized consistently.
- Clean vs dirty quote semantics are ambiguous.
- Calibration and repricing paths may disagree across product families.

Required fix:

- Define canonical quote objects and settlement decomposition by product:
  - single-name/index CDS: standard coupon + upfront + accrued settlement cash;
  - tranche: running coupon + upfront + accrued settlement cash;
  - option: premium and/or implied vol quote.
- Normalize all quote forms into a single internal economic representation before pricing.

#### 9. `build_cds_instrument()` weakens convention traceability

Evidence:

- Market build flow converts quotes into `CDSConvention::Custom`.
- `doc_clause` is dropped.
- The instrument no longer carries the original convention identity cleanly.

Impact:

- Convention-specific behavior is harder to preserve and audit.
- Restructuring clause semantics and lifecycle defaults are less transparent.

Required fix:

- Preserve convention and doc-clause metadata on the built instrument.
- Only use bespoke/custom mode when the quote actually requests non-standard behavior.

#### 10. Duplicate bootstrap logic drifts from the canonical calibration path

Evidence:

- `HazardCurveTarget` plus `SequentialBootstrapper` sets day count and builds calibrated curves consistently.
- `CDSBootstrapper::bootstrap_hazard_curve()` in `cds/pricer.rs` does not appear to be the strongest path and lacks equivalent guardrails and coverage.

Impact:

- Multiple bootstrap paths can diverge operationally.
- Day-count and pillar handling can drift.
- Future bug fixes may land in one path but not the other.

Required fix:

- Make `HazardCurveTarget` plus `SequentialBootstrapper` the only canonical CDS hazard bootstrap path.
- Remove, deprecate, or internally delegate any duplicate helper bootstrap logic.

#### 11. Current parity tests are not strong enough to certify ISDA compliance

Evidence:

- Existing parity tests are helpful but do not look like a complete frozen set of official lifecycle fixtures.
- Missing coverage for:
  - step-in date behavior,
  - clean-vs-dirty upfront settlement,
  - holiday-adjusted settlement dates,
  - front-end protection,
  - default-between-trade-and-settlement cases,
  - option settlement semantics,
  - tranche seasoned-trade accrual.

Impact:

- "passes tests" is not the same as "meets market standard".

Required fix:

- Add frozen vendor-style or official-reference fixtures and treat them as release gates for ISDA compliance claims.

## Goals

- Make single-name CDS valuation conventionally correct for standard ISDA workflows.
- Make the CDS-family product surface honest: implemented behaviors are supported, unsupported behaviors are rejected.
- Normalize lifecycle semantics across single-name, index, tranche, and option products.
- Use one canonical hazard-curve calibration path.
- Add enough regression and parity coverage that "ISDA compliant" is a test-backed statement.

## Non-Goals

- No redesign of the underlying tranche copula model beyond lifecycle, scheduling, settlement, and documentation/compliance issues.
- No stochastic recovery redesign.
- No attempt to make bespoke exotic credit structures fully standardized in this work.
- No barrier, Asian, or non-credit derivative cleanup.

## Proposed Solution

## Workstream 1: Introduce Explicit CDS Lifecycle Semantics

### Requirement

All CDS-family pricing must distinguish contract lifecycle dates and settlement roles explicitly.

### Deliverables

- Add lifecycle fields or pricing-context equivalents for:
  - `trade_date`
  - `step_in_date`
  - `cash_settlement_date`
  - `default_settlement_lag`
  - `front_end_protection`
- Separate:
  - scheduled premium payment timing,
  - trade settlement timing,
  - default settlement timing.

### Acceptance Criteria

- Single-name CDS, index CDS, and tranche all price with the same lifecycle vocabulary.
- Upfront settlement can be expressed as clean and dirty cash.
- Tests cover roll-date trades, holiday-adjusted settlement, and default between trade and step-in.

## Workstream 2: Fix Single-Name CDS Valuation Defects

### Requirement

Single-name CDS must be defensible as an ISDA-style reference implementation.

### Deliverables

- Correct AoD discount timing to default settlement.
- Replace grid-dependent `IsdaStandardModel` protection integration with exact breakpoint-based interval integration.
- Ensure premium leg, risky annuity, par spread, PV-per-bp, and protection leg remain internally consistent.

### Acceptance Criteria

- `IsdaStandardModel` does not depend on `steps_per_year` or `adaptive_steps`.
- Flat and piecewise benchmark cases are stable within tight tolerance.
- Regression tests fail under the old coupon-date AoD bug.

## Workstream 3: Normalize Standard Quote Handling

### Requirement

Standard contracts and quotes must be represented consistently across the CDS family.

### Deliverables

- Define canonical quote normalization for:
  - par spread quotes,
  - standard-coupon upfront quotes,
  - clean vs dirty settlement cash.
- Enforce standard coupon validation for standard CDS quote modes.
- Preserve convention and doc-clause identity through market-build flows.

### Acceptance Criteria

- Valid standard quotes roundtrip into priced instruments without losing convention identity.
- Non-standard coupons are either rejected or routed through an explicit bespoke path.
- Single-name CDS and index CDS use the same clean/dirty settlement semantics.

## Workstream 4: Make CDS Index and Tranche Scheduling Contract-Anchored

### Requirement

Index and tranche pricing must be anchored to contractual roll/effective dates, not valuation-date fallbacks.

### Deliverables

- Require or derive standard contract effective dates for standard index/tranche products.
- Compute accrued premium and dirty upfront from the actual previous coupon date.
- Align settlement lag defaults and holiday handling with the supported standard product definitions.

### Acceptance Criteria

- Seasoned index/tranche trades do not restart at `as_of`.
- Accrued-premium tests pass before and after coupon dates.
- Holiday-adjusted settlement tests match documented convention defaults.

## Workstream 5: Narrow or Complete CDS Option Product Surface

### Requirement

The option API must not promise behavior that the pricer does not implement.

### Deliverables

- Preferred near-term path:
  - explicitly define the supported product as European, cash-settled CDS spread option;
  - reject unsupported settlement/exercise combinations during validation;
  - document index-option support as simplified if it remains Black-on-adjusted-forward only.
- Alternative long-term path:
  - implement physical settlement, settlement-style branching, and default-before-expiry rules.

### Acceptance Criteria

- Schema, constructors, validation, and pricer all agree on the supported product surface.
- Unsupported option configurations fail fast.
- Tests cover default-before-expiry, index-option approximation behavior, and supported quote conventions.

## Workstream 6: Canonicalize Hazard Curve Calibration

### Requirement

There must be one authoritative CDS hazard bootstrap/calibration path.

### Deliverables

- Keep `HazardCurveTarget` plus `SequentialBootstrapper` as the canonical path.
- Remove or delegate duplicate local bootstrap helpers.
- Tighten quote validation for upfront quotes, coupon validity, and zero-spread edge cases where appropriate.
- Ensure calibrated curve metadata preserves convention day count and pillar meaning.

### Acceptance Criteria

- All CDS curve-building flows route through the canonical calibration path.
- Input quotes reprice to near zero within stated tolerance.
- The calibrated curve day count matches the resolved market convention.

## Workstream 7: Repair Schemas, Examples, Documentation, and Tests

### Requirement

Documentation and examples must not contradict supported market behavior.

### Deliverables

- Update schema examples for single-name CDS, index CDS, tranche, and option.
- Fix tranche percentage-point example units.
- Update local READMEs and product docs so compliance claims match implementation reality.
- Replace weak parity claims with frozen benchmark fixtures and explicit tolerances.

### Acceptance Criteria

- Examples reflect actual supported defaults.
- No schema example implies unsupported product behavior.
- A compliance claim is gated on benchmark fixtures, not only property tests.

## Validation Matrix

### Single-name CDS

- AoD uses default settlement discount timing.
- Standard coupon + upfront quotes reprice cleanly and dirty cash is explained.
- Step-in and settlement-date tests pass across NA, EU, and Asia defaults.
- Breakpoint-based integration matches flat and piecewise expected values.

### CDS Index

- Standard index constructors populate regional calendar and business-day defaults correctly.
- Clean vs dirty settlement cash is explicit.
- Convention identity survives market build and valuation.

### CDS Tranche

- Contract schedule remains anchored for seasoned trades.
- Accrued premium and dirty upfront are computed off the historical coupon schedule.
- Standard and bespoke settlement defaults are explicit and test-backed.

### CDS Option

- Supported settlement and exercise combinations are explicitly enforced.
- Default-before-expiry behavior is specified.
- Index-option approximation is either implemented correctly enough for its stated scope or rejected.

### Hazard Curve and Calibration

- Canonical bootstrap reprices market quotes within documented tolerance.
- Knot times match actual pillar economics.
- Duplicate bootstrap helpers are removed or delegated.

## Product Requirements

### Functional Requirements

- No CDS-family API may claim ISDA compliance unless the corresponding lifecycle and quote semantics are implemented or rejected explicitly.
- `CDSPricerConfig::isda_standard()` must select a path that is actually consistent with its naming.
- CDS-family schemas must only advertise supported product behavior.
- Calibration flows must use the canonical hazard-curve target/solver path.

### Non-Functional Requirements

- Exact interval integration must not depend on arbitrary tuning parameters.
- Runtime must remain acceptable for standard 5Y and 10Y single-name CDS workloads.
- Changes must remain deterministic and regression-testable.
- Public API churn should be minimized, but correctness takes precedence over preserving misleading fields or names.

### Documentation Requirements

- Update `cds/README.md` to reflect corrected AoD timing and integration behavior.
- Update product docs for index, tranche, and option so supported conventions are explicit.
- Add references for:
  - ISDA CDS Standard Model documentation,
  - standard CDS examples,
  - O'Kane for single-name and tranche conventions,
  - QuantLib / QuantExt style references where option or tranche market-standard behavior is approximated.

## Rollout Plan

### Phase 1: Correct Single-Name CDS Math and Lifecycle Primitives

- Implement lifecycle-date model.
- Fix AoD timing.
- Replace grid-based `IsdaStandardModel` interval integration.

### Phase 2: Normalize Standard Quotes and Calibration

- Unify clean/dirty quote handling.
- Canonicalize hazard calibration path.
- Preserve convention/doc-clause metadata in market-build flows.

### Phase 3: Repair Index, Tranche, and Option Product Surfaces

- Contract-anchor index and tranche schedules.
- Fix tranche schema units and convention defaults.
- Narrow or complete option settlement/exercise support.

### Phase 4: Benchmarking, Docs, and Release Gate

- Add frozen lifecycle fixtures.
- Tighten parity tolerances.
- Update READMEs, schemas, and user-facing examples.
- Do not describe the stack as fully ISDA compliant until this phase is complete.

## Risks

- Downstream users may rely on current incorrect AoD or settlement behavior.
- Fixing clean/dirty settlement semantics may change upfront-priced PVs and benchmark snapshots.
- Exact interval integration may expose existing tolerance assumptions in tests.
- Narrowing unsupported option behavior may be a breaking validation change for callers relying on the looser schema.

## Mitigations

- Land changes behind focused, product-level regression fixtures.
- Prefer explicit validation errors over silent fallback behavior.
- Preserve approximate methods only when clearly labeled as approximate.
- Publish before/after benchmark deltas for single-name CDS and tranche dirty-upfront cases.

## Open Questions

- Do we want explicit legacy toggles for old AoD timing or should the bug be fixed outright?
- For CDS options, is the immediate goal a narrow but honest product, or a broader implementation including physical settlement?
- Should standard coupon enforcement be mandatory in all standard quote builders, or configurable via an explicit bespoke override?
- Do we want a frozen external reference pack for NA, EU, and Asia conventions before claiming full release readiness?

## Release Gate

Do not label the CDS stack "fully ISDA compliant" until all of the following are true:

- Single-name CDS lifecycle semantics are explicit and tested.
- AoD timing uses default settlement timing.
- `IsdaStandardModel` is exact on breakpoint intervals or renamed.
- Standard quote normalization is consistent across single-name and index CDS.
- Tranche schedules are contract-anchored and accrued-premium behavior is regression-tested.
- CDS option surface is either narrowed honestly or fully implemented.
- Hazard curve calibration uses a single canonical path.
- Frozen reference fixtures cover lifecycle, settlement, and quote-conversion behavior.

## References

- ISDA CDS Standard Model documentation and standard CDS examples.
- ISDA Big Bang protocol and subsequent standard-contract documentation.
- O'Kane, *Modelling Single-name and Multi-name Credit Derivatives*.
- QuantLib / QuantExt reference implementations for CDS, CDS option, and index-option product surfaces.
- Existing local implementation notes in:
  - `finstack/valuations/src/instruments/credit_derivatives/cds/pricer.rs`
  - `finstack/valuations/src/instruments/credit_derivatives/cds/types.rs`
  - `finstack/valuations/src/instruments/credit_derivatives/cds_tranche/pricer.rs`
  - `finstack/valuations/src/instruments/credit_derivatives/cds_option/pricer.rs`
