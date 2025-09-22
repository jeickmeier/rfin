# Generic Basket/ETF Implementation

This module implements a generic basket instrument that can handle various asset types including equities, bonds, ETFs, and other instruments. It's designed to support both equity ETFs (like SPY) and bond ETFs (like LQD, HYG) using a unified architecture.

## Key Features

### ✅ Multi-Asset Support
- **Equity securities**: Individual stocks with market data or instrument references
- **Bond securities**: Corporate bonds, treasuries, using existing Bond instrument pricing
- **ETF securities**: Other ETFs for fund-of-funds structures
- **Cash equivalents**: Money market instruments
- **Derivatives**: Futures, swaps, and other structured products

### ✅ Flexible Pricing Infrastructure
- **Leverages existing pricing**: Uses existing `Bond` and `Equity` instrument pricing methods
- **Market data fallback**: Simple price lookups from `MarketContext` for instruments not fully modeled
- **No duplicate logic**: Avoids reimplementing bond DCF or equity pricing calculations

### ✅ NAV and Valuation
- **Real-time NAV calculation**: Net Asset Value per share using constituent pricing
- **Expense ratio application**: Automatic deduction of management fees
- **Currency safety**: Consistent currency handling across all constituents
- **Validation**: Weight sum validation and consistency checks

### ✅ Creation/Redemption Mechanics
- **Creation baskets**: Calculate securities needed for ETF creation units
- **Transaction costs**: Model creation/redemption transaction costs
- **Arbitrage analysis**: Calculate premium/discount to NAV
- **Physical vs synthetic**: Support for different replication methods

### ✅ Comprehensive Metrics
- **NAV**: Net Asset Value per share
- **Basket Value**: Total portfolio value
- **Tracking Error**: Volatility vs benchmark (when benchmark data available)
- **Expense Ratio**: Management fee percentage
- **Premium/Discount**: Trading premium/discount to NAV
- **Asset Exposure**: Breakdown by asset type (equity vs bond allocation)

## Architecture Design

### Constituent Reference Pattern
```rust
pub enum ConstituentReference {
    /// Direct instrument reference (uses existing pricing)
    Instrument(Arc<dyn Instrument>),
    /// Market data reference (simple price lookup)
    MarketData { price_id: String, asset_type: AssetType },
}
```

This pattern allows maximum flexibility:
- **Full instrument modeling**: For complex securities requiring detailed cashflow analysis
- **Simple market data**: For securities where only price is needed
- **Performance optimization**: Choose the right level of detail per constituent

### Generic Asset Type Support
```rust
pub enum AssetType {
    Equity,
    Bond, 
    ETF,
    Cash,
    Commodity,
    Derivative,
}
```

Supports all major asset classes without requiring separate implementations.

## Usage Examples

### Equity ETF (SPY-like)
```rust
let spy = Basket::builder()
    .equity_etf("SPY", "SPY", "SPDR S&P 500 ETF Trust")
    .shares_outstanding(900_000_000.0)
    .add_market_data("AAPL", "AAPL", AssetType::Equity, 0.071, None)
    .add_market_data("MSFT", "MSFT", AssetType::Equity, 0.069, None)
    .add_market_data("GOOGL", "GOOGL", AssetType::Equity, 0.036, None)
    .build()?;

let nav = spy.nav(&market_context, valuation_date)?;
```

### Bond ETF (LQD-like)
```rust
let lqd = Basket::builder()
    .bond_etf("LQD", "LQD", "iShares iBoxx $ IG Corporate Bond ETF")
    .add_bond("AAPL_BOND", aapl_bond, 0.015, Some(15000.0))
    .add_bond("MSFT_BOND", msft_bond, 0.012, Some(12000.0))
    .build()?;

let nav = lqd.nav(&market_context, valuation_date)?;
```

### Mixed Asset Basket
```rust
let balanced = Basket::builder()
    .id("BALANCED")
    .name("Balanced Equity/Bond ETF")
    .currency(Currency::USD)
    .expense_ratio(0.0025)  // 25 bps
    .add_market_data("AAPL", "AAPL", AssetType::Equity, 0.30, None)
    .add_bond("TREASURY", treasury_bond, 0.40, None)
    .add_market_data("CASH", "USD_CASH", AssetType::Cash, 0.30, None)
    .build()?;
```

### AUM‑Aware Valuation (weights without shares)

When constituents are defined by weights and `shares_outstanding` is not available, value the basket using an explicit AUM. All contributions are computed as `weight × AUM` in the basket currency (with FX conversion applied as needed).

```rust
use finstack_valuations::instruments::basket::Basket;
use finstack_core::prelude::*;

let aum = Money::new(1_000_000_000.0, Currency::USD); // 1B USD AUM
let basket_value = basket.basket_value_with_aum(&market_context, valuation_date, aum)?;
let nav = basket.nav_with_aum(&market_context, valuation_date, aum)?; // If shares_outstanding is set, returns per‑share NAV
```

Notes:
- If a constituent specifies `units`, those take precedence over weights (price × units).
- If only `weight` is provided and neither `shares_outstanding` nor AUM is given, valuation fails with an input error (no hardcoded proxies).

## Technical Implementation Details

### Pricing Strategy
1. **Instrument References**: Call `instrument.value(context, as_of)` for full instrument models
2. **Market Data References**: Lookup prices from `context.price(price_id)` for simple cases
3. **FX Conversion (currency safety)**: Constituent values are converted to the basket currency via `MarketContext.fx` and `FxMatrix::rate`, using a configurable `FxConversionPolicy`.
4. **Weight/Units Application**: Units → price × units; Weights → require `shares_outstanding` or AUM
5. **Expense Ratio**: Apply daily accrual using configurable `BasketPricerConfig.days_in_year`

### Currency Handling
- All constituents must be compatible with the basket's base currency
- Uses existing `Money` type for currency safety
- Supports FX conversion through existing `MarketContext` infrastructure

### Memory Efficiency
- Uses `Arc<dyn Instrument>` for shared instrument references
- Avoids cloning heavy instrument objects
- Lazy evaluation of constituent values

### Serialization Support
- Supports serde serialization for basket metadata
- Handles trait object serialization limitations gracefully
- Market data references serialize fully; instrument references use placeholders

### Configuration

- **BasketPricerConfig**: Controls time basis and FX policy.
  - `days_in_year`: e.g., 365.0 or 365.25 (default 365.25)
  - `fx_policy`: `FxConversionPolicy` for conversion lookups (default `CashflowDate`)

Example (custom configuration):
```rust
use finstack_valuations::instruments::basket::pricing::engine::{BasketPricer, BasketPricerConfig};
use finstack_core::money::fx::FxConversionPolicy;

let pricer = BasketPricer::with_config(BasketPricerConfig {
    days_in_year: 365.0,
    fx_policy: FxConversionPolicy::PeriodEnd,
});
```

## Integration with Existing Systems

### Leverages Existing Infrastructure
- **Bond pricing**: Uses existing `Bond` instrument and its pricing methods
- **Equity pricing**: Uses existing `Equity` instrument patterns
- **Market data**: Integrates with `MarketContext` for price lookups
- **Metrics system**: Uses existing `MetricCalculator` and `MetricRegistry`
- **Risk framework**: Compatible with existing risk measurement systems

### Follows Library Patterns
- **Trait implementations**: Implements `Instrument`, `Attributable`
- **Builder pattern**: Consistent with other instrument builders
- **Error handling**: Uses library's unified error types
- **Currency safety**: Follows existing currency validation patterns

## Future Enhancements

### Potential Extensions
- **Historical tracking**: Store historical NAV and tracking error
- **Rebalancing simulation**: Model periodic rebalancing costs and effects
- **Tax efficiency**: Model tax-loss harvesting and in-kind redemptions
- **Sector exposure**: Calculate sector/geographic allocations
- **Performance attribution**: Decompose returns by constituent contribution

### Advanced Features
- **Monte Carlo simulation**: For basket performance under different scenarios
- **Stress testing**: Impact of constituent defaults or market shocks
- **Optimization**: Minimize tracking error subject to constraints
- **Index replication**: Smart sampling algorithms for large indices

## Testing Coverage

The implementation includes comprehensive tests for:
- ✅ Equity ETF creation and pricing
- ✅ Bond ETF creation and pricing  
- ✅ Mixed asset baskets
- ✅ Metrics computation
- ✅ Weight validation
- ✅ Currency consistency
- ✅ Creation/redemption mechanics
- ✅ NAV vs basket value calculations

All tests pass and demonstrate the basket working with both equity and bond constituents using existing pricing infrastructure.
