# Cashflow Schemas Serialization

**Date**: 2025-11-14  
**Status**: Complete  
**Result**: Full schema coverage for all public cashflow builder specs

## Executive Summary

All public, serde-enabled cashflow specification types under `finstack_valuations::cashflow::builder::specs` now have complete JSON schema coverage and example JSON envelopes. This ensures stable wire formats for cashflow configuration across Rust, Python, and WebAssembly bindings.

## Schema Coverage

### Directory Structure

```
finstack/valuations/schemas/cashflow/1/
├── amortization_spec.schema.json
├── coupon_specs.schema.json
├── prepayment_model_spec.schema.json
├── default_model_spec.schema.json
├── recovery_model_spec.schema.json
├── fee_specs.schema.json
└── schedule_params.schema.json

finstack/valuations/tests/cashflow/json_examples/
├── notional_par.example.json
├── notional_percent_per_period.example.json
├── amortization_linear_to.example.json
├── amortization_step_remaining.example.json
├── coupon_type_cash.example.json
├── coupon_type_split.example.json
├── fixed_coupon_spec.example.json
├── floating_rate_spec.example.json
├── floating_coupon_spec.example.json
├── prepayment_model_constant.example.json
├── prepayment_model_psa_100.example.json
├── default_model_constant.example.json
├── default_model_sda_100.example.json
├── default_event.example.json
├── recovery_model_standard.example.json
├── fee_spec_fixed.example.json
├── fee_spec_periodic_bps.example.json
├── fee_tier.example.json
└── schedule_params_usd_standard.example.json
```

## Covered Types

### Amortization & Notional
**Rust Module**: `finstack_valuations::cashflow::builder::specs::amortization`

- `AmortizationSpec` (enum):
  - `None` – Bullet repayment
  - `LinearTo { final_notional }` – Linear paydown
  - `StepRemaining { schedule }` – Explicit schedule
  - `PercentPerPeriod { pct }` – Fixed percentage per period
  - `CustomPrincipal { items }` – Custom exchanges

- `Notional` (struct):
  - `initial: Money` – Initial principal
  - `amort: AmortizationSpec` – Amortization rule

**Schema**: `amortization_spec.schema.json`  
**Examples**: 4 variants covering bullet, percent-based, linear, and step schedules

### Coupons
**Rust Module**: `finstack_valuations::cashflow::builder::specs::coupon`

- `CouponType` (enum):
  - `Cash` – 100% cash payment
  - `PIK` – 100% payment-in-kind
  - `Split { cash_pct, pik_pct }` – Mixed cash/PIK

- `FixedCouponSpec` (struct):
  - Rate, frequency, day count, BDC, calendar, stub

- `FloatingRateSpec` (struct):
  - Index curve, spread, gearing, floor/cap, reset conventions

- `FloatingCouponSpec` (struct):
  - Embeds `FloatingRateSpec` with payment-specific settings

**Schema**: `coupon_specs.schema.json`  
**Examples**: 5 variants covering cash, PIK, split, fixed, and floating coupons

### Prepayment Models
**Rust Module**: `finstack_valuations::cashflow::builder::specs::prepayment`

- `PrepaymentCurve` (tagged enum):
  - `{ curve: "constant" }` – Constant CPR
  - `{ curve: "psa", speed_multiplier }` – PSA curve

- `PrepaymentModelSpec` (struct):
  - `cpr: f64` – Annual prepayment rate
  - `curve: Option<PrepaymentCurve>` – Curve shape

**Schema**: `prepayment_model_spec.schema.json`  
**Examples**: 2 variants (constant CPR, 100% PSA)

### Default Models
**Rust Module**: `finstack_valuations::cashflow::builder::specs::default`

- `DefaultCurve` (tagged enum):
  - `{ curve: "constant" }` – Constant CDR
  - `{ curve: "sda", speed_multiplier }` – SDA curve

- `DefaultModelSpec` (struct):
  - `cdr: f64` – Annual default rate
  - `curve: Option<DefaultCurve>` – Curve shape

- `DefaultEvent` (struct):
  - `default_date`, `defaulted_amount`, `recovery_rate`, `recovery_lag`

**Schema**: `default_model_spec.schema.json`  
**Examples**: 3 variants (constant CDR, 100% SDA, explicit default event)

### Recovery Models
**Rust Module**: `finstack_valuations::cashflow::builder::specs::recovery`

- `RecoveryModelSpec` (struct):
  - `rate: f64` – Recovery rate (0.0 to 1.0)
  - `recovery_lag: u32` – Lag in months

**Schema**: `recovery_model_spec.schema.json`  
**Examples**: 1 standard (40% recovery with 0 lag)

### Fees
**Rust Module**: `finstack_valuations::cashflow::builder::specs::fees`

- `FeeSpec` (enum):
  - `Fixed { date, amount }` – One-time fee
  - `PeriodicBps { base, bps, freq, dc, bdc, calendar_id, stub }` – Recurring fee

- `FeeBase` (enum):
  - `Drawn` – Fee on drawn balance
  - `Undrawn { facility_limit }` – Fee on undrawn

- `FeeTier` (struct):
  - `threshold: f64`, `bps: f64` – Utilization-based tier

**Schema**: `fee_specs.schema.json`  
**Examples**: 3 variants (fixed fee, periodic bps, fee tier)

### Schedule Parameters
**Rust Module**: `finstack_valuations::cashflow::builder::specs::schedule`

- `ScheduleParams` (struct):
  - `freq`, `dc`, `bdc`, `calendar_id`, `stub`

**Schema**: `schedule_params.schema.json`  
**Examples**: 1 standard (quarterly Act/360 USD)

## Envelope Pattern

All cashflow schemas follow the standard Finstack envelope:

```json
{
  "schema": "finstack.cashflow/1",
  "<spec_type>": { /* payload */ }
}
```

Where `<spec_type>` is one of: `notional`, `amortization_spec`, `coupon_type`, `fixed_coupon_spec`, `floating_rate_spec`, `floating_coupon_spec`, `prepayment_model_spec`, `default_model_spec`, `default_event`, `recovery_model_spec`, `fee_spec`, `fee_tier`, `schedule_params`.

## Type Mappings

### Core Primitives

- **`Money`**: `{"amount": "string", "currency": "ISO-4217-3-letter"}`
- **`Date`**: ISO 8601 string (`"YYYY-MM-DD"`)
- **`CurveId`**: String identifier
- **`Frequency`**: `{"Months": integer}`
- **`DayCount`**: String enum (`"Act360"`, `"Act365F"`, `"ActAct"`, `"Thirty360"`)
- **`BusinessDayConvention`**: String enum (snake_case: `"unadjusted"`, `"following"`, `"modified_following"`, `"preceding"`, `"modified_preceding"`)
- **`StubKind`**: String enum (`"None"`, `"ShortFront"`, `"ShortBack"`, `"LongFront"`, `"LongBack"`)

### Enum Encoding

- **Unit variants**: String literal (`"Cash"`, `"PIK"`)
- **Struct variants**: Object with variant name as key (`{"LinearTo": {...}}`)
- **Tagged enums** (with `#[serde(tag = "...")]`): Object with discriminator field (`{"curve": "psa", "speed_multiplier": 1.0}`)

## Testing

All schemas and examples are validated via roundtrip serialization tests in:

**File**: `finstack/valuations/tests/cashflow_schemas_examples.rs`

Each test:
1. Loads example JSON via `include_str!`
2. Deserializes into Rust envelope struct wrapping the spec type
3. Re-serializes to JSON
4. Compares original and reserialized as `serde_json::Value` for equality

**Test count**: 19 tests covering all spec variants

**Run**:
```bash
cargo test -p finstack-valuations --test cashflow_schemas_examples
```

All tests pass ✅

## Documentation

Cashflow schemas are documented in:

**Book**: `book/src/valuations/cashflow-schemas.md`

Includes:
- Schema catalog with descriptions
- Type mappings
- Usage examples (Rust and JSON)
- Schema versioning
- Related documentation links

## Excluded Types

The following `schedule` module types are **not** serializable and excluded by design:

- `FloatCouponParams` – Internal helper (no `#[cfg_attr(feature = "serde", ...)]`)
- `FixedWindow` – Internal helper
- `FloatWindow` – Internal helper

These are transient runtime structures used only within cashflow builder internals.

## Schema Stability

All schemas use version `1` in namespace `finstack.cashflow/1`.

Breaking changes will increment the version (e.g., `finstack.cashflow/2`).

Backward compatibility is maintained via:
1. Strict `additionalProperties: false` on all envelope objects
2. Explicit `required` field lists
3. Stable serde field names (`#[serde(rename_all = "snake_case")]` where applicable)
4. Default values annotated in schemas for optional fields

## Integration

Cashflow schemas are ready for use across:
- **Rust**: Direct serde deserialization
- **Python bindings** (`finstack-py`): PyO3 + Pydantic v2 mirrors
- **WASM bindings** (`finstack-wasm`): `serde_wasm_bindgen` JSON interop

No additional work required for cross-language parity.

## Future Extensions

If new cashflow spec types are added, follow this pattern:

1. Add Rust type with `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`
2. Create schema file under `schemas/cashflow/1/<name>.schema.json`
3. Add at least one example JSON to `schemas/cashflow/1/examples/`
4. Add roundtrip test to `cashflow_schemas_examples.rs`
5. Document in `book/src/valuations/cashflow-schemas.md`

## Related Audits

- [Tree Parameters Serialization Audit](./TREE_PARAMS_SERIALIZATION_AUDIT.md) – Tree models (no serialization required)
- Instrument schemas – `finstack/valuations/schemas/instrument/1/*.schema.json`
- Calibration schemas – `finstack/valuations/schemas/calibration/1/*.schema.json`

