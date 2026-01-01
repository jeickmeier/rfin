# Scenario DSL and Builder Examples

This directory contains examples demonstrating the Python-side scenario DSL parser and builder API.

## Overview

Finstack provides three complementary ways to create scenarios:

1. **Manual Construction** - Explicit `OperationSpec` and `ScenarioSpec` construction
2. **DSL Parser** - Text-based scenario definition (concise, readable)
3. **Builder API** - Fluent, chainable method calls (type-safe, discoverable)

## Files

- **`dsl_examples.py`** - DSL parser examples
- **`builder_examples.py`** - Builder API examples

## DSL Parser

The DSL parser allows you to define scenarios using simple text commands:

```python
from finstack.scenarios import ScenarioSpec

scenario = ScenarioSpec.from_dsl("""
    # Market shocks
    shift USD.OIS +50bp
    shift equities -10%
    shift fx USD/EUR +3%

    # Time decay
    roll forward 1m

    # Statement adjustments
    adjust revenue +10%
    set cogs 500000
""", scenario_id="stress_test")
```

### Supported DSL Syntax

**Curve Shifts**:
- `shift <CURVE_ID> +/-<VALUE>bp` - Default (discount) curve
- `shift discount <CURVE_ID> +/-<VALUE>bp` - Discount curve
- `shift forward <CURVE_ID> +/-<VALUE>bp` - Forward curve
- `shift hazard <CURVE_ID> +/-<VALUE>bp` - Hazard (credit) curve
- `shift inflation <CURVE_ID> +/-<VALUE>bp` - Inflation curve

**Equity Shocks**:
- `shift equities +/-<VALUE>%` - All equities
- `shift equity <ID> +/-<VALUE>%` - Single equity

**FX Shocks**:
- `shift fx <BASE>/<QUOTE> +/-<VALUE>%` - FX rate

**Vol Shocks**:
- `shift vol <ID> +/-<VALUE>%` - Vol surface

**Time Operations**:
- `roll forward <VALUE><UNIT>` - Roll forward (1d, 1w, 1m, 3m, 1y)

**Statement Operations**:
- `adjust <NODE_ID> +/-<VALUE>%` - Forecast percent change
- `set <NODE_ID> <VALUE>` - Forecast assignment

**Features**:
- Case-insensitive
- Comments with `#`
- Operations separated by semicolons or newlines
- Whitespace-tolerant

## Builder API

The builder provides a fluent, chainable API for constructing scenarios:

```python
from finstack.scenarios.builder import scenario

spec = (
    scenario("stress_test")
    .name("Q1 2024 Stress Test")
    .description("Rate and equity shock")
    .priority(1)
    .shift_discount_curve("USD.OIS", 50)
    .shift_forward_curve("EUR.SOFR", -25)
    .shift_equities(-10)
    .shift_fx("USD", "EUR", 3)
    .shift_vol_surface("SPX_VOL", 20)
    .roll_forward("1m")
    .adjust_forecast("revenue", 10)
    .build()
)
```

### Builder Methods

**Metadata**:
- `.name(str)` - Set display name
- `.description(str)` - Set description
- `.priority(int)` - Set composition priority

**Curve Operations**:
- `.shift_curve(id, bp, kind)` - Generic curve shift
- `.shift_discount_curve(id, bp)` - Discount curve
- `.shift_forward_curve(id, bp)` - Forward curve
- `.shift_hazard_curve(id, bp)` - Hazard curve
- `.shift_inflation_curve(id, bp)` - Inflation curve

**Equity Operations**:
- `.shift_equities(pct, ids)` - Equity price shock

**FX Operations**:
- `.shift_fx(base, quote, pct)` - FX rate shock

**Vol Operations**:
- `.shift_vol_surface(id, pct, kind)` - Vol surface shock

**Time Operations**:
- `.roll_forward(period)` - Time roll-forward

**Statement Operations**:
- `.adjust_forecast(node_id, pct, period_id)` - Forecast percent change
- `.set_forecast(node_id, value, period_id)` - Forecast assignment

**Build**:
- `.build()` - Construct final `ScenarioSpec`

## Running Examples

```bash
# DSL examples
python examples/scenarios/dsl_examples.py

# Builder examples
python examples/scenarios/builder_examples.py
```

## When to Use Each Approach

### Use DSL When

- Writing scenarios in configuration files
- Quick prototyping and exploration
- Non-programmers define scenarios
- Conciseness is priority

### Use Builder When

- Type safety and IDE autocomplete are important
- Programmatic scenario generation
- Complex conditional logic
- Integration with other Python code

### Use Manual When

- Maximum control over operation details
- Debugging specific operations
- Custom operation types

## Scenario Composition

All three approaches produce `ScenarioSpec` objects that can be composed:

```python
from finstack.scenarios import ScenarioEngine

# Create scenarios (any method)
base = ScenarioSpec.from_dsl("shift USD.OIS +25bp", priority=0)
overlay = scenario("overlay").shift_equities(-10).build()

# Compose with deterministic ordering
engine = ScenarioEngine()
composed = engine.compose([base, overlay])
```

## Testing

Both DSL and builder have comprehensive test coverage:

```bash
# Run DSL tests
pytest tests/test_scenarios_dsl.py -v

# Run builder tests
pytest tests/test_scenarios_builder.py -v
```

## Next Steps

1. Try the examples: `python examples/scenarios/dsl_examples.py`
2. Read the API documentation (when available)
3. Check the test files for more usage patterns
4. Integrate with `ScenarioEngine.apply()` for execution

## Notes

- The DSL parser is implemented in Python (no Rust parser yet)
- Both DSL and builder generate identical `ScenarioSpec` objects
- All scenarios can be serialized to/from JSON
- Scenario composition uses priority-based ordering (lower = higher priority)
