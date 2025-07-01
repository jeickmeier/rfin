# Bonds – Detailed Design Document

## 1 Overview
The `Bond` family covers fixed‐rate, floating‐rate, and callable structures. All variants share a common principal repayment schedule.

## 2 Variants
| Variant | Key Fields |
|---------|------------|
| Fixed | `coupon`, `schedule`, `notional` |
| Floating | `index`, `spread`, `reset_lag` |
| Callable | `call_schedule`, optional `make_whole_spread` |

## 3 Valuation
*Fixed & Floating*: sum of discounted coupon cash‐flows + principal.

*Callable*: Bermudan optionality

## 4 Yield Analytics
* `yield_to_maturity(price)` solver.
* `yield_to_call`, `yield_to_worst` iterate call schedule.
* Duration, convexity, OAS search.

## 5 Validation
* Positive notional, monotonic schedule dates.
* Call prices ≥ 0.

## 6 Serialization
Serde tag `instr = "bond"` with embedded enum variant.

## 7 Testing
Clean vs dirty price conversions; golden PV vs Bloomberg for UST, corporate, callable municipals.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 