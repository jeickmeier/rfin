# Market Standards Review: Common Instrument Infrastructure

## Executive Summary
The `finstack/valuations/src/instruments/common` module provides a robust, industrial-strength foundation for financial instrument valuation. The architecture enforces strong type safety, explicit currency handling, and clear separation of concerns between instrument definitions, pricing logic, and risk metric calculation. The implementation aligns well with modern quantitative finance standards, particularly in its handling of multi-curve frameworks, Monte Carlo methods, and dependency management.

## Detailed Component Review

### 1. Core Abstractions (`traits.rs`)
*   **Instrument Trait**: The unified `Instrument` trait correctly balances identity (`id`, `key`), metadata (`attributes`), and pricing (`value`, `price_with_metrics`).
*   **Dependency Discovery**: The explicit declaration methods (`required_discount_curves`, `required_hazard_curves`, etc.) enable efficient sensitivity analysis (e.g., bump-and-reprice) without requiring the risk engine to inspect opaque instrument internals.
*   **Attributes**: The flexible tagging system supports both categorical tags and structured metadata, essential for portfolio management.

### 2. Pricing Infrastructure
*   **Generic Pricers**: `GenericInstrumentPricer` reduces boilerplate and ensures consistent error handling.
*   **Metrics Bridge**: `build_with_metrics_dyn` cleanly decouples core valuation (NPV) from auxiliary metrics (Greeks), allowing the metric registry to evolve independently.

### 3. Financial Logic & Standards

#### Periodized Present Value (`period_pv.rs`)
*   **Currency Safety**: The `PeriodizedPvExt` trait returns `IndexMap<PeriodId, IndexMap<Currency, Money>>`. This explicit preservation of currency separation is a critical safety feature.
*   **Credit Adjustment**: The `periodized_pv_credit_adjusted` method correctly applies recovery rates when provided by the hazard curve. It distinguishes between principal (using recovery) and interest (zero recovery) flows when the underlying schedule provides full `CashFlow` objects.

#### Analytical Greeks (`models/closed_form/greeks.rs`)
*   **Correctness**: Black-Scholes-Merton implementations are mathematically correct, including dividend yield adjustments.
*   **Vega Convention**: The implementation explicitly scales Vega by **0.01** (sensitivity per 1% vol change). This standardizes the unit across the library, preventing downstream scaling errors.

#### Monte Carlo Engine (`models/monte_carlo/`)
*   **RNG Standards**: Uses **Philox** (counter-based) and **Sobol** (quasi-random) sequences. This represents the state-of-the-art for parallel financial simulations.
*   **Variance Reduction**: Supports antithetic variates and uses Common Random Numbers (CRN) for finite difference Greeks (`finite_diff.rs`), which significantly improves the stability of Delta/Gamma estimates.
*   **Vega Consistency**: Monte Carlo Vega implementations (LRM, Pathwise) have been updated to adhere to the **0.01 scaling** convention, ensuring consistency with analytical models.

#### Volatility Modeling (`models/volatility/sabr.rs`)
*   **Robustness**: The implementation handles **Shifted SABR** for negative rate environments.
*   **Singularity Handling**: Explicitly handles numerical edge cases (e.g., $\rho \approx 1$, $z \approx 0$) to prevent `NaN` propagation in production.
*   **Calibration**: Includes a Levenberg-Marquardt solver, which is the industry standard for robust non-linear calibration.

### 4. Code Quality & Safety
*   **Type Safety**: Extensive use of newtypes (`CurveId`, `PeriodId`) and Enums prevents "stringly typed" errors.
*   **Error Handling**: Granular `PricingError` types provide clear failure context.
*   **Testing**: Comprehensive unit tests cover success paths, edge cases (empty periods), and parity checks.

## Conclusion
The module meets and exceeds typical market standards for a valuation library core. Its design successfully mitigates common risks (currency errors, opaque dependencies) while providing a flexible foundation for complex instrument modeling.
