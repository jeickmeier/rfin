# Cashflow Schemas

The `finstack.cashflow/1` schema namespace provides JSON schemas and examples for all cashflow builder specifications. These types define how principal, coupons, fees, and credit behaviors are configured for instruments and standalone cashflow legs.

## Overview

Cashflow schemas follow the standard Finstack envelope pattern:

```json
{
  "schema": "finstack.cashflow/1",
  "<spec_type>": { /* specification payload */ }
}
```

All schemas are located under `finstack/valuations/schemas/cashflow/1/` with corresponding example JSON files in `finstack/valuations/tests/cashflow/json_examples/`.

## Schema Catalog

### Amortization & Notional

**File**: `amortization_spec.schema.json`

Defines principal amortization schedules and notional specifications:

- **`AmortizationSpec`**: Enum describing how principal amortizes over time
  - `None`: No amortization (bullet repayment)
  - `LinearTo`: Linear paydown to a target final notional
  - `StepRemaining`: Explicit schedule of remaining principal by date
  - `PercentPerPeriod`: Fixed percentage of original notional paid each period
  - `CustomPrincipal`: Custom principal exchanges on specific dates

- **`Notional`**: Combines initial principal with an amortization rule

**Examples**:
- `notional_par.example.json` – Non-amortizing notional
- `notional_percent_per_period.example.json` – 5% per-period amortization
- `amortization_linear_to.example.json` – Linear paydown schedule
- `amortization_step_remaining.example.json` – Step schedule with explicit dates

### Coupons

**File**: `coupon_specs.schema.json`

Defines fixed and floating rate coupon specifications:

- **`CouponType`**: Payment structure (Cash, PIK, or Split)
- **`FixedCouponSpec`**: Fixed-rate coupon configuration
  - Rate, frequency, day count, business day convention, stub handling
- **`FloatingRateSpec`**: Canonical floating rate specification
  - Index curve, spread, gearing, floor/cap, reset conventions
- **`FloatingCouponSpec`**: Floating coupon wrapping `FloatingRateSpec`

**Examples**:
- `coupon_type_cash.example.json` – 100% cash coupon
- `coupon_type_split.example.json` – 70% cash / 30% PIK split
- `fixed_coupon_spec.example.json` – 4.25% semi-annual fixed coupon
- `floating_rate_spec.example.json` – SOFR + 200bp with 0% floor
- `floating_coupon_spec.example.json` – EURIBOR-6M floating coupon

### Prepayment Models

**File**: `prepayment_model_spec.schema.json`

Defines prepayment behavior for credit instruments:

- **`PrepaymentCurve`**: Curve shape (Constant or PSA)
- **`PrepaymentModelSpec`**: CPR-based prepayment model with optional curve

**Examples**:
- `prepayment_model_constant.example.json` – Constant 6% CPR
- `prepayment_model_psa_100.example.json` – 100% PSA standard curve

### Default Models

**File**: `default_model_spec.schema.json`

Defines default behavior for credit instruments:

- **`DefaultCurve`**: Curve shape (Constant or SDA)
- **`DefaultModelSpec`**: CDR-based default model with optional curve
- **`DefaultEvent`**: Explicit default event specification

**Examples**:
- `default_model_constant.example.json` – Constant 2% CDR
- `default_model_sda_100.example.json` – 100% SDA standard curve
- `default_event.example.json` – Default event with recovery

### Recovery Models

**File**: `recovery_model_spec.schema.json`

Defines recovery assumptions for defaulted amounts:

- **`RecoveryModelSpec`**: Recovery rate and lag specification

**Examples**:
- `recovery_model_standard.example.json` – 40% recovery rate with no lag

### Fees

**File**: `fee_specs.schema.json`

Defines fixed and periodic fee specifications:

- **`FeeSpec`**: Enum for fee types
  - `Fixed`: One-time fee on a specific date
  - `PeriodicBps`: Periodic fee as basis points of a base (drawn/undrawn)
- **`FeeBase`**: Fee calculation base (Drawn or Undrawn)
- **`FeeTier`**: Utilization-based fee tier

**Examples**:
- `fee_spec_fixed.example.json` – $50,000 fixed fee
- `fee_spec_periodic_bps.example.json` – Quarterly 50bp fee on drawn
- `fee_tier.example.json` – Fee tier at 50% utilization

### Schedule Parameters

**File**: `schedule_params.schema.json`

Defines cashflow schedule generation parameters:

- **`ScheduleParams`**: Frequency, day count, business day convention, calendar, stub handling

**Examples**:
- `schedule_params_usd_standard.example.json` – Quarterly Act/360 USD schedule

## Usage

### Rust

Cashflow specs are defined in `finstack_valuations::cashflow::builder::specs`:

```rust
use finstack_valuations::cashflow::builder::specs::{
    Notional, AmortizationSpec, FixedCouponSpec, FloatingRateSpec,
    PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
    FeeSpec, ScheduleParams,
};
use finstack_core::currency::Currency;

// Create a non-amortizing notional
let notional = Notional::par(1_000_000.0, Currency::USD);

// Build a fixed coupon spec
let coupon = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: 0.0425,
    freq: Frequency::semi_annual(),
    dc: DayCount::Thirty360,
    bdc: BusinessDayConvention::ModifiedFollowing,
    calendar_id: None,
    stub: StubKind::None,
};
```

### JSON Deserialization

All specs can be deserialized from JSON envelopes:

```rust
use serde_json;

let json = r#"{
  "schema": "finstack.cashflow/1",
  "notional": {
    "initial": { "amount": "1000000", "currency": "USD" },
    "amort": "None"
  }
}"#;

#[derive(serde::Deserialize)]
struct NotionalEnvelope {
    schema: String,
    notional: Notional,
}

let envelope: NotionalEnvelope = serde_json::from_str(json)?;
```

## Type Mappings

### Common Primitives

- **`Money`**: Object with `amount` (string) and `currency` (ISO 4217 3-letter code)
- **`Date`**: ISO 8601 date string (YYYY-MM-DD)
- **`CurveId`**: String identifier for market curves
- **`Frequency`**: Object like `{"Months": 3}` for quarterly
- **`DayCount`**: String enum (`"Act360"`, `"Act365F"`, `"ActAct"`, `"Thirty360"`)
- **`BusinessDayConvention`**: String enum (`"unadjusted"`, `"following"`, `"modified_following"`, `"preceding"`, `"modified_preceding"`)
- **`StubKind`**: String enum (`"None"`, `"ShortFront"`, `"ShortBack"`, `"LongFront"`, `"LongBack"`)

### Enums

Rust enums are serialized following serde conventions:

- **Unit variants**: String literal (e.g., `"Cash"`, `"PIK"`)
- **Struct variants**: Object with variant name as key (e.g., `{"LinearTo": {"final_notional": {...}}}`)
- **Tagged enums**: Object with tag field (e.g., `{"curve": "psa", "speed_multiplier": 1.0}`)

## Schema Versioning

All cashflow schemas use version `1` in the namespace `finstack.cashflow/1`. The envelope `schema` field must be set to `"finstack.cashflow/1"`.

Future breaking changes will increment the version number (e.g., `finstack.cashflow/2`).

## Testing

Cashflow schema parity is tested via roundtrip serialization tests in `finstack/valuations/tests/cashflow_schemas_examples.rs`. Each example JSON is deserialized into Rust types and re-serialized to ensure schema correctness.

Run tests:

```bash
cargo test -p finstack-valuations --test cashflow_schemas_examples
```

## Related Documentation

- [Instrument Schemas](../io/instrument-schemas.md) – Top-level instrument JSON schemas
- [Calibration Schemas](./calibration.md) – Market data calibration schemas
- [Core Types](../core/currency-money.md) – Primitive types used in cashflow specs

