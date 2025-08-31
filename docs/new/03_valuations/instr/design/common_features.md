# Instruments Module – Common Features Design Document

## 1 Overview
The **instr** module provides concrete financial instrument representations and their valuation adapters. It sits above the curves, cash-flows, and calibration layers and corresponds to TDD section **4.6** and PRD functional requirements C-09 – C-32.

Key design objectives:
* Uniform trait-based valuation (`Priced`) and risk extraction (`Risky`) interfaces.
* Composable with previously defined `CashFlow`, `MarketContext`, and `Schedule` types.
* Zero-unsafe public API, `no_std` compatible except where noted.
* Extensible class hierarchy enabling future exotic products without breaking ABI.

## 2 Goals & Non-Goals
### 2.1 Goals
1. Define **instrument structs** for Spot assets, Money-market contracts, Swaps, Caps/Floors, Swaptions, Bonds, and placeholder enums for deferred products.
2. Provide valuation adapters that map an instrument into primitive `CashFlow`s and call curves for NPV.
3. Support first-order risk – PV01 / DV01 / Delta – via optional risk feature.
4. Allow batch pricing through slice APIs that leverage Rayon when the `parallel` feature is enabled.

### 2.2 Non-Goals
* Analytical Greeks beyond PV01 (handled in the risk module).
* Exotic Monte-Carlo path-dependent pay-offs (deferred).
* Position/PnL management (belongs to the portfolio crate).

## 3 High-Level Traits & API
```rust
pub trait Priced {
    type PVOutput;
    fn pv<C: CurveProvider>(&self, curves: &C, val_date: Date) -> Result<Self::PVOutput, Error>;
}

pub trait CurveProvider {
    fn discount(&self, id: &str) -> Result<&dyn DiscountCurve, Error>;
    fn forward(&self, id: &str) -> Result<&dyn Curve, Error>;
    fn hazard(&self, id: &str) -> Option<&HazardCurve>;
    // … inflation, vol, etc.
}
```
All concrete instruments implement `Priced`. A blanket impl provides `pv_slice` for `&[impl Priced]`.

## 4 Module Layout
```text
src/instr/
  ├─ mod.rs             // facade re-exports
  ├─ common.rs          // shared enums & helper structs
  ├─ spot.rs            // Cash / SpotAsset (C-09)
  ├─ mm.rs              // Deposits, FRAs, Futures (C-10)
  ├─ swap.rs            // Fixed/Floating Swaps (C-23)
  ├─ capfloor.rs        // Caps/Floors & helper (C-24)
  ├─ swaption.rs        // Swaption instrument (uses curves::SwaptionSurface3D)
  ├─ bond.rs            // Fixed/Floating/Callable Bonds (C-25)
  ├─ builders/          // fluent builders for each category
  ├─ tests.rs
```

## 5 Analytics Helpers (non-Greek)
1. **Yield Metrics** (Bonds) – `yield_to_maturity`, `yield_to_call`, `yield_to_worst`, `spread_to_benchmark`, `oas`.
2. **Duration & Convexity** – Modified/Macaulay duration, key-rate tables, convexity.
3. **Price Clean vs Dirty** – `clean_price()`, `accrued_interest()`.
4. **Swap & Futures Carry/Roll** – `carry()`, `rolldown()`, `par_rate()`, basis break-even spread.
5. **Cap/Floor/Swaption Helpers** – `implied_vol()`, analytic vega, ATM vol helpers.
6. **Money-Market Accrual** – `carry()` convenience, forward DV01.
7. **Credit Instruments** – `par_spread()`, PV01, CS01, Jump-to-Default.
8. **FX Analytics** – forward points, delta conventions, vol surface helpers.
9. **Repo / SFT Helpers** – `effective_repo_rate()`, break-even repo.
10. **Commodity Forwards/Futures** – `implied_convenience_yield()`, roll yield.

API additions live in per-asset extension traits (e.g. `SwapAnalytics`, `FxAnalytics`, `RepoAnalytics`) built on the same pattern as `BondAnalytics`.

## 6 Common Functionality
1. **Builder Pattern** – Each instrument has a `Builder` fluent API enforcing mandatory fields at compile-time via the typestate pattern.
2. **Validation** – `validate()` ensures dates ordered, notional positive, etc., returning `Error::Input`.
3. **Serialization** – All instruments derive Serde with a version tag for audit trails.
4. **Currency Guard** – Instruments store `Currency`; PV output is always the same currency; multi-currency aggregation occurs upstream.
5. **Parallel Pricing** – Blanket `pv_slice_parallel()` uses Rayon when the `parallel` feature is enabled.

### 6.1 Additional Instrument Details
| Topic | Notes |
|-------|-------|
| **Floating Fixings** | `CashFlowLeg::needs_fixing(val_date)` returns list of resets not yet fixed; optional `FixingStore` trait provides past fixings. |
| **Instrument Metadata** | All structs embed `id: Option<String>` (ISIN or trade ID) and `tags: BTreeMap<String,String>` behind the `metadata` feature for audit trails. |
| **Leg Conventions** | Each leg carries its own `DayCount` and `BusDayConv`; defaults come from `Schedule` but can be overridden. |
| **Side Enum** | `enum Side { Pay, Rec }` stored per leg; PVs positive when `Rec`. |
| **Callable Bond Schedule** | Placeholder `CallSchedule` referenced by `CallableBond`; evaluation handled in the future greeks module. |
| **Serialization Versioning** | `#[serde(tag = "instr", version = 1)]` on every public instrument struct; additions use `#[serde(default)]`. |
| **Thread-Safety & Sharing** | Instruments are `Send + Sync`; large vectors wrapped in `Arc`. |
| **Error Handling** | Common validation errors map to `Error::Input`. |
| **Performance Scaling** | Target ≥10× speed-up on 16-core machine with `parallel` flag. |
| **Deferred Instruments** | `DeferredInstr` enum placeholder for v1.1+. |

## 7 Algorithms
1. Deposit/FRA PV formulas are analytic.
2. Swap fixed-leg PV uses cash-flow accrual factors from the dates module.
3. Caplet PV uses Black with a forward from curve and vol surface.
4. Swaption PV uses Black-76 with swap annuity from the discount curve.
5. Bonds use a yield-to-price root-find helper for clean-price conversions.

## 8 Feature Flags
* `parallel` – enable Rayon slice pricing.
* `private_credit` – exposes bond extensions C-12–C-17.
* `serde` – serialization derives.

## 9 Integration Points
* **dates & calendar** – schedule generation and business-day adjustment.
* **cashflow** – uses `CashFlow` & leg builders.
* **curves** – forward/discount curves and vol surfaces.
* **calibration** – instruments feed quotes into calibration tests.
* **risk** – DV01/risk metrics extracted via the `Risky` trait (future).

## 10 Testing Strategy
* Golden PVs vs QuantLib for swaps, caps, swaptions.
* Property tests: NPV swap parity fixed-vs-float, cap-floor parity.
* Criterion benches: price 10k swaps < 20 ms single-thread.
* Parallel slice pricing benchmark ensures ≥10× speed-up on 16 cores.

## 11 Timeline
* **v0.1.0** – SpotAsset, Deposits, FRAs, Futures.
* **v0.2.0** – Swaps, Caps/Floors.
* **v0.3.0** – Swaptions, Bonds.
* **v1.0.0** – Private-credit extensions, API freeze.

---
*Last updated: 2025-06-29*) 