# Calibration (v2 plan-driven)

The `calibration` module provides **plan-driven** calibration for term structures and surfaces:

- Discount curves (OIS / risk-free)
- Forward curves (IBOR/RFR projection)
- Hazard curves (CDS bootstrapping)
- Inflation curves (ZCIS bootstrapping)
- Volatility surfaces (SABR)
- Swaption volatility surfaces
- Base correlation curves (CDS tranches)

The public surface is the **v2 schema + engine**, plus shared config/report/validation helpers.

---

## Directory layout

```
calibration/
├── mod.rs            # Public surface + re-exports
├── api/              # v2 schema + plan execution engine
├── domain/           # quotes + pricing + solvers used by the engine
├── adapters/         # step handlers + targets bridging schema → domain
├── bumps/            # curve shock helpers used by scenarios + risk metrics
├── config.rs         # CalibrationConfig (plan.settings) + extension parsing
├── report.rs         # CalibrationReport
├── solver/           # numeric utilities (bracketing, penalties)
└── validation.rs     # no-arbitrage / monotonicity checks
```

---

## The v2 contract

### Request envelope

- Schema id: `finstack.calibration/2`
- Entry point (Rust): `finstack_valuations::calibration::api::engine::execute`
- Envelope type: `finstack_valuations::calibration::api::schema::CalibrationEnvelopeV2`

Conceptually:

1. Provide `quote_sets` (named arrays of `MarketQuote`).
2. Provide ordered `steps` referencing a quote set by name.
3. Provide global `settings: CalibrationConfig` (optional; defaults are deterministic).
4. Optionally provide `initial_market` to seed the plan with existing curves/surfaces.

### Result envelope

The engine returns a result envelope (`CalibrationResultEnvelope`) containing:

- `final_market` (serialized `MarketContextState`)
- `report` (plan-level merged report)
- `step_reports` (per-step reports)

---

## Configuration via `FinstackConfig.extensions`

Global calibration settings can be sourced from:

- **`valuations.calibration.v2`** (key: `CALIBRATION_CONFIG_KEY_V2`)

This is used by `CalibrationConfig::from_finstack_config_or_default`.

---

## Bumps (scenarios + risk)

`calibration::bumps` provides helpers that rebuild curves **by re-calibration** (deterministic),
instead of mutating curves in-place. This is used by:

- the scenarios crate for curve shocks
- risk metrics like DV01/CS01 in “re-bootstrap” modes


