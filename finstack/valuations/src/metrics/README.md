# Metrics Framework

A trait-based architecture for computing financial metrics independently from core pricing logic. The metrics framework provides clean separation between instrument pricing and risk/analytical measure calculation, with built-in dependency management and caching.

## Overview

The metrics framework enables on-demand computation of financial measures (PV, DV01, Greeks, spreads, etc.) with:

- **Trait-based design**: Generic `MetricCalculator` trait for extensibility
- **Dependency management**: Automatic computation ordering based on metric dependencies
- **Efficient caching**: Reuse of intermediate results (cashflows, discount factors, base valuations)
- **Instrument-specific registration**: Metrics can be registered for specific instrument types
- **Standard registry**: Pre-configured registry with common financial metrics

## Directory Structure

```
metrics/
├── README.md                    # This file
├── mod.rs                       # Public API and standard registry
├── core/                        # Core infrastructure
│   ├── mod.rs                   # Core module exports
│   ├── ids.rs                   # Strongly-typed metric identifiers (MetricId)
│   ├── traits.rs                # MetricCalculator trait and MetricContext
│   ├── registry.rs              # MetricRegistry for calculator management
│   ├── registration_macro.rs   # Convenience macros for registration
│   └── finite_difference.rs    # FD utilities and standard bump sizes
└── sensitivities/               # Sensitivity metrics (risk)
    ├── mod.rs                   # Sensitivity module exports
    ├── dv01.rs                  # Interest rate sensitivity (DV01)
    ├── cs01.rs                  # Credit spread sensitivity (CS01)
    ├── vega.rs                  # Volatility sensitivity (Vega)
    ├── theta.rs                 # Time decay (Theta)
    ├── fd_greeks.rs             # Generic finite difference Greeks
    └── tests/                   # Sensitivity metric tests
```

## Key Features

### 1. Trait-Based Architecture

All metrics implement the `MetricCalculator` trait:

```rust
pub trait MetricCalculator: Send + Sync {
    /// Computes the metric value based on the provided context
    fn calculate(&self, context: &mut MetricContext) -> Result<f64>;

    /// Lists metric IDs this calculator depends on
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
```

### 2. Strongly-Typed Metric IDs

All metrics are identified by the `MetricId` type, which provides:

- Compile-time validation
- Autocomplete support
- Safe refactoring when metric names change

```rust
// Standard metrics are constants
let dv01_id = MetricId::Dv01;
let theta_id = MetricId::Theta;

// Custom metrics supported too
let custom_id = MetricId::custom("my_custom_metric");
```

### 3. Dependency Management

The registry automatically resolves dependencies and computes metrics in the correct order:

```rust
struct MacaulayDuration;

impl MetricCalculator for MacaulayDuration {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Can access previously computed YTM
        let ytm = context.computed.get(&MetricId::Ytm)
            .ok_or(Error::Missing)?;

        // Use YTM to compute duration
        // ...
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]  // YTM will be computed first
    }
}
```

### 4. Efficient Caching

The `MetricContext` caches intermediate results to avoid redundant calculations:

```rust
pub struct MetricContext {
    pub instrument: Arc<dyn Instrument>,
    pub curves: Arc<MarketContext>,
    pub as_of: Date,
    pub base_value: Money,
    pub computed: HashMap<MetricId, f64>,           // Scalar metrics
    pub computed_series: HashMap<MetricId, Vec<(String, f64)>>,  // 1D bucketed
    pub computed_matrix: HashMap<MetricId, Structured2D>,        // 2D bucketed
    pub computed_tensor3: HashMap<MetricId, Structured3D>,       // 3D bucketed
    pub cashflows: Option<Vec<(Date, Money)>>,     // Cached cashflows
    // ... other cached data
}
```

### 5. Bucketed Metrics

Support for multi-dimensional risk metrics:

- **1D bucketed**: Key-rate DV01, CS01 by tenor
- **2D structured**: Vega surface (expiry × strike)
- **3D structured**: Advanced risk grids

```rust
// Store bucketed DV01 by tenor
let buckets = vec![
    ("3m".to_string(), 10.5),
    ("1y".to_string(), 42.3),
    ("5y".to_string(), 125.7),
];
context.store_bucketed_series(MetricId::BucketedDv01, buckets);
```

## Standard Metric Reference (184 metrics, 10 groups)

Every standard metric belongs to exactly one group. The grouping is enforced
by compile-time tests in `core/ids.rs`. Use `list_standard_metrics_grouped()`
(Python) or `MetricGroup::all_with_metrics()` (Rust) to query at runtime.

### Pricing (20)

Static pricing outputs: prices, yields, spreads, durations, implied levels.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `dirty_price` | Full price with accrued | Currency | Bond, FRN | Dirty price (includes accrued interest) |
| `clean_price` | Dirty price - accrued | Currency | Bond, FRN | Clean price (excludes accrued interest) |
| `accrued` | Coupon x accrual fraction | Currency | Bond, FRN, CDS | Accrued interest since last coupon payment |
| `ytm` | IRR of all cashflows at dirty price | Decimal | Bond | Yield to maturity |
| `ytw` | min(YTM, yield to each call/put date) | Decimal | Callable/Putable Bond | Yield to worst |
| `z_spread` | Constant spread s.t. PV(DF x exp(-z*t)) = dirty price | Decimal | Bond | Zero-volatility spread over discount curve |
| `oas` | Z-spread net of embedded option value | Decimal | Callable/Putable Bond | Option-adjusted spread |
| `i_spread` | YTM - interpolated swap rate | Decimal | Bond | Yield over interpolated swap curve |
| `g_spread` | YTM - interpolated govt rate | Decimal | Bond | Yield spread over government curve |
| `asw_par` | Par asset swap spread | Decimal | Bond | Par asset swap spread (market-standard ASW quote) |
| `asw_market` | Price-based ASW spread | Decimal | Bond | Market (price-based) asset swap spread |
| `discount_margin` | Spread s.t. PV(floating+DM) = price | Decimal | FRN | Discount margin for floating-rate bonds |
| `embedded_option_value` | P_straight - P_callable (or P_putable - P_straight) | Currency | Callable/Putable Bond | Embedded option value |
| `duration_mac` | Weighted-average time of cashflows | Years | Bond | Macaulay duration |
| `duration_mod` | -dP/P / dy | Years | Bond | Modified duration under quoted yield |
| `real_duration` | Duration adjusted for inflation | Years | Inflation-Linked Bond | Real (inflation-adjusted) duration |
| `yield_dv01` | -(dP/dy) x 0.0001 | Currency/bp | Bond | Dollar price change per 1bp yield change |
| `convexity` | d^2P / (P x dy^2) | Years^2 | Bond | Bond convexity under yield convention |
| `implied_vol` | Vol s.t. model price = market price | Decimal | Options | Implied volatility inferred from observed price |
| `time_to_maturity` | (maturity - as_of) in years | Years | All | Time to maturity |

### Carry (11)

Time-driven P&L: theta decomposition, carry components, financing.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `theta` | PV(T+1) - PV(T) | Currency | All | 1-day time decay P&L |
| `theta_carry` | Coupon accrual + pull-to-par + funding | Currency | All | Carry component of theta |
| `theta_roll_down` | PV change from aging along same curve | Currency | All | Roll-down component of theta |
| `theta_decay` | Theta - carry - roll_down | Currency | Options | Pure time-value (optionality) decay |
| `carry_total` | coupon_income + pull_to_par + roll_down - funding_cost | Currency | Bond, Swap | Total carry decomposition |
| `coupon_income` | Coupon x accrual fraction | Currency | Bond, Swap | Coupon/interest income during carry horizon |
| `pull_to_par` | PV change from amortization at flat yield | Currency | Bond | PV convergence toward par |
| `roll_down` | PV change from aging along sloped curve | Currency | Bond, Swap | Curve shape benefit (slide) |
| `funding_cost` | Dirty price x funding rate x DCF | Currency | Bond | Cost of financing the position |
| `implied_financing_rate` | Annualized rate from dollar roll drop | Decimal | TBA/MBS | Implied financing rate from dollar roll |
| `roll_specialness` | Implied financing rate - repo rate | bps | TBA/MBS | Roll specialness vs. repo rate |

### Sensitivity (19)

First-order bump sensitivities to market curves: DV01, PV01, bucketed DV01,
rho, and other rates-focused "01" metrics.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `dv01` | (PV(r+1bp) - PV(r-1bp)) / 2 | Currency/bp | All FI | Dollar value of 01 -- parallel rates bump |
| `bucketed_dv01` | Per-tenor key-rate DV01 | Currency/bp | All FI | Rate sensitivity by tenor bucket |
| `duration_dv01` | Notional x Duration x 0.0001 | Currency/bp | FI Index TRS | Duration-based DV01 for FI index TRS |
| `pv01` | PV(r+1bp) - PV(r) | Currency/bp | IRS, Swap | Present value of a basis point |
| `forward_pv01` | PV with projection curve +1bp | Currency/bp | IRS | Forward/projection curve sensitivity |
| `npv01` | PV change per 1bp inflation curve bump | Currency/bp | Inflation Swap | Inflation swap NPV sensitivity |
| `rho` | PV(r+1bp) - PV(r) (domestic rate) | Currency/bp | Options | Domestic rate sensitivity |
| `foreign_rho` | PV(r_f+1bp) - PV(r_f) | Currency/bp | FX/Quanto Options | Foreign/dividend rate sensitivity |
| `dv01_domestic` | Domestic leg DV01 | Currency/bp | FX Swap | DV01 for domestic currency leg |
| `dv01_foreign` | Foreign leg DV01 | Currency/bp | FX Swap | DV01 for foreign currency leg |
| `dv01_primary` | Primary leg DV01 | Currency/bp | Basis Swap | DV01 of primary floating leg |
| `dv01_reference` | Reference leg DV01 | Currency/bp | Basis Swap | DV01 of reference floating leg |
| `dividend01` | PV change per 1bp dividend yield | Currency/bp | Equity Options | Dividend yield sensitivity |
| `inflation01` | PV change per 1bp inflation curve | Currency/bp | Inflation-Linked | Inflation curve sensitivity |
| `dm01` | PV change per 1bp discount margin | Currency/bp | FRN, Structured | Discount margin sensitivity |
| `conversion01` | PV change per 1% conversion ratio | Currency/% | Convertible Bond | Conversion ratio sensitivity |
| `collateral_haircut01` | PV change per 1bp haircut | Currency/bp | Collateralized | Collateral haircut sensitivity |
| `collateral_price01` | PV change per 1% collateral price | Currency/% | Collateralized | Collateral price sensitivity |
| `convexity_adjustment_risk` | CMS convexity adjustment sensitivity | Currency | CMS Options | Convexity adjustment risk |

### Greeks (22)

Options-style Greeks and all second-order / higher-order sensitivities.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `delta` | dPV/dS | Currency/unit | Options, Equity | Cash delta to spot driver |
| `gamma` | d^2PV/dS^2 | Currency/unit^2 | Options | Cash gamma to spot driver |
| `vega` | PV(sigma+1pt) - PV(sigma) | Currency/vol pt | Options | Cash vega per 1 vol point |
| `bucketed_vega` | Per-surface-point vega | Currency/vol pt | Options | Vega by vol surface node |
| `vanna` | d^2PV / (dS x dsigma) | Currency/(unit x vol) | Options | Mixed spot-vol sensitivity |
| `volga` | d^2PV / dsigma^2 | Currency/vol^2 | Options | Second-order vol sensitivity |
| `veta` | d(vega)/dt | Currency/(vol x day) | Options | Theta sensitivity to volatility |
| `charm` | d(delta)/dt | Currency/(unit x day) | Options | Delta sensitivity to time |
| `color` | d(gamma)/dt | Currency/(unit^2 x day) | Options | Gamma sensitivity to time |
| `speed` | d(gamma)/dS = d^3PV/dS^3 | Currency/unit^3 | Options | Gamma sensitivity to underlying |
| `ir_convexity` | d^2PV/dr^2 | Currency/bp^2 | IRS | Interest rate convexity (swap/rates) |
| `ir_cross_gamma` | d^2PV / (dr_disc x dr_fwd) | Currency/bp^2 | IRS | Cross-gamma: discount vs forward curve |
| `inflation_convexity` | d^2PV / d(infl)^2 | Currency | Inflation-Linked | Inflation second-order sensitivity |
| `cs_gamma` | d^2PV/ds^2 | Currency/bp^2 | Credit | Credit spread gamma |
| `cross_gamma_rates_credit` | d^2V / (dr x ds) | Currency | Multi-factor | Cross-gamma: rates x credit |
| `cross_gamma_rates_vol` | d^2V / (dr x dsigma) | Currency | Multi-factor | Cross-gamma: rates x vol |
| `cross_gamma_spot_vol` | d^2V / (dS x dsigma) | Currency | Multi-factor | Cross-gamma: spot x vol |
| `cross_gamma_spot_credit` | d^2V / (dS x ds) | Currency | Multi-factor | Cross-gamma: spot x credit |
| `cross_gamma_fx_vol` | d^2V / (dFX x dsigma) | Currency | Multi-factor | Cross-gamma: FX x vol |
| `cross_gamma_fx_rates` | d^2V / (dFX x dr) | Currency | Multi-factor | Cross-gamma: FX x rates |
| `theta_gamma` | d(theta)/d(gamma) | Currency | Options, VaR | Conditional second-order theta |
| `variance_vega` | dPV / d(variance) | Currency/var pt | Variance Swap | Vega per variance point |

### Credit (16)

CDS/credit analytics and credit-specific sensitivities.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `cs01` | (PV(s+1bp) - PV(s-1bp)) / 2 | Currency/bp | CDS, Bond, Tranche | Credit spread sensitivity (parallel par-spread bump) |
| `bucketed_cs01` | Per-tenor key-rate CS01 | Currency/bp | CDS, Bond, Tranche | Credit spread risk by tenor bucket |
| `cs01_hazard` | (PV(h+1bp) - PV(h-1bp)) / 2 | Currency/bp | CDS, Bond, Tranche | CS01 via direct hazard-rate bump |
| `bucketed_cs01_hazard` | Per-tenor hazard-rate CS01 | Currency/bp | CDS, Bond, Tranche | Bucketed CS01 via hazard-rate bumps |
| `par_spread` | Protection PV / Risky Annuity | bps | CDS, CDS Index, Tranche | Par spread (coupon setting NPV to zero) |
| `risky_pv01` | Risky Annuity x Notional / 10000 | Currency/bp | CDS, CDS Index | PV change per 1bp premium spread |
| `risky_annuity` | Sum(DF(t) x SP(t) x YearFrac) | Years | CDS, CDS Index | Survival-weighted premium leg annuity |
| `spread_dv01` | (PV(c+1bp) - PV(c-1bp)) / 2 | Currency/bp | CDS Tranche | Running coupon sensitivity (finite difference) |
| `correlation01` | (PV(rho+1%) - PV(rho-1%)) / (2 x 0.01) | Currency/% | CDS Tranche | Correlation sensitivity per 1% change |
| `default01` | PV change per 1bp default rate | Currency/bp | Credit | Default rate sensitivity |
| `protection_leg_pv` | Sum(DF x dSP x LGD) | Currency | CDS, CDS Index | Protection leg present value |
| `premium_leg_pv` | Spread x Sum(DF x SP x YearFrac) + AoD | Currency | CDS, CDS Index | Premium leg present value |
| `jump_to_default` | Immediate P&L from default event | Currency | CDS, Bond, Tranche | Jump-to-default amount |
| `expected_loss` | Sum(DF x dSP x LGD) discounted | Currency | CDS, Tranche | Expected discounted credit loss |
| `default_probability` | 1 - SP(T) | Decimal [0,1] | CDS | Default probability to horizon |
| `recovery_01` | PV change per 1% recovery rate | Currency/% | CDS, Tranche | Recovery rate sensitivity |

### Rates (18)

Rates instrument decomposition: IRS legs, annuities, par rates, basis swap,
TRS, deposit/calibration intermediates.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `annuity` | Sum(DF(t) x YearFrac) | Years | IRS | Annuity factor for fixed leg |
| `par_rate` | Rate s.t. NPV = 0 | Decimal | IRS | Par swap rate (ATM fixed rate) |
| `pv_fixed` | Sum(C x DF x YearFrac) | Currency | IRS | Present value of fixed leg |
| `pv_float` | Sum(fwd x DF x YearFrac) | Currency | IRS | Present value of floating leg |
| `pv_primary` | PV of primary floating leg | Currency | Basis Swap | PV of primary leg (includes spread) |
| `pv_reference` | PV of reference floating leg | Currency | Basis Swap | PV of reference floating leg |
| `annuity_primary` | Annuity of primary leg | Years | Basis Swap | Primary leg annuity factor |
| `annuity_reference` | Annuity of reference leg | Years | Basis Swap | Reference leg annuity factor |
| `basis_par_spread` | Spread s.t. NPV = 0 | bps | Basis Swap | Par spread (absolute) |
| `incremental_par_spread` | Par spread - current spread | bps | Basis Swap | Additional spread to reach par |
| `financing_annuity` | Financing leg annuity | Currency | TRS | TRS financing annuity |
| `index_delta` | dV/dS (equity) or duration-weighted (FI) | Currency/unit | TRS | Index-level delta for TRS |
| `yf` | Year fraction (start, end) | Years | Deposit | Day-count year fraction |
| `df_start` | DF(0, start) | Dimensionless | Deposit | Discount factor at start date |
| `df_end` | DF(0, end) | Dimensionless | Deposit | Discount factor at end date |
| `deposit_par_rate` | Rate implied by calibrated curve | Decimal | Deposit | Deposit par rate from curve |
| `df_end_from_quote` | 1 / (1 + rate x yf) | Dimensionless | Deposit | DF implied by market quote |
| `quote_rate` | Market-observed rate | Decimal | Deposit | Quoted market rate for deposit |

### FX (7)

FX instrument pricing and analytics.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `spot_rate` | FX spot rate | CCY2/CCY1 | FX Spot | Spot exchange rate |
| `base_amount` | Notional in base currency | Currency | FX Spot | Base currency amount |
| `quote_amount` | Notional in quote currency | Currency | FX Spot | Quote currency amount |
| `inverse_rate` | 1 / spot_rate | CCY1/CCY2 | FX Spot | Inverse exchange rate |
| `fx01` | PV change per 1bp FX rate | Currency/bp | FX Swap, Quanto | FX spot rate sensitivity |
| `fx_delta` | PV change per 1% FX move | Currency/% | FX Spot, FX Swap | FX spot rate delta |
| `fx_vega` | PV change per 1% FX vol | Currency/% | Quanto Options | FX volatility sensitivity |

### Equity (18)

Equity/basket/ETF pricing and equity-derivative analytics.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `equity_price_per_share` | Market spot price | Currency/share | Equity | Equity price per share |
| `equity_shares` | Adjusted share count | Shares | Equity | Number of effective shares |
| `equity_dividend_yield` | Annualized continuous yield | Decimal | Equity Options | Dividend yield |
| `equity_forward_price` | S x exp((r-q) x T) | Currency/share | Equity Forward | Equity forward price |
| `delta_vol` | dPV per 1 vol-index point | Currency/pt | Vol Index Futures | Volatility index delta |
| `constituent_delta` | Per-name basket delta | Currency/unit | Basket/ETF | Per-constituent delta decomposition |
| `nav` | Total value / shares outstanding | Currency/share | ETF/Fund | Net asset value per share |
| `basket_value` | Sum(weight x price) | Currency | Basket/ETF | Total basket value |
| `constituent_count` | Number of basket constituents | Count | Basket/ETF | Number of constituents |
| `expense_ratio` | Annual expenses / AUM | Decimal | ETF/Fund | Expense ratio |
| `tracking_error` | Std dev of return vs benchmark | Decimal | ETF/Fund | Tracking error vs benchmark |
| `utilization` | Position / creation unit size | Decimal | ETF | Utilization vs creation unit |
| `premium_discount` | (Price - NAV) / NAV | Decimal | ETF | Premium/discount to NAV |
| `variance_expected` | E[sigma^2] under pricing model | Variance | Variance Swap | Expected variance |
| `variance_realized` | Realized sigma^2 from observed paths | Variance | Variance Swap | Realized variance |
| `variance_notional` | Payout multiplier | Currency/var | Variance Swap | Variance notional exposure |
| `variance_strike_vol` | sqrt(strike variance) | Decimal | Variance Swap | Strike volatility equivalent |
| `variance_time_to_maturity` | TTM under variance swap conventions | Years | Variance Swap | Time to maturity |

### Structured Credit (29)

Securitization pool and tranche analytics.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `wal` | Sum(t_i x Principal_i) / Total Principal | Years | ABS, MBS, CLO | Weighted average life |
| `wam` | Weighted average maturity of pool | Years | ABS, MBS, CLO | Weighted average maturity |
| `expected_maturity` | Expected final payment date | Date | Structured | Expected maturity under base assumptions |
| `pool_factor` | Remaining balance / original balance | Decimal | ABS, MBS | Pool factor (% remaining) |
| `cpr` | Annualized prepayment rate | Decimal | MBS | Constant prepayment rate |
| `smm` | Monthly prepayment rate | Decimal | MBS | Single monthly mortality |
| `cdr` | Annualized default rate | Decimal | ABS, CLO | Constant default rate |
| `loss_severity` | 1 - recovery_rate | Decimal | ABS, CLO | Loss severity fraction |
| `spread_duration` | Time-weighted spread sensitivity | Years | Structured | Spread duration |
| `prepayment01` | PV change per 1bp prepayment rate | Currency/bp | MBS, ABS | Prepayment rate sensitivity |
| `severity01` | PV change per 1% loss severity | Currency/% | ABS, CLO | Loss severity sensitivity |
| `abs_delinquency` | % of pool in delinquency | Decimal | ABS | Delinquency rate |
| `abs_charge_off` | % of pool charged off | Decimal | ABS | Charge-off rate |
| `abs_excess_spread` | Spread available to absorb losses | Decimal | ABS | Excess spread |
| `abs_ce_level` | Subordination as % of pool | Decimal | ABS | Credit enhancement level |
| `clo_warf` | Weighted average rating factor | Score | CLO | Weighted average rating factor |
| `clo_was` | Weighted average spread | bps | CLO | Weighted average spread |
| `clo_wac` | Weighted average coupon | Decimal | CLO | Weighted average coupon |
| `clo_diversity` | Portfolio diversity score | Score | CLO | Diversity score |
| `clo_oc_ratio` | Senior par / tranche par | Ratio | CLO | Overcollateralization ratio |
| `clo_ic_ratio` | Interest income / interest expense | Ratio | CLO | Interest coverage ratio |
| `clo_recovery_rate` | Average recovery on defaults | Decimal | CLO | Average recovery rate |
| `cmbs_dscr` | NOI / debt service | Ratio | CMBS | Debt service coverage ratio |
| `cmbs_waltv` | Weighted average loan-to-value | Decimal | CMBS | Weighted average LTV |
| `cmbs_ce_level` | Subordination as % of pool | Decimal | CMBS | Credit enhancement level |
| `rmbs_psa_speed` | PSA prepayment speed (e.g., 100% PSA) | % PSA | RMBS | PSA prepayment speed |
| `rmbs_sda_speed` | SDA default speed | % SDA | RMBS | SDA default speed |
| `rmbs_waltv` | Weighted average LTV | Decimal | RMBS | Weighted average LTV |
| `rmbs_wafico` | Weighted average FICO | Score | RMBS | Weighted average FICO score |

### Alternatives (24)

PE fund metrics, DCF valuation, repo analytics, inflation-linked bond metrics, VaR.

| Metric | Formula | Units | Instrument | Description |
|--------|---------|-------|------------|-------------|
| `real_yield` | Inflation-adjusted yield | Decimal | Inflation-Linked Bond | Real yield |
| `index_ratio` | Current CPI / base CPI | Ratio | Inflation-Linked Bond | Inflation index ratio |
| `breakeven_inflation` | Nominal yield - real yield | Decimal | Inflation-Linked Bond | Breakeven inflation rate |
| `lp_irr` | LP cash-flow IRR | Decimal | PE Fund | LP internal rate of return |
| `gp_irr` | GP cash-flow IRR | Decimal | PE Fund | GP internal rate of return |
| `moic_lp` | Total value / invested capital | Multiple | PE Fund | LP multiple on invested capital |
| `dpi_lp` | Distributions / paid-in capital | Multiple | PE Fund | LP distributions to paid-in |
| `tvpi_lp` | (Distributions + NAV) / paid-in | Multiple | PE Fund | LP total value to paid-in |
| `carry_accrued` | Accrued carry for GP | Currency | PE Fund | Accrued carry amount |
| `nav01` | PV change per 1% NAV | Currency/% | PE Fund | NAV sensitivity |
| `carry01` | PV change per 1bp carry rate | Currency/bp | PE Fund | GP carry sensitivity |
| `hurdle01` | PV change per 1bp hurdle rate | Currency/bp | PE Fund | Hurdle rate sensitivity |
| `enterprise_value` | PV(FCFs) + PV(terminal value) | Currency | DCF | Enterprise value |
| `equity_value` | Enterprise value - net debt | Currency | DCF | Equity value |
| `terminal_value_pv` | PV of terminal value | Currency | DCF | Present value of terminal value |
| `collateral_value` | Market value of collateral | Currency | Repo | Collateral market value |
| `required_collateral` | Collateral value x (1 - haircut) | Currency | Repo | Required collateral (with haircut) |
| `collateral_coverage` | Collateral value / loan | Ratio | Repo | Collateral coverage ratio |
| `repo_interest` | Principal x rate x DCF | Currency | Repo | Repo interest amount |
| `funding_risk` | PV change per 1bp repo rate | Currency/bp | Repo | Funding/repo rate sensitivity |
| `effective_rate` | Adjusted repo rate | Decimal | Repo | Effective repo rate |
| `implied_collateral_return` | Implied return from repo structure | Decimal | Repo | Implied collateral return |
| `hvar` | Historical VaR at confidence level | Currency | Portfolio | Historical Value-at-Risk |
| `expected_shortfall` | E[loss given loss > VaR] | Currency | Portfolio | Expected shortfall (CVaR) |

## How to Add a New Metric

### Step 1: Add Metric ID

Add your metric identifier to `core/ids.rs`:

```rust
impl MetricId {
    // ... existing metrics

    /// Your new metric description
    pub const MyNewMetric: Self = Self(Cow::Borrowed("my_new_metric"));
}
```

Don't forget to add it to the `ALL_STANDARD` array if it's a standard metric:

```rust
pub const ALL_STANDARD: &'static [MetricId] = &[
    // ... existing metrics
    MetricId::MyNewMetric,
];
```

### Step 2: Implement the Calculator

Create a calculator struct that implements `MetricCalculator`:

```rust
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct MyNewMetricCalculator;

impl MetricCalculator for MyNewMetricCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // 1. Downcast instrument if needed
        let bond: &Bond = context.instrument_as()?;

        // 2. Access dependencies
        let ytm = context.computed.get(&MetricId::Ytm)
            .copied()
            .unwrap_or(0.0);

        // 3. Perform calculation
        let result = ytm * bond.face_value().amount();

        Ok(result)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]  // Declare dependencies
    }
}
```

### Step 3: Register the Metric

Add registration in the appropriate instrument's `metrics.rs` module:

```rust
pub fn register_bond_metrics(registry: &mut MetricRegistry) {
    // ... existing registrations

    registry.register_metric(
        MetricId::MyNewMetric,
        Arc::new(MyNewMetricCalculator),
        &["Bond"],  // Applies to Bond only
    );
}
```

Or register for all instruments:

```rust
registry.register_metric(
    MetricId::MyNewMetric,
    Arc::new(MyNewMetricCalculator),
    &[],  // Empty = applies to all instruments
);
```

### Step 4: Add Tests

Create comprehensive tests in the appropriate test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{MetricRegistry, MetricContext};
    use std::sync::Arc;

    #[test]
    fn test_my_new_metric() {
        // Setup
        let bond = create_test_bond();
        let market = create_test_market();
        let as_of = create_date(2024, Month::January, 1).unwrap();

        // Create context
        let base_value = bond.value(&market, as_of).unwrap();
        let mut context = MetricContext::new(
            Arc::new(bond),
            Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        // Calculate metric
        let calculator = MyNewMetricCalculator;
        let result = calculator.calculate(&mut context).unwrap();

        // Assert
        assert!((result - expected).abs() < 1e-6);
    }
}
```

### Step 5: Document the Metric

Add comprehensive documentation to `METRICS.md`:

```markdown
## MyNewMetric

**Category**: Bond Metrics
**Unit**: Dollars
**Sign Convention**: Positive = gains value when X increases

### Definition

[Clear mathematical definition or business explanation]

### Formula

```

MyNewMetric = YTM × Face Value

```

### Example

[Working code example showing usage]

### See Also

- Related metrics
- References to standards or papers
```

## Common Patterns

### Generic Calculators with Type Parameters

For reusable calculators across multiple instrument types:

```rust
use std::marker::PhantomData;

pub struct GenericDv01Calculator<I> {
    _phantom: PhantomData<I>,
}

impl<I: Instrument + CurveDependencies + 'static> MetricCalculator
    for GenericDv01Calculator<I>
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let curve_id = instrument.discount_curve_id();

        // Bump and reprice
        let bumped_market = bump_curve(&context.curves, curve_id, 0.0001)?;
        let bumped_pv = instrument.value(&bumped_market, context.as_of)?;

        let dv01 = (bumped_pv.amount() - context.base_value.amount()) / 10_000.0;
        Ok(dv01)
    }
}
```

### Bucketed/Key-Rate Metrics

For metrics that compute sensitivities across multiple buckets:

```rust
impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let buckets = standard_ir_dv01_buckets();  // [0.25, 0.5, 1.0, ...]
        let mut series = Vec::new();
        let mut total = 0.0;

        for bucket_time in buckets {
            let label = format_bucket_label(bucket_time);

            // Bump at key rate
            let bumped_market = bump_key_rate(
                &context.curves,
                &curve_id,
                bucket_time,
                0.0001
            )?;

            let bumped_pv = instrument.value(&bumped_market, context.as_of)?;
            let bucket_dv01 = (bumped_pv.amount() - context.base_value.amount()) / 10_000.0;

            series.push((label, bucket_dv01));
            total += bucket_dv01;
        }

        // Store bucketed series
        context.store_bucketed_series(MetricId::BucketedDv01, series);

        Ok(total)
    }
}
```

### Metrics with Configuration

For calculators that need configuration:

```rust
pub struct ConfigurableThetaCalculator {
    period: String,  // "1D", "1W", "1M", etc.
}

impl ConfigurableThetaCalculator {
    pub fn new(period: String) -> Self {
        Self { period }
    }

    pub fn daily() -> Self {
        Self::new("1D".to_string())
    }
}

impl MetricCalculator for ConfigurableThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        use finstack_valuations::metrics::sensitivities::theta::calculate_theta_date;

        let expiry = context.instrument.expiry();
        let forward_date = calculate_theta_date(context.as_of, &self.period, expiry)?;

        // Price at forward date
        let forward_pv = context.instrument.value(&context.curves, forward_date)?;
        let theta = forward_pv.amount() - context.base_value.amount();

        Ok(theta)
    }
}
```

## Finite Difference Utilities

The `finite_difference` module provides standard bump sizes and helper functions:

```rust
use crate::metrics::{bump_sizes, bump_scalar_price, bump_discount_curve_parallel};

// Standard bump sizes
let spot_bump = bump_sizes::SPOT;              // 1% (0.01)
let vol_bump = bump_sizes::VOLATILITY;         // 1% (0.01)
let rate_bump = bump_sizes::INTEREST_RATE_BP;   // 1bp (in bp units: 1.0)
let spread_bump = bump_sizes::CREDIT_SPREAD_BP; // 1bp (in bp units: 1.0)

// Helper functions
let bumped_market = bump_scalar_price(&context.curves, "AAPL", 0.01)?;
let bumped_market = bump_discount_curve_parallel(&context.curves, &curve_id, 1.0)?;
```

## Best Practices

### 1. Type Safety

Always use strong typing and avoid runtime downcasting when possible:

```rust
// Good: Use trait bounds
impl<I: Instrument + CurveDependencies> MetricCalculator for MyCalc<I> {
    // ...
}

// Avoid: Runtime downcasting
fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
    // Only when absolutely necessary
    let bond: &Bond = context.instrument_as()?;
    // ...
}
```

### 2. Error Handling

Provide clear error messages:

```rust
fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
    let ytm = context.computed.get(&MetricId::Ytm)
        .ok_or_else(|| Error::Validation(
            "MyMetric requires YTM to be computed first".to_string()
        ))?;

    // ...
}
```

### 3. Determinism

Ensure calculations are deterministic:

```rust
// For Monte Carlo pricing in finite differences
instrument.pricing_overrides_mut().mc_seed_scenario = Some("delta_up".to_string());
let pv_up = instrument.value(&bumped_market, as_of)?;

instrument.pricing_overrides_mut().mc_seed_scenario = Some("delta_down".to_string());
let pv_down = instrument.value(&bumped_market, as_of)?;
```

### 4. Performance

Cache intermediate results to avoid redundant calculations:

```rust
fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
    // Check if already computed
    if let Some(&cached) = context.computed.get(&MetricId::MyMetric) {
        return Ok(cached);
    }

    // Compute expensive calculation once
    let cashflows = context.cashflows.get_or_insert_with(|| {
        context
            .instrument
            .cashflow_schedule(context.curves.as_ref(), context.as_of)
    });

    // Use cached cashflows
    // ...
}
```

### 5. Documentation

Follow the documentation standards:

- Document all public types, traits, and functions
- Include working examples in doc comments
- Add mathematical formulas for complex metrics
- Reference industry standards where applicable

## Testing Strategy

### Unit Tests

Test individual calculators in isolation:

```rust
#[test]
fn test_theta_calculator() {
    let calculator = ThetaCalculator::daily();
    let mut context = create_test_context();

    let theta = calculator.calculate(&mut context).unwrap();

    assert!((theta - expected_theta).abs() < TOLERANCE);
}
```

### Integration Tests

Test metrics within the full registry:

```rust
#[test]
fn test_bond_metrics_integration() {
    let registry = standard_registry();
    let bond = create_test_bond();
    let market = create_test_market();
    let as_of = test_date();

    let base_value = bond.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(
        Arc::new(bond),
        Arc::new(market),
        as_of,
        base_value,
        MetricContext::default_config(),
    );

    let metrics = vec![MetricId::Ytm, MetricId::DurationMod, MetricId::Convexity];
    let results = registry.compute(&metrics, &mut context).unwrap();

    assert!(results.contains_key(&MetricId::Ytm));
    assert!(results.contains_key(&MetricId::DurationMod));
}
```

### Property Tests

Test invariants and mathematical properties:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_dv01_sign_convention(coupon_rate in 0.01..0.10) {
        let bond = create_bond_with_coupon(coupon_rate);
        let dv01 = compute_dv01(&bond);

        // DV01 should be negative for bonds (lose value when rates rise)
        prop_assert!(dv01 < 0.0);
    }
}
```

## See Also

- **`METRICS.md`**: Comprehensive documentation of all metrics including formulas, conventions, and examples
- **`core/traits.rs`**: Core trait definitions and interfaces
- **`core/registry.rs`**: Registry implementation and dependency resolution
- **Documentation standards**: `.cursor/rules/rust/documentation.mdc`

## Contributing

When adding new metrics:

1. Follow the step-by-step guide above
2. Add comprehensive tests
3. Update `METRICS.md` with metric documentation
4. Ensure all lints pass: `make lint`
5. Ensure all tests pass: `make test-rust`
6. Add examples showing realistic usage

For questions or discussions, refer to the main project documentation or consult the development team.
