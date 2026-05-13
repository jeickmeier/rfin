# Default Tolerances for Golden Tests

Reference table for the default absolute and relative tolerances used by
`finstack.golden/1` fixtures. Authors should start from these defaults and
override only when necessary, including a `tolerance_reason` field next to the
override.

See `docs/archive/plans/2026-04-30-golden-tests-framework-design.md` section 5.3 and section
6 for the full semantics. A metric matches if either the absolute or relative
tolerance is satisfied:

```text
|actual - expected| <= abs OR |actual - expected| / max(|expected|, 1e-12) <= rel
```

| Metric class | Examples | Default abs | Default rel |
|---|---|---:|---:|
| Money, NPV, clean price, dirty price | `npv`, `clean_price`, `dirty_price`, `accrued`, `upfront` | `0.01` | `1e-6` |
| Sensitivities | `dv01`, `cs01`, `delta`, `gamma`, `vega`, `theta`, `rho` | `0.5` | `1e-4` |
| Rates and spreads | `par_rate`, `ytm`, `z_spread`, `oas`, `discount_margin`, `breakeven_inflation` | `1e-4` | N/A |
| Implied volatility | `implied_vol`, `atm_vol`, `wing_vol` | `1e-4` | N/A |
| Discount factors | `discount_factor` | `1e-8` | N/A |
| Hazard and probabilities | `survival_probability`, `hazard_rate` | `1e-6` | N/A |
| Calibration residual | `calibration_rmse`, `repricing_error` | `1e-4` | N/A |
| Pure analytics math | `arith_return`, `log_return`, `sharpe`, `vol`, `max_dd`, `var`, `es` | `1e-8` | N/A |
| Attribution residual | `residual` after carry, rates, and credit | `5e-5` | N/A |

## Override Format

```json
{
  "tolerances": {
    "ytm": {
      "abs": 2e-6,
      "tolerance_reason": "QL uses Brent on YTM, finstack uses Newton; about 1e-6 disagreement expected"
    }
  }
}
```

The `tolerance_reason` is required when overriding a default. It gives future
reviewers the context they need when reading fixture diffs.
