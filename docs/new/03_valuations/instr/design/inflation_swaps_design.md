# Inflation Swaps – Detailed Design Document

## 1 Overview
Supports Zero‐Coupon (ZC) and Year‐on‐Year (YY) swaps exchanging fixed inflation payments for floating realised CPI growth.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `fixed_leg` | `CashFlowLeg` | Pays fixed CPI accrual. |
| `float_leg` | `CashFlowLeg` | Pays realised inflation. |
| `variant` | `InflSwapType` | `ZC` or `YY`. |

## 3 Valuation
Both legs discounted with nominal curve; inflation forward curve supplies CPI expectation.

## 4 Validation
* Legs aligned in payment dates.
* Variant correctly matches leg definitions.

## 5 Serialization
Serde tag `instr = "infl_swap"` with enum variant.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 