# SpotAsset – Detailed Design Document

## 1 Overview
`SpotAsset` represents a cash‐settled transfer of a notional amount in a single currency on a known settlement date.

## 2 Data Model
| Field | Type | Description |
|-------|------|-------------|
| `ccy` | `Currency` | ISO currency of the notional amount. |
| `amount` | `F` (generic floating) | Notional amount to be received (positive) or paid (negative). |
| `settle_date` | `Date` | Cash settlement date. |

## 3 Valuation
The net present value (NPV) is analytic:

\[ \text{PV} = \text{amount} \times DF(\text{val\_date} \rightarrow \text{settle\_date}) \]
where \(DF\) is obtained from the discount curve provided by the caller.

## 4 Builder API
A fluent builder enforces mandatory fields at compile-time:
```rust
let pv = SpotAsset::builder()
    .ccy(EUR)
    .amount(10_000_000.0)
    .settle_date(spot.plus_business_days(2))
    .build()
    .pv(&curves, val_date)?;
```

## 5 Validation Rules
* `amount` must be non-zero.
* `settle_date` ≥ `val_date`.
* Supported currencies enumerated in `Currency`.

## 6 Serialization
Derives `Serialize`, `Deserialize` with version tag `instr = "spot"` and `version = 1`.

## 7 Testing
Golden PV checked against analytic formula; property test ensures PV sign flips when `amount` sign flips.

---
*Extracted from original consolidated design document, last updated: 2025-06-29* 