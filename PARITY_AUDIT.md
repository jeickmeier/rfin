# Python-WASM Bindings Parity Audit

**Generated:** compare_apis.py

## Executive Summary

- **Classes in both bindings:** 202
- **Only in Python:** 88
- **Only in WASM:** 27
- **Total unique classes:** 317

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
- **In Python:** 12 (92%)
- **In WASM:** 12 (92%)

### Missing in Python

```
- SimpleCalibration
```

### Missing in WASM

```
- SimpleCalibration
```

## Complete Class Comparison

### Classes in Both Bindings

**Count:** 202

```
✓ AgencyCmo
✓ AgencyMbsPassthrough
✓ AgencyTba
✓ AggregatedMetric
✓ AmortizationSpec
✓ AmountOrScalar
✓ ApplicationReport
✓ AsianOption
✓ AttributionMeta
✓ AttributionMethod
✓ Autocallable
✓ BarrierOption
✓ BarrierType
✓ BaseCorrelationCalibrator
✓ BaseCorrelationCurve
✓ BasisSwap
✓ BasisSwapLeg
✓ Bond
✓ Bps
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
✓ CmoTranche
✓ CmoWaterfall
✓ CmsOption
✓ CommodityForward
✓ CommoditySwap
✓ Compounding
✓ ConversionPolicy
✓ ConversionSpec
✓ ConvertibleBond
✓ CorkscrewExtension
✓ CouponType
✓ Covenant
✓ CovenantForecast
✓ CovenantForecastConfig
✓ CovenantSpec
✓ CovenantType
✓ CreditDefaultSwap
✓ CreditIndexData
✓ CreditQuote
✓ CreditScorecardExtension
✓ Currency
✓ CurveKind
✓ DayCount
✓ DayCountContext
✓ DayCountContextState
✓ DebtInstrumentSpec
✓ Deposit
✓ DiscountCurve
✓ DiscountCurveCalibrator
✓ DividendEvent
✓ DividendSchedule
✓ DividendScheduleBuilder
✓ DollarRoll
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
✓ MarketHistory
✓ MarketQuote
✓ MarketScalar
✓ MarketScenario
✓ MetricDefinition
✓ MetricId
✓ MetricRegistry
✓ ModelParamsAttribution
✓ Money
✓ MonteCarloResult
✓ NettingSet
✓ NettingSetId
✓ NettingSetManager
✓ NettingSetMargin
✓ NewtonSolver
✓ NodeSpec
✓ NodeType
✓ NotchedRating
✓ OperationSpec
✓ OptionType
✓ PathDataset
✓ PathPoint
✓ PayReceive
✓ Percentage
✓ Period
✓ PeriodId
✓ PeriodPlan
✓ PnlAttribution
✓ Portfolio
✓ PortfolioAttribution
✓ PortfolioBuilder
✓ PortfolioCashflowBuckets
✓ PortfolioCashflows
✓ PortfolioMarginAggregator
✓ PortfolioMarginResult
✓ PortfolioMetrics
✓ PortfolioResults
✓ PortfolioValuation
✓ PortfolioValuationOptions
✓ Position
✓ PositionUnit
✓ PositionValue
✓ PricerRegistry
✓ PrivateMarketsFund
✓ ProcessParams
✓ QuantoOption
✓ RangeAccrual
✓ Rate
✓ RateBindingSpec
✓ RatesCurvesAttribution
✓ RatesQuote
✓ Repo
✓ RepoCollateral
✓ ResultsMeta
✓ RevolvingCredit
✓ RiskFactorShift
✓ RiskFactorType
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
✓ ScheduleSpec
✓ SeasonalMode
✓ SeriesInterpolation
✓ SettlementType
✓ SimulatedPath
✓ SolverKind
✓ StubKind
✓ Swaption
✓ Tenor
✓ TenorMatchMode
✓ TermLoan
✓ TimeRollMode
✓ TrsScheduleSpec
✓ UnitType
✓ ValidationConfig
✓ ValuationResult
✓ VarConfig
✓ VarMethod
✓ VarResult
✓ VarianceSwap
✓ VolQuote
✓ VolSurface
✓ VolSurfaceCalibrator
✓ VolSurfaceKind
✓ VolatilityConvention
✓ VolatilityIndexCurve
✓ VolatilityIndexFuture
✓ VolatilityIndexOption
✓ WaterfallTier
```

### Classes Only in Python

**Count:** 88

```
- Adjustment
- Alignment
- AntiDilutionPolicy
- AppliedAdjustment
- AveragingMethod
- Basket
- BondBuilder
- BridgeChart
- BridgeStep
- BumpMode
- BumpSpec
- BumpType
- BumpUnits
- BusinessDayConvention
- CalibrationMethod
- CdsIndex
- CdsPayReceive
- ConversionEvent
- CovenantReport
- CovenantScope
- CreditAssessmentReport
- CreditCurvesAttribution
- CreditRating
- CurrencyScalePolicy
- CurveId
- DebtSummaryReport
- DependencyTracer
- DependencyTree
- DividendAdjustment
- EcfSweepSpec
- EnhancedMonteCarloResult
- EquityUnderlyingParams
- ExplainOpts
- Explanation
- ExplanationStep
- ExplanationTrace
- FeeBase
- FeeSpec
- FinancingLegSpec
- FixedWindow
- FloatWindow
- FormulaExplainer
- Frequency
- FutureBreach
- IndexId
- IndexUnderlyingParams
- InstrumentId
- InterestRateSwapBuilder
- LevenbergMarquardtSolver
- LookbackType
- MarketBump
- NormalizationConfig
- NormalizationEngine;
- NormalizationResult
- NumericMode
- PLSummaryReport
- PacCollar
- PathResult
- PikToggleSpec
- PriceId
- RateBounds
- RatingFactorTable
- RatingLabel
- RatingNotch
- RealizedVarMethod
- RoundingContext
- RoundingPolicy
- ScenarioDefinition
- ScenarioDiff
- ScenarioResults
- ScenarioSet
- SimpleRng
- SpringingCondition
- TableBuilder
- TenorSamplingMethod
- TenorUnit
- Thirty360Convention
- ThreeFactorPathData
- TraceEntry
- TrsSide
- UnderlyingId
- ValidationMode
- VarianceAnalyzer
- VarianceConfig
- VarianceReport
- VarianceRow
- WaterfallSpec
- ZeroKind
```

### Classes Only in WASM

**Count:** 27

```
- CDSIndex
- CompiledExpr
- CoverageTestRules
- CoverageTrigger
- EquityUnderlying
- EvalOpts
- EvaluationResult
- Evaluator
- ExecutionPlan
- Expr
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
- WaterfallDistribution
- WaterfallEngine
- applyAndRevalue
- applyScenario
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
3. **Complete calibration API in WASM:** 1 types missing
4. **Complete calibration API in Python:** 1 types missing

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