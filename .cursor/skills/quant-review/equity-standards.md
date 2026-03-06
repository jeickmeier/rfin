# Equity Instrument Standards

## Equity Options

### Market conventions

| Market | Vol quote | Settlement | Premium |
|--------|-----------|------------|---------|
| US listed | Implied vol | T+1 | Upfront |
| European listed | Implied vol | T+1 | Upfront |
| OTC equity | Lognormal vol | Negotiated | Upfront |

### Black-Scholes-Merton formula (professional standard)

```
Call = S × e^(-q × T) × N(d1) - K × e^(-r × T) × N(d2)
Put  = K × e^(-r × T) × N(-d2) - S × e^(-q × T) × N(-d1)

d1 = [ln(S/K) + (r - q + σ²/2) × T] / (σ × √T)
d2 = d1 - σ × √T

where:
  S = spot price
  K = strike
  r = risk-free rate
  q = dividend yield (continuous)
  σ = volatility
  T = time to expiry
```

### Discrete dividend handling

```
For known dividends D_i at times t_i:

Method 1: Spot adjustment (QuantLib default)
S_adj = S - Σ D_i × e^(-r × t_i)  [for t_i < T]

Method 2: Forward adjustment
F = (S - PV(divs)) × e^(r × T)

Method 3: Escrowed dividend model (more accurate)
Use tree/MC with dividend drops on ex-dates
```

### Greeks (analytical)

```
Delta = e^(-q × T) × N(d1)                          [call]
Delta = e^(-q × T) × (N(d1) - 1)                    [put]
Gamma = e^(-q × T) × φ(d1) / (S × σ × √T)
Vega  = S × e^(-q × T) × φ(d1) × √T
Theta_call = -S × e^(-q × T) × φ(d1) × σ / (2√T)
            + q × S × e^(-q × T) × N(d1)
            - r × K × e^(-r × T) × N(d2)
Rho   = K × T × e^(-r × T) × N(d2)                  [call]

where φ(x) = standard normal PDF
```

### Audit checklist - Equity options

- [ ] BSM includes dividend yield (not vanilla Black)
- [ ] Discrete dividends handled appropriately
- [ ] American exercise uses binomial/finite diff (not BSM)
- [ ] Greeks consistent with pricing formula
- [ ] Vol surface interpolation for OTM strikes

---

## Variance Swaps

### Market conventions

| Component | Standard | Notes |
|-----------|----------|-------|
| Variance calculation | Realized variance (log returns) | Not price returns |
| Annualization | 252 trading days | US equity markets |
| Settlement | Cash, on expiry + 2 | T+2 from final fixing |

### Variance swap pricing (ISDA/professional standard)

```
Realized variance (annualized):
σ²_realized = (252 / N) × Σ [ln(S_i / S_{i-1})]²

P&L = Var_notional × (σ²_realized - K²_var)

where:
  N = number of observations
  K²_var = variance strike (quoted as vol² × 10000)

Var_notional relationship to vega_notional:
Var_notional = Vega_notional / (2 × K_vol)
```

### Log contract replication

```
Fair variance (no jumps):
E[σ²_realized] = (2/T) × [F/S₀ - 1 - ln(F/S₀)
                  - ∫₀^F (Put(K)/K²) dK
                  - ∫_F^∞ (Call(K)/K²) dK]

This is the basis for VIX calculation
```

### Audit checklist - Variance swaps

- [ ] Log returns (not simple returns)
- [ ] 252 trading days annualization
- [ ] Variance notional vs vega notional conversion correct
- [ ] No weekends/holidays in return calculation
- [ ] Handle stock splits/dividends in return series

---

## Equity Total Return Swaps (TRS)

### Market conventions

| Component | Standard | Notes |
|-----------|----------|-------|
| Equity leg | Total return (price + dividends) | Gross or net of tax |
| Financing leg | SOFR + spread | Quarterly payment |
| Reset | Daily or periodic | Mark-to-market |
| Dividend treatment | Pass-through | Paid when received |

### TRS pricing

```
Equity leg PV:
PV_equity = Notional × [(S_T / S_0) - 1] × DF(T)
          + Σ Div_i × DF(t_i)

Financing leg PV:
PV_financing = Notional × Σ (r + spread) × τ_i × DF(t_i)

where:
  S_T = expected terminal price (forward)
  S_0 = initial price
  Div_i = expected dividend at t_i
  τ_i = accrual fraction for period i
```

### Audit checklist - Equity TRS

- [ ] Total return = price return + dividend return
- [ ] Dividend withholding tax treatment (gross vs net)
- [ ] Financing spread matches funding
- [ ] Margin/collateral properly handled
- [ ] Corporate actions (splits, M&A) handled

---

## Equity Index Futures

### Market conventions

| Index | Point value | Settlement | Roll cycle |
|-------|-------------|------------|------------|
| S&P 500 (ES) | $50/point | Cash | Mar, Jun, Sep, Dec |
| E-mini NASDAQ (NQ) | $20/point | Cash | Mar, Jun, Sep, Dec |
| Euro Stoxx 50 | €10/point | Cash | Mar, Jun, Sep, Dec |
| FTSE 100 | £10/point | Cash | Mar, Jun, Sep, Dec |
| Nikkei 225 | ¥1000/point | Cash | Mar, Jun, Sep, Dec |

### Fair value formula

```
Fair value = Index × e^((r - q) × T)

or with discrete dividends:
Fair value = (Index - PV(divs)) × e^(r × T)

Basis = Futures - Spot
Fair basis = Fair_value - Spot
```

### Audit checklist - Index futures

- [ ] Point value matches exchange specification
- [ ] Fair value uses dividend yield/discrete dividends
- [ ] Roll date handling for continuous series
- [ ] Settlement price methodology (mark to close)

---

## Exotic equity options

### Barrier options

| Type | Barrier condition | Payoff |
|------|-------------------|--------|
| Down-and-out | Knockout if S < H | Vanilla if survives |
| Down-and-in | Knock in if S < H | Vanilla if triggered |
| Up-and-out | Knockout if S > H | Vanilla if survives |
| Up-and-in | Knock in if S > H | Vanilla if triggered |

### Barrier monitoring convention

```
Professional standard: Continuous monitoring
Implementation: Discrete (daily close) with adjustment

Broadie-Glasserman-Kou adjustment for discrete:
H_adj = H × exp(±0.5826 × σ × √(Δt))

where + for up barriers, - for down barriers
```

### Asian options

```
Arithmetic average (market standard):
A = (1/n) × Σ S(t_i)

Geometric average (for closed-form):
G = [Π S(t_i)]^(1/n)

Payoff: max(A - K, 0) for average price call
```

### Audit checklist - Exotic options

- [ ] Barrier monitoring convention explicit
- [ ] Discrete monitoring adjustment applied
- [ ] Asian averaging dates specified clearly
- [ ] Monte Carlo paths sufficient (>10k for pricing)
- [ ] Boundary conditions correct (at barrier, at expiry)

---

## Common implementation errors

### 1. Missing dividend yield in BSM

```rust
// WRONG: Vanilla Black (no dividends)
let d1 = ((s / k).ln() + (r + 0.5 * vol * vol) * t) / (vol * t.sqrt());

// CORRECT: BSM with dividend yield
let d1 = ((s / k).ln() + (r - q + 0.5 * vol * vol) * t) / (vol * t.sqrt());
```

### 2. Simple returns instead of log returns for variance

```rust
// WRONG: Simple returns
let ret = (price[i] - price[i-1]) / price[i-1];
let var = returns.iter().map(|r| r * r).sum::<f64>() / n;

// CORRECT: Log returns
let ret = (price[i] / price[i-1]).ln();
let var = returns.iter().map(|r| r * r).sum::<f64>() * 252.0 / n;
```

### 3. Wrong barrier adjustment for discrete monitoring

```rust
// WRONG: Using exact barrier level
let knocked_out = prices.iter().any(|&p| p < barrier);

// CORRECT: Adjust barrier for discrete monitoring
let adjustment = (0.5826 * vol * dt.sqrt()).exp();
let adjusted_barrier = barrier * adjustment;  // for down barrier
let knocked_out = prices.iter().any(|&p| p < adjusted_barrier);
```

### 4. Variance notional vs vega notional confusion

```rust
// WRONG: Using vega notional directly
let pnl = vega_notional * (realized_var - strike_var);

// CORRECT: Convert to variance notional
let var_notional = vega_notional / (2.0 * strike_vol);
let pnl = var_notional * (realized_var - strike_var);
```
