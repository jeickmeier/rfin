# Commodity Futures & Forwards – Detailed Design Document

## 1 Overview
Models exchange‐traded futures and OTC forwards on commodities with storage and convenience yield adjustments.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `commodity` | `String` | Commodity identifier. |
| `contract_month` | `Date` | Delivery month. |
| `price` | `F` | Quoted futures price. |
| `storage_cost` | `Option<F>` | Optional storage cost curve. |
| `conv_yield_curve` | `Option<String>` | Convenience yield curve id. |

## 3 Valuation
Forward curve constructed from collateral curve + convenience yield; PV is discounted difference to market price.

## 4 Analytics
* `implied_convenience_yield()` solver.
* Roll yield to next contract.

## 5 Serialization
Serde tag `instr = "cmdty_future"`.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 