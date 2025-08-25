# Equity Options – Detailed Design Document

## 1 Overview
Supports European and American style vanilla options on equity underlyings, using volatility surfaces from curves module.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `underlying` | `String` | Equity ticker. |
| `is_call` | `bool` | Call (true) or put. |
| `strike` | `F` | Strike price. |
| `expiry` | `Date` | Option expiry. |
| `style` | `OptionStyle` | `European` or `American`. |
| `vol_surface_id` | `String` | Vol surface key. |

## 3 Valuation
*European*: Black‐Scholes with continuous dividend yield.

*American*: Binomial lattice (Cox‐Ross‐Rubinstein) with early exercise check.

## 4 Validation
* `strike` > 0,
* Surface exists.

## 5 Serialization
Tags `instr = "eq_opt"`.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 