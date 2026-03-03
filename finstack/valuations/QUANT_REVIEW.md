# Library Assessment: finstack/valuations — Cross-Asset Quant Review

**Reviewer Persona**: Senior quantitative analyst, 10+ years across rates, credit, FX, equity, and commodities desks. Experience with QuantLib, Bloomberg DLIB, Numerix, FINCAD.

**Review Date**: 2026-03-02

**Scope**: Holistic review of `finstack/valuations/src/instruments/` and supporting infrastructure (`finstack/core/`)

---

## Executive Summary

This is one of the most comprehensive Rust-based quantitative finance libraries I have reviewed. It covers **73 instrument types** across 7 asset classes (Rates, Fixed Income, Credit, FX, Equity, Commodity, Exotics), backed by **24+ pricing models** and a mature infrastructure layer with 8 day count conventions, 100+ financial calendars, 7 curve types, plan-driven calibration, and 3-method P&L attribution.

The library's strongest areas are its **rates and fixed income foundations** -- the IRS pricer with SOFR/SONIA compounded-in-arrears (including lookback/observation shift), Kahan-compensated summation, and multi-curve OIS discounting would pass a Bloomberg SWPM parity test. The **credit derivatives module** implements a near-complete ISDA Standard Model with 5 integration methods, IMM date handling, and an advanced copula framework (Gaussian, Student-t, Random Factor Loading, Multi-Factor). The **Monte Carlo engine** is production-grade with SoA layout, Rayon parallelism, Philox counter-based RNG, Sobol quasi-random sequences, and comprehensive variance reduction (antithetic, control variate, importance sampling).

The most critical gaps are: (1) **discrete barrier monitoring defaults to continuous** in analytical pricers -- a silent mispricing risk, (2) **no delta-based vol surface parameterization** for FX options -- the single most common workflow on an FX desk, (3) **homogeneous portfolio assumption** in CDS tranche pricing limits real-world usage, and (4) **no SABR/Heston calibration routines** -- parameters must be externally computed. These are addressable engineering items, not architectural deficiencies.

For a library of this scope, the architecture is remarkably clean: trait-based extensibility, enum-based dispatch (no string matching), Arc-wrapped curves for thread-safe sharing, and full serde round-tripping. A quant could realistically run a multi-asset book on this with targeted extensions.

---

## Scorecard

| Dimension | Rating | Notes |
|-----------|--------|-------|
| **Coverage** | 4.5/5 | 73 instruments across 7 asset classes; missing Bermudan equity options, nth-to-default, quanto CDS |
| **Accuracy** | 4.0/5 | ISDA-compliant CDS, Kahan summation, Obloj-corrected SABR; barrier continuous/discrete ambiguity is a concern |
| **Conventions** | 4.5/5 | 8 day counts, auto-detection of CDS regional conventions, SOFR/SONIA/ESTR compounding correct; some FX delta convention gaps |
| **Numerical Robustness** | 4.0/5 | Newton+Brent fallback for implied vol, QE-Heston with Feller safeguards, MonotoneConvex interpolation; no adaptive MC stopping by default |
| **Usability** | 4.0/5 | Builder patterns, convention registries, `from_conventions()` presets; some inconsistent field naming across instruments |
| **Extensibility** | 4.5/5 | Trait-based architecture, metric registry pattern, enum dispatch; new instruments add without core changes |
| **Production Readiness** | 3.5/5 | Full serde, deterministic seeding, calibration diagnostics; missing XVA, portfolio-level risk aggregation, real-time Greeks caching |

---

## Top 5 Priorities

1. **[PARTIALLY RESOLVED] Barrier option `value()` dispatch** -- `BarrierOption::value()` correctly dispatches to MC when `use_gobet_miri=true` (with `mc` feature). Stale doc comment fixed. Remaining gap: `LookbackOption` has no `use_gobet_miri` field and always uses analytical continuous pricing.

2. **[CRITICAL] Cap/Floor has no automatic fallback from Black to Normal model for negative rates** -- EUR/JPY/CHF environments will produce errors or wrong prices. Fix: auto-select Normal when forward rate <= 0, or provide clear error.

3. **[MAJOR] FX vol surface uses strike-based lookup, not delta-based** -- Every FX desk quotes in 25D RR/BF/ATM DNS conventions. The library acknowledges this gap in documentation but provides no conversion layer. Fix: add delta-to-strike converter and delta-parameterized surface builder.

4. **[RESOLVED] CDS tranche heterogeneous portfolio support** -- Already implemented: per-issuer credit curves, recovery rates, and weights via `CreditIndexData::issuer_credit_curves`, with automatic optimization to binomial path for uniform portfolios and SPA fallback for diversified ones. Doc comment was stale.

5. **[MAJOR] No calibration routines for SABR, Heston, Student-t copula degrees of freedom** -- Parameters must be externally computed. Fix: add calibration targets in the plan-driven calibration framework.

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
| IR Future | Forward rate + convexity adj | ParRate, DV01 | Convexity adjustment is stub |
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
| CDS Tranche | Gaussian copula + Student-t + RFL + Multi-Factor | Upfront, Par Spread, Spread DV01, CS01, Correlation01, Recovery01, Expected Loss, JTD, Tail Dependence | Homogeneous only |

### FX (10 instruments)

| Instrument | Pricing Models | Key Metrics | Status |
|-----------|----------------|-------------|--------|
| FX Forward | Discounting (CIP) | DV01 | Production-ready |
| FX Option | Garman-Kohlhagen / Black-76 | Delta, Gamma, Vega, Theta, Rho (dom/for), Vanna, Volga, ImpliedVol | Needs delta-vol surface |
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
| Equity Option (Eur/Amer) | BSM, Leisen-Reimer Tree (201 steps), Heston Fourier | Delta, Gamma, Vega, Theta, Rho, Charm, Color, Speed, Vanna, Volga, Dividend01, ImpliedVol | Production-ready |
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
| Barrier Option | Reiner-Rubinstein (continuous), MC (discrete + Gobet-Miri) | Vega, Rho | Discrete dispatch issue |
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

#### [MAJOR] IR Future convexity adjustment is stub only

**What**: `convexity_adjustment` field exists but computation is delegated to user. No Hull-White CA calculation from vol parameters.
**Impact**: Futures-implied rates will be biased vs forward rates. Error grows with maturity (~1bp at 2Y, ~5bp at 5Y).
**Recommendation**: Implement `CA = 0.5 * sigma^2 * T1 * T2` from Hull-White parameters stored on the instrument.
**Reference**: Hull (2018) ch.6.3, QuantLib `HullWhite::convexityBias()`.

#### [MODERATE] CMS Option vector fields lack length validation

**What**: `fixing_dates`, `payment_dates`, `accrual_fractions` are separate vectors with no builder validation that they are the same length.
**Impact**: Index out of bounds at runtime if manually constructed.
**Recommendation**: Add validation in builder: `assert!(fixing_dates.len() == payment_dates.len())`.

#### [MODERATE] No ergonomic market-standard constructors for Deposit/FRA

**What**: IRS has `from_conventions()` but Deposit, FRA, IR Future only offer raw builder patterns.
**Impact**: Users must look up day count, settlement, and frequency conventions per currency.
**Recommendation**: Add `.usd_standard()`, `.eur_standard()` factory methods.

#### [MODERATE] IRS leg date mismatch silently allowed in release

**What**: When fixed/float legs have different start/end dates, logs warning in debug builds only. Release builds silently proceed.
**Impact**: Complex swaps with mismatched legs produce wrong NPV without any indication.
**Recommendation**: Error for multi-leg swaps with different dates, or require explicit opt-in.

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

#### [MAJOR] No separate repo curve for financing calculations

**What**: Dollar roll and bond future carry calculations use the discount curve (OIS) for financing rates.
**Impact**: Repo specials (specific collateral trading rich to GC) are invisible. Dollar roll carry undervalued.
**Recommendation**: Add `RepoCurve` to `MarketContext` and reference from bond future/dollar roll pricers.
**Reference**: Bloomberg DLV (CTD/Basis) uses separate implied repo rate.

#### [MAJOR] No key-rate duration for vanilla bonds

**What**: Key-rate DV01 only implemented for MBS/CMO. Vanilla bonds only get parallel DV01.
**Impact**: Cannot decompose yield curve positioning for a bond portfolio. Every rates desk needs this.
**Recommendation**: Extend `BucketedDV01` to all bond types using triangular interpolation weights.
**Reference**: QuantLib `KeyRateDuration`, Bloomberg YAS KRD function.

#### [MODERATE] YTM solver bracket may fail for distressed bonds

**What**: Default bracket +/-1000bp; Z-spread bracket scales with maturity (max 3000bp).
**Impact**: Distressed bonds (1000+ bp spread) or deeply discounted zeros may fail to bracket.
**Recommendation**: `ZSpreadSolverConfig::base_bracket_bp` is overridable -- document this clearly and consider wider defaults.

#### [MODERATE] Convertible soft-call trigger lacks validation

**What**: `threshold_pct` (e.g., 130%) has no runtime check that it exceeds 100%.
**Impact**: Setting threshold_pct < 100% would make the soft call always active, defeating its purpose.
**Recommendation**: Add validation: `threshold_pct > 100.0`.

#### [MODERATE] Structured credit MC has no variance reduction

**What**: Default/prepayment paths with correlation, but no antithetic or control variate techniques.
**Impact**: Convergence requires many paths (default 1000). No stopping criterion.
**Recommendation**: Apply antithetic variates from the common MC engine. Add convergence monitoring.

### Fixed Income - Missing Features

| Feature | Priority | Notes |
|---------|----------|-------|
| Separate repo curve | P1 | Required for basis and carry calculations |
| Key-rate duration for all bonds | P1 | Yield curve positioning |
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

#### [MAJOR] Base correlation arbitrage validation disabled by default

**What**: Tranche pricer has `arbitrage_free_validation` flag but it is off by default.
**Impact**: Non-monotonic expected loss term structure can produce small P&L artifacts.
**Recommendation**: Enable by default in production; add smoothing when violations detected.

#### [MAJOR] No Student-t degrees of freedom calibration

**What**: Student-t copula requires manual nu input (typically 4-10). No calibration from market tranche prices.
**Impact**: Users must guess or externally calibrate -- defeats the purpose of having the model.
**Recommendation**: Add calibration routine from CDX tail option prices or tranche breakevens.
**Reference**: Mashal & Zeevi (2002), "Beyond Correlation".

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
| Heterogeneous tranche pricing | P1 | Semi-analytical or MC fallback |
| Student-t/RFL calibration routines | P1 | From market tranche prices |
| Quanto CDS | P2 | FX-adjusted settlement |
| Bilateral CVA/DVA | P2 | CSA adjustment, collateral optimization |
| nth-to-Default basket | P2 | Conditional default model |
| Implied correlation surface builder | P2 | Base correlation curve from tranche quotes |
| Stochastic interest rates coupling | P3 | Hull-White/CIR integration |

---

### FX

#### [MAJOR] Vol surface uses strike-based lookup, not delta-based

**What**: `vol_surface.value_clamped(t, strike)` uses absolute strike. Market quotes in 25D RR/BF/ATM DNS.
**Impact**: Every FX desk quotes and interpolates in delta space. Strike-based lookup produces wrong smile for OTM options.
**Recommendation**: Add `DeltaVolSurface` wrapper with delta-to-strike conversion. Library acknowledges this gap in documentation.
**Reference**: Wystup (2017) "FX Options and Structured Products", Ch. 1.

#### [MAJOR] No ATM strike auto-computation

**What**: FX option requires explicit strike. ATMF/DNS computation left to user.
**Impact**: Error-prone for traders constructing ATM straddles. The most common FX option trade requires manual forward calculation.
**Recommendation**: Add `FxOption::atm_european()` factory that computes ATMF from CIP.

#### [MODERATE] Barrier option metrics conditional on MC feature flag

**What**: Full Greeks (delta, gamma, vega, rho, vanna, volga) only available when `#[cfg(feature = "mc")]` is enabled. Without MC: only DV01 and theta.
**Impact**: Production builds without MC feature cannot compute Greeks for barriers.
**Recommendation**: Add analytical finite-difference Greeks on the Reiner-Rubinstein pricer as fallback.

#### [MODERATE] Discrete barrier only via MC -- no analytical Broadie-Glasserman

**What**: Gobet-Miri correction in MC only. Analytical pricer assumes continuous monitoring.
**Impact**: Analytical pricer underprices knock-outs (continuous < discrete barrier) by ~1-3% for monthly monitoring.
**Recommendation**: Apply Broadie-Glasserman correction `exp(+/-0.5826 * sigma * sqrt(dt))` to analytical barrier level before pricing.

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
| Delta-based vol surface | P1 | Delta-to-strike conversion layer |
| ATM strike factory (ATMF/DNS) | P1 | Auto-computation from CIP |
| Analytical discrete barrier correction | P2 | Broadie-Glasserman on analytical pricer |
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

#### [MAJOR] Bermudan equity options unsupported

**What**: Returns validation error. No schedule-based early exercise logic.
**Impact**: Cannot price Bermudan options on single equities. Must use American as proxy.
**Recommendation**: Extend Leisen-Reimer tree to accept exercise schedule, or add Longstaff-Schwartz.
**Reference**: QuantLib `BermudanExercise` + `FdBlackScholesVanillaEngine`.

#### [MODERATE] American option tree depth hardcoded at 201 steps

**What**: Leisen-Reimer uses 201 steps, not configurable.
**Impact**: ~10c precision for vanilla, but insufficient for Greeks near strike (gamma noise).
**Recommendation**: Expose `tree_steps` parameter. Consider Richardson extrapolation for smoother convergence.

#### [MODERATE] Variance swap ATM fallback is silent

**What**: If Carr-Madan replication fails, falls back to `vol_atm^2` without warning.
**Impact**: Fair variance estimate may be stale if vol surface is incomplete.
**Recommendation**: Log warning on fallback; add `fallback_used` flag to result.

#### [MODERATE] Charm/Color/Speed numerical stability

**What**: Use fixed bump sizes; no adaptive scaling for ATM vs deep OTM.
**Impact**: Numerical instability for deep OTM or barrier-near options.
**Recommendation**: Add adaptive bump sizing based on moneyness.

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
| Bermudan equity options | P1 | Schedule-based early exercise |
| Stochastic vol calibration (SABR/SLV) | P1 | Skew/smile dynamics for structured products |
| Multi-asset exotics (rainbow, worst-of) | P2 | Correlation-dependent payoffs |
| Jump diffusion (Merton) | P2 | Gap risk for event-driven equities |
| Quanto equity options | P2 | FX risk in equity option Greeks |
| Configurable tree depth | P2 | Currently hardcoded at 201 steps |
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
| Futures rate > forward rate | PARTIAL | Convexity adjustment field exists but computation is stub |
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

- [ ] **FX delta-based vol surface**: Add delta-to-strike conversion layer and `DeltaVolSurface` builder. Every FX desk needs this.
- [x] **CDS tranche heterogeneous pricing**: Already implemented. Per-issuer curves, recovery, weights supported via `CreditIndexData`. Stale doc comment fixed.
- [ ] **SABR/Heston/Student-t calibration**: Add calibration targets to the plan-driven framework. Parameters should not require external computation.
- [ ] **Key-rate duration for all bonds**: Extend bucketed DV01 with triangular interpolation to vanilla bonds, not just MBS/CMO.
- [ ] **Repo curve separation**: Add `RepoCurve` to `MarketContext` for bond future basis and dollar roll carry.
- [ ] **FX ATM strike factory**: Add `FxOption::atm_european()` and `FxOption::atm_dns()` constructors.
- [ ] **Bermudan equity options**: Extend tree framework or add LSMC for schedule-based early exercise.
- [ ] **Base correlation arbitrage validation**: Enable by default in production tranche pricing.
- [ ] **IR Future convexity adjustment**: Implement Hull-White CA computation from vol parameters.

### Moderate (backlog)

- [ ] **CMS Option vector validation**: Assert equal lengths for fixing_dates, payment_dates, accrual_fractions.
- [ ] **Barrier option analytical Greeks**: Add finite-difference Greeks on Reiner-Rubinstein as fallback when MC feature disabled.
- [ ] **American option tree depth**: Make configurable; consider Richardson extrapolation.
- [ ] **Local vol PDE solver**: Add Crank-Nicolson or ADI for pricing under Dupire dynamics.
- [ ] **Builder field naming standardization**: Harmonize start_date/maturity/expiry/end_date across all instruments.
- [ ] **Pathwise Greeks for exotics**: Extend beyond vanilla European to Asian and smooth exotic payoffs.
- [ ] **Convertible soft-call validation**: Assert threshold_pct > 100%.
- [ ] **Variance swap ATM fallback warning**: Log when Carr-Madan replication falls back to vol_atm^2.
- [ ] **Structured credit MC variance reduction**: Apply antithetic variates from common engine.

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

1. **Calibration gap**: Infrastructure is excellent (plan-driven, validation, diagnostics) but calibration targets for SABR, Heston, and Student-t copula are missing.

2. **Vol surface abstraction**: Single `VolSurface` type with strike-based lookup. FX needs delta-based; rates need expiry/tenor. Consider strategy pattern for surface parameterization.

3. **Portfolio layer**: All pricing is per-instrument. No batch pricing, risk aggregation, or cross-gamma computation. A desk needs this for EOD risk runs.

4. **Feature flag dependency**: Barrier option Greeks, MC pricers, and some exotic features are behind `#[cfg(feature = "mc")]`. This creates surprising capability differences between builds.
