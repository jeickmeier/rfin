# Money-Market Contracts – Detailed Design Document

## 1 Overview
This category covers short-dated interest-rate products valued off a single money-market curve. Implementations share a common accrual helper and parameter validation logic.

Supported instrument structs:
1. `Deposit`
2. `FRA`
3. `Future` (SOFR, LIBOR, etc.)

## 2 Data Model
### 2.1 Deposit
| Field | Type | Notes |
|-------|------|-------|
| `rate` | `F` | Quoted annualised simple rate. |
| `start` | `Date` | Spot or forward start date. |
| `tenor` | `Period` | Deposit tenor (e.g. 3M). |
| `notional` | `F` | Principal amount (sign indicates Pay/Rec). |

### 2.2 FRA
| Field | Type | Notes |
|-------|------|-------|
| `fixed_rate` | `F` | Contract rate. |
| `start` | `Date` | FRA period start. |
| `end` | `Date` | FRA period end. |
| `notional` | `F` | Principal amount. |

### 2.3 Futures
| Field | Type | Notes |
|-------|------|-------|
| `price` | `F` | Exchange quoted price (e.g. 98.75). |
| `delivery` | `Date` | Contract delivery date. |
| `convexity_adj` | `Option<F>` | Optional convexity adjustment. |

## 3 Valuation
*Deposit* PV:
\[ PV = N \times \left(1 + r \times \alpha \right) \times DF(\text{val},\text{end}) - N \]
where \(\alpha\) is year-fraction.

*FRA* PV: discounted difference between forward rate and fixed rate.

*Future* PV: implied forward rate from price \(= 100 - P\) with optional convexity correction.

## 4 Helpers
`carry(val_date, fwd_date)` convenience function and forward DV01 are shared across all money-market types.

## 5 Validation Rules
* `end` > `start` and both ≥ `val_date`.
* Rate bounds \(-100\,\% < r < 100\,\%\).
* Supported currencies/stub adjustments consistent with `Schedule` conventions.

## 6 Serialization
All contracts derive Serde with tag `instr = "mm"` and type-specific discriminator.

## 7 Testing
Golden PVs versus QuantLib; property tests verify rate-PV monotonicity.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 