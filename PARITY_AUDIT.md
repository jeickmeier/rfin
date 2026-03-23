# Rust-Python-WASM Bindings Parity Audit

**Generated:** compare_apis.py

## Executive Summary

- **Total types in Rust:** 2051
- **Total classes in Python:** 395
- **Total classes in WASM:** 341
- **In all three:** 215
- **Only in Rust:** 1700
- **Only in Python:** 35
- **Only in WASM:** 39

## Instrument Coverage

- **Expected instruments:** 38
- **In Rust:** 35 (92%)
- **In Python:** 34 (89%)
- **In WASM:** 38 (100%)
- **In all three:** 33

### Missing in Rust

```
- CdsOption
- CdsTranche
- FiIndexTotalReturnSwap
```

### Missing in Python

```
- CDSIndex
- CdsOption
- CdsTranche
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

**Count:** 215

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
✓ BaseCorrelationCurve
✓ BasisSwap
✓ BasisSwapLeg
✓ Basket
✓ Bond
✓ BondFuture
✓ BondFutureBuilder
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
✓ CreditIndexData
✓ CreditScorecardExtension
✓ CsaSpec
✓ CurveKind
✓ DayCount
✓ DebtInstrumentSpec
✓ DependencyTree
✓ Deposit
✓ DiscountCurve
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
✓ FxVarianceSwapBuilder
✓ GaussHermiteQuadrature
✓ HazardCurve
✓ ImMethodology
✓ ImParameters
✓ InflationCapFloor
✓ InflationCapFloorBuilder
✓ InflationCurve
✓ InflationLinkedBond
✓ InflationQuote
✓ InflationSwap
✓ InflationSwapBuilder
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
✓ MarketScalar
✓ MarketScenario
✓ MetricDefinition
✓ MetricId
✓ MetricRegistry
✓ ModelParamsAttribution
✓ Money
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
✓ VolatilityIndexCurve
✓ VolatilityIndexFuture
✓ VolatilityIndexOption
✓ WaterfallTier
```

### In Rust and Python (missing in WASM)

**Count:** 97

```
- AccountType
- Alignment
- AntiDilutionPolicy
- AveragingMethod
- Book
- BookId
- BridgeChart
- BridgeStep
- BumpMode
- BumpRequest
- BumpUnits
- BusinessDayConvention
- CDSOption
- CDSTranche
- CDSTrancheBuildOverrides
- CDSTrancheQuote
- CandidatePosition
- CdsQuote
- Constraint
- ConversionEvent
... and 77 more
```

### In Rust and WASM (missing in Python)

**Count:** 39

```
- AsianOptionBuilder
- BarrierOptionBuilder
- CDSIndex
- ClearingStatus
- CompiledExpr
- CoverageTestRules
- CoverageTrigger
- CreditDefaultSwapBuilder
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
... and 19 more
```

### In Python and WASM (missing in Rust)

**Count:** 48

```
- BasisSwapBuilder
- BondBuilder
- CashflowBuilder
- CommodityForwardBuilder
- CommodityOptionBuilder
- CommoditySwapBuilder
- ConvertibleBondBuilder
- CreditQuote
- Currency
- DayCountContext
- DayCountContextState
- DepositBuilder
- EquityBuilder
- EquityIndexFutureBuilder
- EquityOptionBuilder
- EquityTotalReturnSwapBuilder
- FiIndexTotalReturnSwap
- FiIndexTotalReturnSwapBuilder
- ForwardRateAgreementBuilder
- Frequency
... and 28 more
```

### Only in Rust

**Count:** 1700

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
- AVERAGE_DAYS_PER_YEAR
- AbsCreditEnhancementCalculator
- AbsDelinquencyCalculator
... and 1680 more
```

### Only in Python

**Count:** 35

```
- AgencyCmoBuilder
- AgencyMbsPassthroughBuilder
- AgencyTbaBuilder
- AmericanCall
- AmericanPut
- BaseCorrelationCalibrator
- BuildCtx
- BuiltInstrument
- CDSOptionBuilder
- CDSTrancheBuilder
- CdsIndex
- CdsIndexBuilder
- CdsPayReceive
- CrossCurrencySwap
- CrossCurrencySwapBuilder
- DebtSummaryReport
- DiscountCurveCalibrator
- DollarRollBuilder
- EcfSweepSpec
- EnhancedMonteCarloResult
... and 15 more
```

### Only in WASM

**Count:** 39

```
- AutocallableBuilder
- BasketBuilder
- CDSIndexBuilder
- CdsOption
- CdsOptionBuilder
- CdsTranche
- CdsTrancheBuilder
- CdsTrancheQuote
- CliquetOptionBuilder
- CmsOptionBuilder
- CorrelatedBernoulliDist
- EquityUnderlying
- FsDate
- FuturePosition
- FxBarrierOptionBuilder
- FxForwardBuilder
- IndexUnderlying
- LeveredRealEstateEquity
- LeveredRealEstateEquityBuilder
- MonteCarloPathGenerator
... and 19 more
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

1. **Add 3 missing instruments to Rust:** CdsOption, CdsTranche, FiIndexTotalReturnSwap
3. **Add 4 missing instruments to Python:** CDSIndex, CdsOption, CdsTranche...
4. **Complete calibration API in Rust:** 9 types missing
5. **Complete calibration API in WASM:** 7 types missing
6. **Complete calibration API in Python:** 1 types missing

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
