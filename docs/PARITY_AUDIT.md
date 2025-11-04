# Python-WASM Bindings Parity Audit

**Generated:** compare_apis.py

## Executive Summary

- **Classes in both bindings:** 159
- **Only in Python:** 20
- **Only in WASM:** 20
- **Total unique classes:** 199

## Instrument Coverage

- **Expected instruments:** 38
- **In Python:** 36 (94%)
- **In WASM:** 36 (94%)
- **In both:** 35

### Missing in Python

```
- CDSIndex
- StructuredCredit
```

### Missing in WASM

```
- Basket
- StructuredCredit
```

## Calibration API Coverage

- **Expected calibration types:** 13
- **In Python:** 13 (100%)
- **In WASM:** 13 (100%)

## Complete Class Comparison

### Classes in Both Bindings

**Count:** 159

```
✓ AggregatedMetric
✓ AmortizationSpec
✓ AmountOrScalar
✓ ApplicationReport
✓ AsianOption
✓ Autocallable
✓ BarrierOption
✓ BarrierType
✓ BaseCorrelationCalibrator
✓ BaseCorrelationCurve
✓ BasisSwap
✓ BasisSwapLeg
✓ Bond
✓ BrentSolver
✓ CFKind
✓ Calendar
✓ CalibrationConfig
✓ CalibrationReport
✓ CapitalStructureSpec
✓ CashFlow
✓ CashFlowSchedule
✓ CashflowBuilder
✓ CdsOption
✓ CdsTranche
✓ CliquetOption
✓ CmsOption
✓ ConversionPolicy
✓ ConversionSpec
✓ ConvertibleBond
✓ CorkscrewExtension
✓ CouponType
✓ CreditDefaultSwap
✓ CreditIndexData
✓ CreditQuote
✓ CreditScorecardExtension
✓ Currency
✓ CurveKind
✓ DayCount
✓ DayCountContext
✓ DebtInstrumentSpec
✓ Deposit
✓ DiscountCurve
✓ DiscountCurveCalibrator
✓ DividendEvent
✓ DividendSchedule
✓ DividendScheduleBuilder
✓ Entity
✓ Equity
✓ EquityOption
✓ EquityTotalReturnSwap
✓ ExecutionContext
✓ ExerciseStyle
✓ ExtrapolationPolicy
✓ FiIndexTotalReturnSwap
✓ FinancialModelSpec
✓ FinstackConfig
✓ FiscalConfig
✓ FixedCouponSpec
✓ FloatCouponParams
✓ FloatingCouponSpec
✓ ForecastMethod
✓ ForecastSpec
✓ ForwardCurve
✓ ForwardCurveCalibrator
✓ ForwardRateAgreement
✓ Frequency
✓ FutureSpecs
✓ FxBarrierOption
✓ FxConfig
✓ FxConversionPolicy
✓ FxMatrix
✓ FxOption
✓ FxRateResult
✓ FxSpot
✓ FxSwap
✓ GaussHermiteQuadrature
✓ HazardCurve
✓ HazardCurveCalibrator
✓ HybridSolver
✓ InflationCurve
✓ InflationCurveCalibrator
✓ InflationLinkedBond
✓ InflationQuote
✓ InflationSwap
✓ InterestRateFuture
✓ InterestRateOption
✓ InterestRateSwap
✓ InterpStyle
✓ LookbackOption
✓ MarketContext
✓ MarketQuote
✓ MarketScalar
✓ MetricDefinition
✓ MetricId
✓ MetricRegistry
✓ Money
✓ MonteCarloResult
✓ MultiCurveConfig
✓ NewtonSolver
✓ NodeSpec
✓ NodeType
✓ OperationSpec
✓ OptionType
✓ PathDataset
✓ PathPoint
✓ PayReceive
✓ Period
✓ PeriodId
✓ PeriodPlan
✓ Portfolio
✓ PortfolioBuilder
✓ PortfolioMetrics
✓ PortfolioResults
✓ PortfolioValuation
✓ Position
✓ PositionUnit
✓ PositionValue
✓ PricerRegistry
✓ PrivateMarketsFund
✓ ProcessParams
✓ QuantoOption
✓ RangeAccrual
✓ RatesQuote
✓ Repo
✓ RepoCollateral
✓ ResultsMeta
✓ RevolvingCredit
✓ RollForwardReport
✓ RoundingMode
✓ SABRCalibrationDerivatives
✓ SABRMarketData
✓ SABRModelParams
✓ ScalarTimeSeries
✓ ScenarioEngine
✓ ScenarioSpec
✓ Schedule
✓ ScheduleBuilder
✓ ScheduleParams
✓ SeasonalMode
✓ SeriesInterpolation
✓ SettlementType
✓ SimpleCalibration
✓ SimulatedPath
✓ SolverKind
✓ StubKind
✓ Swaption
✓ TenorMatchMode
✓ TermLoan
✓ TrsScheduleSpec
✓ UnitType
✓ ValidationConfig
✓ ValidationError
✓ ValuationResult
✓ VarianceSwap
✓ VolQuote
✓ VolSurface
✓ VolSurfaceCalibrator
✓ VolSurfaceKind
✓ WaterfallTier
```

### Classes Only in Python

**Count:** 20

```
- AntiDilutionPolicy
- AveragingMethod
- Basket
- BusinessDayConvention
- CdsIndex
- CdsPayReceive
- ConversionEvent
- CovenantReport
- DividendAdjustment
- EquityUnderlyingParams
- FeeBase
- FeeSpec
- FinancingLegSpec
- FixedWindow
- FloatWindow
- IndexUnderlyingParams
- LookbackType
- RealizedVarMethod
- Thirty360Convention
- TrsSide
```

### Classes Only in WASM

**Count:** 20

```
- CDSIndex
- EquityUnderlying
- Evaluator
- ExtensionMetadata
- ExtensionRegistry
- ExtensionResult
- ExtensionStatus
- FsDate
- IndexUnderlying
- ModelBuilder
- MonteCarloPathGenerator
- PricingRequest
- Registry
- Results
- TrsFinancingLegSpec
- WasmExplanationTrace
- applyAndRevalue
- applyScenario
- cs01Ladder
- krdDv01Ladder
```

## Naming Convention Patterns

### Identified Patterns

| Python | WASM | Pattern |
|--------|------|---------|
| `build_periods` | `buildPeriods` | snake_case → camelCase |
| `from_code` | `fromCode` | snake_case → camelCase |
| `next_imm` | `nextImm` | snake_case → camelCase |
| `is_actual` | `isActual` | snake_case → camelCase |
| `Currency` | `Currency` | PascalCase → PascalCase |
| `Money` | `Money` | PascalCase → PascalCase |

## Recommendations

### High Priority

1. **Add 2 missing instruments to WASM:** Basket, StructuredCredit
2. **Add 2 missing instruments to Python:** CDSIndex, StructuredCredit

### Medium Priority

1. **Create comprehensive method parity report** - Compare methods within each class
2. **Document naming convention mapping** - Create NAMING_CONVENTIONS.md
3. **Add TypeScript type definitions** - Generate .d.ts files with JSDoc
4. **Create cross-language test suite** - Verify identical behavior

### Low Priority

1. **Create migration guide** - Help developers switch between languages
2. **Add side-by-side examples** - Show equivalent code in both languages
3. **Set up CI parity checks** - Prevent future regressions

## Next Steps

1. Run `scripts/compare_apis.py` to regenerate this report after changes
2. Address high-priority gaps in both bindings
3. Create detailed method-level comparison for shared classes
4. Generate TypeScript definitions from wasm-bindgen
5. Implement cross-language test suite with golden values

---

*This report was automatically generated. Do not edit manually.*