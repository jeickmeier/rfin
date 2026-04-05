# Metrics

Finstack computes risk sensitivities via bump-and-reprice on the market data
inputs. Each metric has a standardized key format and well-defined computation.

## Interest Rate Sensitivities

### DV01 (Dollar Value of a Basis Point)

Change in value for a 1bp parallel shift in the discount curve:

$$\text{DV01} = V(r + 1\text{bp}) - V(r)$$

**Key:** `dv01`

### Bucketed DV01

DV01 decomposed by curve knot point:

**Key format:** `bucketed_dv01::CURVE_ID::TENOR`

```text
bucketed_dv01::USD-OIS::6m
bucketed_dv01::USD-OIS::2y
bucketed_dv01::USD-OIS::10y
bucketed_dv01::USD-OIS::30y
```

### PV01

Present value of a basis point on the floating leg of a swap:

**Key:** `pv01::CURVE_ID` (e.g., `pv01::usd_ois`)

## Credit Sensitivities

### CS01 (Credit Spread 01)

Change in value for a 1bp shift in the credit spread / hazard curve:

**Key format:**
- `cs01::HAZARD_CURVE_ID` — for CDS, CDX, bonds with hazard curves
- `cs01::INSTRUMENT_ID` — for bonds priced via z-spread

```text
cs01::ACME-HZD        # CDS on ACME
cs01::BOND_A          # Bond priced via z-spread bump
```

> **Convention:** For bonds without a hazard curve, CS01 uses the z-spread
> bump method. The metric key uses the instrument ID, not "z_spread".

### Bucketed CS01

**Key format:** `bucketed_cs01::HAZARD_CURVE_ID::TENOR`

## Greeks (Options)

| Metric | Key | Definition |
|--------|-----|------------|
| Delta | `delta` | $\partial V / \partial S$ |
| Gamma | `gamma` | $\partial^2 V / \partial S^2$ |
| Vega | `vega` or `vega::VOL_ID::EXPIRY` | $\partial V / \partial \sigma$ |
| Theta | `theta` | $\partial V / \partial t$ |
| Rho | `rho` | $\partial V / \partial r$ |

## Bond-Specific Metrics

| Metric | Key | Description |
|--------|-----|-------------|
| Yield to Maturity | `ytm` | Internal rate of return |
| Yield to Worst | `ytw` | Min yield across call dates |
| Macaulay Duration | `duration_mac` | Weighted avg time to cashflows |
| Modified Duration | `duration_mod` | $-\frac{1}{V}\frac{dV}{dy}$ |
| Convexity | `convexity` | $\frac{1}{V}\frac{d^2V}{dy^2}$ |
| Z-Spread | `z_spread` | Spread over the zero curve |
| OAS | `oas` | Option-adjusted spread |
| Clean Price | `clean_price` | Quoted price |
| Dirty Price | `dirty_price` | Clean price + accrued |
| Accrued Interest | `accrued` | Accrued since last coupon |

## Carry & Roll Metrics

| Metric | Key | Description |
|--------|-----|-------------|
| Carry | `carry` | P&L from passage of time |
| Roll-down | `rolldown` | P&L from rolling down the curve |
| Pull-to-par | `pull_to_par` | Convergence toward par at maturity |

## Requesting Metrics

```python
result = registry.price_with_metrics(
    bond, "discounting", market, as_of,
    metrics=["dv01", "bucketed_dv01", "cs01", "ytm", "duration_mod"],
)

print(result.npv)                                 # Money(-5432.10, USD)
print(result.get("ytm"))                          # 0.0472
print(result.get("bucketed_dv01::USD-OIS::10y"))  # -823.45
```
