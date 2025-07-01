# Bermudan Swaps – Detailed Design Document

## 1 Overview
`BermudanSwap` embeds multiple call/put options allowing the holder to terminate or extend a swap at predefined dates.

## 2 Data Model
| Field | Type | Notes |
|-------|------|-------|
| `underlying_swap` | `Swap` | Base swap definition. |
| `call_dates` | `Vec<Date>` | Option exercise dates. |
| `puttable` | `bool` | If `true`, holder may shorten (put). |

## 3 Valuation
Full Longstaff–Schwartz lattice implemented in the risk module; current placeholder stores pay‐off schedule only.

## 4 Serialization
Tag `instr = "berm_swap"`.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 