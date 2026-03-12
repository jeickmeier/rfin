# Binding Parity Inventory

This document is the execution inventory for bringing `finstack-py` back to strict Rust public API parity.

## Core

### Missing Binding

- `finstack/core/src/lib.rs`: `error`, `prelude`, `validation`, `Error`, `InputError`, `NonFiniteKind`, `Result`
- `finstack/core/src/math/linalg.rs`: `CholeskyError`, `SINGULAR_THRESHOLD`, `DIAGONAL_TOLERANCE`, `SYMMETRY_TOLERANCE`, `cholesky_solve`
- `finstack/core/src/math/volatility/pricing.rs`: `brenner_subrahmanyam_approx`, `manaster_koehler_approx`, `implied_vol_initial_guess`

### Runtime/Stub Mismatch

- `finstack-py/src/core/math/linalg.rs`: `validate_correlation_matrix(matrix)` vs `finstack-py/finstack/core/math/linalg.pyi`: `validate_correlation_matrix(matrix, tolerance=1e-10)`

### Renamed / Relocated

- `finstack/core/src/math/volatility/mod.rs` moved into `finstack-py/src/core/volatility.rs` and `finstack-py/src/core/volatility_models.rs`
- `finstack/core/src/math/volatility/pricing.rs`: `bachelier_call`, `black_call`, `black_shifted_call` exposed as `bachelier_price`, `black_price`, `black_shifted_price`

### Python-Only Drift

- `finstack-py/src/core/market_data/volatility.rs`: `convert_volatility`
- `finstack-py/finstack/core/expr_helpers.py`: `ExprWrapper`, `col`, `lit`, `lag`, `lead`, `diff`, `pct_change`, `growth_rate`, `cumsum`, `cumprod`, `cummin`, `cummax`, `rolling_mean`, `rolling_sum`, `rolling_std`, `rolling_var`, `rolling_min`, `rolling_max`, `if_then_else`

## Scenarios

### Missing Binding

- `finstack/scenarios/src/lib.rs`: `error`, `Error`, `Result`

### Missing Stub

- `finstack-py/finstack/scenarios/builder.py`: `ScenarioBuilder`, `scenario` missing `finstack-py/finstack/scenarios/builder.pyi`
- `finstack-py/finstack/scenarios/dsl.py`: `DSLParseError`, `DSLParser`, `from_dsl` missing `finstack-py/finstack/scenarios/dsl.pyi`

### Python-Only Drift

- `finstack-py/finstack/scenarios/builder.py`: `ScenarioBuilder`, `scenario`
- `finstack-py/finstack/scenarios/dsl.py`: `DSLParseError`, `DSLParser`, `from_dsl`

## Valuations

### Missing Binding

- `finstack/valuations/src/instruments/mod.rs`: `CmsSwap`, `IrFutureOption`, `CommoditySpreadOption`, `CommoditySwaption`, `CollateralType`, `BarrierDirection`, `DigitalPayoutType`, `PayoutTiming`, `TouchType`
- `finstack/valuations/src/instruments/equity/mod.rs`: `DiscountedCashFlow`
- `finstack/valuations/src/calibration/mod.rs`: `CurveValidator`, `SurfaceValidator`, `QuoteQuality`, `CalibrationDiagnostics`
- `finstack/valuations/src/xva/types.rs`: `FundingConfig`, `ExposureDiagnostics`, `StochasticExposureConfig`, `StochasticExposureProfile`
- `finstack/valuations/src/attribution/mod.rs`: `AttributionInput`, `CarryDetail`, `CorrelationsAttribution`, `FxAttribution`, `InflationCurvesAttribution`, `ScalarsAttribution`, `VolAttribution`, `JsonEnvelope`, `attribute_pnl_taylor`, `TaylorAttributionConfig`, `TaylorAttributionResult`, `TaylorFactorResult`, `default_waterfall_order`, `CurveRestoreFlags`, `MarketSnapshot`, `ScalarsSnapshot`, `VolatilitySnapshot`, `compute_pnl`, `compute_pnl_with_fx`, `convert_currency`, `reprice_instrument`

### Missing Stub

- `finstack-py/finstack/valuations/__init__.pyi`: `VarMethod`, `VarConfig`, `VarResult`, `RiskFactorType`, `RiskFactorShift`, `MarketScenario`, `MarketHistory`, `calculate_var`, `krd_dv01_ladder`, `cs01_ladder`, `AttributionMethod`, `AttributionMeta`, `RatesCurvesAttribution`, `CreditCurvesAttribution`, `ModelParamsAttribution`, `PnlAttribution`, `PortfolioAttribution`, `attribute_pnl`, `attribute_portfolio_pnl`, `attribute_pnl_from_json`, `attribution_result_to_json`, `CalibrationReport`, `RateBounds`, `RateBoundsPolicy`, `SolverKind`, `ValidationConfig`, `ValidationMode`, `CALIBRATION_SCHEMA`, `execute_calibration`, `bump_discount_curve`
- `finstack-py/finstack/valuations/xva.pyi`: `ExposureProfile.diagnostics`

### Runtime/Stub Mismatch

- `finstack-py/src/valuations/attribution/mod.rs`: `AttributionMeta.tolerance_abs`, `AttributionMeta.tolerance_pct` vs `finstack-py/finstack/valuations/attribution.pyi`: `AttributionMeta.tolerance`
- `finstack-py/src/valuations/attribution/mod.rs`: `PnlAttribution` vs `finstack-py/finstack/valuations/attribution.pyi`: `PnlAttribution.credit_detail_to_csv`

### Renamed / Relocated

- `finstack/valuations/src/calibration/mod.rs`: `SolverConfig` exposed as `SolverKind`
- `finstack/valuations/src/instruments/mod.rs`: `CollateralSpec` exposed as `RepoCollateral`
- `finstack/valuations/src/instruments/mod.rs`: `XccySwap` exposed as `CrossCurrencySwap`

## Statements

### Missing Binding

- `finstack/statements/src/analysis/mod.rs`: `PercentileSeries`, `Report`

### Renamed / Relocated

- `finstack/statements/src/analysis/sensitivity.rs`: `generate_tornado_entries` exposed as `generate_tornado_chart`
- `finstack/statements/src/analysis/goal_seek.rs`: `goal_seek` exposed as `FinancialModelSpec.goal_seek`

### Python-Only Drift

- `finstack-py/src/statements/templates.rs`: constructor validation for lease-related specs
- `finstack-py/src/statements/builder/mod.rs`: formula validation outside Rust

## Portfolio

### Missing Binding

- `finstack/portfolio/src/types.rs`: `EntityId`, `PositionId`
- `finstack/portfolio/src/optimization/mod.rs`: `ConstraintValidationError`, `PortfolioOptimizer`
- `finstack/portfolio/src/margin/mod.rs`: `CurrencyMismatchError`

### Missing Stub

- `finstack-py/src/portfolio/valuation.rs`: `PortfolioValuation.to_polars`, `PortfolioValuation.entities_to_polars` missing from `finstack-py/finstack/portfolio/valuation.pyi`

### Renamed / Relocated

- `finstack/portfolio/src/optimization/result.rs`: `PortfolioOptimizationResult` exposed as `OptimizationResult`
- `finstack/portfolio/src/dataframe.rs`: `positions_to_dataframe`, `entities_to_dataframe`, `metrics_to_dataframe`, `aggregated_metrics_to_dataframe` exposed as `*_to_polars`

### Python-Only Drift

- `finstack-py/src/portfolio/margin.rs`: `NettingSetId`
