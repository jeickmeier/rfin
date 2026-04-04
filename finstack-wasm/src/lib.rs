#![deny(unsafe_code)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::panic))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! JavaScript/TypeScript bindings for the Finstack financial computation library.
//!
//! This crate exposes a **flat** JS/TS API surface (curves, instruments, pricers, scenarios)
//! backed by the Rust `finstack-*` crates.
//!
//! ## Initialization
//!
//! When using the web build (`wasm-pack --target web`), you must initialize the WASM module
//! once at application startup:
//!
//! @example
//! ```javascript
//! import init, { standardRegistry, MarketContext, FsDate } from "finstack-wasm";
//!
//! await init();
//! const registry = standardRegistry();
//! const market = new MarketContext();
//! const asOf = new FsDate(2024, 1, 2);
//! // ... build curves and instruments, then price ...
//! ```
//!
//! ## Documentation delivery
//!
//! Documentation is written in Rust doc comments and is surfaced to JS/TS consumers through the
//! generated declarations in `pkg/finstack_wasm.d.ts`.

use wasm_bindgen::prelude::*;

mod core;
mod correlation;
mod genui;
mod portfolio;
mod scenarios;
mod statements;
mod utils;
mod valuations;

// Analytics exports
pub use core::analytics::benchmark::{
    batting_average, calc_beta, capture_ratio, down_capture, greeks_js, information_ratio,
    m_squared, m_squared_from_returns, multi_factor_greeks, r_squared, tracking_error,
    treynor_ratio, up_capture,
};
pub use core::analytics::consecutive::{count_consecutive_above, count_consecutive_below};
pub use core::analytics::drawdown::{
    average_drawdown, burke_ratio, calmar_ratio, calmar_ratio_from_returns, cdar, martin_ratio,
    martin_ratio_from_returns, max_drawdown, max_drawdown_from_returns, pain_index, pain_ratio,
    pain_ratio_from_returns, recovery_factor, recovery_factor_from_returns, sterling_ratio,
    sterling_ratio_from_returns, to_drawdown_series, ulcer_index,
};
pub use core::analytics::lookback::{lookback_returns, mtd_select, qtd_select, ytd_select};
pub use core::analytics::performance::JsPerformance as Performance;
pub use core::analytics::returns::{
    compounded_cumulative_returns, compounded_total_return, convert_to_prices, excess_returns,
    rebase_prices, simple_returns,
};
pub use core::analytics::risk_metrics::{
    cagr_from_periods, cornish_fisher_var, downside_deviation, expected_shortfall, gain_to_pain,
    geometric_mean_return, historical_var, mean_return, modified_sharpe, omega_ratio,
    parametric_var, returns_kurtosis, returns_skewness, returns_volatility, sharpe_ratio,
    sortino_ratio, tail_ratio,
};

pub use core::cashflow::{JsCFKind as CFKind, JsCashFlow as CashFlow};
pub use core::config::{
    JsFinstackConfig as FinstackConfig, JsRoundingMode as RoundingMode,
    JsToleranceConfig as ToleranceConfig,
};
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
    addJointBusinessDays, adjustJointCalendar, canResolveCalendar,
    continuous_to_periodic as continuousToPeriodic, continuous_to_simple as continuousToSimple,
    periodic_to_continuous as periodicToContinuous, periodic_to_simple as periodicToSimple,
    rollSpotDate, simple_to_continuous as simpleToContinuous,
    simple_to_periodic as simpleToPeriodic,
};
pub use core::dates::{
    adjust, BusinessDayConvention, Calendar, CompositeCalendar, CompositeMode, DayCount,
    DayCountContext, DayCountContextState, FiscalConfig, Frequency, FsDate, Period, PeriodId,
    PeriodPlan, Schedule, ScheduleBuilder, ScheduleSpec, StubKind, Tenor,
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
pub use core::factor_model::{
    JsBumpSizeConfig as BumpSizeConfig, JsFactorCovarianceMatrix as FactorCovarianceMatrix,
    JsFactorDefinition as FactorDefinition, JsFactorId as FactorId,
    JsFactorModelConfig as FactorModelConfig, JsMarketDependency as MarketDependency,
};
pub use core::market_data::{
    atm_moneyness as atmMoneyness, default_vol_expiry as defaultVolExpiry,
    measureBucketedDiscountShift, measureCorrelationShift, measureDiscountCurveShift,
    measureFxShift, measureHazardCurveShift, measureInflationCurveShift, measureScalarShift,
    measureVolSurfaceShift, standard_tenors as standardTenors, BaseCorrelationCurve, BumpMode,
    BumpSpec, BumpType, BumpUnits, CreditIndexData, CurveKind, DiscountCurve, DividendEvent,
    DividendSchedule, DividendScheduleBuilder, FlatCurve, ForwardCurve, FxConfig,
    FxConversionPolicy, FxMatrix, FxPolicyMeta, FxQuery, FxRateResult, HazardCurve,
    HierarchyBuilder, HierarchyNode, HierarchyTarget, InflationCurve, InflationIndex,
    InflationInterpolation, InflationLag, MarketBump, MarketContext, MarketDataHierarchy,
    MarketScalar, PriceCurve, ResolutionMode, ScalarTimeSeries, SeriesInterpolation, TagFilter,
    TagPredicate, TenorSamplingMethod, VolSurface,
};
pub use core::math::{
    // Integration
    adaptiveSimpson,
    // Linear Algebra
    applyCorrelation,
    bachelierCall_vol,
    bachelierPut_vol,
    bachelierVega_vol,
    // Distributions
    binomialDistribution,
    binomialProbability,
    // Volatility pricing
    blackCall,
    blackDeltaCall,
    blackDeltaPut,
    blackGamma,
    blackPut,
    blackScholesSpotCall,
    blackScholesSpotPut,
    blackShiftedCall,
    blackShiftedPut,
    blackShiftedVega,
    blackVega,
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
    geometricAsianCall,
    impliedVolBachelier,
    impliedVolBlack,
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
    LevenbergMarquardtSolver,
    MathCompounding,
    NewtonSolver,
    Rng,
    SumAccumulator,
    TimeGrid,
};
pub use core::money::JsMoney as Money;
pub use core::types::moodys_warf_factor_js as moodysWarfFactor;
pub use core::types::{
    CalendarId, CurveId, DealId, IndexId, InstrumentId, JsAttributes as Attributes, JsBps as Bps,
    JsCreditRating as CreditRating, JsNotchedRating as NotchedRating, JsPercentage as Percentage,
    JsRate as Rate, JsRatingLabel as RatingLabel, JsRatingNotch as RatingNotch, PoolId, PriceId,
    UnderlyingId,
};
pub use core::volatility::{
    convert_atm_volatility_js as convertAtmVolatility,
    JsVolatilityConvention as VolatilityConvention,
};

// Correlation infrastructure
pub use correlation::copulas::{
    JsCopulaSpec as CopulaSpec, JsGaussianCopula as GaussianCopula,
    JsMultiFactorCopula as MultiFactorCopula,
    JsRandomFactorLoadingCopula as RandomFactorLoadingCopula, JsStudentTCopula as StudentTCopula,
};
pub use correlation::factor_models::{
    JsFactorSpec as FactorSpec, JsMultiFactorModel as MultiFactorModel,
    JsSingleFactorModel as SingleFactorModel, JsTwoFactorModel as TwoFactorModel,
};
pub use correlation::recovery::{
    JsConstantRecovery as ConstantRecovery, JsCorrelatedRecovery as CorrelatedRecovery,
    JsRecoverySpec as RecoverySpec,
};
pub use correlation::utils::{
    cholesky_decompose_correlation as choleskyDecomposeCorrelation,
    validate_correlation_matrix_strict as validateCorrelationMatrixStrict,
};

pub use valuations::calibration::{
    calibrate_hull_white as calibrateHullWhite, JsCDSTrancheQuote as CDSTrancheQuote,
    JsCalibrationConfig as CalibrationConfig, JsCalibrationMethod as CalibrationMethod,
    JsCalibrationReport as CalibrationReport, JsCreditQuote as CreditQuote,
    JsHullWhiteParams as HullWhiteParams, JsInflationQuote as InflationQuote,
    JsMarketQuote as MarketQuote, JsRateBounds as RateBounds,
    JsRateBoundsPolicy as RateBoundsPolicy, JsRatesQuote as RatesQuote,
    JsResidualWeightingScheme as ResidualWeightingScheme, JsSolverKind as SolverKind,
    JsSwapFrequency as SwapFrequency, JsSwaptionQuote as SwaptionQuote,
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
    BasisSwap, Basket, Bond, BondFuture, BondFutureSpecs, CDSIndex, CDSOption, CDSTranche,
    CliquetOption, CmsOption, CommodityOption, ConvertibleBond, CoverageTestRules, CoverageTrigger,
    CreditDefaultSwap, CreditDefaultSwapBuilder, Deposit, DepositBuilder, Equity, EquityBuilder,
    EquityFutureSpecs, EquityIndexFuture, EquityOption, EquityOptionBuilder, EquityTotalReturnSwap,
    FiIndexTotalReturnSwap, ForwardRateAgreement, FuturePosition, FxBarrierOption, FxForward,
    FxOption, FxOptionBuilder, FxSpot, FxSpotBuilder, FxSwap, FxVarianceSwap, InflationCapFloor,
    InflationCapFloorType, InflationLinkedBond, InflationSwap, InterestRateFuture,
    InterestRateOption, InterestRateSwap, InterestRateSwapBuilder, LegSide, LookbackOption,
    LookbackType, Ndf, NotionalExchange, Pool, PrivateMarketsFund, QuantoOption, RangeAccrual,
    RealEstateAsset, RealEstateValuationMethod, RealizedVarMethod, Repo, RevolvingCredit,
    StructuredCredit, Swaption, SwaptionBuilder, TermLoan, TrancheStructure, VarianceSwap,
    VarianceSwapSide, WaterfallDistribution, WaterfallEngine, XccySwap, XccySwapLeg,
    YoYInflationSwap,
};
pub use valuations::performance::{
    calculate_npv_wasm as calculateNpv, count_sign_changes_wasm as countSignChanges,
    irr_detailed_wasm as irrDetailed, irr_periodic_wasm as irrPeriodic,
    xirr_detailed_wasm as xirrDetailed, xirr_wasm as xirr, JsIrrResult as IrrResult,
};
pub use valuations::pricer::{
    create_credit_registry_js as createCreditRegistry,
    create_equity_registry_js as createEquityRegistry, create_fx_registry_js as createFxRegistry,
    create_rates_registry_js as createRatesRegistry, standard_registry_js as standardRegistry,
    JsPricerRegistry as PricerRegistry,
};
pub use valuations::results::JsValuationResult as ValuationResult;
// Note: ResultsMeta already exported from statements evaluator
// Using valuations::results::JsResultsMeta for ValuationResult.meta
pub use valuations::results::JsResultsMeta as ValuationResultsMeta;

// Covenants forecasting
pub use valuations::covenants::{
    cov_lite_covenants as covLiteCovenants, forecast_covenant as forecastCovenant,
    lbo_standard_covenants as lboStandardCovenants, JsCovenant as Covenant,
    JsCovenantBreach as CovenantBreach, JsCovenantEngine as CovenantEngine,
    JsCovenantForecast as CovenantForecast, JsCovenantForecastConfig as CovenantForecastConfig,
    JsCovenantReport as CovenantReport, JsCovenantSpec as CovenantSpec,
    JsCovenantType as CovenantType,
};

// DataFrame conversion
pub use valuations::dataframe::{
    results_to_json_wasm as resultsToJson, results_to_rows_wasm as resultsToRows,
};

// Common parameter types
pub use valuations::common::parameters::{
    JsBarrierType as BarrierType, JsExerciseStyle as ExerciseStyle, JsFixedLegSpec as FixedLegSpec,
    JsFloatLegSpec as FloatLegSpec, JsFxPair as FxPair, JsOptionType as OptionType,
    JsPayReceive as PayReceive, JsSettlementType as SettlementType,
};

// Pricing overrides
pub use valuations::common::pricing_overrides::{
    JsBumpConfig as BumpConfig, JsInstrumentPricingOverrides as InstrumentPricingOverrides,
    JsMarketQuoteOverrides as MarketQuoteOverrides,
    JsMetricPricingOverrides as MetricPricingOverrides, JsModelConfig as ModelConfig,
    JsPricingOverrides as PricingOverrides, JsScenarioPricingOverrides as ScenarioPricingOverrides,
};

// JSON schema accessors
pub use valuations::schema::{
    bond_schema as bondSchema, instrument_envelope_schema as instrumentEnvelopeSchema,
    instrument_schema as instrumentSchema, instrument_types as instrumentTypes,
    valuation_result_schema as valuationResultSchema,
};

// LSMC (Longstaff-Schwartz Monte Carlo) pricer
pub use valuations::lsmc::{
    JsAmericanCall as LsmcAmericanCall, JsAmericanPut as LsmcAmericanPut,
    JsLaguerreBasis as LsmcLaguerreBasis, JsLsmcConfig as LsmcConfig, JsLsmcPricer as LsmcPricer,
    JsLsmcResult as LsmcResult, JsPolynomialBasis as LsmcPolynomialBasis,
};

// Attribution helpers
pub use valuations::attribution::WasmAttributionMethod as AttributionMethod;
pub use valuations::attribution::{
    attribute_pnl_from_json as attributePnlFromJson, JsAttributionConfig as AttributionConfig,
    JsAttributionEnvelope as AttributionEnvelope, JsAttributionSpec as AttributionSpec,
    JsTaylorAttributionConfig as TaylorAttributionConfig,
};

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

// XVA exports
pub use valuations::xva::apply_collateral as applyXvaCollateral;
pub use valuations::xva::{
    apply_netting as applyNetting, compute_bilateral_xva as computeBilateralXva,
    compute_cva as computeCva, compute_dva as computeDva, compute_fva as computeFva,
    JsExposureProfile as ExposureProfile, JsFundingConfig as FundingConfig,
    JsXvaConfig as XvaConfig, JsXvaCsaTerms as XvaCsaTerms, JsXvaNettingSet as XvaNettingSet,
    JsXvaResult as XvaResult,
};

// Valuations factor model exports
pub use valuations::factor_model::{
    JsFactorPnlProfile as FactorPnlProfile, JsScenarioGrid as ScenarioGrid,
    JsSensitivityMatrix as SensitivityMatrix,
};

pub use genui::*;

// Statements exports
pub use statements::{
    JsAdjustment as Adjustment, JsAmountOrScalar as AmountOrScalar,
    JsAppliedAdjustment as AppliedAdjustment,
    JsCapitalStructureCashflows as CapitalStructureCashflows,
    JsCapitalStructureSpec as CapitalStructureSpec, JsCashflowBreakdown as CashflowBreakdown,
    JsCorkscrewExtension as CorkscrewExtension,
    JsCreditScorecardExtension as CreditScorecardExtension,
    JsDebtInstrumentSpec as DebtInstrumentSpec, JsDependencyTree as DependencyTree,
    JsEcfSweepSpec as EcfSweepSpec, JsEvaluator as Evaluator,
    JsExtensionMetadata as ExtensionMetadata, JsExtensionRegistry as ExtensionRegistry,
    JsExtensionResult as ExtensionResult, JsExtensionStatus as ExtensionStatus,
    JsFinancialModelSpec as FinancialModelSpec, JsForecastMethod as ForecastMethod,
    JsForecastSpec as ForecastSpec, JsFreeRentWindowSpec as FreeRentWindowSpec,
    JsLeaseGrowthConvention as LeaseGrowthConvention, JsLeaseSpec as LeaseSpec,
    JsLeaseSpecV2 as LeaseSpecV2, JsManagementFeeBase as ManagementFeeBase,
    JsManagementFeeSpec as ManagementFeeSpec, JsMetricDefinition as MetricDefinition,
    JsMetricRegistry as StatementsMetricRegistry, JsModelBuilder as ModelBuilder,
    JsMonteCarloConfig as MonteCarloConfig, JsMonteCarloResults as MonteCarloResults,
    JsNodeSpec as NodeSpec, JsNodeType as NodeType, JsNormalizationConfig as NormalizationConfig,
    JsNormalizationEngine as NormalizationEngine, JsNormalizationResult as NormalizationResult,
    JsPercentileSeries as PercentileSeries, JsPeriodDateConvention as PeriodDateConvention,
    JsPikToggleSpec as PikToggleSpec, JsPropertyTemplateNodes as PropertyTemplateNodes,
    JsRegistry as Registry, JsRenewalSpec as RenewalSpec,
    JsRentRollOutputNodes as RentRollOutputNodes, JsRentStepSpec as RentStepSpec,
    JsSeasonalMode as SeasonalMode, JsStatementResult as StatementResult,
    JsStatementResultMeta as StatementResultsMeta, JsStmtExpr as StmtExpr, JsUnitType as UnitType,
    JsWaterfallSpec as WaterfallSpec,
};

// Statements analysis functions
pub use statements::{
    all_dependencies as allDependencies, dependency_tree as dependencyTree, dependents,
    direct_dependencies as directDependencies,
    render_dependency_tree_ascii as renderDependencyTreeAscii,
};

// Statements DSL functions
pub use statements::{
    compile_formula as compileFormula, parse_and_compile as parseAndCompile,
    parse_formula as parseFormula,
};

// Statements forecast functions
pub use statements::{
    apply_forecast as applyForecast, apply_override as applyOverride, curve_pct as curvePct,
    forward_fill as forwardFill, growth_pct as growthPct, lognormal_forecast as lognormalForecast,
    normal_forecast as normalForecast, seasonal_forecast as seasonalForecast,
    timeseries_forecast as timeseriesForecast,
};

// Statements capital structure functions
pub use statements::aggregate_instrument_cashflows as aggregateInstrumentCashflows;

// Statements evaluator cashflow export
pub use statements::node_to_dated_schedule as nodeToDatedSchedule;

// Scenarios exports
pub use scenarios::{
    JsApplicationReport as ApplicationReport, JsAssetClass as ScenarioAssetClass,
    JsCompounding as Compounding, JsCurveKind as ScenarioCurveKind,
    JsExecutionContext as ExecutionContext, JsOperationSpec as OperationSpec,
    JsRateBindingSpec as RateBindingSpec, JsRollForwardReport as RollForwardReport,
    JsScenarioEngine as ScenarioEngine, JsScenarioSpec as ScenarioSpec,
    JsScenarioSpecBuilder as ScenarioSpecBuilder, JsSeverity as ScenarioSeverity,
    JsTemplateMetadata as ScenarioTemplateMetadata, JsTemplateRegistry as ScenarioTemplateRegistry,
    JsTenorMatchMode as TenorMatchMode, JsTimeRollMode as TimeRollMode,
    JsVolSurfaceKind as VolSurfaceKind,
};

// Scenarios utility functions
pub use scenarios::{
    calculate_interpolation_weights as calculateInterpolationWeights,
    calculate_interpolation_weights_with_info as calculateInterpolationWeightsWithInfo,
    parse_period_to_days as parsePeriodToDays, parse_tenor_to_years as parseTenorToYears,
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
    JsPortfolioResult as PortfolioResult, JsPortfolioValuation as PortfolioValuation,
    JsPortfolioValuationOptions as PortfolioValuationOptions, JsPosition as Position,
    JsPositionUnit as PositionUnit, JsPositionValue as PositionValue,
};

// Portfolio factor model, book, and dependency exports
pub use portfolio::{
    JsBook as Book, JsBookId as BookId, JsDependencyIndex as DependencyIndex,
    JsFactorContribution as FactorContribution,
    JsFactorContributionDelta as FactorContributionDelta, JsFactorModel as PortfolioFactorModel,
    JsFactorModelBuilder as PortfolioFactorModelBuilder, JsMarketFactorKey as MarketFactorKey,
    JsRiskDecomposition as RiskDecomposition, JsStressResult as StressResult,
    JsWhatIfEngineWrapper as WhatIfEngine, JsWhatIfResult as WhatIfResult,
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
