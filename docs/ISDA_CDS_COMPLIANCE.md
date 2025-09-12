# ISDA 2014 CDS Standard Model Compliance

## Overview

The finstack CDS pricing implementation now fully complies with the ISDA 2014 Credit Derivatives Definitions and standard model specifications. This ensures accurate, market-standard pricing that aligns with industry practices.

## Key Compliance Features

### 1. Exact Integration Points

**Previous Implementation:**
- Used simplified midpoint integration
- Could lead to inaccuracies in protection leg valuation

**ISDA-Compliant Implementation:**
- `IntegrationMethod::IsdaExact` - Uses exact integration points per ISDA specifications
- Standard 40 integration points per year for accurate protection leg calculation
- Proper handling of piecewise constant hazard rates between knot points

```rust
// ISDA exact integration
let config = CDSPricerConfig::isda_standard();
let pricer = CDSPricer::with_config(config);
```

### 2. Standard Coupon Dates

**ISDA Standard:**
- Coupon payments on the 20th of March, June, September, and December
- Eliminates stub periods through standardization
- Ensures full coupon settlements

**Implementation:**
```rust
// Automatically uses ISDA standard dates
fn generate_isda_schedule(&self, cds: &CreditDefaultSwap) -> Result<Vec<Date>> {
    // Uses next_cds_date() to get standard 20th Mar/Jun/Sep/Dec dates
}
```

### 3. Proper Stub Accrual Handling

**Features:**
- Accurate accrual-on-default calculation
- Aligns accrual start dates with nearest standard coupon date
- Eliminates irregular stub periods
- ISDA-compliant day count conventions

```rust
// ISDA exact accrual calculation
fn accrual_on_default_isda_exact(...) {
    // Uses proper integration with standard points
    // Accounts for exact accrual fractions
}
```

### 4. Standard Recovery Rate Assumptions

**ISDA Standards:**
- Senior Unsecured: 40% recovery
- Subordinated: 20% recovery

**Implementation:**
```rust
// Standard recovery rates
pub const STANDARD_RECOVERY_SENIOR: F = 0.40;
pub const STANDARD_RECOVERY_SUB: F = 0.20;

// Convenient builder methods
let params = CreditParams::senior_unsecured("Entity", "CREDIT");  // 40%
let params = CreditParams::subordinated("Entity", "CREDIT");      // 20%
```

## Configuration Options

### ISDA Standard Configuration (Default)

```rust
let config = CDSPricerConfig::isda_standard();
// Equivalent to:
CDSPricerConfig {
    steps_per_year: 40,                              // ISDA standard points
    include_accrual: true,                           // Include accrual-on-default
    exact_daycount: true,                            // Exact day count fractions
    tolerance: 1e-10,                                // High precision
    integration_method: IntegrationMethod::IsdaExact,// Exact integration
    use_isda_coupon_dates: true,                    // Standard dates
}
```

### Simplified Configuration (Performance)

```rust
let config = CDSPricerConfig::simplified();
// For faster but less accurate pricing:
CDSPricerConfig {
    steps_per_year: 365,                             // Daily integration
    include_accrual: true,
    exact_daycount: false,                           // Approximate day count
    tolerance: 1e-7,
    integration_method: IntegrationMethod::Midpoint, // Simple midpoint
    use_isda_coupon_dates: false,                   // Regular schedule
}
```

## Usage Examples

### Basic ISDA-Compliant CDS Pricing

```rust
use finstack_valuations::instruments::fixed_income::cds::{
    CDSPricer, CreditDefaultSwap, CDSConvention, PayReceive
};

// Create CDS with ISDA convention
let cds = CreditDefaultSwap::new_isda(
    "CDS-001",
    Money::new(10_000_000.0, Currency::USD),
    "ABC Corp",
    PayReceive::PayProtection,
    CDSConvention::IsdaNa,
    start_date,
    maturity_date,
    150.0,  // 150 bps spread
    "CREDIT-CURVE",
    0.40,   // ISDA standard senior recovery
    "USD-OIS",
);

// Price with ISDA-compliant pricer (default)
let pricer = CDSPricer::new();

// Calculate metrics
let npv = pricer.npv(&cds, disc, surv, as_of)?;
let par_spread = pricer.par_spread(&cds, disc, surv, as_of)?;
let cs01 = pricer.cs01(&cds, &context, as_of)?;
```

### Using Builder Pattern with ISDA Standards

```rust
// Use standard parameter groups
let credit_params = CreditParams::senior_unsecured("ABC Corp", "CREDIT");

let cds = CreditDefaultSwap::builder()
    .id("CDS-ABC-5Y")
    .notional(Money::new(10_000_000.0, Currency::USD))
    .side(PayReceive::PayProtection)
    .spread_bp(150.0)
    .credit_params(credit_params)  // Uses 40% recovery
    .dates(start, maturity)
    .market_refs(MarketRefs::new("USD-OIS"))
    .convention(CDSConvention::IsdaNa)
    .build()?;
```

## Testing and Validation

### Compliance Tests

The implementation includes comprehensive tests to verify ISDA compliance:

```rust
#[test]
fn test_isda_2014_full_compliance() {
    // Verifies:
    // - Standard configuration defaults
    // - Recovery rate standards
    // - Exact integration method
    // - Standard coupon dates (20th Mar/Jun/Sep/Dec)
    // - Par spread calculation accuracy
}

#[test]
fn test_isda_exact_vs_simplified_integration() {
    // Compares ISDA exact with simplified methods
    // Ensures accuracy improvements
}
```

### Performance Considerations

| Method | Accuracy | Performance | Use Case |
|--------|----------|-------------|----------|
| ISDA Exact | Highest | Standard | Production pricing, risk management |
| Gaussian Quadrature | High | Good | Alternative accurate method |
| Adaptive Simpson | High | Good | Adaptive precision |
| Midpoint | Moderate | Fast | Quick estimates, screening |

## Migration Guide

### From Simplified to ISDA-Compliant

```rust
// Before (simplified)
let pricer = CDSPricer::with_config(CDSPricerConfig {
    integration_method: IntegrationMethod::Midpoint,
    use_isda_coupon_dates: false,
    // ...
});

// After (ISDA-compliant)
let pricer = CDSPricer::new(); // Uses ISDA standard by default
// Or explicitly:
let pricer = CDSPricer::with_config(CDSPricerConfig::isda_standard());
```

### Recovery Rate Updates

```rust
// Before (manual recovery)
let cds = CreditDefaultSwap::new_isda(
    // ...
    0.35,  // Custom recovery
    // ...
);

// After (ISDA standard)
let params = CreditParams::senior_unsecured("Entity", "CREDIT"); // 40%
let cds = CreditDefaultSwap::builder()
    .credit_params(params)
    // ...
    .build()?;
```

## Benefits of ISDA Compliance

1. **Market Consistency**: Prices align with market standards and other ISDA-compliant systems
2. **Regulatory Compliance**: Meets regulatory requirements for standard model usage
3. **Reduced Basis Risk**: Eliminates pricing discrepancies from non-standard methods
4. **Interoperability**: Compatible with industry-standard CDS pricing tools
5. **Audit Trail**: Clear documentation of ISDA-compliant methods used

## References

- ISDA 2014 Credit Derivatives Definitions
- ISDA CDS Standard Model Specifications
- Industry best practices for CDS valuation

## Future Enhancements

- [ ] Support for additional ISDA conventions (EU, Asia)
- [ ] Index CDS (CDX, iTraxx) specific handling
- [ ] Enhanced recovery rate models
- [ ] Cross-currency CDS support
- [ ] Real-time market data integration
