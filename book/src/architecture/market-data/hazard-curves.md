# Hazard Curves

A `HazardCurve` models credit default risk through piecewise-constant hazard
rates $\lambda(t)$. The survival probability is:

$$S(t) = e^{-\int_0^t \lambda(s)\, ds}$$

Hazard curves are used for CDS pricing, credit-risky bond valuation, and CVA
calculations.

## Construction

**Rust**

```rust,no_run
use finstack_core::market_data::term_structures::HazardCurve;

let curve = HazardCurve::builder("ACME-HZD")
    .base_date(date!(2025-01-15))
    .knots(&[
        (0.5, 0.008),   // 6M hazard rate
        (1.0, 0.010),   // 1Y
        (3.0, 0.012),   // 3Y
        (5.0, 0.015),   // 5Y
        (10.0, 0.020),  // 10Y
    ])
    .recovery_rate(0.40)  // 40% senior unsecured
    .build()?;
```

**Python**

```python
from finstack.core.market_data.term_structures import HazardCurve
from datetime import date

curve = HazardCurve("ACME-HZD", date(2025, 1, 15), [
    (0.5,  0.008),
    (1.0,  0.010),
    (3.0,  0.012),
    (5.0,  0.015),
    (10.0, 0.020),
], recovery_rate=0.40)
```

## Key Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `sp(t)` | `f64` | Survival probability at time `t`: $S(t)$ |
| `hazard_rate(t)` | `f64` | Piecewise-constant $\lambda$ at time `t` |
| `sp_on_date(date)` | `f64` | Survival probability to a calendar date |
| `hazard_rate_on_date(date)` | `f64` | Hazard rate on a calendar date |
| `survival_at_dates(dates)` | `Vec<f64>` | Batch survival probabilities |

## Builder Options

| Option | Default | Description |
|--------|---------|-------------|
| `recovery_rate(f64)` | `0.40` | Assumed recovery rate (metadata) |
| `issuer(str)` | None | Issuer identifier |
| `currency(Currency)` | None | Protection currency |
| `seniority(Seniority)` | None | Debt seniority level |
| `par_spreads(knots)` | None | Original par spread points (for reporting) |

## Invariants

- $S(t)$ is monotonically decreasing (enforced by construction)
- $\lambda_i \geq 0$ for all intervals
- $S(0) = 1.0$ always

## Bootstrap from CDS Par Spreads

In practice, hazard curves are bootstrapped from CDS market quotes using the
ISDA Standard Model:

1. Market CDS par spreads at standard tenors (6M, 1Y, 3Y, 5Y, 7Y, 10Y)
2. Given a discount curve and recovery assumption
3. Solve for piecewise-constant $\lambda$ that reprices each CDS to par

See the [Credit Analysis cookbook](../../cookbooks/credit-analysis.md) for a
complete example.
