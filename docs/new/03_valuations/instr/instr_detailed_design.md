# Instruments Module – Detailed Design Document

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
4. Allow batch pricing through slice APIs that leverage Rayon when `parallel` feature is enabled.

### 2.2 Non-Goals
* Analytical Greeks beyond PV01 (handled in risk module).
* Exotic Monte-Carlo path-dependent pay-offs (deferred).
* Position/PnL management (belongs to portfolio crate).

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
```
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

### 5 Analytics Helpers (non-Greek)
1. **Yield Metrics** (Bonds)
   * `yield_to_maturity(price)` solves YTM via Newton; `yield_to_call` uses first call date; `yield_to_worst` loops over call/redemption schedule; `spread_to_benchmark(curve)`; `oas(spread_guess)` iterative search adjusting discount curve.
2. **Duration & Convexity** (Bonds & Legs)
   * Modified / Macaulay duration, key-rate durations table, convexity.
3. **Price Clean vs Dirty** (Bonds)
   * `clean_price()` = PV – accrued; `accrued_interest()` via dates year-fraction.
4. **Swap & Futures Carry/Roll**
   * `carry(horizon_days)` and `rolldown(horizon_days)` based on forward-rate projection; `par_rate(start,end)` solver; basis breakeven spread function.
5. **Cap/Floor/Swaption Helpers**
   * `implied_vol(price)` inverse Black; analytic vega `vega(price)`; ATM vol extraction helpers.
6. **Money-Market Accrual** (Deposits, FRAs)  (C-11)
   * `carry(val_date, fwd_date)` convenience function; forward DV01.
7. **Credit Instruments** (CDS) – post-v1.1
   * `par_spread(pv)` solver; PV01, CS01, Jump-to-Default metrics.
8. **FX Analytics**
   * Forward points `fwd_points()`, premium- vs spot-delta calculation, RR/Strangle implied vols.
9. **Repo / SFT Helpers**
   * `effective_repo_rate()` given haircut & collateral price; break-even repo calculator.
10. **Commodity Forwards/Futures**
   * `implied_convenience_yield()`, roll yield to next contract.

API additions will live in per-asset extension traits (`SwapAnalytics`, `FxAnalytics`, `RepoAnalytics`, etc.) built on same pattern as `BondAnalytics`.

## 5 Instrument Specifications
### 5.1 SpotAsset
* Fields: `ccy: Currency`, `amount: F`, `settle_date: Date`.
* NPV: `amount * df(val_date→settle)`.

### 5.2 Money-Market Contracts (C-10)
| Type | Quote Inputs | Valuation |
|------|--------------|-----------|
| **Deposit** | rate, start, tenor | Single cash-flow principal+interest.
| **FRA** | fixed rate, start, end | Discounted forward rate vs fixed.
| **SOFR/Libor Futures** | price, delivery date | PV from implied forward rate; convexity adjustment optional.

### 5.3 Interest-Rate Swap (C-23)
* Legs: fixed and floating `CashFlowLeg`.
* Collateral CSA selects discount curve via `MarketContext`.
* PV = PV_fixed – PV_float.

### 5.4 Caps/Floors (C-24)
* Generated from floating leg periods plus Black's model using vol2D surface.
* Each caplet valued as option on forward rate; sum for PV.

### 5.5 Swaption
* Underlying swap definition + 3-D swaption vol surface.
* Black or SABR–implied vol selected from surface; PV via Black-76.

### 5.6 Bonds (C-25)
| Variant | Features |
|---------|----------|
| Fixed | level coupon cash-flow leg + principal repay |
| Floating | float leg coupons, reset lag | 
| Callable | embeds Bermudan option |

#### 5.7.1 Inflation-Linked Bonds (C-26)
* Struct `InflBond` embeds `index: String` (e.g., US CPI-U), `base_cpi`, and fixed coupon leg.  
* Cash-flows adjusted by ratio `CPI(t)/base_cpi`. PV uses inflation curve from `CurveProvider`.

#### 5.7.2 Inflation Swaps (C-27)
* ZC and YY variants.  
* Legs: fixed CPI vs floating inflation leg; valuation similar to nominal swap but using inflation DF.

#### 5.7.3 XCCY Basis Swaps (C-28)
* Two floating legs in different currencies; notional exchanges optional.  
* Discount with respective collateral curves; PV in reporting currency via FX spot / FX forward from curves module.

#### 5.7.4 FX Forwards & Options (C-29)
* `FxForward { notional_base, rate, settle }` valued via covered-interest parity.  
* `FxOption` Black-Scholes with `vol2D` surface (currency pair ID).  

#### 5.7.5 Credit Derivatives (C-30)
* `Cds { spread, maturity, pay_freq }` valued with hazard curve; accrual on default.  
* CDS option/tranche placeholder for v1.2.

#### 5.7.6 Equity Options (C-31)
* European & American support; PV via Black-Scholes (Euro) or binomial tree (Amer).  
* Uses equity vol surface from curves.

#### 5.7.7 Convertible Bonds (C-32)
* Combines fixed-coupon bond + embedded equity call option; split into debt and option legs; iterative yield solver.

#### 5.7.8 Repos / Securities-Financing (C-51)
* PV = discounted cash-leg ± haircut; accrual on repo rate; collateral market-to-market schedule placeholder.

#### 5.7.9 Commodity Futures & Forwards (C-52)
* Storage/convexity adjustments parameterised; discount with collateral curve plus convenience yield curve.

#### 5.7.10 Bermudan Swaps (C-55)
* Multi-callable swap; PV via Longstaff-Schwartz lattice in risk module; here only data structure & pay-off schedule stored.

### 5.8 Common Functionality
1. **Builder Pattern**: each instrument has `Builder` fluent API that enforces mandatory fields at compile-time via typestate pattern.
2. **Validation**: `validate()` ensures dates ordered, notional positive, etc., returning `Error::Input`.
3. **Serialization**: all instruments derive Serde with version tag for audit trails.
4. **Currency Guard**: instruments store `Currency`; PV output is always same currency; multi-currency aggregation occurs upstream.
5. **Parallel Pricing**: blanket `pv_slice_parallel()` uses Rayon when feature enabled.

### 5.8 Additional Instrument Details
| Topic | Notes |
|-------|-------|
| **Floating Fixings** | `CashFlowLeg::needs_fixing(val_date)` returns list of resets not yet fixed; optional `FixingStore` trait will provide past fixings. |
| **Instrument Metadata** | All structs embed `id: Option<String>` (ISIN or trade ID) and `tags: BTreeMap<String,String>` behind `metadata` feature for audit trails. |
| **Leg Conventions** | Each leg carries its own `DayCount` and `BusDayConv`; defaults come from `Schedule` but can be overridden. |
| **Side Enum** | `enum Side { Pay, Rec }` stored per leg; PVs positive when `Rec`. |
| **Callable Bond Schedule** | Placeholder `CallSchedule { dates: Vec<Date>, prices: Vec<F> }` referenced by `CallableBond`; evaluation handled in future greeks module. |
| **Serialization Versioning** | `#[serde(tag = "instr", version = 1)]` on every public instrument struct; additions use `#[serde(default)]`. |
| **Thread-Safety & Sharing** | Instruments are `Send + Sync`; large vectors wrapped in `Arc`. |
| **Error Handling** | Common validation errors map to `Error::Input`: negative notional, unsupported currency, strike ≤ 0, end < start dates. |
| **Performance Scaling** | Goal: slice pricing speed-up ≥ 10× on 16-core machine with `parallel` flag enabled. |
| **Deferred Instruments** | `#[non_exhaustive] enum DeferredInstr { Repo, CommodityFuture, XccyBasisSwap, CDS, EquityOption }` placeholder for v1.1+. |

## 6 Algorithms
1. Deposit/FRA PV formulas analytic.  
2. Swap fixed-leg PV uses cash-flow accrual factors from dates module.  
3. Caplet PV uses Black formula with forward from curve and vol from surface.  
4. Swaption PV uses Black formula with swap annuity from discount curve.  
5. Bonds use yield-to-price root-find helper for clean-price conversions.

## 7 Feature Flags
* `parallel` – enable Rayon slice pricing.  
* `private_credit` – exposes bond extensions C-12-C-17.  
* `serde` – serialization derives.

## 8 Integration Points
* **dates** & **calendar**: schedule generation and business-day adjustment.  
* **cashflow**: uses `CashFlow` & leg builders.  
* **curves**: forward/discount curves and vol surfaces.  
* **calibration**: instruments feed quotes into calibration tests.  
* **risk**: DV01/risk metrics extracted via `Risky` trait (future).

## 9 Testing Strategy
* Golden PVs vs QuantLib for swaps, caps, swaptions.  
* Property tests: NPV swap parity fixed-vs-float, cap-floor parity.  
* Criterion bench: price 10 k swaps < 20 ms single-thread.
* Parallel slice pricing benchmark ensures ≥10× speed-up on 16 cores.

## 10 Timeline
* **v0.1.0** – SpotAsset, Deposits, FRAs, Futures.  
* **v0.2.0** – Swaps, Caps/Floors.  
* **v0.3.0** – Swaptions, Bonds.  
* **v1.0.0** – Private-credit extensions, API freeze.

---
*Last updated: 2025-06-29* 