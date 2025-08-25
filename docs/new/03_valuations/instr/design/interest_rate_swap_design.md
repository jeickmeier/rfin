# Interest-Rate Swap – Detailed Design Document

## 1 Overview
The `Swap` instrument represents an exchange of fixed and floating cash-flow legs. The implementation supports vanilla, basis, and amortising notionals via leg builders.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `fixed_leg` | `CashFlowLeg` | Fixed-rate coupons. |
| `float_leg` | `CashFlowLeg` | Floating coupons referencing an index curve. |
| `notional_exchange` | `bool` | If `true` notionals are exchanged at start/end. |
| `collateral` | `CsaSpec` | Determines discount curve selection. |

## 3 Valuation
\[ PV = PV_{\text{fixed}} - PV_{\text{float}} \]
Each leg PV is the sum of discounted cash-flows using discount factors from `CurveProvider`.

## 4 Analytics Helpers
* `par_rate(start,end)` root-finds the fixed rate that zero-prices the swap.
* `carry(h)` and `rolldown(h)` project forward-rate evolution.

## 5 Builder API Example
```rust
let swap = Swap::builder()
    .fixed_leg(fixed_leg)
    .float_leg(float_leg)
    .notional_exchange(false)
    .build();
```

## 6 Validation
* Legs must share common notional currency and frequency.
* Payment calendar conventions validated via `Schedule`.

## 7 Serialization
Serde tag `instr = "swap"`, versioned payload.

## 8 Testing
Fixed-vs-float PV parity property tests; QuantLib golden prices across 1000 random swaps.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 