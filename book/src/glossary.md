# Glossary

## A

**ACT/360** - Actual/360 day count convention, commonly used in money markets.

**Amount** - A currency-safe monetary value in Finstack, combining a Decimal value with an ISO currency code.

**ABS** - Asset-Backed Security, a structured product backed by pools of assets.

## B

**Barrier Option** - An exotic option whose payoff depends on whether the underlying asset crosses a barrier level.

**Basis Point (bp)** - One hundredth of one percent (0.01%). Used to describe interest rate changes.

**Bond** - A fixed-income debt security paying periodic coupons and returning principal at maturity.

## C

**Calibration** - The process of fitting model parameters to match market prices.

**CDS** - Credit Default Swap, a derivative for transferring credit risk.

**CLO** - Collateralized Loan Obligation, a structured credit product.

**CMBS** - Commercial Mortgage-Backed Security.

**Coupon** - Periodic interest payment on a bond.

**Currency Safety** - Type-level guarantee that prevents accidental mixing of different currencies.

## D

**Day Count Convention** - Method for calculating the time fraction between two dates (e.g., ACT/360, 30/360).

**Decimal** - Fixed-precision decimal type used for all financial calculations (avoiding floating point errors).

**Determinism** - Property that guarantees identical results across runs, platforms, and serial/parallel execution modes.

**Discount Curve** - Term structure representing present value of future cash flows.

**DV01** - Dollar Value of a 01, the change in value for a 1 basis point change in rates.

## E

**Expression Engine** - Finstack's formula evaluation system supporting Polars DataFrame operations.

## F

**FX (Foreign Exchange)** - Currency conversion and exchange rates.

**FxProvider** - Interface for obtaining exchange rates between currency pairs.

## G

**Greeks** - Sensitivity measures for options (Delta, Gamma, Vega, Theta, Rho).

## H

**Hazard Rate** - Instantaneous probability of default in credit modeling.

## I

**IRS** - Interest Rate Swap, exchanging fixed and floating rate cash flows.

**ISDA** - International Swaps and Derivatives Association, sets standards for derivatives.

## M

**Market Context** - Container for all market data (curves, surfaces, FX rates) as of a valuation date.

**Metrics Registry** - Collection of computed risk and valuation metrics for an instrument.

**Monte Carlo** - Simulation-based pricing method using random paths.

## P

**Period** - Time interval with start and end dates, used in financial modeling.

**Present Value (PV)** - Current value of future cash flows, discounted to today.

**Pricer** - Component that prices an instrument given market data.

## R

**Rate** - Interest rate or return, represented as a fraction (e.g., 5% = 0.05).

**RMBS** - Residential Mortgage-Backed Security.

**Rounding Context** - Global policy for decimal rounding (mode, scale, precision).

## S

**Scenario** - Hypothetical market conditions for stress testing (e.g., "rates +100bp").

**Serde** - Rust serialization framework; Finstack uses strict serde for stable wire formats.

**Surface** - Two-dimensional term structure (e.g., volatility surface by strike and expiry).

**Swaption** - Option on an interest rate swap.

## T

**Term Structure** - Relationship between time to maturity and rates, yields, or other quantities.

## V

**Valuation Date** - The "as of" date for pricing and risk calculations.

**Vol Surface** - Volatility surface showing implied volatility by strike and expiry.

## Y

**Yield** - Return on an investment, often expressed as an annualized percentage.

**Yield Curve** - Term structure of interest rates across different maturities.

---

*For definitions specific to Python or WASM bindings, see the respective sections.*
