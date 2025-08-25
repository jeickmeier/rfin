# Calibration Module – Detailed Design Document

## 1 Overview
The **calibration** module calibrates concrete term structures (yield curves, credit-hazard curves, inflation curves, volatility/variance surfaces) from market quotes. It fulfils PRD/TDD section **4.5** and requirements C-02b/c, C-34, C-46, C-47, and C-50.

Key features:
• Trait-based: each curve implements `Bootstrappable` allowing common calibration pipeline.  
• Solver-agnostic: root-finding strategy injected (Newton, Brent, Bisection).  
• Deterministic & parallel-ready: no random seeds, Rayon optional.

## 2 Goals & Non-Goals
### Goals
1. Provide generic `Bootstrappable` trait with async `calibrate(&mut self, quotes)` (C-02b).  
2. Implement concrete bootstrappers:  
   • Piecewise discount/zero curve bootstrap (C-46)  
   • Hazard-curve bootstrap (piecewise flat λ)  
   • Inflation curve CPI-level interpolation  
   • Vol-surface SABR calibration (C-47) and grid interpolation.  
   • Tree calibration for option pricing across asset classes:  
      ◦ Equity/FX price lattices (binomial/trinomial CRR/Jarrow-Rudd).  
      ◦ Short-rate lattices for interest-rate derivatives (Hull-White, Black-Derman-Toy, Ho-Lee).  
      ◦ Default-intensity lattices for credit products (Jarrow-Turnbull, Duffie-Singleton).  
3. Support **multi-curve solver** for OIS + IBOR pairs (C-34).  
4. Offer stress-test mode re-using bootstrappers with shocked inputs (C-50).

### Non-Goals
* Market-data download; quotes passed in by caller.  
* GPU/SIMD specific acceleration (future work O-2).

## 3 High-Level API
```rust
pub trait Bootstrappable {
    type Quote;    // e.g., Deposit, Swap, CDS, CapFloorQuote
    async fn calibrate<S: Solver>(&mut self, quotes: &[Self::Quote], solver: &S) -> Result<(), Error>;
}

pub trait Solver {
    fn solve(&self, f: impl Fn(F) -> F, guess: F) -> Result<F, Error>;
}

let mut curve = YieldCurve::empty("USD-OIS", base);
curve.calibrate(&ois_quotes, &NewtonSolver::default()).await?;
```

## 4 Module Layout
```
src/calibration/
  ├─ mod.rs           // facade re-exports
  ├─ solver.rs        // Newton, Brent, Secant, Bisection
  ├─ bootstrap/       // sequential bootstrappers (yield, hazard, inflation)
  │     ├─ yield.rs
  │     ├─ hazard.rs
  │     └─ inflation.rs
  ├─ sabr.rs          // SABR calibration helpers
  ├─ surface.rs       // grid-vol surface fits
  ├─ tree.rs          // generic lattice calibration helpers
  │     ├─ tree_equity.rs   // equity / FX price trees
  │     ├─ tree_rate.rs     // short-rate trees (HW, BDT, HL)
  │     └─ tree_credit.rs   // default-intensity lattices
  ├─ multi_curve.rs   // coupled multi-curve solver (C-34)
  ├─ stress.rs        // stress-test mode (C-50)
  └─ tests.rs
```

## 5 Algorithms
### 5.1 Yield-Curve Bootstrap (Piecewise Log DF)  (C-46)
* Instruments: O/N depo, term depo, futures, swaps.  
* Objective: match PV = 0 per instrument using piecewise log-DF segments.  
* Workflow:
 1. Sort quotes by maturity.  
 2. For each quote `i`: root-find DF/zero at knot `t_i` such that instrument PV = 0 using current curve for previous knots.  
 3. Default interpolator `LogDf` ensures monotone DF.  
* Convergence: Newton with fallback to Brent; max 20 iterations, tolerance 1e-12 DF.

### 5.2 Hazard-Curve Bootstrap (Piecewise Flat λ)  (C-43)
* Inputs: CDS par spreads.  
* Unknowns: hazard rates λ_k per knot.  
* PV equation solved sequentially using survival prob up to knot.  
* Uses analytic formula; secant step per knot.

### 5.3 Inflation Curve Calibration
* Quotes: ZC-swaps, CPI fixings.  
* Bootstraps CPI level curve: cumulative CPI at each fixing date solved so that swap PV = 0.  
* Linear-in-log CPI interpolation between knots.

### 5.4 Vol Surface Calibration (Cap/Floor & Swaption)
* Cap/floor strips: solve Black implied vol per maturity/strike; populate 2-D grid.  
* Swaption SABR: for each expiry/tenor pair fit (α,ν,ρ) using least-squares to market vol-strike smile.  
* Global surface assembled from merged pair fits; optional trilinear interpolation fallback.

### 5.5 Coupled Multi-Curve Solver (C-34)
*Detailed algorithmic and module design has been moved to a dedicated document*

The multi-curve engine now follows a **hybrid two-stage workflow**:

1. **Iterative projection (block Gauss–Seidel)** bootstraps each curve in turn to generate a _robust seed_ that reproduces the market's standard sequential calibration.
2. An optional **global Newton / Levenberg–Marquardt** step stacks every curve node into one vector and refines the solution quadratically using `nalgebra::DMatrix/DVector`.

This design reproduces the trader-friendly behaviour of traditional spreadsheets while unlocking the speed and consistency of a global solver when needed.  All configuration knobs (tolerances, damping, whether to run the Newton polish, etc.) live in `SolverConfig`.

See `docs/core/calibration/multi_curve_detailed_design.md` for the full mathematical derivation, module layout, and regression tests.

### 5.6 Stress-Test Mode (C-50)
* `stress_bootstrap(curve, shocked_quotes)` re-evaluates equations **without** root-finding; useful for scenario risk.

### 5.7 Tree Calibration (Extended)

#### 5.7.1 Equity / FX Price Trees
* **Models**: Cox-Ross-Rubinstein (binomial), Jarrow-Rudd, Trigeorgis (trinomial).
* **Inputs**: spot, discount curve, dividend yield, vol curve/surface.
* **Unknowns**: up/down factors or local vol per step so that vanilla option PVs match market.
* **Calibration**: sequential root-finding per maturity (Brent) on local vol; tolerance 1e-8 PV.

#### 5.7.2 Interest-Rate Trees
* **Models**: Ho-Lee, Black-Derman-Toy (BDT), Hull-White 1-factor recombining trees.
* **Inputs**: initial zero curve, volatility term-structure, mean-reversion (HW).
* **Unknowns**: short-rate shifts (HL) or node volatilities (BDT) per time step to replicate swaption/ cap-floor vol surface.
* **Calibration Workflow**:
 1. Build initial tree assuming flat vol.
 2. For each expiry bucket, iterate on node parameters so that model cap/floor or swaption PV equals market.
* **Convergence**: Newton with fallback to Brent, tolerance 1e-8 vol.

#### 5.7.3 Credit-Spread Trees
* **Models**: Jarrow-Turnbull default-intensity lattice, Duffie-Singleton recovery models.
* **Inputs**: discount curve, CDS/par-spread term structure, recovery rate.
* **Unknowns**: default intensity λ nodes per time step.
* **Calibration**: sequential solution of CDS PV = 0 at each tenor using tree survival probabilities.

**Outputs for All Trees**: serialisable lattice structs (`EquityTree`, `RateTree`, `CreditTree`) consumed by pricing & risk engines; amenable to scenario shocks without re-calibration.

## 6 Feature Flags
* `parallel` – enables Rayon in multi-curve solver.  
* `sabr` – pulls `sabr_rs` crate for analytic vol calibration.  
* `decimal128` – calculations in high precision.

## 7 Integration Points
* Consumes concrete curve builders (`YieldCurve::empty`, etc.) from curves module.  
* Uses dates & calendar for year-fractions and adjustments.  
* Exposes results back as populated curves to analytics & risk engines.

## 8 Testing Strategy
* Unit tests per instrument type: PV close to 0 within 1e-10 after bootstrap.  
* Regression fixtures vs QuantLib.  
* Property tests: monotone DF & survival probabilities.  
* Benchmarks: bootstrap OIS+LIBOR curves 100 quotes < 3 ms single-thread.

## 9 Open Questions
1. Use analytic bootstrappers for futures (convexity adjustment) or assume quoted PV?  
2. Global optimisation vs sequential – evaluate Levenberg-Marquardt once SABR complete.  
3. Handling of overlapping instruments (e.g., OIS vs SOFR futures) – weighting scheme?

## 10 Timeline
* **v0.1.0** – Solver infra + yield curve bootstrap.  
* **v0.2.0** – Hazard & inflation bootstrappers.  
* **v0.3.0** – Vol surface & SABR calibration; multi-curve solver.  
* **v1.0.0** – Stress-test mode, API freeze.

---
*Last updated: 2025-06-29* 