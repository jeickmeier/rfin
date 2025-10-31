# Risk Metrics Documentation

Comprehensive documentation on metric definitions, formulas, conventions, units, and sign conventions for all supported risk metrics.

## Metric Categories

- **Standard Greeks**: Delta, Gamma, Vega, Theta, Rho
- **Higher-Order Greeks**: Vanna, Volga, Charm, Color, Speed
- **Interest Rate Risk**: DV01, BucketedDV01, IR01
- **Credit Risk**: CS01, BucketedCS01, Recovery01
- **Volatility Risk**: Vega, BucketedVega
- **Dividend Risk**: Dividend01
- **FX Risk**: FX Delta, FX Vega
- **Inflation Risk**: Inflation01
- **Model-Specific Risks**: Prepayment01, Default01, Severity01, Conversion01, etc.

## Standard Greeks

### Delta

**Price sensitivity to underlying spot price change**

**Formula**: `Delta = (PV(spot + bump) - PV(spot - bump)) / (2 * bump_size)`

- **Units**: Unitless (dimensionless)
- **Sign Convention**: Positive for long calls, negative for long puts
- **Bump Size**: 1% of spot price (0.01)
- **Defined in**: `finite_difference::bump_sizes::SPOT`

### Gamma

**Delta sensitivity to underlying spot price change**

**Formula**: `Gamma = (Delta(spot + bump) - Delta(spot - bump)) / (2 * bump_size)`

- **Units**: Per unit spot (1/spot units)
- **Sign Convention**: Always non-negative for long option positions
- **Bump Size**: 1% of spot price (0.01)

### Vega

**Price sensitivity to volatility change**

**Formula**: `Vega = (PV(vol + bump) - PV(vol - bump)) / (2 * bump_size)`

- **Units**: Per 1% volatility change (price units / 0.01)
- **Sign Convention**: Positive for long option positions
- **Bump Size**: 1% absolute volatility (0.01)
- **Defined in**: `finite_difference::bump_sizes::VOLATILITY`

### Theta

**Price sensitivity to time decay**

**Formula**: `Theta = (PV(time + period) - PV(base)) - Sum(Cashflows during period)`

- **Units**: Price units per time period
- **Sign Convention**: Typically negative for long option positions
- **Period**: Default 1 day, configurable via `PricingOverrides::theta_period`

### Rho

**Price sensitivity to interest rate change**

**Formula**: `Rho = (PV(rate + 1%) - PV(rate - 1%)) / (2 * 0.01)`

- **Units**: Per 1% interest rate change (price units / 0.01)
- **Sign Convention**: Positive for long calls, negative for puts/bonds
- **Bump Size**: 1% (0.01) absolute rate change

## Higher-Order Greeks

### Vanna

**Delta sensitivity to volatility change**

**Formula**: `Vanna = (Delta(vol + bump) - Delta(vol - bump)) / (2 * vol_bump)`

- **Units**: Per 1% volatility change (1/vol units)
- **Cross-derivative**: ∂²P/(∂S ∂σ)
- **Interpretation**: Measures how delta changes as volatility moves

### Volga

**Vega sensitivity to volatility change**

**Formula**: `Volga = (Vega(vol + bump) - Vega(vol - bump)) / (2 * vol_bump)`

- **Units**: Per 1% volatility change squared
- **Second derivative**: ∂²P/(∂σ²)
- **Interpretation**: Measures convexity of price with respect to volatility

### Charm

**Delta sensitivity to time decay**

**Formula**: `Charm = (Delta(time + period) - Delta(time)) / period`

- **Units**: Per time period (1/day for 1D period)
- **Cross-derivative**: ∂²P/(∂S ∂t)
- **Interpretation**: Measures how delta changes over time

### Color

**Gamma sensitivity to time decay**

**Formula**: `Color = (Gamma(time + period) - Gamma(time)) / period`

- **Units**: Per time period (1/day for 1D period)
- **Cross-derivative**: ∂³P/(∂S² ∂t)
- **Interpretation**: Measures how gamma changes over time

### Speed

**Gamma sensitivity to spot price change**

**Formula**: `Speed = (Gamma(spot + bump) - Gamma(spot - bump)) / (2 * spot_bump)`

- **Units**: Per unit spot squared
- **Third derivative**: ∂³P/(∂S³)
- **Interpretation**: Measures how gamma changes as spot moves

## Interest Rate Risk Metrics

### DV01

**Dollar value of a 1 basis point rate change**

**Formula**: `DV01 = (PV(rate + 1bp) - PV(rate - 1bp)) / (2 * 0.0001)`

- **Units**: Price units per basis point (price units / 0.0001)
- **Sign Convention**: Negative for bonds (price falls as rates rise)
- **Bump Size**: 1 basis point (0.0001)
- **Defined in**: `finite_difference::bump_sizes::INTEREST_RATE_BP`

### BucketedDV01

**Key-rate duration across maturity buckets**

**Formula**: `BucketedDV01[t] = (PV(curve with bumped knot t) - PV(base)) / bump_size`

- **Units**: Price units per basis point for each bucket
- **Buckets**: [3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y]
- **Defined in**: `bucketed::standard_ir_dv01_buckets()`
- **Implementation**: Each bucket bumps one key rate while others held constant
- **Output**: Bucketed series stored in `MetricContext`

## Credit Risk Metrics

### CS01

**Credit spread sensitivity per basis point**

**Formula**: `CS01 = (PV(spread + 1bp) - PV(spread - 1bp)) / (2 * 0.0001)`

- **Units**: Price units per basis point (price units / 0.0001)
- **Sign Convention**: Negative (price decreases as spreads widen)
- **Bump Size**: 1 basis point (0.0001)
- **Defined in**: `finite_difference::bump_sizes::CREDIT_SPREAD_BP`

### BucketedCS01

**Key-rate credit spread sensitivity**

**Formula**: `BucketedCS01[t] = (PV(hazard curve with bumped knot t) - PV(base)) / bump_size`

- **Units**: Price units per basis point for each bucket
- **Buckets**: [3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y]
- **Defined in**: `bucketed_cs01::standard_credit_cs01_buckets()`
- **Bump Modes**:
  - **Hazard rate bump**: Adds 1bp to hazard rate at each bucket
  - **Spread bump**: Converts spread to hazard rate, bumps, converts back

### Recovery01

**Recovery rate sensitivity**

**Formula**: `Recovery01 = (PV(recovery + 1%) - PV(recovery - 1%)) / (2 * 0.01)`

- **Units**: Price units per 1% recovery rate change
- **Sign Convention**: Positive (higher recovery increases value)
- **Bump Size**: 1% (0.01)
- **Applies To**: CDS, CDSTranche, CDSIndex, CdsOption, StructuredCredit

## Volatility Risk Metrics

### Vega

See Standard Greeks section above.

### BucketedVega

**Volatility sensitivity by expiry and strike**

**Formula**: `BucketedVega[expiry, strike] = (PV(vol bumped at point) - PV(base)) / vol_bump`

- **Units**: Price units per 1% volatility change for each grid point
- **Expiry buckets**: [1M, 3M, 6M, 1Y, 2Y, 5Y]
- **Strike ratios**: [0.8, 0.9, 0.95, 1.0, 1.05, 1.1, 1.2]
- **Stored as**: 2D matrix in `MetricContext`
- **Bump Size**: 1% absolute volatility (0.01)
- **Defined in**: `bucketed_vega::VOL_BUMP_PCT`

## Dividend Risk Metrics

### Dividend01

**Dividend yield sensitivity per basis point**

**Formula**: `Dividend01 = (PV(dividend_yield + 1bp) - PV(dividend_yield - 1bp)) / (2 * 0.0001)`

- **Units**: Price units per basis point dividend yield change
- **Sign Convention**: Negative for long equity positions (dividends reduce option value)
- **Bump Size**: 1 basis point (0.0001)
- **Applies To**: EquityOption, ConvertibleBond, EquityTotalReturnSwap

## FX Risk Metrics

### FX Delta

**FX spot rate sensitivity**

**Formula**: `FX Delta = (PV(fx_rate * 1.01) - PV(fx_rate * 0.99)) / (2 * 0.01 * fx_rate)`

- **Units**: Price units per 1% FX rate change
- **Sign Convention**: Positive for long FX exposure
- **Applies To**: FxSpot, FxSwap, QuantoOption
- **Bump Size**: 1% of FX rate

### FX Vega

**FX volatility sensitivity (for quanto products)**

- **Applies To**: QuantoOption

## Inflation Risk Metrics

### Inflation01

**Inflation curve sensitivity per basis point**

**Formula**: `Inflation01 = (PV(inflation_curve + 1bp) - PV(inflation_curve - 1bp)) / (2 * 0.0001)`

- **Units**: Price units per basis point inflation rate change
- **Sign Convention**: Positive for long inflation-linked positions
- **Bump Size**: 1 basis point (0.0001)
- **Applies To**: InflationLinkedBond, InflationSwap
- **Implementation**: Bumps inflation curve using `BumpSpec::inflation_shift_pct()`

## Model-Specific Risk Metrics

### Prepayment01

**Prepayment rate sensitivity per basis point**

**Formula**: `Prepayment01 = (PV(prepayment + 1bp CPR) - PV(prepayment - 1bp CPR)) / (2 * 0.0001)`

- **Units**: Price units per basis point CPR change
- **Bump Size**: 1 basis point CPR (0.0001)
- **Applies To**: StructuredCredit (ABS, RMBS, CMBS, CLO)
- **Implementation**: Handles PSA multiplier, ConstantCpr, ConstantSmm, AssetDefault specs

### Default01

**Default rate sensitivity per basis point**

**Formula**: `Default01 = (PV(default_rate + 1bp CDR) - PV(default_rate - 1bp CDR)) / (2 * 0.0001)`

- **Units**: Price units per basis point CDR change
- **Bump Size**: 1 basis point CDR (0.0001)
- **Applies To**: StructuredCredit
- **Implementation**: Handles SDA multiplier, ConstantCdr, ConstantMdr, AssetDefault specs

### Severity01

**Loss severity sensitivity**

**Formula**: `Severity01 = (PV(severity + 1%) - PV(severity - 1%)) / (2 * 0.01)`

- **Units**: Price units per 1% loss severity change
- **Relationship**: Loss Severity = 1 - Recovery Rate (LGD)
- **Note**: Severity01 ≈ -Recovery01 for constant recovery models
- **Bump Size**: 1% (0.01)

### Conversion01

**Conversion ratio/price sensitivity**

**Formula**: `Conversion01 = (PV(conversion_ratio * 1.01) - PV(conversion_ratio * 0.99)) / (2 * 0.01)`

- **Units**: Price units per 1% conversion ratio change
- **Bump Size**: 1% (0.01)
- **Applies To**: ConvertibleBond
- **Implementation**: Bumps conversion ratio or inversely bumps conversion price

## Instrument-Specific Risk Metrics

### Constituent Delta

**Basket constituent price sensitivity**

**Formula**: `ConstituentDelta[i] = (PV(basket with bumped constituent i) - PV(base)) / (bump_size * price_i)`

- **Units**: Price units per 1% constituent price change
- **Bump Size**: 1% of constituent price (0.01)
- **Applies To**: Basket
- **Output**: Bucketed series with one entry per constituent

### Weight Risk

**Basket weight sensitivity**

**Formula**: `WeightRisk[i] = (PV(weight_i + 1bp, others adjusted) - PV(base)) / 0.0001`

- **Units**: Price units per basis point weight change
- **Bump Size**: 1 basis point (0.0001)
- **Applies To**: Basket
- **Implementation**: Bumps one weight, proportionally adjusts others to maintain sum = 1.0

### Haircut01

**Repo haircut sensitivity**

**Formula**: `Haircut01 = (PV(haircut + 1bp) - PV(haircut - 1bp)) / (2 * 0.0001)`

- **Units**: Price units per basis point haircut change
- **Bump Size**: 1 basis point (0.0001 = 0.01%)
- **Applies To**: Repo

### CollateralPrice01

**Repo collateral price sensitivity**

**Formula**: `CollateralPrice01 = (PV(collateral_price * 1.01) - PV(collateral_price * 0.99)) / (2 * 0.01 * price)`

- **Units**: Price units per 1% collateral price change
- **Bump Size**: 1% of collateral price
- **Applies To**: Repo

### NAV01

**Private markets fund NAV sensitivity**

**Formula**: `NAV01 = (PV(events scaled * 1.01) - PV(events scaled * 0.99)) / (2 * 0.01)`

- **Units**: Price units per 1% NAV change
- **Implementation**: Scales all distribution/proceeds events by ±1%
- **Applies To**: PrivateMarketsFund

### Carry01

**GP carry sensitivity**

**Formula**: `Carry01 = (PV(GP_share + 1bp) - PV(GP_share - 1bp)) / (2 * 0.0001)`

- **Units**: Price units per basis point GP share change
- **Bump Size**: 1 basis point (0.0001 = 0.01%)
- **Applies To**: PrivateMarketsFund

### Hurdle01

**Hurdle rate sensitivity**

**Formula**: `Hurdle01 = (PV(hurdle_rate + 1bp) - PV(hurdle_rate - 1bp)) / (2 * 0.0001)`

- **Units**: Price units per basis point hurdle IRR change
- **Bump Size**: 1 basis point (0.0001)
- **Applies To**: PrivateMarketsFund

## Finite Difference Implementation

### Standard Bump Sizes

All bump sizes are defined in `instruments::common::metrics::finite_difference::bump_sizes`:

- **SPOT**: 0.01 (1%)
- **VOLATILITY**: 0.01 (1% absolute)
- **INTEREST_RATE_BP**: 0.0001 (1 basis point)
- **CREDIT_SPREAD_BP**: 0.0001 (1 basis point)
- **CORRELATION**: 0.01 (1%)

### Symmetric Bumping

Most metrics use symmetric finite differences:

```
Metric = (PV(up) - PV(down)) / (2 * bump_size)
```

This provides better accuracy than one-sided bumps and cancels second-order errors.

### Determinism

All finite difference calculations are deterministic:
- Same inputs → same outputs
- No random number generation
- Parallel execution produces identical results to serial

**For Monte Carlo-priced instruments** (Asian, Autocallable, etc.):
- Use fixed seed derived from instrument/metric combination
- Ensures serial ≡ parallel results
- Seed should be deterministic: `hash(instrument_id + metric_id)`

### Caching

Finite difference calculations benefit from caching:
- Base PV cached in `MetricContext::base_value`
- Intermediate results can be stored in `MetricContext`
- Reduces redundant repricing calls

## Metric Conventions Summary

### Units Summary

| Metric | Units | Bump Size |
|--------|-------|-----------|
| DV01, CS01 | Price / bp | 1 bp |
| Delta | Unitless | 1% spot |
| Gamma | 1/spot | 1% spot |
| Vega | Price / 1% vol | 1% absolute |
| Rho | Price / 1% rate | 1% absolute |
| Theta | Price / day | 1 day (default) |
| Dividend01 | Price / bp | 1 bp yield |
| Inflation01 | Price / bp | 1 bp inflation |
| Prepayment01, Default01 | Price / bp | 1 bp rate |
| Recovery01, Severity01 | Price / 1% | 1% |
| Conversion01 | Price / 1% | 1% ratio |
| NAV01 | Price / 1% | 1% NAV |
| Carry01, Hurdle01 | Price / bp | 1 bp |

### Sign Conventions

| Metric Type | Typical Sign | Meaning |
|-------------|--------------|---------|
| Long Call Delta | Positive | Value increases with spot |
| Long Put Delta | Negative | Value decreases with spot |
| Long Vega | Positive | Value increases with vol |
| Long Theta | Negative | Time decay |
| Bond DV01 | Negative | Price falls as rates rise |
| CS01 | Negative | Price falls as spreads widen |
| Recovery01 | Positive | Higher recovery increases value |

## Error Handling Conventions

### When Metrics Return 0.0

Some metrics return 0.0 instead of `Err` when:

- **Metric not applicable**: Metric is not relevant for the instrument type
  - Example: FX Delta for non-FX instruments
  - Example: Prepayment01 for non-structured credit instruments
- **Feature not yet implemented**: Metric calculation is marked as "Placeholder" in code comments
  - Example: FX Vanna (pending VolSurface point bumping completion)
  - Example: ConstituentDelta for instrument-based basket constituents
- **Edge cases**: Instrument state makes metric meaningless
  - Example: Expired options (T ≤ 0) return 0.0 for time-sensitive greeks
  - Example: Zero notional instruments
  - Example: At-the-money options with zero volatility (rare edge case)

### When Metrics Return Err

Metrics return `Err` for:

- **Missing market data**: Required curves, surfaces, or prices not found
  - Example: Discount curve lookup failure
  - Example: FX matrix missing for FX instruments
  - Example: Volatility surface not found
- **Invalid instrument configuration**: Instrument setup is inconsistent
  - Example: Maturity date before issue date
  - Example: Negative notional or strike
  - Example: Currency mismatch between instrument and market data
- **Numerical failures**: Computation issues
  - Example: Non-convergence in numerical solvers
  - Example: NaN or infinite values in intermediate calculations
  - Example: Division by zero (should be caught and handled gracefully)

### Checking Applicability

Before computing a metric, check:

1. **Instrument type**: Use pattern matching or `as_any()` downcasting
2. **Instrument state**: Verify expiry dates, maturity, etc.
3. **Market data availability**: Check for required curves/surfaces in `MarketContext`

Future enhancement: Consider adding `MetricCalculator::is_applicable(&self, instrument: &dyn Instrument) -> bool` trait method for explicit applicability checks.

## Known Limitations

### Basket Constituent Delta

The `constituent_delta` metric for `Basket` instruments currently only fully supports `ConstituentReference::MarketData`. For `ConstituentReference::Instrument`, the metric returns 0.0 as a placeholder.

**Reason**: Bumping instrument-based constituents requires instrument cloning and price override capabilities that are not yet implemented.

**Workaround**: 
- Convert instrument references to synthetic market data prices
- Use the instrument's own delta directly if exposed as a metric

**Future**: Will add `price_override` field to `BasketConstituent` for full support.

### Adaptive Bump Sizes

Generic finite difference greek calculators (`GenericFdDelta`, `GenericFdGamma`) support adaptive bump sizes when `PricingOverrides::adaptive_bumps` is enabled. Adaptive bumps adjust based on volatility and time to expiry to improve numerical stability for high-volatility or long-dated options. When adaptive bumps are disabled (default), fixed 1% spot bumps are used.

**Adaptive Spot Bump Formula**:
- Base bump: 1% of spot
- Volatility-adjusted: `0.1% * spot * σ * √T`
- Uses the larger of base and vol-scaled, capped at 5% maximum
- Manual overrides via `PricingOverrides::spot_bump_pct` take precedence

**Implementation**: Both `GenericFdDelta` and `GenericFdGamma` automatically detect instrument vol, expiry, and day_count fields to calculate adaptive bump sizes. If adaptive data is unavailable, falls back to fixed 1% bumps.

