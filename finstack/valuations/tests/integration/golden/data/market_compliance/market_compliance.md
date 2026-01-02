# Market Compliance Golden Fixtures

This document tracks the valuation parity fixtures used by market compliance tests.

## Current Coverage

- Rates: bond PV and IRS PV via `finstack/valuations/tests/golden/rates.json`
- Credit/FX/Equity fixtures are staged but awaiting validated reference values

Rates fixtures currently mirror the values in `../parity/golden_values.json` until the
reference engine outputs are finalized.

These cases are interpreted with standard USD conventions when the fixture does not
specify leg schedules explicitly (semi-annual fixed 30/360, quarterly float ACT/360,
Modified Following, T-2 reset, USNY calendar).

## Fixture Plan

Dedicated JSON fixtures per instrument family live under `finstack/valuations/tests/golden/`:

- `rates.json` (bonds, swaps, caps/floors)
- `credit.json` (CDS, CDS index, tranches)
- `fx.json` (fx spot/forwards/options)
- `equity.json` (equity options, variance swaps)

## Fixture Schema

Rates fixtures currently use a bespoke schema aligned to the bond/IRS tests.
Credit/FX/Equity fixtures use a generic schema so any instrument can be priced
via `InstrumentJson` and a fully specified `MarketContext`:

```json
{
  "valuation_date": "2024-01-02",
  "instrument": { "type": "...", "spec": { /* InstrumentJson */ } },
  "market_context": { /* MarketContextState */ },
  "expected": { "present_value": 0.0, "tolerance": 0.0, "currency": "USD" }
}
```

Each fixture should include:

- Explicit conventions (day count, calendars, settlement, compounding)
- Market context (curve/surface IDs and data)
- Expected PV and greeks with tolerances
- Source metadata (reference engine, version, date)

## Fixture Status Gating

Parity tests run automatically once a fixture declares `status = "certified"`.
Any other status (e.g., `provisional`, `pending_reference_values`) causes the
parity tests to skip while still validating that the JSON parses.

## Reference Sources

Use external reference engines (e.g., QuantLib, Bloomberg) to populate expected values.
Document any intentional convention differences in the fixture metadata.
