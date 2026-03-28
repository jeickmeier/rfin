# Instrument Cashflow Unification Design

**Status:** Approved for implementation planning

**Date:** 2026-03-28

**Scope:** `finstack/cashflows`, `finstack/valuations`, `finstack/portfolio`, Python bindings, WASM bindings

---

## Summary

Finstack should expose one mandatory cashflow pathway for every instrument.

Every concrete instrument will emit a `CashFlowSchedule`, even when that schedule is empty. Empty schedules are valid and meaningful; they must carry explicit metadata describing whether the absence of flows means:

- the instrument has no residual dated cashflows under the current market mechanics, or
- the library intentionally has no accepted waterfall policy yet and is returning a placeholder.

Portfolio cashflow waterfalls will consume only this single schedule pathway. The existing optional `Instrument::as_cashflow_provider()` bridge will be removed.

Pricing remains authoritative and separate. The unified schedule API is the canonical dated-cashflow surface for:

- portfolio cashflow waterfalls
- scenario time-roll carry collection
- theta carry collection
- periodized PV helpers
- any future reporting/export layer

It is **not** required to be a full symbolic decomposition of every stochastic pricing term for every contingent product.

---

## Problem Statement

The current design has four core problems:

1. **Cashflow extraction is optional.**
   `Instrument::as_cashflow_provider()` defaults to `None`, so some instruments participate in portfolio waterfalls and others do not.

2. **The portfolio layer has parallel pathways.**
   `aggregate_full_cashflows()` and `aggregate_cashflows()` are not just different views of the same source. The simpler path can silently drop unsupported instruments and can bypass instrument-specific holder-view filtering semantics.

3. **“Supported” does not have one meaning.**
   Some products emit real schedules, some emit empty schedules, and some emit synthetic zero-amount placeholder events. This is hard to reason about and produces fragile downstream logic.

4. **The API surface is internally named rather than externally semantic.**
   `build_full_schedule()` and `build_dated_flows()` describe implementation mechanics rather than the user-visible concept. The bridge pattern adds another layer of indirection without adding useful semantics.

This makes it difficult to build a trustworthy portfolio waterfall API and obscures which instruments are intentionally empty vs simply unsupported.

---

## Goals

- Provide **one obvious way** to obtain an instrument cashflow schedule.
- Make cashflow emission a **universal instrument capability**, not an optional bridge.
- Ensure **all instruments participate** in portfolio cashflow waterfalls.
- Preserve a clean distinction between:
  - non-empty schedules
  - intentionally empty schedules
  - placeholder-empty schedules
- Make the portfolio cashflow API consume a **single canonical schedule source**.
- Remove silent omission of unsupported instruments from cashflow ladders.
- Keep pricing logic correct and authoritative while simplifying the reporting surface.
- Avoid fake zero-amount maturity events.

---

## Non-Goals

- Introduce probabilistic expected-payoff cashflows for options in this refactor.
- Force every pricer to internally use the unified schedule API.
- Solve the separate question of forward-FX conversion for foreign-currency waterfalls.
- Introduce multiple new public cashflow surfaces now (for example, separate “contractual” and “pricing-complete” APIs).

---

## Design Principles

- **One obvious way:** one canonical instrument schedule method, one canonical portfolio waterfall builder.
- **Public simplicity, private complexity:** shared helpers are fine internally; the public API should be small and explicit.
- **Bias to deletion:** remove the bridge instead of layering more behavior on it.
- **Empty is a first-class result:** empty schedules are valid outcomes, not failures.
- **No fake events:** never represent “no schedule” using synthetic zero-cash placeholder flows.
- **Future-only semantics:** the schedule surface should only expose flows strictly after `as_of`.
- **Holder-view semantics:** schedule amounts are from the long holder perspective.
- **Pricing remains authoritative:** schedule extraction is a waterfall/reporting surface, not a promise that every stochastic pricing term has been projected into dated cash payments.

---

## Canonical Instrument API

The singular pathway should be:

`instrument -> cashflow_schedule(market, as_of) -> portfolio waterfall aggregation -> optional FX collapse -> optional bucketing`

### Proposed Trait Shape

```rust
pub trait CashflowProvider: Send + Sync {
    fn cashflow_schedule(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<CashFlowSchedule>;

    fn dated_cashflows(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        let schedule = self.cashflow_schedule(market, as_of)?;
        Ok(schedule
            .flows
            .iter()
            .map(|cf| (cf.date, cf.amount))
            .collect())
    }
}

pub trait Instrument: Send + Sync + CashflowProvider {
    // existing pricing, metadata, dependency, and cloning methods
}
```

### Breaking Changes

- Rename `build_full_schedule()` to `cashflow_schedule()`.
- Rename `build_dated_flows()` to `dated_cashflows()`.
- Remove `Instrument::as_cashflow_provider()`.
- Make `Instrument` extend `CashflowProvider`.
- Add the cashflow methods to the public lean trait in `finstack/valuations/src/instruments/public_traits.rs`.

### Why this is the right simplification

- No trait-object bridge is needed because every instrument now has the capability.
- Generic consumers can directly depend on `Instrument`.
- Future refinements in schedule policy do not require more public entry points.

---

## Schedule Semantics

`cashflow_schedule(market, as_of)` has these mandatory semantics:

1. **Future-only**
   - Only flows with `date > as_of` may be returned.
   - Historical and same-day flows are excluded from this surface.

2. **Holder-view sign convention**
   - Positive = cash received by a long holder.
   - Negative = cash paid by a long holder.

3. **Undiscounted dated cash amounts**
   - Amounts represent payment-date cash, not PV.

4. **Sorted and stable**
   - Flows must be sorted by ascending date.

5. **Classified where non-empty**
   - Real flows must preserve `CFKind`.

6. **No synthetic zero placeholders**
   - If there is no meaningful schedule under current policy, return an empty schedule with metadata.

`dated_cashflows()` is a derived convenience view only. The schedule is the canonical source.

---

## Schedule Metadata

`CashFlowMeta` should carry one new required field:

```rust
pub enum CashflowRepresentation {
    Contractual,
    Projected,
    Placeholder,
    NoResidual,
}
```

### Representation meanings

- `Contractual`
  - Fixed or contractually scheduled future cash amounts.
  - Examples: bonds, term loans, repo, deposits, fixed swap legs.

- `Projected`
  - Current-market or model-projected future dated cash amounts under the library’s chosen waterfall policy.
  - Examples: floating coupons, NDF settlement, real-estate DCF flows, structured-credit tranche flows.

- `Placeholder`
  - Intentionally empty because the library has not yet chosen an accepted schedule policy for this contingent payoff class.
  - Examples: options and similar contingent products.

- `NoResidual`
  - Intentionally empty because there are no residual future dated cashflows under the instrument’s market mechanics.
  - Examples: spot equity, daily-margined futures.

### Public distinction

`Placeholder` and `NoResidual` should stay distinct in public metadata.

That does **not** add another public API method or a second cashflow surface. It only makes empty schedules explainable downstream.

This is especially important for portfolio waterfalls because an empty result can otherwise mean two very different things:

- “nothing remains to be paid”
- “we do not model this payoff as a dated waterfall yet”

---

## Instrument Classification Policy

The schedule API is the canonical **waterfall surface**, not a promise of a full symbolic pricing decomposition for every stochastic product.

### `Contractual`

These instruments should emit non-empty contractual schedules when future flows exist:

- `Bond`
- `InflationLinkedBond`
- `ConvertibleBond`
- `TermLoan`
- `RevolvingCredit`
- `Repo`
- `Deposit`
- `ForwardRateAgreement`
- `InterestRateSwap`
- `BasisSwap`
- `XccySwap`
- `InflationSwap`
- `YoYInflationSwap`
- `CmsSwap`
- `FxSpot` when settlement is still future-dated
- `FxForward`
- `FxSwap`
- `CommodityForward`
- `CommoditySwap`
- `DollarRoll`
- `AgencyTba`
- `AgencyMbsPassthrough`
- `AgencyCmo`
- `BondFuture` if there is a dated invoice/delivery settlement policy
- `PrivateMarketsFund`

### `Projected`

These instruments should emit projected schedules when the library already has a coherent dated-cashflow policy:

- `Ndf`
- `StructuredCredit`
- `FIIndexTotalReturnSwap`
- `EquityTotalReturnSwap`
- `CreditDefaultSwap` premium/upfront waterfall policy
- `CDSIndex` premium/upfront waterfall policy
- `CDSTranche` premium/upfront waterfall policy
- `DiscountedCashFlow`
- `RealEstateAsset`
- `LeveredRealEstateEquity`

### `NoResidual`

These instruments should emit empty schedules with `NoResidual`:

- `Equity`
- `InterestRateFuture`
- `EquityIndexFuture`
- `VolatilityIndexFuture`

### `Placeholder`

These instruments should emit empty schedules with `Placeholder` until an explicit expected-payoff policy is chosen:

- `Swaption`
- `BermudanSwaption`
- `CmsOption`
- `InterestRateOption`
- `InflationCapFloor`
- `RangeAccrual`
- `CDSOption`
- `EquityOption`
- `FxOption`
- `FxDigitalOption`
- `FxBarrierOption`
- `FxTouchOption`
- `QuantoOption`
- `VolatilityIndexOption`
- `IrFutureOption`
- `VarianceSwap`
- `FxVarianceSwap`
- `BarrierOption`
- `AsianOption`
- `LookbackOption`
- `Basket`
- `CliquetOption`
- `Autocallable`
- `CommodityOption`
- `CommoditySwaption`
- `CommoditySpreadOption`
- `CommodityAsianOption`

This policy keeps the public surface simple while leaving room to later evolve specific products from `Placeholder` to `Projected` without another API redesign.

---

## Portfolio-Level Design

The portfolio layer should also have one canonical pathway.

### Canonical Builder

Replace the current duality of:

- `aggregate_full_cashflows()`
- `aggregate_cashflows()`

with one canonical builder, conceptually:

```rust
pub fn build_portfolio_cashflow_waterfall(
    portfolio: &Portfolio,
    market: &MarketContext,
) -> Result<PortfolioCashflowWaterfall>
```

### Canonical Waterfall Output

The canonical portfolio output should preserve:

- all scaled events
- per-position event drill-down
- by-date / by-currency / by-`CFKind` aggregation
- per-position summary metadata, including representation
- build failures

Suggested additional type:

```rust
pub struct PortfolioCashflowPositionSummary {
    pub position_id: PositionId,
    pub instrument_id: String,
    pub instrument_type: String,
    pub representation: CashflowRepresentation,
    pub event_count: usize,
}
```

This is required so empty schedules still remain visible in portfolio waterfalls. A placeholder option should not vanish simply because it contributes zero events.

### Issues

After the refactor, portfolio issues should only represent **schedule construction failures**.

The current `Unsupported` issue kind should be removed because there will no longer be unsupported instruments.

### Derived Views

Any simpler view should be derived from the canonical waterfall object rather than rebuilding schedules.

That includes:

- date/currency ladders
- base-currency collapse
- period bucketing
- `CFKind`-bucketed period views

---

## Pricing Compatibility

This refactor should **not** redefine the pricing architecture.

### Invariants

- `value()` and `price_with_metrics()` remain the authoritative pricing entry points.
- Pricers may continue using bespoke internal logic where appropriate.
- `cashflow_schedule()` should stay aligned with existing waterfall/reporting policy for a product family, but it is not required to encode every stochastic pricing term as an explicit dated event.

### Examples

- A bond schedule is a direct pricing and reporting schedule.
- A floating-rate note schedule is projected from current market inputs and still suitable for waterfall reporting.
- An option remains correctly priced even when its schedule is an empty placeholder.

This is the key simplification: pricing correctness does not depend on prematurely forcing every contingent model into an expected-cashflow schedule.

---

## Zero-Amount Placeholder Events

Current zero-dollar maturity placeholders should be removed.

They are actively misleading because they look like real events while conveying no economic amount.

Specifically:

- `VarianceSwap`
- `FxVarianceSwap`

should return empty placeholder schedules instead of synthetic zero fixed flows.

---

## API Naming

Preferred public naming:

- `cashflow_schedule()` for the canonical schedule
- `dated_cashflows()` for the convenience flattened view
- `build_portfolio_cashflow_waterfall()` for the portfolio canonical aggregator

Avoid `build_*` naming at the public API layer for this surface because it emphasizes internal construction mechanics rather than stable semantics.

---

## Breaking Changes

This design intentionally allows breaking changes because the library is pre-alpha.

Expected breaks:

- trait signatures and trait bounds
- removal of `as_cashflow_provider()`
- method renames (`build_full_schedule` / `build_dated_flows`)
- portfolio type/function renames
- portfolio issue model changes
- test rewrites for universal coverage
- binding API changes where old names are exposed

---

## Risks and Mitigations

### Risk: forcing all instruments into one schedule surface degrades pricing accuracy

**Mitigation:** the schedule surface is explicitly a waterfall/reporting contract. Pricing remains authoritative and independent.

### Risk: empty schedules become ambiguous

**Mitigation:** require `CashflowRepresentation` in `CashFlowMeta` and preserve position summaries in the portfolio waterfall output.

### Risk: contingent credit or exotic products are oversimplified

**Mitigation:** classify them as `Placeholder` until a specific expected-payoff policy is explicitly designed and approved.

### Risk: dual APIs linger after the refactor

**Mitigation:** delete the bridge, make all simpler portfolio views derived, and document one canonical schedule method everywhere.

---

## Final Design Decision

The library should adopt this singular pathway:

1. Every `Instrument` must implement `CashflowProvider`.
2. Every instrument must return a `CashFlowSchedule`.
3. Empty schedules are valid and explicitly tagged.
4. Options and other unresolved contingent payoff products return empty `Placeholder` schedules.
5. True no-residual products return empty `NoResidual` schedules.
6. The portfolio layer consumes only this schedule path and builds one canonical waterfall object.

This is the smallest design that gives:

- universal instrument participation
- clear semantics for empty results
- a clean portfolio waterfall API
- room to later add projected contingent-payoff policies without redesigning the public surface
