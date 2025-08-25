# Convertible Bonds – Detailed Design Document

## 1 Overview
`ConvertibleBond` combines a fixed‐coupon debt component with an embedded equity call option. Pricing splits the instrument into bond + option legs.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `bond_leg` | `Bond` | Underlying fixed‐coupon bond parameters. |
| `conversion_ratio` | `F` | Shares per 100 par. |
| `call_option` | `EquityOption` | Embedded call option struct. |

## 3 Valuation
Iteratively solves for yield such that PV(bond) + PV(option) equals market price.

## 4 Validation
* `conversion_ratio` > 0.

## 5 Serialization
Serde tag `instr = "conv_bond"` with nested structs.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 