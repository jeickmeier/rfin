#![allow(clippy::module_inception)]

use wasm_bindgen::prelude::*;

mod core;
mod genui;
mod portfolio;
mod scenarios;
mod statements;
mod utils;
mod valuations;

pub use core::cashflow::{JsCFKind as CFKind, JsCashFlow as CashFlow};
pub use core::config::{JsFinstackConfig as FinstackConfig, JsRoundingMode as RoundingMode};
pub use core::currency::JsCurrency as Currency;
pub use core::dates::add_months as addMonths;
pub use core::dates::available_calendar_codes as availableCalendarCodes;
pub use core::dates::available_calendars as availableCalendars;
pub use core::dates::build_fiscal_periods as buildFiscalPeriods;
pub use core::dates::build_periods as buildPeriods;
pub use core::dates::business_day_convention_from_name as businessDayConventionFromName;
pub use core::dates::business_day_convention_name as businessDayConventionName;
pub use core::dates::date_to_days_since_epoch as dateToDaysSinceEpoch;
pub use core::dates::days_in_month as daysInMonth;
pub use core::dates::days_since_epoch_to_date as daysSinceEpochToDate;
pub use core::dates::get_calendar as getCalendar;
pub use core::dates::is_leap_year as isLeapYear;
pub use core::dates::last_day_of_month as lastDayOfMonth;
pub use core::dates::next_cds_date as nextCdsDate;
pub use core::dates::next_equity_option_expiry as nextEquityOptionExpiry;
pub use core::dates::next_imm as nextImm;
pub use core::dates::next_imm_option_expiry as nextImmOptionExpiry;
pub use core::dates::{
    adjust, BusinessDayConvention, Calendar, DayCount, DayCountContext, DayCountContextState,
    FiscalConfig, Frequency, FsDate, Period, PeriodId, PeriodPlan, Schedule, ScheduleBuilder,
    ScheduleSpec, StubKind, Tenor,
};
pub use core::dates::{
    imm_option_expiry as immOptionExpiry, third_friday as thirdFriday,
    third_wednesday as thirdWednesday,
};
pub use core::expr::{
    JsBinOp as BinOp, JsCompiledExpr as CompiledExpr, JsEvalOpts as EvalOpts,
    JsEvaluationResult as EvaluationResult, JsExecutionPlan as ExecutionPlan, JsExpr as Expr,
    JsFunction as Function, JsUnaryOp as UnaryOp,
};
pub use core::market_data::{
    atm_moneyness as atmMoneyness, default_vol_expiry as defaultVolExpiry,
    measureBucketedDiscountShift, measureCorrelationShift, measureDiscountCurveShift,
    measureFxShift, measureHazardCurveShift, measureInflationCurveShift, measureScalarShift,
    measureVolSurfaceShift, standard_tenors as standardTenors, BaseCorrelationCurve, BumpMode,
    BumpSpec, BumpType, BumpUnits, CreditIndexData, CurveKind, DiscountCurve, DividendEvent,
    DividendSchedule, DividendScheduleBuilder, ForwardCurve, FxConfig, FxConversionPolicy,
    FxMatrix, FxRateResult, HazardCurve, InflationCurve, MarketBump, MarketContext, MarketScalar,
    ScalarTimeSeries, SeriesInterpolation, TenorSamplingMethod, VolSurface,
};
pub use core::math::{
    // Integration
    adaptiveSimpson,
    // Linear Algebra
    applyCorrelation,
    // Distributions
    binomialDistribution,
    binomialProbability,
    // Random
    boxMullerTransform,
    buildCorrelationMatrix,
    chiSquaredCdf,
    chiSquaredPdf,
    chiSquaredQuantile,
    choleskyDecomposition,
    // Statistics
    correlation,
    // Probability
    correlationBounds,
    covariance,
    // Special Functions
    erf,
    exponentialCdf,
    exponentialPdf,
    exponentialQuantile,
    gaussLegendreIntegrate,
    gaussLegendreIntegrateAdaptive,
    gaussLegendreIntegrateComposite,
    jointProbabilities,
    // Summation
    kahanSum,
    logBinomialCoefficient,
    logFactorial,
    lognormalCdf,
    lognormalPdf,
    lognormalQuantile,
    mean,
    neumaierSum,
    normCdf,
    normInvCdf,
    normPdf,
    simpsonRule,
    studentTCdf,
    studentTInvCdf,
    trapezoidalRule,
    validateCorrelationMatrix,
    variance,
    // Solvers
    BrentSolver,
    CorrelatedBernoulliDist,
    GaussHermiteQuadrature,
    NewtonSolver,
    Rng,
    SumAccumulator,
};
pub use core::money::JsMoney as Money;
pub use core::types::{
    CurveId, IndexId, InstrumentId, JsBps as Bps, JsCreditRating as CreditRating,
    JsNotchedRating as NotchedRating, JsPercentage as Percentage, JsRate as Rate,
    JsRatingNotch as RatingNotch, PriceId, UnderlyingId,
};
pub use core::volatility::{
    convert_atm_volatility_js as convertAtmVolatility,
    JsVolatilityConvention as VolatilityConvention,
};
pub use valuations::calibration::{
    JsCalibrationConfig as CalibrationConfig, JsCalibrationMethod as CalibrationMethod,
    JsCalibrationReport as CalibrationReport, JsCdsTrancheQuote as CdsTrancheQuote,
    JsCreditQuote as CreditQuote, JsInflationQuote as InflationQuote, JsMarketQuote as MarketQuote,
    JsRateBounds as RateBounds, JsRateBoundsPolicy as RateBoundsPolicy, JsRatesQuote as RatesQuote,
    JsResidualWeightingScheme as ResidualWeightingScheme,
    JsSABRCalibrationDerivatives as SABRCalibrationDerivatives, JsSABRMarketData as SABRMarketData,
    JsSABRModelParams as SABRModelParams, JsSolverKind as SolverKind,
    JsValidationConfig as ValidationConfig, JsValidationMode as ValidationMode,
    JsVolQuote as VolQuote,
};
// Validation functions
pub use valuations::calibration::{
    validate_discount_curve as validateDiscountCurve,
    validate_forward_curve as validateForwardCurve, validate_hazard_curve as validateHazardCurve,
    validate_inflation_curve as validateInflationCurve,
    validate_market_context as validateMarketContext, validate_vol_surface as validateVolSurface,
};
pub use valuations::cashflow::{
    CashFlowSchedule, CashflowBuilder, CouponType, FixedCouponSpec, FloatCouponParams,
    FloatingCouponSpec, JsAmortizationSpec as AmortizationSpec, ScheduleParams,
};
pub use valuations::metrics::{JsMetricId as MetricId, JsMetricRegistry as MetricRegistry};
// Instruments and their helper types
pub use valuations::instruments::{
    evaluate_dcf_wasm as evaluateDcf, AsianOption, Autocallable, AveragingMethod, BarrierOption,
    BasisSwap, Basket, Bond, BondFuture, BondFutureSpecs, CDSIndex, CdsOption, CdsTranche,
    CliquetOption, CmsOption, CommodityOption, ConvertibleBond, CoverageTestRules, CoverageTrigger,
    CreditDefaultSwap, Deposit, Equity, EquityFutureSpecs, EquityIndexFuture, EquityOption,
    EquityTotalReturnSwap, FiIndexTotalReturnSwap, ForwardRateAgreement, FuturePosition,
    FxBarrierOption, FxForward, FxOption, FxSpot, FxSwap, FxVarianceSwap, InflationCapFloor,
    InflationCapFloorType, InflationLinkedBond, InflationSwap, InterestRateFuture,
    InterestRateOption, InterestRateSwap, LegSide, LookbackOption, LookbackType, Ndf,
    NotionalExchange, PayReceiveInflation, Pool, PrivateMarketsFund, QuantoOption, RangeAccrual,
    RealEstateAsset, RealEstateValuationMethod, RealizedVarMethod, Repo, RevolvingCredit,
    StructuredCredit, Swaption, TermLoan, TrancheStructure, VarianceSwap, VarianceSwapSide,
    WaterfallDistribution, WaterfallEngine, XccySwap, XccySwapLeg, YoYInflationSwap,
};
pub use valuations::performance::{
    calculate_npv_wasm as calculateNpv, irr_periodic_wasm as irrPeriodic, xirr_wasm as xirr,
};
pub use valuations::pricer::{
    create_standard_registry_js as createStandardRegistry, JsPricerRegistry as PricerRegistry,
};
pub use valuations::results::JsValuationResult as ValuationResult;
// Note: ResultsMeta already exported from statements evaluator
// Using valuations::results::JsResultsMeta for ValuationResult.meta
pub use valuations::results::JsResultsMeta as ValuationResultsMeta;

// Covenants forecasting
pub use valuations::covenants::{
    forecast_covenant as forecastCovenant, JsCovenant as Covenant,
    JsCovenantForecast as CovenantForecast, JsCovenantForecastConfig as CovenantForecastConfig,
    JsCovenantSpec as CovenantSpec, JsCovenantType as CovenantType,
};

// Monte Carlo path generation (now under common::mc)
pub use valuations::common::mc::{
    JsMonteCarloPathGenerator as MonteCarloPathGenerator, JsMonteCarloResult as MonteCarloResult,
    JsPathDataset as PathDataset, JsPathPoint as PathPoint, JsProcessParams as ProcessParams,
    JsSimulatedPath as SimulatedPath,
};

// DataFrame conversion
pub use valuations::dataframe::{
    results_to_json_wasm as resultsToJson, results_to_rows_wasm as resultsToRows,
};

// Common parameter types
pub use valuations::common::parameters::{
    JsBarrierType as BarrierType, JsExerciseStyle as ExerciseStyle, JsOptionType as OptionType,
    JsPayReceive as PayReceive, JsSettlementType as SettlementType,
};

// Attribution helpers
pub use valuations::attribution::WasmAttributionMethod as AttributionMethod;

// Risk analysis functions
pub use valuations::risk::{
    calculate_var_js as calculateVar, cs01_ladder as cs01Ladder, krd_dv01_ladder as krdDv01Ladder,
    JsMarketHistory as MarketHistory, JsMarketScenario as MarketScenario,
    JsRiskFactorShift as RiskFactorShift, JsRiskFactorType as RiskFactorType,
    JsVarConfig as VarConfig, JsVarMethod as VarMethod, JsVarResult as VarResult,
};

// Margin and collateral management
pub use valuations::margin::{
    JsClearingStatus as ClearingStatus, JsCsaSpec as CsaSpec, JsImMethodology as ImMethodology,
    JsImParameters as ImParameters, JsMarginCallTiming as MarginCallTiming,
    JsMarginTenor as MarginTenor, JsVmCalculator as VmCalculator, JsVmParameters as VmParameters,
    JsVmResult as VmResult,
};

// Market conventions registry
pub use valuations::conventions::{
    JsCdsConventionKey as CdsConventionKey, JsCdsConventions as CdsConventions,
    JsCdsDocClause as CdsDocClause, JsConventionRegistry as ConventionRegistry,
    JsInflationSwapConventions as InflationSwapConventions,
    JsIrFutureConventions as IrFutureConventions, JsOptionConventions as OptionConventions,
    JsRateIndexConventions as RateIndexConventions, JsRateIndexKind as RateIndexKind,
    JsSwaptionConventions as SwaptionConventions,
};

pub use genui::*;

// Statements exports
pub use statements::{
    JsAdjustment as Adjustment, JsAmountOrScalar as AmountOrScalar,
    JsAppliedAdjustment as AppliedAdjustment, JsCapitalStructureSpec as CapitalStructureSpec,
    JsCorkscrewExtension as CorkscrewExtension,
    JsCreditScorecardExtension as CreditScorecardExtension,
    JsDebtInstrumentSpec as DebtInstrumentSpec, JsDependencyTree as DependencyTree,
    JsEvaluator as Evaluator, JsExtensionMetadata as ExtensionMetadata,
    JsExtensionRegistry as ExtensionRegistry, JsExtensionResult as ExtensionResult,
    JsExtensionStatus as ExtensionStatus, JsFinancialModelSpec as FinancialModelSpec,
    JsForecastMethod as ForecastMethod, JsForecastSpec as ForecastSpec,
    JsMetricDefinition as MetricDefinition, JsMetricRegistry as StatementsMetricRegistry,
    JsModelBuilder as ModelBuilder, JsNodeSpec as NodeSpec, JsNodeType as NodeType,
    JsNormalizationConfig as NormalizationConfig, JsNormalizationEngine as NormalizationEngine,
    JsNormalizationResult as NormalizationResult, JsRegistry as Registry, JsResults as Results,
    JsResultsMeta as ResultsMeta, JsSeasonalMode as SeasonalMode, JsUnitType as UnitType,
};

// Statements analysis functions
pub use statements::{
    all_dependencies as allDependencies, dependency_tree as dependencyTree, dependents,
    direct_dependencies as directDependencies,
    render_dependency_tree_ascii as renderDependencyTreeAscii,
};

// Scenarios exports
pub use scenarios::{
    JsApplicationReport as ApplicationReport, JsCompounding as Compounding,
    JsCurveKind as ScenarioCurveKind, JsExecutionContext as ExecutionContext,
    JsOperationSpec as OperationSpec, JsRateBindingSpec as RateBindingSpec,
    JsRollForwardReport as RollForwardReport, JsScenarioEngine as ScenarioEngine,
    JsScenarioSpec as ScenarioSpec, JsTenorMatchMode as TenorMatchMode,
    JsTimeRollMode as TimeRollMode, JsVolSurfaceKind as VolSurfaceKind,
};

// Portfolio exports
pub use portfolio::{
    js_aggregate_by_attribute as aggregateByAttribute,
    js_aggregate_cashflows as aggregateCashflows, js_aggregate_metrics as aggregateMetrics,
    js_attribute_portfolio_pnl as attributePortfolioPnl,
    js_cashflows_to_base_by_period as cashflowsToBaseByPeriod,
    js_collapse_cashflows_to_base_by_date as collapseCashflowsToBaseByDate,
    js_create_position_from_bond as createPositionFromBond,
    js_create_position_from_deposit as createPositionFromDeposit,
    js_group_by_attribute as groupByAttribute, js_is_summable as isSummable,
    js_optimize_max_yield_with_ccc_limit as optimizeMaxYieldWithCccLimit,
    js_value_portfolio as valuePortfolio,
    js_value_portfolio_with_options as valuePortfolioWithOptions,
    JsAggregatedMetric as AggregatedMetric, JsEntity as Entity, JsNettingSet as NettingSet,
    JsNettingSetId as NettingSetId, JsNettingSetManager as NettingSetManager,
    JsNettingSetMargin as NettingSetMargin, JsPnlAttribution as PnlAttribution,
    JsPortfolio as Portfolio, JsPortfolioAttribution as PortfolioAttribution,
    JsPortfolioBuilder as PortfolioBuilder, JsPortfolioCashflowBuckets as PortfolioCashflowBuckets,
    JsPortfolioCashflows as PortfolioCashflows,
    JsPortfolioMarginAggregator as PortfolioMarginAggregator,
    JsPortfolioMarginResult as PortfolioMarginResult, JsPortfolioMetrics as PortfolioMetrics,
    JsPortfolioResults as PortfolioResults, JsPortfolioValuation as PortfolioValuation,
    JsPortfolioValuationOptions as PortfolioValuationOptions, JsPosition as Position,
    JsPositionUnit as PositionUnit, JsPositionValue as PositionValue,
};

#[cfg(feature = "scenarios")]
pub use portfolio::{js_apply_and_revalue as applyAndRevalue, js_apply_scenario as applyScenario};

#[cfg(feature = "console_error_panic_hook")]
fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    init_panic_hook();
}
