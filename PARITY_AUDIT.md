# Rust-Python-WASM Bindings Parity Audit

**Generated:** compare_apis.py

## Executive Summary

- **Total types in Rust:** 3025
- **Total classes in Python:** 290
- **Total classes in WASM:** 229
- **In all three:** 160
- **Only in Rust:** 2783
- **Only in Python:** 20
- **Only in WASM:** 13

## Instrument Coverage

- **Expected instruments:** 38
- **In Rust:** 37 (97%)
- **In Python:** 36 (94%)
- **In WASM:** 36 (94%)
- **In all three:** 34

### Missing in Rust

```
- FiIndexTotalReturnSwap
```

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
- **In Rust:** 4 (30%)
- **In Python:** 12 (92%)
- **In WASM:** 12 (92%)
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
- SimpleCalibration
```

## Complete Type/Class Comparison

### Types/Classes in All Three

**Count:** 160

```
✓ AgencyCmo
✓ AgencyMbsPassthrough
✓ AgencyTba
✓ AggregatedMetric
✓ AmortizationSpec
✓ AmountOrScalar
✓ ApplicationReport
✓ AsianOption
✓ Autocallable
✓ BarrierOption
✓ BarrierType
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
✓ CdsOption
✓ CdsTranche
✓ CliquetOption
✓ CmoTranche
✓ CmoWaterfall
✓ CmsOption
✓ CommodityForward
✓ CommoditySwap
✓ Compounding
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
✓ CurveKind
✓ DayCount
✓ DebtInstrumentSpec
✓ Deposit
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
✓ FinancialModelSpec
✓ FinstackConfig
✓ FixedCouponSpec
✓ FloatCouponParams
✓ FloatingCouponSpec
✓ ForecastMethod
✓ ForecastSpec
✓ ForwardRateAgreement
✓ FxBarrierOption
✓ FxOption
✓ FxSpot
✓ FxSwap
✓ GaussHermiteQuadrature
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
✓ MarketScenario
✓ MetricDefinition
✓ MetricRegistry
✓ Money
✓ MonteCarloResult
✓ NettingSet
✓ NettingSetId
✓ NettingSetManager
✓ NettingSetMargin
✓ NewtonSolver
✓ NodeSpec
✓ NodeType
✓ OperationSpec
✓ OptionType
✓ PathDataset
✓ PathPoint
✓ PayReceive
✓ Percentage
✓ Period
✓ PeriodId
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
✓ Tenor
✓ TenorMatchMode
✓ TermLoan
✓ TimeRollMode
✓ TrsScheduleSpec
✓ UnitType
✓ ValuationResult
✓ VarConfig
✓ VarMethod
✓ VarResult
✓ VarianceSwap
✓ VolQuote
✓ VolSurface
✓ VolSurfaceKind
✓ VolatilityConvention
✓ VolatilityIndexFuture
✓ VolatilityIndexOption
✓ WaterfallTier
```

### In Rust and Python (missing in WASM)

**Count:** 68

```
- Adjustment
- Alignment
- AppliedAdjustment
- AveragingMethod
- Basket
- BridgeChart
- BridgeStep
- BumpMode
- BumpSpec
- BumpType
- BumpUnits
- BusinessDayConvention
- CalibrationMethod
- CovenantReport
- CovenantScope
- CreditAssessmentReport
- CurrencyScalePolicy
- CurveId
- DependencyTracer
- DependencyTree
... and 48 more
```

### In Rust and WASM (missing in Python)

**Count:** 14

```
- CDSIndex
- CompiledExpr
- CoverageTestRules
- CoverageTrigger
- EvalOpts
- EvaluationResult
- Evaluator
- ExecutionPlan
- Expr
- ExtensionRegistry
- ModelBuilder
- Registry
- Results
- WaterfallDistribution
```

### In Python and WASM (missing in Rust)

**Count:** 42

```
- AttributionMeta
- AttributionMethod
- BaseCorrelationCalibrator
- BaseCorrelationCurve
- CashflowBuilder
- ConversionPolicy
- ConversionSpec
- CreditIndexData
- CreditQuote
- Currency
- DayCountContext
- DayCountContextState
- DiscountCurve
- DiscountCurveCalibrator
- FiIndexTotalReturnSwap
- FiscalConfig
- ForwardCurve
- ForwardCurveCalibrator
- FxConfig
- FxConversionPolicy
... and 22 more
```

### Only in Rust

**Count:** 2783

```
- ABS_AUTO_STANDARD_CDR
- ABS_AUTO_STANDARD_RECOVERY
- ABS_AUTO_STANDARD_SPEED
- ABS_SERVICING_FEE_BPS
- ABS_TRUSTEE_FEE_ANNUAL
- ANNUITY_EPSILON
- ATM_MONEYNESS
- AbsChargeOffCalculator
- AbsCreditEnhancementCalculator
- AbsDelinquencyCalculator
- AbsExcessSpreadCalculator
- AbsSpeedCalculator
- AccountType
- AccrualConfig
- AccrualMethod
- AccruedCalculator
- AccruedInterestCalculator
- AdjustmentCap
- AdjustmentValue
- AgencyCmoDiscountingPricer
... and 2763 more
```

### Only in Python

**Count:** 20

```
- AntiDilutionPolicy
- BondBuilder
- CdsIndex
- CdsPayReceive
- ConversionEvent
- CreditCurvesAttribution
- CreditRating
- DebtSummaryReport
- DividendAdjustment
- EcfSweepSpec
- Frequency
- InterestRateSwapBuilder
- NormalizationEngine;
- PikToggleSpec
- RateBounds
- RatingFactorTable
- RatingLabel
- RatingNotch
- SimpleRng
- ValidationMode
```

### Only in WASM

**Count:** 13

```
- EquityUnderlying
- ExtensionMetadata
- ExtensionResult
- ExtensionStatus
- FsDate
- IndexUnderlying
- MonteCarloPathGenerator
- PricingRequest
- TrsFinancingLegSpec
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
2. **Add 2 missing instruments to WASM:** Basket, StructuredCredit
3. **Add 2 missing instruments to Python:** CDSIndex, StructuredCredit
4. **Complete calibration API in Rust:** 9 types missing
5. **Complete calibration API in WASM:** 1 types missing
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