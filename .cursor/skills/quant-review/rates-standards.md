# Interest Rate Instrument Standards

## Interest Rate Swaps (IRS)

### Standard market conventions

| Currency | Float index | Day count (fixed) | Day count (float) | Frequency (fixed) | Frequency (float) | Bus day |
|----------|-------------|-------------------|-------------------|-------------------|-------------------|---------|
| USD | SOFR | 30/360 | ACT/360 | Semi-annual | Annual | NYC |
| EUR | ESTR | 30/360 | ACT/360 | Annual | Annual | TARGET |
| GBP | SONIA | ACT/365F | ACT/365F | Annual | Annual | London |
| JPY | TONAR | ACT/365F | ACT/365F | Semi-annual | Annual | Tokyo |
| CHF | SARON | 30/360 | ACT/360 | Annual | Annual | Zurich |
| AUD | AONIA | ACT/365F | ACT/365F | Semi-annual | Quarterly | Sydney |

### QuantLib/Bloomberg defaults

```
IRS defaults (QuantLib MakeOIS, Bloomberg SWDF):
- Effective date: T+2 (spot start)
- Roll convention: Modified Following
- End of month rule: True (if start is EOM, roll to EOM)
- Payment lag: 2 business days (for OIS)
- Compounding: OIS = compounded with observation shift
- Observation shift: 2 days lookback
- Stub: Short front stub
- Notional: No exchange (single currency)
```

### Compounding methods

| Index type | Method | Shift | Professional library implementation |
|------------|--------|-------|-------------------------------------|
| OIS (SOFR, SONIA) | Compounded | Lookback/Observation shift | QuantLib: `OvernightIndexedSwap`, BBG: SWDF |
| IBOR (legacy) | Simple | None | QuantLib: `VanillaSwap` |
| Term SOFR | Simple | None | Same as legacy IBOR |

### OIS compounding formula (ISDA standard)

```
Compounded rate = (∏(1 + r_i × d_i/360) - 1) × 360/D

where:
  r_i = overnight rate for day i
  d_i = day count fraction for day i (usually 1/360)
  D = total accrual days

With observation shift (lockout):
  - 2-day shift: use rate from 2 business days prior
  - Payment delay: pay 2 days after period end
```

### Audit checklist - IRS

- [ ] Day count matches market convention for currency
- [ ] Compounding method correct (daily for OIS, simple for IBOR)
- [ ] Observation shift implemented for OIS (2 days standard)
- [ ] Payment lag matches convention (2 days for most OIS)
- [ ] Stub period handling (front stub default)
- [ ] Business day calendar correct for currency
- [ ] Roll convention respects EOM rule

---

## Forward Rate Agreements (FRA)

### Market conventions

| Currency | Day count | Fixing lag | Settlement | Rate convention |
|----------|-----------|------------|------------|-----------------|
| USD | ACT/360 | T-2 | Settlement date | Discounted |
| EUR | ACT/360 | T-2 | Settlement date | Discounted |
| GBP | ACT/365F | T-0 | Maturity date | Discounted |

### FRA pricing formula (ISDA/QuantLib standard)

```
Settlement amount = Notional × (r_fra - r_fix) × τ / (1 + r_fix × τ)

where:
  r_fra = contract FRA rate
  r_fix = fixing rate (reference index)
  τ = accrual period (day count fraction)

Note: Payment is at settlement date, discounted from maturity
```

### QuantLib implementation reference

```cpp
// QuantLib ForwardRateAgreement
FRA(
    valueDate,           // Settlement date
    maturityDate,        // End date of underlying period
    position,            // Long/Short
    strikeRate,          // Contract rate
    notional,
    index,               // e.g., USDLibor3M
    discountingCurve     // For PV calculation
)

// Key: FRA settles discounted at valueDate
```

### Audit checklist - FRA

- [ ] Settlement is discounted (not paid at maturity)
- [ ] Fixing date calculated correctly (spot lag from settlement)
- [ ] Day count matches underlying index
- [ ] FRA rate is simple rate (not compounded)
- [ ] Discount factor uses settlement date

---

## Basis Swaps

### Market conventions

| Swap type | Leg 1 | Leg 2 | Spread convention |
|-----------|-------|-------|-------------------|
| OIS-OIS | SOFR | FF | Spread on FF leg |
| Tenor basis | 3M SOFR | 1M SOFR | Spread on shorter tenor |
| XCCY basis | USD SOFR | EUR ESTR | Spread on non-USD leg |

### QuantLib defaults

```
Basis swap conventions:
- Spread: Added to second leg (typically shorter tenor or non-USD)
- Notional: Single currency = no exchange, XCCY = exchange at start/end
- Payment: Same dates or different based on tenor
- Day count: Each leg follows its index convention
```

### Audit checklist - Basis swap

- [ ] Spread applied to correct leg (market convention)
- [ ] Each leg uses its own day count convention
- [ ] XCCY includes notional exchanges (initial + final)
- [ ] FX rate for notional exchange is spot at trade date
- [ ] Payment frequencies can differ between legs

---

## Caps/Floors

### Market conventions

| Currency | Day count | Volatility quote | Settlement |
|----------|-----------|-----------------|------------|
| USD | ACT/360 | Lognormal (Black) | T+2 |
| EUR | ACT/360 | Normal (Bachelier) | T+2 |
| GBP | ACT/365F | Lognormal | T+0 |

### Pricing models

| Model | When to use | Formula reference |
|-------|-------------|-------------------|
| Black-76 | Lognormal vol quotes | Hull Ch. 29 |
| Bachelier | Normal vol quotes (EUR post-2015) | Hull Ch. 29 |
| Shifted Black | Low/negative rate environment | Brigo-Mercurio |
| SABR | Vol smile calibration | Hagan et al. 2002 |

### Black-76 formula (QuantLib/Bloomberg standard)

```
Caplet_price = DF(T_pay) × τ × [F × N(d1) - K × N(d2)]
Floorlet_price = DF(T_pay) × τ × [K × N(-d2) - F × N(-d1)]

d1 = [ln(F/K) + 0.5 × σ² × T] / (σ × √T)
d2 = d1 - σ × √T

where:
  F = forward rate
  K = strike
  σ = lognormal volatility
  T = time to expiry (fixing date)
  τ = accrual period
  DF(T_pay) = discount to payment date
```

### Audit checklist - Caps/Floors

- [ ] Volatility type matches market quote (lognormal vs normal)
- [ ] Time to expiry is to fixing date, not payment date
- [ ] Payment date discounting (not fixing date)
- [ ] Accrual factor uses correct day count
- [ ] For SOFR caps: use compounded forward rate
- [ ] Premium settlement convention (upfront vs running)

---

## Swaptions

### Market conventions

| Currency | Vol quote | Exercise | Settlement |
|----------|-----------|----------|------------|
| USD | Lognormal | European | Physical or cash |
| EUR | Normal (Bachelier) | European | Cash (ISDA formula) |
| GBP | Lognormal | European | Physical |

### Cash settlement (ISDA annuity)

```
Cash settlement amount = Black_price × Annuity_factor

ISDA annuity (for cash-settled swaptions):
A = Σ τ_i × DF(t_i)

where DF is calculated using the UNDERLYING swap rate (not market curve)
This is the "ISDA" or "par" annuity convention.

Alternative (market annuity): use market discount factors
```

### QuantLib implementation

```cpp
// European swaption
Swaption(
    swap,                    // Underlying swap
    exercise,                // ExerciseDate
    settlementType,          // Physical or Cash
    settlementMethod         // ParYieldCurve (ISDA) or PhysicalOTC
)

// Pricing engines
BlackSwaptionEngine        // Lognormal vol
BachelierSwaptionEngine    // Normal vol
```

### Audit checklist - Swaptions

- [ ] Vol type matches market (lognormal USD/GBP, normal EUR)
- [ ] Cash settlement uses ISDA annuity (not market annuity)
- [ ] Expiry date is exercise date (not settlement)
- [ ] Underlying swap terms match market standard
- [ ] Physical delivery creates actual swap position

---

## Common implementation errors

### 1. OIS compounding without shift

```rust
// WRONG: Simple average
let rate = daily_rates.iter().sum::<f64>() / daily_rates.len() as f64;

// WRONG: Compounding without observation shift
let compound = daily_rates.iter()
    .fold(1.0, |acc, &r| acc * (1.0 + r * day_frac));

// CORRECT: Compounding with 2-day observation shift
let compound = (0..accrual_days).fold(1.0, |acc, i| {
    let obs_date = business_days_before(accrual_start + i, 2);
    let rate = get_fixing(obs_date);
    acc * (1.0 + rate * 1.0/360.0)
});
```

### 2. FRA settling at maturity

```rust
// WRONG: Pay undiscounted at maturity
let payment = notional * (fra_rate - fixing) * tau;

// CORRECT: Pay discounted at settlement
let payment = notional * (fra_rate - fixing) * tau / (1.0 + fixing * tau);
```

### 3. Wrong cap/floor time to expiry

```rust
// WRONG: Time to payment
let time = year_fraction(today, payment_date);

// CORRECT: Time to fixing
let time = year_fraction(today, fixing_date);
```
