# Metric Keys

Metric keys are fully qualified strings that identify a computed risk measure.
They follow the pattern:

```text
metric_type::curve_or_entity::tenor_or_qualifier
```

## Scalar Metrics

| Key | Description | Unit |
|-----|-------------|------|
| `npv` | Net present value | Currency |
| `dv01` | Dollar value of 1bp parallel rate shift | Currency |
| `cs01` | Credit spread sensitivity (1bp) | Currency |
| `duration` | Macaulay duration | Years |
| `modified_duration` | Modified duration | Years |
| `convexity` | Convexity | Years² |
| `ytm` | Yield to maturity | Decimal |
| `z_spread` | Z-spread over discount curve | bps |
| `oas` | Option-adjusted spread | bps |
| `vega` | Sensitivity to vol (1% shift) | Currency |
| `theta` | Time decay (1 day) | Currency |
| `delta` | Spot sensitivity | Currency |
| `gamma` | Second-order spot sensitivity | Currency |
| `rho` | Rate sensitivity | Currency |

## Curve-Level Metrics

| Pattern | Example | Description |
|---------|---------|-------------|
| `pv01::CURVE` | `pv01::usd_ois` | PV01 per curve |
| `cs01::ENTITY` | `cs01::ACME-HZD` | CS01 per hazard curve |
| `cs01::INSTRUMENT` | `cs01::BOND_A` | Z-spread CS01 (bonds without hazard curve) |

## Bucketed Metrics

| Pattern | Example | Description |
|---------|---------|-------------|
| `bucketed_dv01::CURVE::TENOR` | `bucketed_dv01::USD-OIS::10y` | Key-rate DV01 |
| `bucketed_cs01::ENTITY::TENOR` | `bucketed_cs01::ACME-HZD::5y` | Key-rate CS01 |
| `vega::UNDERLYING::EXPIRY` | `vega::AAPL::6m` | Vega by expiry bucket |

## Standard Tenor Buckets

```text
1m, 3m, 6m, 1y, 2y, 3y, 5y, 7y, 10y, 15y, 20y, 25y, 30y
```

## Aggregation Rules

| Metric | Aggregation | Notes |
|--------|-------------|-------|
| `npv` | Sum | Total portfolio value |
| `dv01` | Sum | Additive across positions |
| `cs01` | Sum | Additive across positions |
| `bucketed_dv01` | Sum | Per-bucket sum |
| `duration` | Notional-weighted | Weighted average |
| `ytm` | Notional-weighted | Weighted average |
| `z_spread` | No aggregation | Position-level only |
| `vega` | Sum | Additive |

## Key Naming Rules

1. Metric names: `snake_case`
2. Separator: `::` (never `/` or `.`)
3. Curve IDs: verbatim from market data (e.g., `USD-OIS`)
4. Entity IDs: verbatim (e.g., `ACME-HZD`)
5. Tenors: standard abbreviations (`6m`, `1y`, `10y`)
6. Z-spread CS01 uses instrument ID (not `z_spread`)
