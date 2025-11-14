# Attribution Serialization

**Date**: 2025-11-14  
**Status**: Complete  
**Result**: Full JSON envelope support for attribution requests and results

## Executive Summary

The attribution module now provides complete JSON serialization support for attribution requests and results via versioned envelopes (`finstack.attribution/1`). This enables external systems to trigger attribution runs via stable JSON contracts and retrieve structured results.

## Schema Coverage

### Directory Structure

```
finstack/valuations/schemas/attribution/1/
├── attribution.schema.json          # Request envelope schema
└── attribution_result.schema.json   # Result envelope schema

finstack/valuations/tests/attribution/json_examples/
└── bond_attribution_parallel.example.json
```

## Covered Types

### Request Envelope (`AttributionEnvelope`)

**Rust Module**: `finstack_valuations::attribution::spec`

- `AttributionEnvelope` (struct):
  - `schema: String` – Version identifier (`"finstack.attribution/1"`)
  - `attribution: AttributionSpec` – The attribution specification

- `AttributionSpec` (struct):
  - `instrument: InstrumentJson` – Instrument to attribute (via `InstrumentEnvelope`)
  - `market_t0: MarketContextState` – Market snapshot at T₀
  - `market_t1: MarketContextState` – Market snapshot at T₁
  - `as_of_t0: Date` – Valuation date at T₀
  - `as_of_t1: Date` – Valuation date at T₁
  - `method: AttributionMethod` – Methodology (Parallel/Waterfall/MetricsBased)
  - `config: Option<AttributionConfig>` – Optional config overrides

- `AttributionConfig` (struct):
  - `tolerance_abs: Option<f64>` – Absolute tolerance for residual
  - `tolerance_pct: Option<f64>` – Percentage tolerance for residual
  - `metrics: Option<Vec<String>>` – Metrics for metrics-based attribution

**Schema**: `attribution.schema.json`  
**Examples**: 1 minimal bond example with parallel attribution

### Result Envelope (`AttributionResultEnvelope`)

**Rust Module**: `finstack_valuations::attribution::spec`

- `AttributionResultEnvelope` (struct):
  - `schema: String` – Version identifier
  - `result: AttributionResult` – The attribution result

- `AttributionResult` (struct):
  - `attribution: PnlAttribution` – Complete P&L attribution
  - `results_meta: ResultsMeta` – Metadata (timestamp, version, rounding)

**Schema**: `attribution_result.schema.json`

### Configuration Types

All attribution configuration types now support full JSON serialization:

- `AttributionMethod` (enum): `Parallel`, `Waterfall(Vec<Factor>)`, `MetricsBased`
- `AttributionFactor` (enum): All 9 factors (Carry, RatesCurves, etc.)
- `ModelParamsSnapshot` (enum): StructuredCredit, Convertible, None

**Tests**: `tests/attribution/config_serialization.rs` (9 roundtrip tests)

## Envelope Pattern

All attribution requests/results follow the standard Finstack envelope:

```json
{
  "schema": "finstack.attribution/1",
  "attribution": { /* payload */ }
}
```

## API Surface

### Rust

```rust
use finstack_valuations::attribution::{AttributionEnvelope, AttributionSpec};

// Parse from JSON
let envelope = AttributionEnvelope::from_json(json_str)?;

// Execute attribution
let result_envelope = envelope.execute()?;

// Serialize result
let result_json = result_envelope.to_string()?;
```

### Python

```python
from finstack.valuations import attribute_pnl_from_json, attribution_result_to_json

# Execute attribution from JSON spec
attribution = attribute_pnl_from_json(json_spec_str)

# Serialize result to JSON
result_json = attribution_result_to_json(attribution)
```

## Testing

### Rust Tests

**File**: `finstack/valuations/tests/attribution/serialization_roundtrip.rs`

Tests:
1. `test_attribution_envelope_json_roundtrip` – Full envelope roundtrip
2. `test_attribution_envelope_waterfall_roundtrip` – Waterfall method preservation
3. `test_attribution_config_roundtrip` – Config serialization
4. `test_attribution_envelope_from_example_json` – Example JSON loading
5. `test_attribution_envelope_to_from_json_helpers` – Helper methods
6. `test_attribution_result_envelope_roundtrip` – Result serialization

**Config Tests**: `tests/attribution/config_serialization.rs` (9 tests)

All tests pass ✅

**Run**:
```bash
cargo test -p finstack-valuations --test attribution_tests serialization_roundtrip
cargo test -p finstack-valuations --test attribution_tests config_serialization
```

### Python Tests

**File**: `finstack-py/tests/test_attribution_serialization.py`

Tests:
1. `test_attribution_from_json_minimal` – JSON parsing validation
2. `test_attribution_from_json_with_waterfall` – Waterfall method support
3. `test_attribution_from_json_with_config` – Config overrides
4. `test_attribution_result_to_json` – Result serialization (skipped, requires complex setup)

**Run**:
```bash
uv run pytest finstack-py/tests/test_attribution_serialization.py -v
```

## Type Mappings

### Core Primitives

- **`Money`**: `{"amount": "string", "currency": "ISO-4217-3-letter"}`
- **`Date`**: ISO 8601 string (`"YYYY-MM-DD"`)
- **`AttributionMethod`**: Enum with variants:
  - String literal: `"Parallel"`, `"MetricsBased"`
  - Tagged object: `{"Waterfall": ["Carry", "RatesCurves", ...]}`

### Enum Encoding

- **Unit variants**: String literal (`"Parallel"`, `"MetricsBased"`)
- **Struct variants**: Object with variant name as key (`{"Waterfall": [...]}`)
- **Factors**: String enum (`"Carry"`, `"RatesCurves"`, `"CreditCurves"`, etc.)

## Usage Examples

### Rust: JSON-Driven Attribution

```rust
use finstack_valuations::attribution::AttributionEnvelope;

// Load from JSON file
let json = std::fs::read_to_string("attribution_request.json")?;
let envelope = AttributionEnvelope::from_json(&json)?;

// Execute
let result = envelope.execute()?;

// Serialize result
let result_json = result.to_string()?;
std::fs::write("attribution_result.json", result_json)?;
```

### Python: JSON-Driven Attribution

```python
import json
from finstack.valuations import attribute_pnl_from_json, attribution_result_to_json

# Load request
with open("attribution_request.json") as f:
    spec_json = f.read()

# Execute
attribution = attribute_pnl_from_json(spec_json)

# Print results
print(attribution.explain())

# Serialize and save result
result_json = attribution_result_to_json(attribution)
with open("attribution_result.json", "w") as f:
    f.write(result_json)
```

### Python: Programmatic API (No JSON)

The traditional programmatic API remains available and is often more convenient for in-process workflows:

```python
from datetime import date
from finstack.valuations import attribute_pnl, AttributionMethod

# Use objects directly
attribution = attribute_pnl(
    instrument=bond,
    market_t0=market_yesterday,
    market_t1=market_today,
    as_of_t0=date(2025, 1, 15),
    as_of_t1=date(2025, 1, 16),
    method=AttributionMethod.parallel()
)

print(f"Total P&L: {attribution.total_pnl}")
print(f"Carry: {attribution.carry}")
```

## Design Decisions

### Results Are Compute-Oriented

Attribution **results** (`PnlAttribution`, `PortfolioAttribution`) derive `Serialize`/`Deserialize` for ad-hoc exports but are primarily designed for in-process use:

- Export methods (`to_csv()`, `to_json()`, `explain()`) provide structured outputs
- Full round-trip is supported but not the primary workflow
- Results contain trait-object references (via indices/IDs) rather than embedded market data

### Requests Use Envelopes

Attribution **requests** use the envelope pattern for stable wire formats:

- `AttributionEnvelope` wraps complete specs (instrument + markets + config)
- Enables external systems to submit attribution jobs via JSON
- Markets are serialized as `MarketContextState` snapshots
- Instruments use the existing `InstrumentJson` tagged union

## Schema Stability

All schemas use version `1` in namespace `finstack.attribution/1`.

Breaking changes will increment the version (e.g., `finstack.attribution/2`).

Backward compatibility is maintained via:
1. Strict `additionalProperties: false` on all envelope objects
2. Explicit `required` field lists
3. Stable serde field names (`#[serde(rename_all = "snake_case")]`)
4. Optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`

## Integration

Attribution serialization is ready for use across:
- **Rust**: Direct serde deserialization with `AttributionEnvelope`
- **Python bindings** (`finstack-py`): `attribute_pnl_from_json()` and `attribution_result_to_json()`
- **WASM bindings** (`finstack-wasm`): Future work (not yet implemented)

## Parity with Calibration

The attribution serialization follows the same patterns as calibration:

| Feature | Calibration | Attribution |
|---------|------------|-------------|
| Request envelope | `CalibrationEnvelope` | `AttributionEnvelope` |
| Result envelope | `CalibrationResultEnvelope` | `AttributionResultEnvelope` |
| Schema namespace | `finstack.calibration/1` | `finstack.attribution/1` |
| Market snapshots | `MarketContextState` | `MarketContextState` |
| Execution helper | `.execute()` | `.execute()` |
| Python JSON API | ✅ | ✅ |
| WASM support | ✅ | 🔲 Future |

## Related Documentation

- [Cashflow Schemas](./CASHFLOW_SCHEMAS_SERIALIZATION.md) – Cashflow spec serialization
- [P&L Attribution](../book/src/valuations/pnl-attribution.md) – User guide
- [Attribution README](../finstack/valuations/src/attribution/README.md) – Implementation status

## Summary

✅ All attribution config types now serialize/deserialize  
✅ Full envelope support for requests and results  
✅ JSON schemas and examples provided  
✅ 15 Rust roundtrip tests passing  
✅ 3 Python integration tests passing  
✅ Python type stubs updated with new JSON API  
✅ Ready for use in external integration pipelines  

The attribution module now provides complete serialization parity with calibration, enabling JSON-driven attribution workflows across Rust and Python.

