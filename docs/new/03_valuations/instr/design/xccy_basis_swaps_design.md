# Cross-Currency Basis Swaps – Detailed Design Document

## 1 Overview
`XccyBasisSwap` exchanges floating legs in two different currencies, optionally including notional exchanges. Pricing requires discount curves, forward curves, and FX spot/forward points.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `leg_domestic` | `CashFlowLeg` | Domestic currency floating leg. |
| `leg_foreign` | `CashFlowLeg` | Foreign currency floating leg. |
| `fx_spot` | `F` | Spot FX rate (domestic/foreign). |
| `notional_exchange` | `bool` | Exchange notionals at start/end? |

## 3 Valuation
PV in reporting currency (default domestic):
\[ PV = PV_{\text{dom}} + FX_{\text{fwd}} \times PV_{\text{for}} \]
FX forwards taken from curves module.

## 4 Validation
* Both legs have identical accrual schedules if `reset_fixing_offset == 0`.
* Supported currency pair exists in FX curves.

## 5 Serialization
Serde tag `instr = "xccy_basis"`.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 