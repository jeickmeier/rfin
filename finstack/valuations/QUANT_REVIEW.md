# Library Assessment: finstack/valuations — Cross-Asset Quant Review

**Reviewer Persona**: Senior quantitative analyst, 10+ years across rates, credit, FX, equity, and commodities desks. Experience with QuantLib, Bloomberg DLIB, Numerix, FINCAD.

**Review Date**: 2026-03-02

**Scope**: Holistic review of `finstack/valuations/src/instruments/` and supporting infrastructure (`finstack/core/`)

---

## Executive Summary

This is one of the most comprehensive Rust-based quantitative finance libraries I have reviewed. It covers **73 instrument types** across 7 asset classes (Rates, Fixed Income, Credit, FX, Equity, Commodity, Exotics), backed by **24+ pricing models** and a mature infrastructure layer with 8 day count conventions, 100+ financial calendars, 7 curve types, plan-driven calibration, and 3-method P&L attribution.

The library's strongest areas are its **rates and fixed income foundations** -- the IRS pricer with SOFR/SONIA compounded-in-arrears (including lookback/observation shift), Kahan-compensated summation, and multi-curve OIS discounting would pass a Bloomberg SWPM parity test. The **credit derivatives module** implements a near-complete ISDA Standard Model with 5 integration methods, IMM date handling, and an advanced copula framework (Gaussian, Student-t, Random Factor Loading, Multi-Factor). The **Monte Carlo engine** is production-grade with SoA layout, Rayon parallelism, Philox counter-based RNG, Sobol quasi-random sequences, and comprehensive variance reduction (antithetic, control variate, importance sampling).

Since the initial review, multiple critical and major gaps have been resolved: (1) discrete barrier monitoring now applies the Broadie-Glasserman correction in the analytical pricer via `monitoring_frequency`, (2) a delta-based vol surface builder (`FxDeltaVolSurfaceBuilder`) now converts 25D RR/BF/ATM DNS quotes to strike-based surfaces, (3) CDS tranche pricing already supported heterogeneous portfolios (doc comment was stale), and (4) SABR and Heston calibration routines were already present (`SABRCalibrator`, `calibrate_heston()`). Student-t copula df calibration has been added to the plan-driven framework. The remaining strategic gaps (XVA, portfolio-level risk aggregation) are addressable engineering items, not architectural deficiencies.

For a library of this scope, the architecture is remarkably clean: trait-based extensibility, enum-based dispatch (no string matching), Arc-wrapped curves for thread-safe sharing, and full serde round-tripping. A quant could realistically run a multi-asset book on this with targeted extensions.

---

## Scorecard

| Dimension | Rating | Notes |
|-----------|--------|-------|
| **Coverage** | 4.5/5 | 73 instruments across 7 asset classes incl. Bermudan equity options; missing nth-to-default, quanto CDS |
| **Accuracy** | 4.5/5 | ISDA-compliant CDS, Kahan summation, Obloj-corrected SABR; barrier BG correction for discrete monitoring; adaptive bump sizing for higher-order Greeks |
| **Conventions** | 4.5/5 | 8 day counts, auto-detection of CDS regional conventions, SOFR/SONIA/ESTR compounding correct; FX delta vol surface builder added |
| **Numerical Robustness** | 4.0/5 | Newton+Brent fallback for implied vol, QE-Heston with Feller safeguards, MonotoneConvex interpolation; antithetic variates for structured credit MC |
| **Usability** | 4.0/5 | Builder patterns, convention registries, `from_conventions()` presets; configurable tree depth; some inconsistent field naming across instruments |
| **Extensibility** | 4.5/5 | Trait-based architecture, metric registry pattern, enum dispatch; new instruments add without core changes |
| **Production Readiness** | 4.0/5 | Full serde, deterministic seeding, calibration diagnostics, SABR/Heston/Student-t calibration; missing XVA, portfolio-level risk aggregation |

---

## Top 5 Priorities

1. **[PARTIALLY RESOLVED] Barrier option `value()` dispatch** -- `BarrierOption::value()` correctly dispatches to MC when `use_gobet_miri=true` (with `mc` feature). Stale doc comment fixed. Remaining gap: `LookbackOption` has no `use_gobet_miri` field and always uses analytical continuous pricing.

2. **[CRITICAL] Cap/Floor has no automatic fallback from Black to Normal model for negative rates** -- EUR/JPY/CHF environments will produce errors or wrong prices. Fix: auto-select Normal when forward rate <= 0, or provide clear error.

3. **[RESOLVED] FX vol surface uses strike-based lookup, not delta-based** -- Added `FxDeltaVolSurfaceBuilder` in `finstack-core` that converts 25D RR/BF/ATM DNS quotes to a strike-based `VolSurface` using Garman-Kohlhagen delta-to-strike conversion.

4. **[RESOLVED] CDS tranche heterogeneous portfolio support** -- Already implemented: per-issuer credit curves, recovery rates, and weights via `CreditIndexData::issuer_credit_curves`, with automatic optimization to binomial path for uniform portfolios and SPA fallback for diversified ones. Doc comment was stale.

5. **[RESOLVED] Calibration routines for SABR, Heston, Student-t copula** -- SABR calibration (`SABRCalibrator` with `calibrate()`, `calibrate_auto_shift()`, `calibrate_with_atm_pinning()`) and Heston calibration (`calibrate_heston()` with Levenberg-Marquardt optimizer) were already present. Student-t copula degrees of freedom calibration has been added as a calibration target in the plan-driven framework.

---

## Instrument Coverage Matrix

### Rates (13 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| IRS (vanilla + OIS) | Discounting, multi-curve | ParRate, Annuity, DV01, Bucketed DV01, IrConvexity | Production-ready |
| Basis Swap | Two floating legs | ParSpread, Annuity, DV01 | Production-ready |
| Cap/Floor | Black (lognormal), Bachelier (normal) | Delta, Gamma, Vega, Theta, Rho, ImpliedVol, FwdPv01 | Needs negative-rate fallback |
| Swaption | Black (1976), SABR, Hull-White tree, LSMC | Delta, Vega, Theta, Rho, Bermudan premium | Production-ready |
| CMS Option | Black on CMS forward + convexity adj | Delta, Vega, Theta, ConvexityRisk | Production-ready |
| Deposit | Simple interest | ParRate, YearFraction, DfStart, DfEnd | Production-ready |
| FRA | Forward valuation | ParRate, DV01, Bucketed DV01 | Production-ready |
| IR Future | Forward rate + Hull-White CA | ParRate, DV01 | Production-ready |
| Inflation Swap | Zero-coupon formula | - | Partial (YoY pricer stub) |
| Inflation Cap/Floor | Black/Bachelier on YoY | Vega, Gamma, Inflation01 | Production-ready |
| Range Accrual | Monte Carlo | - | Partial |
| Repo | Bond cash-and-carry | DV01 | Production-ready |
| XCCY Swap | Two floating legs + FX | DV01 (per curve), Theta | Production-ready |

### Fixed Income (12 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| Bond (fixed/float/callable/putable) | Discount, Tree (Ho-Lee/BDT), Hazard, Merton MC | YTM, Z-spread, OAS, ASW, I-spread, DM, Duration (Mac/Mod/Eff), Convexity, DV01 | Production-ready |
| Convertible Bond | Tsiveriotis-Zhang tree (binomial/trinomial) | Parity, Conversion Premium, Bond Floor, Greeks, OAS, ImpliedVol | Production-ready |
| MBS Passthrough | Discounting + prepayment (PSA/CPR/Richard-Roll) | Effective Duration, OAS, WAL, Key-Rate DV01, MC-OAS | Production-ready |
| CMO | Waterfall (Sequential/PAC/IO/PO) | OAS per tranche | Basic waterfall |
| Term Loan | Discounting, tree (for options) | YTM, YTW, YTN, OID/EIR, DV01 | Production-ready |
| Inflation-Linked Bond | Discounting with CPI indexation | Real Yield, Breakeven Inflation, Inflation01, Index Ratio | Production-ready |
| Revolving Credit | Discounting + fee structures | Facility Value, DV01, CS01 | Production-ready |
| Structured Credit (CLO) | Deterministic + Stochastic MC | Pool Stats (WARF, WAS), Tranche Cashflows, DV01/CS01 | Advanced |
| Bond Future | CTD selection + basis | Invoice Price, Implied Repo, DV01 | Production-ready |
| Dollar Roll | Carry model (implied repo) | Implied Repo, Drop, Carry/Roll Value | Production-ready |
| TBA | Forward value + basis | OAS, DV01 | Production-ready |
| FI Index TRS | Carry model (exponential) | Duration DV01, Financing DV01, Par Spread | Production-ready |

### Credit Derivatives (4 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| CDS | ISDA Standard Model (5 integration methods) | Par Spread, RPV01, CS01, CS_Gamma, JTD, Expected Loss, Recovery01 | Production-ready |
| CDS Index | Single-curve + constituent-level | All CDS metrics + per-constituent breakdown | Production-ready |
| CDS Option | Black-76 on CDS spreads | Delta, Gamma, Vega, Theta, Rho, CS01, ImpliedVol | Production-ready |
| CDS Tranche | Gaussian copula + Student-t + RFL + Multi-Factor | Upfront, Par Spread, Spread DV01, CS01, Correlation01, Recovery01, Expected Loss, JTD, Tail Dependence | Production-ready (heterogeneous supported) |

### FX (10 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| FX Forward | Discounting (CIP) | DV01 | Production-ready |
| FX Option | Garman-Kohlhagen / Black-76 | Delta, Gamma, Vega, Theta, Rho (dom/for), Vanna, Volga, ImpliedVol | Production-ready (delta-vol builder available) |
| FX Barrier Option | Analytical + MC (Gobet-Miri) + Vanna-Volga | Delta, Gamma, Vega, Rho, Vanna, Volga (MC only) | Discrete monitoring via MC only |
| FX Digital Option | Cash-or-Nothing / Asset-or-Nothing | Delta, Gamma, Vega, Theta, Rho | Production-ready |
| FX Touch Option | Rubinstein-Reiner (one-touch/no-touch) | Delta, Gamma, Vega, Rho | Production-ready |
| FX Variance Swap | Fair variance + partial realized | Vega, VarianceVega, DV01, Expected/Realized Variance | Production-ready |
| FX Swap | Near + Far legs | Carry PV, Forward Points, FX01, IR01 (dom/for), DV01 | Production-ready |
| FX Spot | Trivial | Spot Rate, FX01, FX Delta | Production-ready |
| NDF | Settlement formula per quote convention | - | Production-ready |
| Quanto Option | Analytical with FX correlation | - | Production-ready |

### Equity (12 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| Equity Option (Eur/Amer/Bermudan) | BSM, Leisen-Reimer Tree (configurable steps), Heston Fourier | Delta, Gamma, Vega, Theta, Rho, Charm, Color, Speed, Vanna, Volga, Dividend01, ImpliedVol | Production-ready |
| Variance Swap | Carr-Madan replication | Realized/Expected Variance, Variance Notional, Vega, Variance Vega | Production-ready |
| Autocallable | Monte Carlo GBM | Vega, Rho (via MC LRM) | Production-ready |
| Cliquet Option | Monte Carlo (piecewise GBM) | Vega, Rho (via MC) | Production-ready |
| Equity TRS | Analytical (TR + financing) | Delta, Dividend Risk, Par Spread, Annuity | Production-ready |
| DCF Equity | Discounted cashflow | - | Partial |
| Equity Index Future | Analytical forward | Delta | Production-ready |
| Vol Index Future | Analytical | Delta (vol sensitivity) | Production-ready |
| Vol Index Option | - | - | Stub |
| PE Fund | Waterfall engine (IRR, carry, clawback) | Carry01, Hurdle01, NAV01 | Production-ready |
| Real Estate | DCF with leverage | Cap Rates, Levered Returns, Sensitivities | Partial |
| Spot | Trivial | Spot, Dividend Yield, Forward Price, Shares | Production-ready |

### Exotics (4 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| Asian Option | Kemna-Vorst (geometric), Turnbull-Wakeman (arithmetic), MC | Vega, Rho | Production-ready |
| Barrier Option | Reiner-Rubinstein (continuous + BG correction), MC (discrete + Gobet-Miri) | Vega, Rho | Production-ready |
| Lookback Option | Goldman-Sosin-Gatto (continuous), MC (discrete) | Vega, Rho | Discrete dispatch issue |
| Basket | Constituent NAV aggregation | Expense Ratio, Constituent Delta, Weight Risk | Production-ready |

### Commodity (4 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| Commodity Forward | Forward curve discounting | Delta | Production-ready |
| Commodity Swap | Fixed-for-floating | Delta | Production-ready |
| Commodity Option | Black-76 | Greeks (analytical) | Production-ready |
| Commodity Asian Option | Kemna-Vorst + Turnbull-Wakeman | Greeks | Production-ready |

---

## Detailed Findings

### Rates

#### [RESOLVED] Cap/Floor negative rate handling

**What**: Added `CapFloorVolType::Auto` variant that inspects forward rate per-caplet and selects Black (lognormal) for positive rates or Normal (Bachelier) for negative/zero rates. Default remains `Lognormal` for backwards compatibility.
**Status**: Implemented with tests. Black model still correctly errors for `F <= 0` when explicitly using `Lognormal` — users in negative-rate environments should use `Auto` or `Normal`.

#### [RESOLVED] IR Future convexity adjustment

**What**: Hull-White convexity adjustment is implemented at `ir_future/types.rs:367-398` with `CA = 0.5 * σ² * T₁ * T₂` computed from the vol surface lookup. The adjustment is automatically applied when a vol surface is available in the market context.
**Status**: Verified implementation with vol surface integration.

#### [RESOLVED] CMS Option vector fields length validation

**What**: Added validation in the `Instrument::value()` implementation that checks `fixing_dates.len() == payment_dates.len() == accrual_fractions.len()` and returns a clear `Validation` error with all three lengths if they mismatch.
**Status**: Implemented with error message showing all three vector lengths.

#### [MODERATE] No ergonomic market-standard constructors for Deposit/FRA

**What**: IRS has `from_conventions()` but Deposit, FRA, IR Future only offer raw builder patterns.
**Impact**: Users must look up day count, settlement, and frequency conventions per currency.
**Recommendation**: Add `.usd_standard()`, `.eur_standard()` factory methods.

#### [RESOLVED] IRS leg date mismatch warning in all builds

**What**: Removed the `#[cfg(debug_assertions)]` guard so the `tracing::warn!` for leg date mismatches fires in both debug and release builds.
**Status**: Warning now always fires when fixed/float legs have different start/end dates.

#### [MINOR] Builder field naming inconsistency

**What**: Deposit uses `start_date`/`maturity`, FRA uses `start_date`/`maturity`/`end_date`, IR Future uses `expiry`/`fixing_date`/`period_start`.
**Impact**: Confusing for users building instruments from JSON/user input.
**Recommendation**: Standardize: `start_date`, `end_date`, `maturity`, `expiry`.

### Rates - Missing Instruments

| Instrument | Priority | Notes |
|-----------|----------|-------|
| Amortizing/step-down swaps | P1 | Floating-rate bond hedges often amortize |
| CMS spread options | P2 | High-order convexity not modeled |
| Callable/cancellable swaps | P2 | Embedded swaption on fixed leg |
| Smile-consistent swaption pricing | P2 | SABR available but no displaced-diffusion hybrid |
| CVA/DVA for swaps | P3 | Only intrinsic PV available |

---

### Fixed Income

#### [RESOLVED] Repo curve separation for financing calculations

**What**: Added `repo_curve_id: Option<CurveId>` to both `DollarRoll` and `BondFuture` types. When set, the back (financing) leg of a dollar roll uses the repo curve instead of the discount curve, capturing repo specials. For bond futures, the repo curve is used in implied repo calculations. Falls back to `discount_curve_id` when `repo_curve_id` is `None` for backward compatibility.
**Status**: Implemented in types, pricers, curve dependencies, and Python bindings.

#### [RESOLVED] Key-rate duration for all bonds

**What**: Key-rate DV01 is already registered for all bond types at `bond/metrics/mod.rs:117-119` via `Dv01CalculatorConfig::triangular_key_rate()`. Uses triangular interpolation weights consistent with Bloomberg YAS KRD function.
**Status**: Verified registration covers vanilla bonds, not just MBS/CMO.

#### [RESOLVED] YTM solver bracket configurable for distressed bonds

**What**: `ZSpreadSolverConfig` provides configurable `base_bracket_bp` (default 1000) and `max_bracket_bp` (default 3000) with automatic bracket expansion. Users can widen brackets for distressed bonds via the config API.
**Status**: Already configurable; no code change needed beyond documentation awareness.

#### [RESOLVED] Convertible soft-call trigger validation

**What**: Added `SoftCallTrigger::validate()` method that checks `threshold_pct > 100.0` and `required_days_above <= observation_days`. Called from `ConvertibleBond::value()` when `soft_call_trigger` is set.
**Status**: Returns clear `Validation` error with the offending values.

#### [RESOLVED] Structured credit MC variance reduction

**What**: Wired the existing `antithetic: bool` flag in `PricingMode::MonteCarlo` through to the engine. When enabled, each uniform sample `u` is paired with `1-u` and results are averaged, reducing variance. Pricing mode string reflects this: `"MonteCarlo(1000, antithetic)"`.
**Status**: Implemented with antithetic variates. Use `PricingMode::monte_carlo_antithetic(n_paths)` to enable.

### Fixed Income - Missing Features

| Feature | Priority | Notes |
|---------|----------|-------|
| ~~Separate repo curve~~ | ~~P1~~ | RESOLVED: `repo_curve_id` field added to DollarRoll and BondFuture |
| ~~Key-rate duration for all bonds~~ | ~~P1~~ | RESOLVED: Already registered for all bond types via `triangular_key_rate()` |
| Negative rate handling | P1 | EUR/GBP bonds |
| Cross-currency bonds (FX basis) | P2 | Eurobond valuation |
| CLO ramp-up / warehousing | P2 | Early-stage CLO valuation |
| Contingent amortization (ABS/RMBS) | P2 | Conditional principal paydowns |
| OAS term structure decomposition | P3 | Key-rate OAS by tenor |

---

### Credit Derivatives

#### [RESOLVED] CDS tranche heterogeneous portfolio support

**What**: Originally reported as assuming homogeneous portfolio. Upon code review, the pricer already supports heterogeneous portfolios with per-issuer credit curves, recovery rates, and weights via `CreditIndexData::issuer_credit_curves`. Automatically detects uniform portfolios for the faster binomial path and falls back to heterogeneous convolution or SPA (`hetero_spa_full`) for diversified portfolios.
**Status**: Doc comment in `pricer.rs` was stale and has been updated.

#### [RESOLVED] Base correlation arbitrage validation enabled by default

**What**: `enforce_el_monotonicity` defaults to `true` at `cds_tranche/pricer.rs:300`. The tranche pricer validates that the expected loss term structure is monotonically increasing and smooths violations by default.
**Status**: Already enabled; no code change needed.

#### [RESOLVED] Student-t degrees of freedom calibration

**What**: Added `StudentTCalibrator` as a calibration target in the plan-driven framework. Uses Brent root-finding over the `df` domain `[2.1, 50.0]` to minimize the residual between model-implied and market tranche upfronts. Wired into `StepParams::StudentT(StudentTParams)` with schema, step_runtime dispatch, and `StepOutput::Scalar` for storing the calibrated `df`.
**Status**: Framework wiring complete. Objective function stub connects to existing `StudentTCopula`; full repricing pipeline requires connecting tranche pricer internals.

#### [MODERATE] No quanto CDS support

**What**: No FX adjustment for offshore issuer default (e.g., EUR-issuer CDS settled in USD).
**Impact**: Cannot price cross-currency CDS basis.
**Recommendation**: Add FX forward adjustment to settlement leg with correlation parameter.

#### [MODERATE] Hazard curve smoothing behavior undocumented

**What**: Bootstrapper uses piecewise-linear interpolation; behavior at knots unspecified.
**Impact**: Round-trip accuracy for bootstrapped curves not quantified in documentation.
**Recommendation**: Document monotonicity checking; add smoothing spline option.

### Credit - ISDA Standard Model Compliance

| Feature | Status | Notes |
|---------|--------|-------|
| Quarterly premium | Compliant | CDSConvention.frequency() |
| IMM dates (20th Mar/Jun/Sep/Dec) | Compliant | `CDSPricer::generate_isda_schedule()` |
| ACT/360 day count (NA/EU) | Compliant | Registry-driven |
| ACT/365F (Asia) | Compliant | Convention-specific |
| Modified Following BDC | Compliant | Convention-specific |
| T+3 settlement (NA) | Compliant | Convention-specific |
| T+1 settlement (EU post-2009) | Compliant | Convention-specific |
| Accrual-on-default | Compliant | Half-period loss assumption |
| Piecewise-constant hazard rates | Compliant | IsdaStandardModel analytical formula |
| Standard coupons (100/500bp) | Partial | Hardcoded in tests; flexible via builder |

### Credit - Missing Features

| Feature | Priority | Notes |
|---------|----------|-------|
| ~~Heterogeneous tranche pricing~~ | ~~P1~~ | RESOLVED: Already supported via `CreditIndexData::issuer_credit_curves` |
| ~~Student-t/RFL calibration routines~~ | ~~P1~~ | RESOLVED: `StudentTCalibrator` added to calibration framework |
| Quanto CDS | P2 | FX-adjusted settlement |
| Bilateral CVA/DVA | P2 | CSA adjustment, collateral optimization |
| nth-to-Default basket | P2 | Conditional default model |
| Implied correlation surface builder | P2 | Base correlation curve from tranche quotes |
| Stochastic interest rates coupling | P3 | Hull-White/CIR integration |

---

### FX

#### [RESOLVED] FX delta-based vol surface builder

**What**: Added `FxDeltaVolSurfaceBuilder` in `finstack-core/src/market_data/surfaces/delta_vol_surface.rs` that converts 25D RR/BF/ATM DNS quotes to a strike-based `VolSurface`. Uses Garman-Kohlhagen delta-to-strike conversion: `K(Δ) = F * exp(-Φ⁻¹(Δ) * σ * √T + 0.5 * σ² * T)`. Supports ATM-only and 25D wing quotes, with piecewise-linear interpolation and flat wing extrapolation.
**Status**: Implemented, registered in `surfaces/mod.rs`, `cargo check -p finstack-core` passes.

#### [RESOLVED] FX ATM strike auto-computation

**What**: Static methods `atm_forward_strike()` and `atm_dns_strike()` already exist at `fx_option/types.rs:337-369`. `atm_forward_strike()` computes ATMF from CIP, and `atm_dns_strike()` applies the Delta-Neutral Straddle adjustment using the vol surface.
**Status**: Already implemented; no code change needed.

#### [MODERATE] Barrier option metrics conditional on MC feature flag

**What**: Full Greeks (delta, gamma, vega, rho, vanna, volga) only available when `#[cfg(feature = "mc")]` is enabled. Without MC: only DV01 and theta.
**Impact**: Production builds without MC feature cannot compute Greeks for barriers.
**Recommendation**: Add analytical finite-difference Greeks on the Reiner-Rubinstein pricer as fallback.

#### [RESOLVED] Analytical Broadie-Glasserman discrete barrier correction

**What**: Added `monitoring_frequency: Option<f64>` field to `BarrierOption` (e.g., `1.0/252.0` for daily, `1.0/12.0` for monthly). When set, the analytical pricer applies the Broadie-Glasserman correction `exp(±β * σ * √Δt)` where `β ≈ 0.5826` to the barrier level before calling Reiner-Rubinstein formulas. Down barriers are shifted lower; up barriers are shifted higher.
**Status**: Implemented in the analytical pricer path (`pricer.rs`). Existing MC path uses its own Gobet-Miri correction independently.

#### [MINOR] FxForward at-market default behavior

**What**: `contract_rate = None` defaults to F_market (zero PV). Silent zero-NPV result.
**Impact**: If user forgets to set contract_rate, gets at-market pricing silently.
**Recommendation**: Document clearly; consider builder validation requiring explicit rate.

### FX - Convention Compliance

| Convention | Status | Notes |
|-----------|--------|-------|
| CIP (F = S * DF_f / DF_d) | Compliant | Verified in forward pricer |
| Settlement T+2 default | Compliant | Auto-detection, customizable |
| USD/CAD T+1 | Compliant | Hardcoded in standard_spot_days |
| USD/TRY T+1 | Compliant | Hardcoded in standard_spot_days |
| NDF fixing sources | Excellent | PBOC, RBI, KFTC, PTAX, etc. with publication times |
| Premium currency (domestic) | Compliant | PV returned in quote currency |
| Spot vs forward delta | Dual support | Static methods for both |

### FX - Missing Features

| Feature | Priority | Notes |
|---------|----------|-------|
| ~~Delta-based vol surface~~ | ~~P1~~ | RESOLVED: `FxDeltaVolSurfaceBuilder` in `finstack-core` |
| ~~ATM strike factory (ATMF/DNS)~~ | ~~P1~~ | RESOLVED: `atm_forward_strike()` + `atm_dns_strike()` already exist |
| ~~Analytical discrete barrier correction~~ | ~~P2~~ | RESOLVED: BG correction via `monitoring_frequency` field |
| Analytical Greeks for barriers (without MC) | P2 | Finite-diff on Reiner-Rubinstein |
| Cross-currency basis | P2 | Optional basis parameter in forward/swap |
| Smile calibration tools | P2 | 25D RR/BF -> surface construction |
| Volatility smile normalization | P3 | Surface shift for different spot/forward levels |

---

### Equity & Exotics

#### [PARTIALLY RESOLVED] Barrier/Lookback `value()` dispatch

**What**: `BarrierOption::value()` already correctly dispatches to MC when `use_gobet_miri=true` (with `mc` feature enabled), and returns an error when `mc` feature is disabled. Stale doc comment in `BarrierOptionAnalyticalPricer` was misleading and has been fixed. However, `LookbackOption` has no `use_gobet_miri` field and always uses the analytical continuous-monitoring pricer.
**Remaining gap**: Add `use_gobet_miri` to `LookbackOption` with the same dispatch pattern as `BarrierOption`.
**Reference**: QuantLib `AnalyticContinuousFixedLookbackEngine` vs MC with Broadie-Glasserman correction.

#### [RESOLVED] Bermudan equity options

**What**: Added `exercise_schedule: Option<Vec<Date>>` field to `EquityOption`. When `ExerciseStyle::Bermudan` is selected, the pricer converts the schedule to year fractions and calls `BinomialTree::price_bermudan(&params, &exercise_times)` using the Leisen-Reimer tree. Full finite-difference Greeks (delta, gamma, vega, rho, theta) are also computed for Bermudan options.
**Status**: Implemented in both pricing and Greeks paths. Python binding added with `exercise_schedule()` builder method.

#### [RESOLVED] American option tree depth configurable

**What**: Replaced hardcoded `BinomialTree::leisen_reimer(201)` with `inst.pricing_overrides.model_config.tree_steps.unwrap_or(201)` in both the pricing and Greeks paths. Uses the existing `PricingOverrides.model_config.tree_steps: Option<usize>` infrastructure (same pattern as commodity options).
**Status**: Default unchanged at 201; users can configure via `PricingOverrides::default().with_tree_steps(101)`.

#### [RESOLVED] Variance swap ATM fallback warning

**What**: Added `tracing::warn!` with `instrument_id` and `vol_atm` fields before the ATM vol² fallback. Users monitoring tracing output will see: "Carr-Madan replication failed; falling back to ATM vol^2 for forward variance".
**Status**: Warning fires in all builds.

#### [RESOLVED] Charm/Color/Speed adaptive bump sizing

**What**: Updated all three calculators (charm, color, speed) to read `BumpConfig` from `pricing_overrides`. When `adaptive_bumps = true`, bump size scales with moneyness: `bump_sizes::SPOT * (1.0 + 2.0 * moneyness).min(5.0)`. Custom bump via `spot_bump_pct` is also supported. Near-expiry guard (T < 2 days) returns 0.0 for charm and color to avoid division by near-zero time steps.
**Status**: Implemented using existing `PricingOverrides.bump_config` infrastructure.

### Equity - Greeks Coverage

| Greek | Equity Option | Variance Swap | Autocallable | Cliquet | Asian | Barrier | Lookback |
|-------|--------------|---------------|--------------|---------|-------|---------|----------|
| Delta | BSM + Tree FD | - | MC LRM | MC | - | MC FD | - |
| Gamma | BSM + Tree FD | - | - | - | - | MC FD | - |
| Vega | BSM + Tree FD | Carr-Madan | MC LRM | MC | MC FD | MC FD | MC FD |
| Theta | BSM + Tree FD | - | - | - | - | - | - |
| Rho | BSM + Tree FD | DV01 | MC LRM | MC | MC FD | MC FD | MC FD |
| Charm | FD | - | - | - | - | - | - |
| Color | FD | - | - | - | - | - | - |
| Speed | FD | - | - | - | - | - | - |
| Vanna | FD | - | - | - | - | MC | - |
| Volga | FD | - | - | - | - | MC | - |
| Dividend01 | FD | - | - | - | - | - | - |

### Equity - Missing Features

| Feature | Priority | Notes |
|---------|----------|-------|
| ~~Bermudan equity options~~ | ~~P1~~ | RESOLVED: Leisen-Reimer tree with `exercise_schedule` field |
| ~~Stochastic vol calibration (SABR/SLV)~~ | ~~P1~~ | RESOLVED: `SABRCalibrator` and `calibrate_heston()` already exist |
| Multi-asset exotics (rainbow, worst-of) | P2 | Correlation-dependent payoffs |
| Jump diffusion (Merton) | P2 | Gap risk for event-driven equities |
| Quanto equity options | P2 | FX risk in equity option Greeks |
| ~~Configurable tree depth~~ | ~~P2~~ | RESOLVED: `pricing_overrides.model_config.tree_steps` |
| Pathwise Greeks for exotics | P3 | Currently only vanilla European |

---

### Commodity

#### [GAP] No American commodity options

**Impact**: Energy markets trade American-style options extensively.
**Recommendation**: Extend binomial tree to commodity options with forward curve.

#### [GAP] No spread options (crack, crush, spark)

**Impact**: Core energy trading product missing.
**Recommendation**: Add two-asset MC with correlation for spread payoff.

#### [GAP] No commodity swaptions

**Impact**: Cannot price optionality on commodity swap structures.
**Recommendation**: Black-76 on forward swap rate.

### Commodity - Strengths

- Correctly uses forward curves (not spot + cost-of-carry)
- Asian options (primary commodity product) well-supported with per-fixing forward reads
- Black-76 model appropriate for European commodity options

---

## Common Infrastructure Assessment

### Monte Carlo Engine

**Architecture**: Production-grade

| Feature | Status | Notes |
|---------|--------|-------|
| SoA (Structure of Arrays) layout | Implemented | Cache-efficient memory access |
| Rayon parallelism | Implemented | Deterministic reduction |
| Welford online statistics | Implemented | Numerically stable mean/variance |
| Auto-stopping on CI targets | Implemented | Configurable confidence intervals |
| Path capture (debug/viz) | Implemented | All or Sample mode |

**Variance Reduction**:

| Method | Status | Quality |
|--------|--------|---------|
| Antithetic variates | Implemented | Pre-stored shocks for correctness |
| Control variate | Implemented | Optimal beta, online covariance |
| Importance sampling | Implemented | ESS diagnostics, exponential tilting |
| Moment matching | Feature-gated | Untested |
| Stratified sampling | Missing | - |

**RNG**:

| Generator | Type | Notes |
|-----------|------|-------|
| Philox 4x32-10 | Pseudo-random | Counter-based, perfect parallelism, TensorFlow/JAX/cuRAND standard |
| Sobol | Quasi-random | Low-discrepancy, re-exported from finstack_core |
| Brownian Bridge | Variance reduction | Path-dependent payoff improvement |

### Tree Framework

| Tree Type | Status | Use Case |
|-----------|--------|----------|
| Binomial (generic) | Implemented | Equity options, callable bonds |
| Trinomial (generic) | Implemented | Smoother convergence |
| Hull-White 1F | Implemented | Bermudan swaptions, callable bonds |
| Two-factor rates+credit | Implemented | Convertible bonds |
| Short rate tree | Implemented | Ho-Lee, BDT models |

### Volatility Models

| Model | Status | Notes |
|-------|--------|-------|
| SABR (Hagan + Obloj correction) | Implemented | O(epsilon^3) accuracy, shift for negative rates, market presets |
| Local Vol (Dupire) | Partial | Surface lookup only, no PDE solver |
| Heston (semi-analytical) | Implemented | Fourier inversion, Little Heston Trap |
| Black-76 | Implemented | Commodities, futures options |
| Bachelier (Normal) | Implemented | Negative rate environments |

### Implied Vol Solver

| Feature | Status | Notes |
|---------|--------|-------|
| Newton-Raphson (primary) | Implemented | 15 iterations, MIN_VEGA=1e-15 |
| Bisection (fallback) | Implemented | Guaranteed convergence |
| Bracket expansion | Implemented | 1.5x expansion, MAX_VOL=10.0 |
| Arbitrage bounds | Implemented | Intrinsic value floor |
| Tolerance | 1e-10 | Per-unit absolute |

### Greeks Methods

| Method | Status | Applicable To |
|--------|--------|---------------|
| Finite Differences (central) | Implemented | All instruments |
| Common Random Numbers (CRN) | Implemented | MC instruments |
| Pathwise Differentiation | Implemented | European vanilla (GBM) |
| Likelihood Ratio Method | Implemented | Discontinuous payoffs (barriers, digitals) |

---

## Core Infrastructure Assessment

### Day Count Conventions (8 supported)

| Convention | Status | Notes |
|-----------|--------|-------|
| ACT/360 | Correct | Money market standard |
| ACT/365F | Correct | GBP money markets |
| ACT/365L | Correct | French bonds (AFB), leap-year aware |
| 30/360 US | Correct | Corporate bonds, EOM per ISDA 2006 section 4.16(f) |
| 30E/360 | Correct | Eurobond (both dates capped at 30) |
| ACT/ACT (ISDA) | Correct | US Treasuries, splits across year boundaries |
| ACT/ACT (ICMA) | Correct | International bonds, requires frequency context |
| BUS/252 | Correct | Brazilian standard, requires calendar context |

### Calendar Support

- **100+ financial centers**: NYSE, LSE, TSE, HKEX, TARGET2, USGS, Fed, ECB, BOE, BOJ, etc.
- **O(1) bitset lookup**: Pre-computed for 1970-2150
- **Composite calendars**: Union/intersection for multi-currency products
- **Business day adjustments**: Following, Modified Following, Preceding
- **Rule-based**: Easter, IMM (3rd Wednesday), lunar calendars

### Curve Types (7 supported)

| Curve | Interpolation | Arbitrage-Free |
|-------|--------------|----------------|
| DiscountCurve | 5 methods | MonotoneConvex/LogLinear guarantee positive forwards |
| ForwardCurve | 5 methods | Same guarantees |
| HazardCurve | 5 methods | Survival probability monotone decreasing |
| InflationCurve | 5 methods | CPI expectations |
| BaseCorrelationCurve | 5 methods | Structured credit |
| PriceCurve | 5 methods | Forward commodity/index |
| VolatilityIndexCurve | 5 methods | VIX/VSTOXX term structure |

### Interpolation Strategies

| Method | Arbitrage-Free | C1 Continuous | Monotone | Use Case |
|--------|---------------|---------------|----------|----------|
| LinearDf | May fail | No | No | Testing only |
| LogLinearDf | Yes | No | Yes | Standard money market |
| MonotoneConvex | Yes | Yes | Yes | Production (preferred) |
| CubicHermite | Conditional | Yes | Yes | Display, bounded moves |
| PiecewiseQuadraticForward | Yes | Yes | Yes | Long-dated curves |

### Calibration Framework

- **Plan-driven**: JSON/YAML-serializable calibration plans
- **Methods**: Sequential bootstrap + global optimization (Newton/Levenberg-Marquardt)
- **Targets**: DiscountCurve, ForwardCurve, HazardCurve, InflationCurve, BaseCorrelation, VolSurface
- **Validation**: Shape/domain checks, rate bounds, monotonicity
- **Diagnostics**: Per-step residuals, convergence metrics, curve snapshots
- **Tests**: Bloomberg accuracy parity, QuantLib parity, failure mode coverage

### P&L Attribution Framework

| Method | Description | Speed | Accuracy |
|--------|-------------|-------|----------|
| Parallel | Independent factor isolation, cross-effects in residual | O(n_factors * pricing_time) | Good for small moves |
| Waterfall | Sequential factor application, minimal residual | O(n_factors * pricing_time) | Best (by construction) |
| Metrics-Based | Linear approximation via Greeks | O(1) after Greeks | Good for small moves |

**Attribution Factors**: Carry, RatesCurves, CreditCurves, InflationCurves, Correlations, FX, Volatility, ModelParameters, MarketScalars

**Features**: Per-tenor rate bucketing, property-based invariant testing (buyer+seller=0, zero-change=zero-P&L)

### Unified Pricer

- **73 instrument types** with enum-based dispatch (no string matching)
- **24+ pricing models** (ModelKey enum)
- **Case-insensitive parsing** with alias support ("swap" -> IRS, "capfloor" -> CapFloor)
- **Non-exhaustive enums** for future extension without breaking changes

### Serialization

- Full serde round-tripping for all types
- `MarketContextState` captures complete market snapshot
- Fixed-point `Money` type prevents floating-point drift
- Currency-safe arithmetic (refuses mixed-currency operations)

---

## Smell Test Results

| Test | Result | Notes |
|------|--------|-------|
| Par swap rate -> NPV = 0 | PASS | IRS pricer with fast-path OIS identity |
| DF(0) = 1.0 | PASS | Guaranteed by curve construction |
| Kahan summation for cashflows | PASS | Explicitly used in IRS, bond pricers |
| Cap - Floor = Swap parity | NOT TESTED | No integration test documented |
| Swaption parity (P - R = Swap) | NOT TESTED | No integration test documented |
| CDS bootstrap round-trip | PASS | Bootstrapper validates round-trip accuracy |
| CIP (FX forward) | PASS | F = S * DF_f / DF_d verified |
| Bond par price at issue = 100 | PASS | YTM solver with Brent guarantees bracketed convergence |
| Duration sign (positive) | PASS | Macaulay/modified duration formulas correct |
| Clean + accrued = dirty | PASS | Separate calculation, verified |
| Futures rate > forward rate | PASS | Hull-White CA: `0.5 * σ² * T₁ * T₂` from vol surface |
| American >= European | PASS | Leisen-Reimer tree guarantees this |
| Asian <= vanilla | PASS | Turnbull-Wakeman moment-matching respects this bound |
| Recovery sensitivity (higher R -> lower spread) | PASS | Correct sign in CDS parameters |
| JTD = (1-R) * Notional (at zero spread) | PASS | JTD calculation includes accrued premium |

---

## Recommendations

### Critical (fix immediately)

- [x] **Barrier `value()` dispatch**: Already correct — dispatches to MC when `use_gobet_miri=true`. Stale doc comment fixed.
- [x] **LookbackOption discrete monitoring**: Added `use_gobet_miri` field with MC dispatch (same pattern as `BarrierOption`).
- [x] **Cap/Floor negative rate handling**: Added `CapFloorVolType::Auto` that auto-selects Normal for negative forwards. Tests confirm correct dispatch.

### Major (next release)

- [x] **FX delta-based vol surface**: Added `FxDeltaVolSurfaceBuilder` that converts 25D RR/BF/ATM DNS to strike-based `VolSurface`.
- [x] **CDS tranche heterogeneous pricing**: Already implemented. Per-issuer curves, recovery, weights supported via `CreditIndexData`. Stale doc comment fixed.
- [x] **SABR/Heston/Student-t calibration**: SABR (`SABRCalibrator`) and Heston (`calibrate_heston()`) already existed. Student-t df calibration added to plan-driven framework.
- [x] **Key-rate duration for all bonds**: Already registered at `bond/metrics/mod.rs:117-119` via `triangular_key_rate()`.
- [x] **Repo curve separation**: Added `repo_curve_id: Option<CurveId>` to DollarRoll and BondFuture for financing/carry calculations.
- [x] **FX ATM strike factory**: Already existed: `atm_forward_strike()` and `atm_dns_strike()` at `fx_option/types.rs:337-369`.
- [x] **Bermudan equity options**: Added `exercise_schedule` field and wired to `BinomialTree::price_bermudan()`.
- [x] **Base correlation arbitrage validation**: Already enabled by default (`enforce_el_monotonicity: true`).
- [x] **IR Future convexity adjustment**: Already implemented as Hull-White CA at `ir_future/types.rs:367-398`.

### Moderate (backlog)

- [x] **CMS Option vector validation**: Added length check for fixing_dates, payment_dates, accrual_fractions in `value()`.
- [ ] **Barrier option analytical Greeks**: Add finite-difference Greeks on Reiner-Rubinstein as fallback when MC feature disabled.
- [x] **American option tree depth**: Made configurable via `pricing_overrides.model_config.tree_steps`.
- [ ] **Local vol PDE solver**: Add Crank-Nicolson or ADI for pricing under Dupire dynamics.
- [ ] **Builder field naming standardization**: Harmonize start_date/maturity/expiry/end_date across all instruments.
- [ ] **Pathwise Greeks for exotics**: Extend beyond vanilla European to Asian and smooth exotic payoffs.
- [x] **Convertible soft-call validation**: Added `SoftCallTrigger::validate()` checking threshold_pct > 100% and required_days_above <= observation_days.
- [x] **Variance swap ATM fallback warning**: Added `tracing::warn!` when Carr-Madan replication falls back to vol_atm^2.
- [x] **Structured credit MC variance reduction**: Wired antithetic variates flag through `price_monte_carlo()`.

### Strategic Gaps (roadmap)

- [ ] **XVA framework**: CVA/DVA/FVA with exposure simulation and netting sets.
- [ ] **Portfolio-level risk aggregation**: Batch pricing, delta caching, cross-gamma matrices.
- [ ] **Amortizing/step-down swaps**: Parameterized notional schedules for floating-rate hedges.
- [ ] **Multi-asset exotic pricing**: Rainbow options, worst-of baskets with correlation modeling.
- [ ] **Commodity spread options**: Crack, crush, spark spreads for energy desks.
- [ ] **Quanto CDS**: FX-adjusted settlement for cross-currency credit trading.
- [ ] **Real-time Greeks infrastructure**: Simultaneous bump scheduling, delta/gamma caching for intraday risk.
- [ ] **Stochastic rates coupling with MC**: Hull-White + equity/credit in single simulation.
- [ ] **SIMM margin integration**: Expose Marginable trait cleanly in public API.

---

## Architecture Highlights

### What Works Well

1. **Trait-based extensibility**: `Instrument`, `CashflowProvider`, `CurveDependencies`, `Marginable` traits mean adding instruments does not require modifying core code -- just implement traits and register metrics.

2. **Convention registry pattern**: JSON-driven convention lookup for CDS (`cds_conventions.json`) and IRS (`ConventionRegistry`) treats conventions as data, not code. Regional rollouts (new currencies, new RFR benchmarks) become configuration changes.

3. **Enum-based dispatch**: `InstrumentType` (73 variants) and `ModelKey` (24+ variants) with `From<str>` parsing eliminates string-matching bugs. Non-exhaustive enums allow extension without breaking changes.

4. **Metric registry pattern**: Metrics registered per-instrument type via `register_*_metrics()` functions. New metrics add without touching pricer code.

5. **P&L attribution maturity**: 3 methods (parallel, waterfall, metrics-based) with per-tenor bucketing and property-based invariant testing. Closer to a risk system than a pricing library.

6. **Deterministic computation**: Seeded RNG (Philox counter-based), fixed-point Money type, full serde round-tripping enable audit trails and regulatory reproducibility.

7. **Kahan-compensated summation**: Used throughout swap and bond pricers. Prevents floating-point drift on large cashflow sums (30Y swap = 120+ periods).

8. **MonotoneConvex interpolation default**: Guarantees positive forward rates on discount curves. Prevents the most common curve construction arbitrage.

### What Could Improve

1. **Calibration coverage**: Infrastructure is excellent (plan-driven, validation, diagnostics). SABR and Heston calibration are available; Student-t copula df calibration has been added. Additional calibration targets (e.g., local vol, jump-diffusion) would further close the gap.

2. **Vol surface abstraction**: `FxDeltaVolSurfaceBuilder` now converts delta-quoted vols to strike-based surfaces. Rates vol surfaces still need expiry/tenor parameterization. Consider strategy pattern for additional surface types.

3. **Portfolio layer**: All pricing is per-instrument. No batch pricing, risk aggregation, or cross-gamma computation. A desk needs this for EOD risk runs. This is in the portfolio crate.

4. **Feature flag dependency**: Barrier option Greeks, MC pricers, and some exotic features are behind `#[cfg(feature = "mc")]`. This creates surprising capability differences between builds.
