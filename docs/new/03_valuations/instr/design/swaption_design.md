# Swaption – Detailed Design Document

## 1 Overview
A `Swaption` is an option to enter into a swap at a future expiry. Valuation supports Black‐76 and SABR‐implied volatility sources.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `underlying_swap` | `Swap` | Definition of the underlying swap. |
| `expiry` | `Date` | Option expiry (European exercise). |
| `strike` | `F` | Strike fixed rate. |
| `vol_surface_id` | `String` | Key into 3-D swaption vol surface. |
| `is_payer` | `bool` | `true` payer swaption, `false` receiver. |

## 3 Valuation
Black-76 price with swap annuity \(A\):
\[ PV = DF(0,\text{expiry}) \times A \times \mathrm{Black}(F, K, \sigma, T) \]
where forward rate \(F\) and annuity come from the discount curve.

## 4 Analytics
* `implied_vol(price)` inverse Black.
* Vega, Gamma per standard formulas.

## 5 Validation & Serialization
* `expiry` > `val_date`.
* Serde tag `instr = "swaption"`.

## 6 Testing
Delta symmetry payer vs receiver; golden PV vs Bloomberg OIS discounting.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 