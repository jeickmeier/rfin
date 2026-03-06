# Fixed Income Instrument Standards

## Bonds

### Market conventions by issuer type

| Type | Day count | Coupon frequency | Settlement | Price quote |
|------|-----------|------------------|------------|-------------|
| US Treasury | ACT/ACT ICMA | Semi-annual | T+1 | Clean, 32nds |
| US Agency | 30/360 | Semi-annual | T+1 | Clean |
| US Corporate | 30/360 | Semi-annual | T+2 | Clean |
| US Municipal | 30/360 | Semi-annual | T+2 | Clean |
| UK Gilt | ACT/ACT ICMA | Semi-annual | T+1 | Clean |
| German Bund | ACT/ACT ICMA | Annual | T+2 | Clean |
| JGB | ACT/365F | Semi-annual | T+2 | Clean |

### Price/yield relationship (ISDA standard)

```
Clean price = Dirty price - Accrued interest

Dirty price = Σ CF_i / (1 + y/f)^(n_i)

where:
  CF_i = cashflow at time i
  y = yield to maturity
  f = coupon frequency (2 for semi-annual)
  n_i = periods to cashflow i

Accrued interest = Coupon × days_accrued / days_in_period
```

### Accrued interest calculation

```
US Treasury (ACT/ACT ICMA):
AI = Coupon × (actual_days / actual_days_in_period)

US Corporate (30/360):
AI = Coupon × (30/360_days / 180)  [for semi-annual]

30/360 adjustment:
- If D1 = 31, set D1 = 30
- If D2 = 31 and D1 >= 30, set D2 = 30
```

### QuantLib implementation reference

```cpp
// Bond pricing
FixedRateBond(
    settlementDays,      // Usually 1-2
    faceAmount,          // Par value
    schedule,            // Coupon dates
    coupons,             // Vector of coupon rates
    accrualDayCounter,   // Day count convention
    paymentConvention,   // Business day adjustment
    redemption           // Typically 100
)

// Clean price from yield
BondFunctions::cleanPrice(bond, yield, dayCounter, compounding, frequency)
```

### Audit checklist - Bonds

- [ ] Day count matches issuer type convention
- [ ] Accrued interest calculated from last coupon to settlement
- [ ] Price quote is clean (excluding accrued)
- [ ] Yield compounding matches coupon frequency
- [ ] Settlement days correct for market
- [ ] Ex-dividend handling for markets that use it

---

## Repos (Repurchase Agreements)

### Market conventions

| Market | Day count | Settlement | Rate quote |
|--------|-----------|------------|------------|
| US Treasury repo | ACT/360 | T+0 (same day) | Simple rate |
| US Agency repo | ACT/360 | T+0 | Simple rate |
| European GC | ACT/360 | T+0 or T+1 | Simple rate |
| UK Gilt repo | ACT/365 | T+0 | Simple rate |

### Repo pricing formula

```
Purchase price = Collateral_MV × (1 - Haircut)

Repurchase price = Purchase_price × (1 + r × T)

where:
  Collateral_MV = Market value of collateral
  Haircut = Margin (e.g., 2% for Treasuries)
  r = repo rate (simple, ACT/360)
  T = repo term in years

Interest = Repurchase_price - Purchase_price
```

### Haircut conventions (professional standard)

| Collateral | Typical haircut | Range |
|------------|-----------------|-------|
| US Treasury | 2% | 1-3% |
| US Agency | 2-3% | 2-5% |
| Investment Grade Corp | 5-10% | 3-15% |
| High Yield | 15-25% | 10-30% |
| Equity | 25-30% | 15-50% |

### Audit checklist - Repos

- [ ] Day count is ACT/360 (US) or ACT/365 (UK)
- [ ] Rate is simple, not compounded
- [ ] Haircut applied correctly to collateral value
- [ ] Settlement typically same-day
- [ ] Accrued interest on collateral handled correctly

---

## Inflation-Linked Bonds

### Market conventions

| Type | Indexation lag | Index | Day count |
|------|----------------|-------|-----------|
| US TIPS | 3 months | CPI-U NSA | ACT/ACT |
| UK IL Gilt | 3 months | RPI | ACT/ACT |
| Euro ILB | 3 months | HICP | ACT/ACT |
| JGBi | 3 months | Japan CPI | ACT/365 |

### Index ratio calculation (Canadian model - most common)

```
Index_ratio = CPI(settlement) / CPI(base)

CPI(settlement) = Linear interpolation between:
  CPI(ref_month - 3)  and  CPI(ref_month - 2)

Interpolation:
CPI(d) = CPI(M-3) + (d - 1) / days_in_month × [CPI(M-2) - CPI(M-3)]

where:
  d = day of month
  M = settlement month
```

### TIPS pricing formula

```
Dirty price = Index_ratio × [Σ CF_i × DF(t_i)]

where:
  CF_i = real cashflow (unadjusted by inflation)
  DF(t_i) = real discount factor

Real yield solves:
Clean_price / Index_ratio = Σ CF_i / (1 + y_real/2)^(n_i)
```

### Audit checklist - Inflation bonds

- [ ] Index ratio uses 3-month lag (standard)
- [ ] Linear interpolation between monthly CPI values
- [ ] Real yield convention matches market (semi-annual for TIPS)
- [ ] Deflation floor (if applicable - TIPS have floor at par)
- [ ] Accrued inflation calculated correctly

---

## Term Loans

### Market conventions

| Type | Rate | Day count | Prepayment |
|------|------|-----------|------------|
| Leveraged loan | SOFR + spread | ACT/360 | Par (with notice) |
| Investment grade | SOFR + spread | ACT/360 | Par |
| Real estate | Fixed or floating | ACT/360 | With penalty |

### Pricing components

```
All-in rate = Base_rate + Spread + Facility_fee

Present value = Σ [Principal_i + Interest_i] × DF(t_i)

OID amortization (for discounted loans):
Effective_rate solves: Purchase_price = Σ CF_i × (1 + r)^(-t_i)
```

### Audit checklist - Term loans

- [ ] Spread added to appropriate base rate (SOFR, Prime)
- [ ] Day count ACT/360 for interest calculation
- [ ] Amortization schedule applied correctly
- [ ] OID amortized to effective yield
- [ ] Prepayment assumptions affect duration

---

## Bond Futures

### Conversion factor calculation (US Treasury futures)

```
CF = [Σ c/2 × v^n + v^N] × (1/v^z)

where:
  c = coupon rate
  v = 1 / (1 + 0.06/2)  [6% notional yield]
  n = number of periods from first delivery to coupon
  N = number of periods from first delivery to maturity
  z = fraction of period from settlement to first coupon

Rounded to 4 decimal places
```

### CTD (Cheapest-to-Deliver) determination

```
Net basis = Clean_price - Futures_price × CF - AI

Implied repo rate:
IRR = [(Futures × CF + AI_delivery - Clean) / Clean] × (360/days)

CTD = bond with highest implied repo rate
```

### Audit checklist - Bond futures

- [ ] Conversion factor uses 6% notional yield
- [ ] CF calculation matches CME/Eurex methodology
- [ ] Delivery option value considered
- [ ] Accrued to delivery date, not trade date
- [ ] Settlement price uses exchange convention

---

## Common implementation errors

### 1. Wrong day count for corporate bonds

```rust
// WRONG: Using ACT/ACT for corporates
let accrued = coupon * actual_days as f64 / actual_period as f64;

// CORRECT: 30/360 for US corporates
let accrued = coupon * day_count_30_360(last_coupon, settlement) * 2.0; // semi-annual
```

### 2. Accrued interest from wrong date

```rust
// WRONG: From issue date
let accrued = calc_accrued(issue_date, settlement);

// CORRECT: From last coupon date
let last_coupon = most_recent_coupon_before(settlement, schedule);
let accrued = calc_accrued(last_coupon, settlement);
```

### 3. Inflation index ratio with wrong lag

```rust
// WRONG: Using current month CPI
let index_ratio = cpi_current / cpi_base;

// CORRECT: 3-month lag with interpolation
let ref_month = settlement_date.month() - 3;
let cpi_interp = interpolate_cpi(ref_month, settlement_date.day());
let index_ratio = cpi_interp / cpi_base;
```

### 4. Repo rate compounded instead of simple

```rust
// WRONG: Continuous compounding
let repurchase = purchase * (repo_rate * term).exp();

// CORRECT: Simple interest
let repurchase = purchase * (1.0 + repo_rate * term);
```
