## Plan: Missing Instruments (High Impact) for `finstack_valuations`

### Goals / Non-goals

- **Goal**: Cover the instruments most commonly held/traded by modern pensions, endowments, multi-asset funds, and hedge funds that are currently missing from `finstack/valuations/src/instruments`.
- **Goal**: Implement the **minimum viable** product mechanics + market conventions + risk metrics needed for real portfolio valuation and hedging workflows.
- **Non-goal**: Achieve “every exotic under the sun.” We target **~90% coverage**, not 99.9999%.

### Reuse / extend existing primitives (important)

- **Prepayment hooks likely already exist**: Before implementing new prepayment logic for Agency MBS/TBA/CMO, double-check and reuse:
- `structured_credit` prepayment specs and plumbing (e.g., `PrepaymentModelSpec`, `PoolAsset` overrides like SMM, deterministic + stochastic prepay paths).
- `cashflow` builder prepayment/credit event specs and helper conversions (CPR/PSA/SMM) and credit emission.
- **Fees likely already exist**: Model servicing fees / guarantee fees / g-fees by reusing or extending the cashflow builder fee specs + fee emission:
- Use existing periodic fee emission patterns and `CFKind::Fee` where appropriate.
- Extend fee bases if needed (e.g., “outstanding pool balance” / “scheduled balance” bases) rather than implementing bespoke fee cashflows inside an Agency MBS pricer.

### Missing instruments (final list)

#### 1) Agency mortgage products (high impact)

- **Agency MBS passthrough (pool / specified pool)**
- **What’s missing**: A first-class Agency MBS instrument (FNMA/FHLMC/GNMA) with pass-through cashflows.
- **Core missing functionality**:
- **Prepayment modeling hooks**: PSA / CPR / SMM (start simple, but must exist).
- **Reuse plan**: Prefer reusing `structured_credit` prepayment specs/models and/or the `cashflow` builder’s `prepayment` spec + CPR/PSA/SMM conversions, rather than duplicating prepayment math.
- **Servicing / guarantee / g-fee**: Ability to represent net coupon vs gross WAC.
- **Reuse plan**: Represent servicing/g-fee as periodic fees using existing cashflow fee specs/emission; only extend fee bases/specs if needed (avoid bespoke per-instrument fee logic).
- **Delay / payment timing**: Delay conventions by program and settlement.
- **Risk**: OAS, effective duration/convexity, key-rate DV01 (optional), price/yield style outputs.
- **Market data needs**: OIS discount curve, optional forward curve; option-adjusted model inputs (vol, mean reversion) if doing OAS.
- **TBA (To‑Be‑Announced) forward**
- **What’s missing**: A TBA trade representation (generic agency, coupon, maturity “bucket”, settlement month).
- **Core missing functionality**:
- **Settlement conventions**: TBA settlement calendar/month and drop date mechanics.
- **Cheapest-to-deliver / allocation**: Allow either a simplified “assumed pool” mapping or explicit pool allocation.
- **Risk**: Dollar price/PV, DV01/OAS-style risk proxies consistent with market usage.
- **Operationally important**: TBAs are a distinct object in real books; representing as generic structured credit loses key conventions.
- **Dollar roll**
- **What’s missing**: A first-class roll trade (sell/buy TBAs different settlements) or explicit modeling via two linked TBAs + financing carry.
- **Core missing functionality**:
- **Implied financing**: Carry/roll specialness representation.
- **Risk**: PV and roll sensitivity.
- **Agency CMO tranches (minimal set)**
- **What’s missing**: A minimal CMO tranche instrument set suitable for portfolio reporting.
- **Core missing functionality**:
- **Tranche types**: Sequential + PAC/support (minimum); consider IO/PO as high-usage additions.
- **Waterfall**: Deterministic tranche waterfall engine (even if simplified at first).
- **Risk**: OAS/effective duration and scenario measures (rate up/down, prepay up/down).

#### 2) Exchange-traded equity index futures (high impact)

- **Equity index future**
- **What’s missing**: Futures contract instrument for equity indices (and optionally single-stock futures later).
- **Core missing functionality**:
- **Contract specs**: Multiplier, tick size, currency, exchange calendar, last trade/expiry rules, margining assumptions (if needed for PnL).
- **Pricing**: Forward-style pricing (carry/dividend yield) or quoted futures price override.
- **Risk**: Delta/beta exposure to underlying index; PV/mark-to-market in settlement currency.

#### 4) FX forwards + NDFs (high impact)

- **FX forward (deliverable)**
- **What’s missing**: A first-class deliverable FX forward, even if it can be replicated via an FX swap internally.
- **Core missing functionality**:
- **Settlement conventions**: Spot lags (T+2/T+1), holidays/calendars, business day adjustment.
- **Pricing**: Forward points from curves or direct quoted forward override.
- **Risk**: FX delta exposure; PV in a chosen reporting currency with explicit FX policy.
- **NDF (non-deliverable forward)**
- **What’s missing**: A first-class NDF for EM currencies (cash-settled in a settlement currency).
- **Core missing functionality**:
- **Fixing**: Fixing date/source, fix-vs-settle separation.
- **Settlement currency**: Cash settlement payoff mechanics.
- **Risk**: FX delta with respect to the fixing/spot reference and discounting in settlement currency.

#### 5) Commodity derivatives (high impact)

- **Commodity forward / future**
- **What’s missing**: Commodity exposure instruments (energy/metals/ag) are absent.
- **Core missing functionality**:
- **Contract specs**: Unit of measure, multiplier, currency, delivery month, exchange conventions (for futures).
- **Pricing**: Curve-based forward price and/or quoted price override.
- **Risk**: Delta to commodity forward curve; FX exposure if priced in non-reporting currency.
- **Commodity swap (fixed-for-floating / index swap)**
- **What’s missing**: The most common institutional commodity derivative besides futures.
- **Core missing functionality**:
- **Floating leg**: References a commodity index/average (monthly average, etc.).
- **Schedules**: Periodic settlement schedules and averaging.
- **Risk**: Curve bucket deltas (or at least parallel delta), PV by period.
- **Commodity vanilla option**
- **What’s missing**: Basic optionality on commodities (hedge fund + producer/consumer hedging workflows).
- **Core missing functionality**:
- **Vol surface**: Commodity vol surface ID + pricing model selection (start with Black-76).
- **Greeks**: Delta/Gamma/Vega/Theta; consistent bump policy.

#### 6) Vol index products (requested)

- **Volatility index future (e.g., VIX future)**
- **What’s missing**: A first-class vol index future instrument.
- **Core missing functionality**:
- **Underlying definition**: Link to a vol index term structure / implied forward index (market data object).
- **Settlement**: Cash settlement conventions at expiry.
- **Risk**: Delta to vol index level/term structure; PV/MTM.
- **Volatility index option (e.g., VIX option)**
- **What’s missing**: Options on vol indices (not equivalent to equity options or variance swaps operationally).
- **Core missing functionality**:
- **Model**: Start with Black-style on vol index forward (market practice varies; keep it configurable).
- **Vol surface**: Dedicated vol-index option surface (distinct quoting/expiry sets).
- **Greeks**: Delta/Gamma/Vega/Theta; plus “vega to vol-of-vol” only if you go beyond MVP.

### Cross-cutting “missing functionality” checklist (applies to all new instruments)

- **JSON interoperability**: Schema-stable `InstrumentJson` variant(s), strict `deny_unknown_fields`, and round-trip tests.
- **Conventions**: Calendars, business-day conventions, settlement lags, accrual conventions where applicable.
- **MarketContext wiring**: Clear required curves/surfaces/fixings per instrument (`required_*` introspection).