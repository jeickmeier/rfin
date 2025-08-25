# Caps & Floors – Detailed Design Document

## 1 Overview
A `Cap`/`Floor` is a portfolio of caplets/floorlets providing protection against rising (or falling) interest rates. Valuation follows Black's model with volatility pulled from an implied vol surface.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `leg` | `CashFlowLeg` | Underlying floating leg schedule. |
| `strike` | `F` | Common strike for all caplets/floorlets. |
| `is_cap` | `bool` | `true` for cap, `false` for floor. |
| `vol_surface_id` | `String` | Lookup key into vol surface set. |

## 3 Valuation
Each period generates a forward rate \(F_i\) and accrual factor \(\alpha_i\). The caplet PV is:
\[ PV_i = DF(t_i) \times \alpha_i \times N \times \mathrm{Black}(F_i, K, \sigma, T_i) \]
Sum across all periods for total PV.

## 4 Analytics
* `implied_vol(price)` inverse-solves \(\sigma\).
* Vega analytic via Black formula.
* ATM vol helper `atm_strike(val_date)`.

## 5 Validation
* `strike` > 0.
* All fixing dates after `val_date`.

## 6 Serialization
Serde tag `instr = "capfloor"`.

## 7 Testing
Cap-floor parity tests; golden PV vs QuantLib for EUR caps 1Y–10Y.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 