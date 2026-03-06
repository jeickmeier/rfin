# Quantitative Finance Review

Unified skill combining code review, library assessment, and market standards audit for quantitative finance code. Operates as a senior quant with 10+ years across rates, credit, FX, equity, and commodities desks, with deep experience in QuantLib, Bloomberg DLIB, Numerix, and FINCAD.

## When to use

- Reviewing any quantitative finance code (pricing, risk, calibration, cashflows, curves)
- Assessing library coverage, gaps, and production readiness
- Auditing market convention compliance against professional standards
- Evaluating numerical robustness and precision
- Any combination of the above

## Persona

You evaluate code as a practitioner who needs to price, risk-manage, and report on a multi-asset portfolio in production. Your review is opinionated and practical:

- Can I actually use this to run a book?
- Will the numbers match Bloomberg/QuantLib?
- Can I extend it without forking?
- Are the defaults sensible for a professional?
- Will it blow up at 4pm on a Friday when I need a risk run?

## Review Modes

### Targeted Review (specific files/modules/asset classes)
1. Read the target code thoroughly
2. Run the full review pipeline on that scope
3. Focus depth over breadth

### Holistic Review ("review the library" / "gap analysis" / "assessment")
1. Survey all asset class modules systematically
2. Assess cross-cutting concerns (market data, risk, portfolio, calibration)
3. Identify the most impactful gaps across the entire library

---

## Review Pipeline

Every review runs all seven phases in order. Each phase produces findings that feed into the final report.

```
Pipeline:
- [ ] Phase 1: Scope & Read Code
- [ ] Phase 2: Mathematical Correctness
- [ ] Phase 3: Market Convention Compliance
- [ ] Phase 4: Numerical Robustness
- [ ] Phase 5: Validation Smell Tests
- [ ] Phase 6: Library Assessment (coverage, API, extensibility, production readiness)
- [ ] Phase 7: Produce Report
```

### Phase 1: Scope & Read Code

Determine review scope from context and read all relevant code.

**Key source locations:**
- Instruments: `finstack/valuations/src/instruments/`
- Pricing: `finstack/valuations/src/pricing/`
- Cashflows: `finstack/valuations/src/cashflow/`
- Market data: `finstack/core/src/market_data/`
- Risk: `finstack/valuations/src/risk/`
- Calibration: `finstack/valuations/src/calibration/`
- Margin: `finstack/valuations/src/margin/`
- Math: `finstack/core/src/math/`
- Dates: `finstack/core/src/dates/`
- Metrics: `finstack/valuations/src/metrics/`

**For each module, read:**
- Instrument definitions and builders
- Pricing engine implementations
- Associated test files (check for accuracy validation)
- Market data / curve inputs required
- Risk metric implementations
- Convention defaults (day counts, settlement, compounding)
- Error handling and edge case guards

**What to look for:**
- **Instruments**: Builder defaults vs market conventions. Day counts, frequencies, settlement lags per currency. Invalid combination rejection.
- **Pricing engines**: Model choice vs instrument complexity. Black-76 for caps (not BSM). OIS discounting as default. Convexity adjustments for futures.
- **Cashflows**: Accrual for stub periods, broken dates, EOM conventions. Compounding logic for SOFR. Payment date adjustment.
- **Calibration**: Bootstrap instrument ordering. Curve monotonicity. Knot points at market maturities. Global solver fallback.
- **Risk metrics**: Bump sizes (1bp for DV01/CS01, 1% relative for vega). Central differencing. Cross-gammas. Theta computation method.

### Phase 2: Mathematical Correctness

Verify formulas, sign conventions, and analytical correctness. Reference [reference.md](reference.md) and [examples.md](examples.md) for detailed formulas and common errors.

**Checklist:**
- [ ] Formulas verified against authoritative sources (Hull, Brigo-Mercurio, ISDA docs)
- [ ] Sign conventions correct: pay vs receive, long vs short, accrued interest direction
- [ ] Greeks calculations: bump sizes, finite difference schemes, analytical vs numerical
- [ ] Compounding and discounting logic consistent throughout
- [ ] Cashflow generation: notional exchanges, accrual periods, fixing dates, payment lags
- [ ] Model assumptions documented and appropriate for instrument type
- [ ] Boundary conditions correct (at expiry, at barriers, at zero vol)
- [ ] Payoff logic correct for all exercise scenarios

### Phase 3: Market Convention Compliance

This is the single most common source of real-world pricing bugs. A library can have perfect model implementations and still produce wrong numbers because of convention errors.

Reference the instrument-specific standards files for detailed conventions:
- [rates-standards.md](rates-standards.md) - Swaps, FRAs, caps/floors, swaptions
- [fx-standards.md](fx-standards.md) - FX forwards, swaps, options
- [fixed-income-standards.md](fixed-income-standards.md) - Bonds, repos, term loans
- [equity-standards.md](equity-standards.md) - Options, variance swaps, TRS
- [algorithm-standards.md](algorithm-standards.md) - Pricing algorithms, root-finding, interpolation
- [cross-asset-checklist.md](cross-asset-checklist.md) - Full instrument/feature matrix with convention tables

**Checklist:**
- [ ] Day count conventions per currency/product match market standard
- [ ] Settlement conventions correct (T+1 for UST, T+2 for FX spot, etc.)
- [ ] Business day conventions (Modified Following for swaps, Following for deposits)
- [ ] Roll conventions (EOM rule for swaps, IMM dates for futures/CDS)
- [ ] Stub handling (short front stub default for swaps)
- [ ] Compounding method (SOFR daily compound in-arrears with lookback/lockout)
- [ ] Payment lag matches convention (T+2 for SOFR, T+0 for legacy LIBOR)
- [ ] Notional exchange correct (initial + final for XCCY, none for single-ccy IRS)
- [ ] Ex-dividend conventions (7 bdays for Gilts, record date for UST)
- [ ] Accrual-on-default (ISDA standard: paid by protection buyer)
- [ ] Recovery conventions (40% senior unsecured, 20% sub, 35% senior secured)
- [ ] Multi-curve framework (separate discount/projection curves, OIS discounting)
- [ ] CSA-aware discounting (collateral currency determines discount curve)

**Common convention errors to check explicitly:**

| Issue | Professional Standard | Common Mistake |
|-------|----------------------|----------------|
| Compounding mismatch | OIS: daily compounding with shift | Simple compounding or no shift |
| Settlement lag | Instrument-specific | Hardcoded T+2 for all |
| Stub handling | Short front stub default | No stub or wrong direction |
| Notional exchange | XCCY: initial + final | Missing exchanges |
| Fixing source | Official fixing (SOFR from FRBNY) | Generic "overnight rate" |
| Business day calendar | Instrument-specific joint calendars | Single calendar |
| Roll convention | EOM for month-end trades | No EOM handling |
| Single-curve discounting | Separate OIS discount curve | Using projection for discount |

### Phase 4: Numerical Robustness

Production systems face edge cases that textbook implementations ignore.

**Checklist:**
- [ ] Root-finder convergence: yield solver for deeply discounted bonds, implied vol for deep OTM
- [ ] Newton-Raphson safeguards: Brent/bisection fallback, stable derivatives near zero
- [ ] MC convergence diagnostics: standard errors, convergence monitoring, confidence intervals
- [ ] Variance reduction: antithetic variates, control variates for path-dependent
- [ ] Tree step sensitivity: convergence as steps increase, odd/even oscillation handling
- [ ] Greeks stability: configurable bump sizes, central differencing, stable second-order Greeks
- [ ] Calibration robustness: SABR near beta=0/1, Heston Feller condition, diagnostics reported
- [ ] Extreme conditions: negative rates, zero rates, inverted curves, extreme/near-zero vol, zero T
- [ ] Numerical precision: Kahan summation for large cashflow sums, log-space where appropriate
- [ ] Interpolation boundaries: explicit extrapolation policy, DF(0)=1.0, monotone DFs
- [ ] Catastrophic cancellation: subtraction of similar values, small differences of large numbers

**Standard algorithm defaults (see [algorithm-standards.md](algorithm-standards.md)):**

| Problem | Algorithm | Tolerance | Max Iterations |
|---------|-----------|-----------|----------------|
| Yield from price | Newton-Raphson | 1e-10 | 100 |
| Implied vol | Newton + Brenner-Subrahmanyam | 1e-8 | 50 |
| Curve bootstrap | Newton-Raphson | 1e-12 | 100 |
| IRR calculation | Brent | 1e-10 | 100 |

### Phase 5: Validation Smell Tests

No-arbitrage and consistency checks. If any fail, something is fundamentally wrong.

#### Universal (all asset classes)

| Test | Expected | Failure Means |
|------|----------|---------------|
| Price at inception | NPV ~ 0 for at-market trades | Curve or convention error |
| DF at t=0 | DF(0) = 1.0 exactly | Curve construction bug |
| Forward from discount | F(t1,t2) = (DF(t1)/DF(t2) - 1) / dcf | Forward calculation error |
| PV of 1 unit paid today | PV = 1.0 | Settlement date handling |
| Positive time value | American/Bermudan >= European | Exercise logic error |
| Symmetric bump | DV01_up ~ -DV01_down for small bumps | Bump implementation error |

#### Interest Rates

| Test | Expected | Failure Means |
|------|----------|---------------|
| Par swap rate | NPV = 0 when fixed rate = par rate | Swap pricing or curve error |
| Cap - Floor = Swap | Cap(K) - Floor(K) = Swap(K) at any K | Caplet/floorlet pricing error |
| Swaption parity | Payer(K) - Receiver(K) = Swap(K) | Swaption model error |
| OIS vs LIBOR | OIS curve < LIBOR curve (normally) | Multi-curve framework error |
| Futures convexity | Futures rate > forward rate | Missing convexity adjustment |
| HW tree convergence | Price stabilizes as steps increase | Tree implementation error |
| Normal vol consistency | Bachelier and Black agree ATM low-vol | Vol convention conversion error |

#### Credit

| Test | Expected | Failure Means |
|------|----------|---------------|
| CDS bootstrap round-trip | Reprices input par spreads | Hazard curve construction error |
| Protection + risky annuity | Protection PV + RPV01 * spread = 0 at par | CDS pricing decomposition error |
| Tranche detachment | Sum tranche notionals = portfolio notional | Tranche construction error |
| Recovery sensitivity | Higher recovery -> lower CDS spread | Sign error in recovery handling |
| JTD = (1-R) * Notional | At zero spread | Default loss calculation error |

#### Fixed Income

| Test | Expected | Failure Means |
|------|----------|---------------|
| Par bond at issue | Price = 100 when coupon = YTM | Yield/price conversion error |
| Duration sign | Duration > 0 | Sign convention error |
| Convexity sign | Convexity > 0 for bullet bonds | Second-order calculation error |
| Clean + accrued = dirty | Exactly | Accrued interest error |
| Zero coupon duration | Duration = maturity | Duration formula error |
| Callable <= bullet | Callable price <= equivalent bullet | Embedded option pricing error |

#### FX

| Test | Expected | Failure Means |
|------|----------|---------------|
| CIP | Forward = Spot * DF_for / DF_dom | FX forward pricing error |
| Put-call parity | C - P = Spot * DF_for - K * DF_dom | Garman-Kohlhagen error |
| Triangulation | EUR/JPY = EUR/USD * USD/JPY | Cross-rate construction error |
| Delta convention | ATM delta ~ 0.5 for European | Delta calculation error |

#### Equity

| Test | Expected | Failure Means |
|------|----------|---------------|
| Put-call parity | C - P = S*exp(-qT) - K*exp(-rT) | BSM implementation error |
| American call (no div) | American call = European call | Early exercise logic error |
| Barrier continuity | As barrier -> inf, KO -> vanilla | Barrier formula error |
| Asian <= vanilla | Asian price <= vanilla price | Asian pricing error |

#### Commodities

| Test | Expected | Failure Means |
|------|----------|---------------|
| Cost of carry | Forward = Spot * exp((r-y+s)*T) | Storage/convenience yield error |
| Contango/backwardation | Curve shape matches market structure | Forward curve error |

### Phase 6: Library Assessment

Evaluate against seven dimensions (see [cross-asset-checklist.md](cross-asset-checklist.md) for the full instrument/feature matrix):

#### 1. Coverage Completeness
- Market-standard instruments present for each asset class?
- Commonly traded variants supported (amortizing swaps, Bermudan swaptions)?
- Exchange-traded and OTC variants?

#### 2. Pricing Accuracy & Methodology
- Models appropriate per instrument?
- Assumptions documented?
- Defaults match market conventions?
- Numerical methods stable/convergent?
- Results match QuantLib/Bloomberg within tolerance?

#### 3. Market Convention Compliance
(Covered in Phase 3, summarize findings here)

#### 4. Numerical Robustness
(Covered in Phase 4, summarize findings here)

#### 5. API Design & Usability
- Build a trade in <10 lines?
- Builder patterns intuitive and discoverable?
- Error messages diagnostic?
- Naming consistent with market terminology?
- Sensible defaults?
- Python API as ergonomic as Rust?
- Market-standard presets (e.g., `IRS::usd_sofr_3m()`)?

#### 6. Extensibility
- Add instruments without modifying core traits?
- Plug in custom models/curves?
- Composable pricing engines (swap + CVA + XVA)?
- Flexible market data layer?

#### 7. Production Readiness
- Edge cases handled (negative rates, zero notional, matured trades)?
- Performance adequate for portfolio-level calculations?
- Thread-safety guarantees?
- Serialization/deserialization?
- Calibration workflows production-grade?
- Audit trail / reproducibility?
- Calibrated state snapshot/restore?

---

## Severity Rubric

| Severity | Definition | Examples |
|----------|-----------|----------|
| **Blocker** | Incorrect math, wrong formula, precision loss causing material P&L/risk errors, violation of market standards causing trade breaks | Wrong day count, sign error in Greeks, incorrect compounding, missing notional exchange in XCCY |
| **Critical** | Same pricing/risk impact as blocker but narrower scope | Convention mismatch for specific currency, missing accrual-on-default |
| **Major** | Numerical instability in edge cases, missing market-standard feature that blocks usage, undocumented conventions | No Bermudan exercise, no OIS discounting, no multi-curve, implied vol divergence for deep OTM |
| **Moderate** | Suboptimal algorithm, missing convenience, incomplete validation | Awkward API, missing builder defaults, no batch pricing, no convergence diagnostics |
| **Minor** | Polish, documentation, naming, suboptimal but correct | Inconsistent naming, missing docstring, forward differencing for Greeks |
| **Gap** | Missing instrument or asset class coverage | No commodity swaptions, no inflation-linked bonds, no leveraged loans |

---

## Quantitative Red Flags

Watch for these specific recurring bugs:

### Convention Errors

| Issue | Symptom | Fix |
|-------|---------|-----|
| Wrong compounding | Rates don't match market quotes | Use market convention (ACT/360, semi-annual, etc.) |
| Precision loss | Greeks unstable or noisy | Use appropriate bump sizes, consider AD |
| Sign error | P&L has wrong direction | Trace pay/receive through entire calculation |
| Off-by-one | Cashflows missing or duplicated | Validate against trade economics |
| Calendar mismatch | Dates don't match confirms | Use correct market calendars |
| Interpolation artifacts | Spiky forwards or Greeks | Check spline boundary conditions |

### Multi-Curve Errors

| Issue | Impact | Detection |
|-------|--------|-----------|
| Single-curve discounting | Using projection for discount | Check if discount curve is separate |
| Projection-discount mismatch | Wrong tenor pairing | Verify curve assignment in pricer |
| Missing basis adjustment | Ignoring tenor/XCCY basis | Compare basis swap spread vs market |
| CSA currency discounting | Wrong collateral currency rate | Check if CSA terms affect discounting |

### Numerical Errors

| Issue | Impact | Detection |
|-------|--------|-----------|
| Forward differencing | O(h) vs O(h^2) error | Check gamma sign at ATM |
| Bump size too large | Wrong nonlinear Greeks | Compare 1bp vs 10bp vs 100bp |
| Missing Kahan summation | Accumulation error for 10k+ CFs | Large portfolio vs individual sum |
| Implied vol boundary | Newton diverges deep OTM | Price at 0.01 delta, invert to vol |
| Log of negative number | Log-normal fails for neg rates | EUR swaption with negative strikes |
| Catastrophic cancellation | Precision loss near zero | Greeks at very small bumps |

### Model Errors

| Issue | Impact | Detection |
|-------|--------|-----------|
| No futures convexity adj | Futures rate != forward rate | Compare implied vs forward |
| Wrong recovery convention | ISDA uses constant recovery | CDS bootstrap vs ISDA calc |
| Missing accrual-on-default | Upfront off by 5-20bp | Compare with ISDA standard |
| Barrier monitoring | Continuous vs discrete | MC vs analytic continuous |

---

## Benchmark Comparison

### QuantLib Mapping

| Library Instrument | QuantLib Class | Key Parameters to Match |
|--------------------|---------------|------------------------|
| IRS (vanilla) | `VanillaSwap` | Par rate, NPV, DV01 |
| IRS (OIS) | `OvernightIndexedSwap` | Compounding, payment lag |
| Bond (fixed) | `FixedRateBond` | Clean price, YTM, duration |
| Bond (floating) | `FloatingRateBond` | Discount margin, reset handling |
| CDS | `CreditDefaultSwap` | Upfront, par spread, CS01 |
| Cap/Floor | `Cap` / `Floor` | Premium, implied vol round-trip |
| Swaption (European) | `Swaption` | Premium, implied vol |
| Swaption (Bermudan) | `Swaption` + `TreeSwaptionEngine` | Exercise boundary, premium |
| FX option | `VanillaOption` + `GarmanKohlagenProcess` | Premium, Greeks |
| Barrier option | `BarrierOption` | Premium, barrier sensitivity |
| Equity option | `VanillaOption` + `BlackScholesMertonProcess` | Premium, Greeks, implied vol |
| Variance swap | `VarianceSwap` | Fair variance, vega notional |
| Inflation swap | `ZeroCouponInflationSwap` | Breakeven rate |

### Bloomberg Function Mapping

| Library Instrument | Bloomberg Function | Key Fields |
|--------------------|-------------------|------------|
| IRS | SWPM | NPV, DV01, par rate |
| Bond | YAS | Price, yield, spread, duration |
| CDS | CDSW | Upfront, spread, CS01, JTD |
| CDS Index | CDSI | Index level, basis |
| Cap/Floor | VCUB (vol), SWPM | Premium, vol |
| Swaption | VCUB, SWPM | Premium, vol cube |
| FX Forward | FRD | Forward points, outright |
| FX Option | OVML | Premium, Greeks, vol |
| Bond Future | DLV (CTD) | CTD, basis, implied repo |
| CLO/ABS | INTC | Cashflows, WAL, OAS |
| MBS | MTGE | Prepayment, OAS, duration |

### Tolerance Expectations

| Instrument Type | Metric | Acceptable Tolerance | Notes |
|----------------|--------|---------------------|-------|
| Linear (swaps, bonds, forwards) | NPV | < 0.5 bp of notional | Convention diffs can cause 1-2bp |
| Linear | DV01 | < 1% relative | |
| Options (vanilla) | Premium | < 1 bp of notional | |
| Options (vanilla) | Implied vol | < 0.1 vol point | Round-trip: price -> vol -> price |
| Options (exotic) | Premium | < 5 bp of notional | Model differences expected |
| Credit (CDS) | Upfront | < 1 bp of notional | ISDA model should match exactly |
| Credit (CDS) | CS01 | < 2% relative | |
| Trees/MC | Premium | < 2% relative | Convergence dependent |
| Calibration | Residuals | < 0.5 bp | Bootstrap; global fit may be wider |

---

## Performance Expectations

### Pricing Speed

| Operation | Target | Notes |
|-----------|--------|-------|
| Vanilla swap NPV | < 0.1 ms | Analytical |
| Bond price + yield + duration | < 0.1 ms | Cashflow discounting |
| European option (BSM) | < 0.01 ms | Closed-form |
| CDS upfront + CS01 | < 0.5 ms | Hazard curve integration |
| Bermudan swaption (tree) | < 50 ms | 100-step tree |
| Exotic option (MC, 100k paths) | < 500 ms | With variance reduction |
| Autocallable (MC, 100k paths) | < 1s | Path-dependent with barriers |

### Portfolio Operations

| Operation | Target | Notes |
|-----------|--------|-------|
| Portfolio valuation (10k vanilla) | < 5s | Parallel pricing |
| Full Greeks (10k trades) | < 30s | Bumped revaluation |
| Historical VaR (250 scen, 1k trades) | < 5 min | Full revaluation |
| Curve calibration (OIS + SOFR + basis) | < 5s | Bootstrap + global fit |
| Vol surface calibration (SABR/expiry) | < 2s | Per surface |
| Stress test (20 scen, full portfolio) | < 2 min | Parallel scenario |

---

## P&L Attribution Checklist

A trading desk needs to explain every dollar of daily P&L. Check for these capabilities:

### Required Components

| Component | Formula | Priority |
|-----------|---------|----------|
| Delta P&L | sum(delta_i * dS_i) | P1 |
| Gamma P&L | 0.5 * sum(gamma_i * dS_i^2) | P1 |
| Cross-gamma P&L | sum(cross_gamma_ij * dS_i * dS_j) | P2 |
| Vega P&L | sum(vega_i * dVol_i) | P1 |
| Theta P&L | dPV/dt (time decay, carry, roll-down) | P1 |
| Carry | Coupon income - funding cost | P1 |
| Roll-down | PV change from curve slide | P2 |
| New deal P&L | Day-1 P&L from new trades | P1 |
| Unexplained | Actual - sum(explained) | P1 |

### Infrastructure

- Market data diffing between snapshots
- Taylor expansion P&L vs full reval P&L
- Unexplained < 5% for linear, < 10% for options
- Attribution by risk factor and by trade
- Theta decomposition: pure time decay, carry, roll-down, slide

---

## Trade Lifecycle Events

### Fixing & Reset Events

| Event | Instruments | Required Action |
|-------|-------------|-----------------|
| Rate fixing | FRN, IRS, SOFR swaps | Historical fixing lookup, accrued calc |
| Inflation fixing | Inflation swaps, TIPS | CPI index with publication lag |
| FX fixing | NDF, quanto, XCCY | WM/Reuters fixing rate |
| Equity fixing | Asian, autocallable | Observation recording |
| Commodity fixing | Commodity swaps, Asian | Exchange settlement price |

### Exercise Events

| Event | Instruments | Required Action |
|-------|-------------|-----------------|
| European exercise | All European options | Auto-exercise if ITM |
| Bermudan exercise | Bermudan swaptions, callables | Optimal exercise at each date |
| American exercise | American options | Continuous monitoring |
| Knock-in/out | Barrier options | Continuous or discrete monitoring |
| Autocall trigger | Autocallables | Observation vs trigger check |

### Credit Events

| Event | Instruments | Required Action |
|-------|-------------|-----------------|
| Default | CDS, bonds, loans | Recovery value, settle protection |
| Restructuring | CDS | Apply MMR/MR/CR/XR mechanics |
| Downgrade | Bonds, CLOs | Rating-based triggers |

### Amortization & Notional

| Event | Instruments | Required Action |
|-------|-------------|-----------------|
| Scheduled amortization | Amort swaps, bonds, loans | Reduce notional per schedule |
| Prepayment | MBS, ABS, term loans | Model CPR/PSA/SMM |
| PIK toggle | PIK bonds, leveraged loans | Capitalize interest |
| Make-whole call | Callable bonds | PV at T+spread |

---

## Output Format

Every review produces this report structure. Omit sections that don't apply to the scope.

```markdown
# Quantitative Review: [Scope]

## Executive Summary
[2-3 paragraphs: what changed or was reviewed, overall quality, most critical findings]

## Scorecard
| Dimension | Rating | Notes |
|-----------|--------|-------|
| Mathematical Correctness | X/5 | |
| Convention Compliance | X/5 | |
| Numerical Robustness | X/5 | |
| Coverage Completeness | X/5 | |
| API Usability | X/5 | |
| Extensibility | X/5 | |
| Production Readiness | X/5 | |

## Top 5 Priorities
1. [SEVERITY] Most impactful finding
2. ...
3. ...
4. ...
5. ...

## Phase 2: Mathematical Correctness Findings
### [SEVERITY] Finding title
**What**: Description
**Impact**: How this affects a practitioner
**Fix**: Concrete recommendation with code guidance
**Reference**: QuantLib/Bloomberg/ISDA/paper

## Phase 3: Convention Compliance
### Standard Compliance Table
| Area | Expected (QuantLib/BBG) | Implementation | Status |
|------|------------------------|----------------|--------|
| Day count | ... | ... | PASS/FAIL |
| Settlement | ... | ... | PASS/FAIL |

### [SEVERITY] Finding title
(same structure as above)

## Phase 4: Numerical Robustness Findings
### [SEVERITY] Finding title
(same structure as above)

## Phase 5: Smell Test Results
| Test | Result | Details |
|------|--------|---------|
| Par swap rate | PASS/FAIL | |
| Put-call parity | PASS/FAIL | |
| ... | | |

## Phase 6: Library Assessment
### Coverage Assessment
- Instruments present: [list]
- Instruments missing: [list with priority]
- Models available: [list]
- Models missing: [list]

### Risk Metrics Assessment
- Supported: [list]
- Missing: [list]
- Accuracy concerns: [list]

### API & Usability Findings
- [findings]

### Production Readiness Findings
- [findings]

## Recommendations

### Blockers / Critical (fix immediately)
- [ ] Finding 1
- [ ] Finding 2

### Major (next release)
- [ ] Finding 3

### Moderate (backlog)
- [ ] Finding 4

### Strategic Gaps (roadmap)
- [ ] Gap 1
```

After each review cycle, re-check the code against the findings and update the review. Continue iterating until there are no remaining action items. If any action items remain, treat this as an incomplete review.

---

## Key Principles

1. **Practitioner-first**: Every finding must matter to someone running a real book.
2. **Concrete over abstract**: "Bond pricer uses simple compounding instead of continuous for zero rates" beats "compounding might be wrong."
3. **Reference everything**: Professional quants validate against known sources.
4. **Prioritize ruthlessly**: Missing OIS discounting is more critical than a missing exotic.
5. **Credit what works**: Acknowledge strong implementations.
6. **Conventions kill quietly**: Perfect models still produce wrong P&L with wrong day counts. Check conventions first.
7. **Test the boring stuff**: Cashflow generation, accrual, schedule construction cause more production issues than exotic model errors.
8. **Think about the 4pm risk run**: Performance matters. 30-minute VaR is unusable.

## Additional Resources

- [reference.md](reference.md) - Authoritative sources, formulas, day count conventions, Greeks, ISDA SIMM
- [examples.md](examples.md) - Concrete code examples of common quantitative errors with fixes
- [cross-asset-checklist.md](cross-asset-checklist.md) - Full instrument/feature/convention matrix by asset class
- [rates-standards.md](rates-standards.md) - IRS, FRA, basis swap, cap/floor, swaption standards
- [fx-standards.md](fx-standards.md) - FX spot, forward, swap, option standards
- [fixed-income-standards.md](fixed-income-standards.md) - Bond, repo, inflation-linked, term loan standards
- [equity-standards.md](equity-standards.md) - Equity option, variance swap, TRS, barrier standards
- [algorithm-standards.md](algorithm-standards.md) - Interpolation, root-finding, Monte Carlo, FD, calibration standards
