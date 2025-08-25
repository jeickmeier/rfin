# FX Forwards & Options – Detailed Design Document

## 1 Overview
FX instruments cover forward contracts and vanilla European options priced via covered‐interest parity and Black‐Scholes respectively.

## 2 FX Forward Data Model
| Field | Type | Notes |
|-------|------|-------|
| `notional_base` | `F` | Amount in base currency. |
| `rate` | `F` | Agreed forward FX rate. |
| `settle_date` | `Date` | Contract settlement date. |

### Valuation
Forward points from interest‐rate parity:
\[ F = S \times \frac{DF_{\text{dom}}}{DF_{\text{for}}} \]
PV = discounted difference between trade rate and fair forward.

## 3 FX Option Data Model
| Field | Type | Notes |
|-------|------|-------|
| `is_call` | `bool` | Call if `true`, put otherwise. |
| `strike` | `F` | Strike FX rate. |
| `expiry` | `Date` | European expiry. |
| `vol_surface_id` | `String` | 2-D surface key. |

### Valuation
Garman‐Kohlhagen (Black‐Scholes with foreign dividend yield):
\[ PV = DF_{\text{dom}} \times ( c N(d_1) - p N(d_2) ) \]
where domestic/foreign discount factors act as rates.

## 4 Validation
* `strike` > 0.
* Surface exists for currency pair.

## 5 Serialization
Tags `instr = "fx_fwd"` / `"fx_opt"`.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 