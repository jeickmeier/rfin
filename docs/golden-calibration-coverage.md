# Calibration Golden Coverage

This document tracks the calibration slice of the `finstack.golden/1` rollout.

The calibration harness now uses executable deterministic inputs rather than
`inputs.actual_outputs` snapshots. Curve fixtures build discount, forward,
inflation, hazard, SABR, or smile inputs and probe scalar outputs in both Rust
and Python. External vendor/reference regeneration can replace the internal
formula values later without changing runner shape.

## Existing Coverage

- `calibration/curves/usd_ois_bootstrap.json`
- `calibration/curves/usd_sofr_3m_bootstrap.json`
- `calibration/curves/eur_estr_bootstrap.json`
- `calibration/curves/gbp_sonia_bootstrap.json`
- `calibration/curves/jpy_tona_bootstrap.json`
- `calibration/vol/usd_swaption_sabr_cube.json`

## Added Coverage

- `calibration/curves/usd_cpi_zc_inflation_bootstrap.json`
- `calibration/vol/spx_equity_vol_smile.json`
- `calibration/vol/eurusd_fx_vol_smile.json`
- `calibration/hazard/cdx_ig_hazard.json`
- `calibration/hazard/single_name_hazard_5y.json`

## Domain Mapping

| Fixture directory | Golden domain | Notes |
|---|---|---|
| `calibration/curves` | `rates.calibration.curves` | Existing rates curve fixtures. |
| `calibration/curves` | `inflation.calibration.curves` | Inflation curve fixture. |
| `calibration/vol` | `rates.calibration.swaption_vol` | Existing swaption SABR fixture. |
| `calibration/vol` | `equity.calibration.vol_smile` | Equity volatility smile fixture. |
| `calibration/vol` | `fx.calibration.vol_smile` | FX volatility smile fixture. |
| `calibration/hazard` | `credit.calibration.hazard` | Credit hazard curve fixtures. |

## Metric Key Notes

- Inflation zero-rate outputs use `inflation_zero_rate::<curve>::<pillar>`.
- Inflation CPI forecast outputs use `cpi_forecast::<curve>::<pillar>`.
- Vol smiles use `atm_vol`, wing vols, `risk_reversal`, `butterfly`, and
  `calibration_rmse` style scalar outputs.
- Hazard calibration uses `hazard_rate::<curve>::<pillar>` and
  `survival_probability::<curve>::<pillar>`.
