# Credit Derivatives (CDS) – Detailed Design Document

## 1 Overview
The `Cds` struct models a single‐name credit default swap referencing a hazard curve for default probability.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `spread` | `F` | Annualised running spread. |
| `maturity` | `Date` | Contract maturity. |
| `pay_freq` | `Frequency` | Premium payment frequency. |
| `notional` | `F` | Protection notional. |

## 3 Valuation
PV = PV of premium leg – PV of protection leg, integrating hazard curve over default times.

## 4 Risk Measures
* PV01 / CS01, Jump‐to‐Default.

## 5 Validation & Serialization
Serde tag `instr = "cds"`.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 