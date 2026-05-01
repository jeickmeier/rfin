# Rates Golden Coverage

This document tracks the rates slice of the `finstack.golden/1` rollout.

The executed framework uses instrument-specific fixture directories under
`finstack/valuations/tests/golden/data/`, such as `pricing/irs`, rather than
the older design-table bucket `pricing/rates`. This keeps Rust one-line tests
and Python parametrized collectors aligned with one instrument kind per
directory.

## Existing Coverage

| Design row | Landed fixture | Notes |
|---|---|---|
| USD SOFR 5Y off-par IRS | `pricing/irs/usd_sofr_5y_receive_fixed_swpm.json` | Bloomberg SWPM receive-fixed fixture. Treat this as the off-par USD SOFR 5Y row; do not recapture a duplicate under a second name. |

## Remaining Rates Rows

### Pricing

- `pricing/deposit/usd_deposit_3m.json`
- `pricing/fra/usd_fra_3x6.json`
- `pricing/fx_swap/eurusd_fx_swap_3m.json`
- `pricing/irs/usd_ois_swap_5y.json`
- `pricing/irs/usd_irs_sofr_5y_par.json`
- `pricing/irs/usd_irs_sofr_10y.json`
- `pricing/irs/usd_irs_sofr_2y.json`
- `pricing/irs/eur_irs_estr_5y.json`
- `pricing/irs/gbp_irs_sonia_5y.json`
- `pricing/ir_future/sofr_3m_quarterly.json`
- `pricing/ir_future/sofr_1m_serial.json`
- `pricing/cap_floor/usd_cap_5y_atm_black.json`
- `pricing/cap_floor/usd_floor_5y_atm_normal.json`
- `pricing/swaption/usd_swaption_5y_into_5y_payer_atm.json`
- `pricing/swaption/usd_swaption_5y_into_5y_receiver_25_otm.json`

### Calibration

- `calibration/curves/usd_ois_bootstrap.json`
- `calibration/curves/usd_sofr_3m_bootstrap.json`
- `calibration/curves/eur_estr_bootstrap.json`
- `calibration/curves/gbp_sonia_bootstrap.json`
- `calibration/curves/jpy_tona_bootstrap.json`
- `calibration/vol/usd_swaption_sabr_cube.json`

### Integration

- `integration/usd_ois_calib_then_price_5y_irs.json`
- `integration/swaption_calib_then_price_atm.json`

## Fixture Workflow

Every new rates fixture follows the foundation workflow:

1. Add or reuse an adapter in `scripts/goldens/adapters/<kind>.py`.
2. Add Rust `dispatch()` and Python `_DOMAIN_RUNNERS` entries.
3. Add Rust and Python runner modules under `runners/`.
4. Run `mise run goldens-regen --kind <kind> ...` or manually capture a screened fixture with provenance.
5. Add a one-line Rust test in `finstack/valuations/tests/golden/pricing.rs`, `calibration.rs`, or `integration.rs`.
6. Add a parametrized Python collector if the fixture directory is new.
7. Run `mise run goldens-test`.
