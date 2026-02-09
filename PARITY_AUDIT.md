# Rust-Python-WASM Bindings Parity Audit

**Generated:** compare_apis.py

## Executive Summary

- **Total types in Rust:** 2320
- **Total classes in Python:** 366
- **Total classes in WASM:** 286
- **In all three:** 214
- **Only in Rust:** 1966
- **Only in Python:** 24
- **Only in WASM:** 14

## Instrument Coverage

- **Expected instruments:** 38
- **In Rust:** 37 (97%)
- **In Python:** 36 (94%)
- **In WASM:** 38 (100%)
- **In all three:** 35

### Missing in Rust

```
- FiIndexTotalReturnSwap
```

### Missing in Python

```
- CDSIndex
- StructuredCredit
```

## Calibration API Coverage

- **Expected calibration types:** 13
- **In Rust:** 4 (30%)
- **In Python:** 12 (92%)
- **In WASM:** 6 (46%)
- **In all three:** 4

### Missing in Rust

```
- BaseCorrelationCalibrator
- CreditQuote
- DiscountCurveCalibrator
- ForwardCurveCalibrator
- HazardCurveCalibrator
- InflationCurveCalibrator
- RatesQuote
- SimpleCalibration
- VolSurfaceCalibrator
```

### Missing in Python

```
- SimpleCalibration
```

### Missing in WASM

```
- BaseCorrelationCalibrator
- DiscountCurveCalibrator
- ForwardCurveCalibrator
- HazardCurveCalibrator
- InflationCurveCalibrator
- SimpleCalibration
- VolSurfaceCalibrator
```

## Complete Type/Class Comparison

### Types/Classes in All Three

**Count:** 214

```
✓ Adjustment
✓ AgencyCmo
✓ AgencyMbsPassthrough
✓ AgencyTba
✓ AggregatedMetric
✓ AmortizationSpec
✓ AmountOrScalar
✓ ApplicationReport
✓ AppliedAdjustment
✓ AsianOption
✓ AttributionMeta
✓ AttributionMethod
✓ Autocallable
✓ BarrierOption
✓ BarrierType
✓ BasisSwap
✓ BasisSwapLeg
✓ Basket
✓ Bond
✓ BondFuture
✓ BondFutureSpecs
✓ Bps
✓ BrentSolver
✓ BumpSpec
✓ BumpType
✓ CFKind
✓ Calendar
✓ CalibrationConfig
✓ CalibrationMethod
✓ CalibrationReport
✓ CapitalStructureSpec
✓ CashFlow
✓ CashFlowSchedule
✓ CdsConventionKey
✓ CdsConventions
✓ CdsDocClause
✓ CdsOption
✓ CdsTranche
✓ CdsTrancheQuote
✓ CliquetOption
✓ CmoTranche
✓ CmoWaterfall
✓ CmsOption
✓ CommodityForward
✓ CommodityOption
✓ CommoditySwap
✓ Compounding
✓ ConventionRegistry
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
✓ CreditScorecardExtension
✓ CsaSpec
✓ CurveKind
✓ DayCount
✓ DebtInstrumentSpec
✓ DependencyTree
✓ Deposit
✓ DividendEvent
✓ DividendSchedule
✓ DividendScheduleBuilder
✓ DollarRoll
✓ Entity
✓ Equity
✓ EquityFutureSpecs
✓ EquityIndexFuture
✓ EquityOption
✓ EquityTotalReturnSwap
✓ ExecutionContext
✓ ExerciseStyle
✓ ExtrapolationPolicy
✓ FinancialModelSpec
✓ FinstackConfig
✓ FiscalConfig
✓ FixedCouponSpec
✓ FloatCouponParams
✓ FloatingCouponSpec
✓ ForecastMethod
✓ ForecastSpec
✓ ForwardCurve
✓ ForwardRateAgreement
✓ FxBarrierOption
✓ FxMatrix
✓ FxOption
✓ FxSpot
✓ FxSwap
✓ FxVarianceSwap
✓ GaussHermiteQuadrature
✓ HazardCurve
✓ ImMethodology
✓ ImParameters
✓ InflationCapFloor
✓ InflationCurve
✓ InflationLinkedBond
✓ InflationQuote
✓ InflationSwap
✓ InflationSwapConventions
✓ InterestRateFuture
✓ InterestRateOption
✓ InterestRateSwap
✓ InterpStyle
✓ IrFutureConventions
✓ LookbackOption
✓ MarginCallTiming
✓ MarginTenor
✓ MarketBump
✓ MarketContext
✓ MarketHistory
✓ MarketQuote
✓ MarketScenario
✓ MetricDefinition
✓ MetricId
✓ MetricRegistry
✓ ModelParamsAttribution
✓ Money
✓ MonteCarloResult
✓ Ndf
✓ NettingSet
✓ NettingSetId
✓ NettingSetManager
✓ NettingSetMargin
✓ NewtonSolver
✓ NodeSpec
✓ NodeType
✓ NormalizationConfig
✓ NormalizationResult
✓ NotchedRating
✓ OperationSpec
✓ OptionConventions
✓ OptionType
✓ PathDataset
✓ PathPoint
✓ PayReceive
✓ Percentage
✓ Period
✓ PeriodId
✓ PnlAttribution
✓ Portfolio
✓ PortfolioAttribution
✓ PortfolioBuilder
✓ PortfolioCashflowBuckets
✓ PortfolioCashflows
✓ PortfolioMarginAggregator
✓ PortfolioMarginResult
✓ PortfolioMetrics
✓ PortfolioResult
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
✓ RateBounds
✓ RateIndexConventions
✓ RateIndexKind
✓ RatesCurvesAttribution
✓ RealEstateAsset
✓ Repo
✓ ResultsMeta
✓ RevolvingCredit
✓ RiskFactorShift
✓ RiskFactorType
✓ RollForwardReport
✓ RoundingMode
✓ SABRCalibrationDerivatives
✓ SABRMarketData
✓ ScenarioEngine
✓ ScenarioSpec
✓ Schedule
✓ ScheduleBuilder
✓ ScheduleParams
✓ ScheduleSpec
✓ SeasonalMode
✓ SettlementType
✓ SimulatedPath
✓ StubKind
✓ Swaption
✓ SwaptionConventions
✓ Tenor
✓ TenorMatchMode
✓ TenorSamplingMethod
✓ TermLoan
✓ TimeRollMode
✓ TrsScheduleSpec
✓ UnitType
✓ ValidationConfig
✓ ValidationMode
✓ ValuationResult
✓ VarConfig
✓ VarMethod
✓ VarResult
✓ VarianceSwap
✓ VmParameters
✓ VolQuote
✓ VolSurface
✓ VolSurfaceKind
✓ VolatilityConvention
✓ VolatilityIndexFuture
✓ VolatilityIndexOption
✓ WaterfallTier
```

### In Rust and Python (missing in WASM)

**Count:** 105

```
- AccountType
- Alignment
- AmericanCall
- AmericanPut
- AntiDilutionPolicy
- AveragingMethod
- BondFutureBuilder
- Book
- BookId
- BridgeChart
- BridgeStep
- BumpMode
- BumpRequest
- BumpUnits
- BusinessDayConvention
- CandidatePosition
- CdsQuote
- CdsTrancheBuildOverrides
- Constraint
- ConversionEvent
... and 85 more
```

### In Rust and WASM (missing in Python)

**Count:** 35

```
- CDSIndex
- ClearingStatus
- CompiledExpr
- CoverageTestRules
- CoverageTrigger
- EvalOpts
- EvaluationResult
- Evaluator
- ExecutionPlan
- Expr
- ExtensionMetadata
- ExtensionRegistry
- ExtensionResult
- ExtensionStatus
- FxForward
- InflationCapFloorType
- LegSide
- ModelBuilder
- NormalizationEngine
- NotionalExchange
... and 15 more
```

### In Python and WASM (missing in Rust)

**Count:** 23

```
- BaseCorrelationCurve
- CashflowBuilder
- CreditIndexData
- CreditQuote
- Currency
- DayCountContext
- DayCountContextState
- DiscountCurve
- FiIndexTotalReturnSwap
- Frequency
- FxConfig
- FxConversionPolicy
- FxRateResult
- MarketScalar
- PeriodPlan
- RatesQuote
- RepoCollateral
- Rng
- SABRModelParams
- ScalarTimeSeries
... and 3 more
```

### Only in Rust

**Count:** 1966

```
- // Configuration CoverageTestConfig
- // Deal-specific metrics AbsChargeOffCalculator
- // Enums AssetType
- // Main instrument StructuredCredit
- // Metadata ConcentrationCheckResult
- // Pool metrics WamCalculator
- // Pool types calculate_pool_stats
- // Pricing metrics AccruedCalculator
- // Reinvestment ReinvestmentManager
- // Result types TrancheCashflows
- // Risk metrics MacaulayDurationCalculator
- // Stochastic specs CorrelationStructure
- // Tranche types CoverageTrigger
- // Waterfall types AllocationMode
- ABS_SERVICING_FEE_BPS
- ATM_MONEYNESS
- ATTRIBUTION_SCHEMA_V1
- AbsCreditEnhancementCalculator
- AbsDelinquencyCalculator
- AbsExcessSpreadCalculator
... and 1946 more
```

### Only in Python

**Count:** 24

```
- BaseCorrelationCalibrator
- BondBuilder
- BuildCtx
- BuiltInstrument
- CdsIndex
- CdsPayReceive
- CrossCurrencySwap
- CrossCurrencySwapBuilder
- DebtSummaryReport
- DiscountCurveCalibrator
- EcfSweepSpec
- EquityIndexFutureBuilder
- ForwardCurveCalibrator
- FxPayReceive
- FxRealizedVarMethod
- HazardCurveCalibrator
- InflationCurveCalibrator
- InterestRateSwapBuilder
- LsmcResult
- NdfBuilder
... and 4 more
```

### Only in WASM

**Count:** 14

```
- CorrelatedBernoulliDist
- EquityUnderlying
- FsDate
- FuturePosition
- IndexUnderlying
- MonteCarloPathGenerator
- PricingRequest
- SumAccumulator
- TrsFinancingLegSpec
- VarianceSwapSide
- WasmExplanationTrace
- WaterfallEngine
- applyAndRevalue
- applyScenario
```

## Naming Convention Patterns

### Identified Patterns

| Rust | Python | WASM | Pattern |
|------|--------|------|---------|
| `build_periods` | `build_periods` | `buildPeriods` | snake_case → snake_case → camelCase |
| `from_code` | `from_code` | `fromCode` | snake_case → snake_case → camelCase |
| `next_imm` | `next_imm` | `nextImm` | snake_case → snake_case → camelCase |
| `is_actual` | `is_actual` | `isActual` | snake_case → snake_case → camelCase |
| `Currency` | `Currency` | `Currency` | PascalCase → PascalCase → PascalCase |
| `Money` | `Money` | `Money` | PascalCase → PascalCase → PascalCase |

## Recommendations

### High Priority

1. **Add 1 missing instruments to Rust:** FiIndexTotalReturnSwap
2. **Add 2 missing instruments to Python:** CDSIndex, StructuredCredit
3. **Complete calibration API in Rust:** 9 types missing
4. **Complete calibration API in WASM:** 7 types missing
5. **Complete calibration API in Python:** 1 types missing

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
