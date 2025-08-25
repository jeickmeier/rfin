# Repos & Securities‐Financing – Detailed Design Document

## 1 Overview
`RepoTrade` models a collateralised borrowing/lending transaction with haircut and margining schedule.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `cash_leg` | `CashFlowLeg` | Cash payments (principal + interest). |
| `collateral_price` | `F` | Price of collateral security. |
| `haircut` | `F` | Percentage haircut on collateral value. |
| `repo_rate` | `F` | Agreed repo interest rate. |

## 3 Valuation
PV = discounted cash leg ± haircut; collateral MTM schedule placeholder for future development.

## 4 Validation
* `haircut` between 0 and 1.

## 5 Serialization
Serde tag `instr = "repo"`.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 