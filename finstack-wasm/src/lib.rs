#![allow(clippy::module_inception)]

use wasm_bindgen::prelude::*;

mod core;
mod portfolio;
mod scenarios;
mod statements;
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
    ScheduleSpec, StubKind,
};
pub use core::dates::{
    imm_option_expiry as immOptionExpiry, third_friday as thirdFriday,
    third_wednesday as thirdWednesday,
};
pub use core::market_data::{
    BaseCorrelationCurve, CreditIndexData, CurveKind, DiscountCurve, DividendEvent,
    DividendSchedule, DividendScheduleBuilder, ForwardCurve, FxConfig, FxConversionPolicy,
    FxMatrix, FxRateResult, HazardCurve, InflationCurve, MarketContext, MarketScalar,
    ScalarTimeSeries, SeriesInterpolation, VolSurface,
};
pub use core::math::{
    adaptiveQuadrature, adaptiveSimpson, binomialProbability, gaussLegendreIntegrate,
    gaussLegendreIntegrateAdaptive, gaussLegendreIntegrateComposite, logBinomialCoefficient,
    logFactorial, simpsonRule, trapezoidalRule, BrentSolver, GaussHermiteQuadrature, NewtonSolver,
};
pub use core::money::JsMoney as Money;
pub use valuations::calibration::{
    JsBaseCorrelationCalibrator as BaseCorrelationCalibrator,
    JsCalibrationConfig as CalibrationConfig, JsCalibrationReport as CalibrationReport,
    JsCreditQuote as CreditQuote, JsDiscountCurveCalibrator as DiscountCurveCalibrator,
    JsForwardCurveCalibrator as ForwardCurveCalibrator,
    JsHazardCurveCalibrator as HazardCurveCalibrator,
    JsInflationCurveCalibrator as InflationCurveCalibrator, JsInflationQuote as InflationQuote,
    JsMarketQuote as MarketQuote, JsMultiCurveConfig as MultiCurveConfig,
    JsRatesQuote as RatesQuote, JsSABRCalibrationDerivatives as SABRCalibrationDerivatives,
    JsSABRMarketData as SABRMarketData, JsSABRModelParams as SABRModelParams,
    JsSolverKind as SolverKind, JsValidationConfig as ValidationConfig,
    JsValidationError as ValidationError, JsVolQuote as VolQuote,
    JsVolSurfaceCalibrator as VolSurfaceCalibrator,
};
// Validation functions
pub use valuations::calibration::validation::{
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
    AsianOption, Autocallable, AveragingMethod, BarrierOption, BasisSwap, Basket, Bond, CDSIndex,
    CdsOption, CdsTranche, CliquetOption, CmsOption, ConvertibleBond, CreditDefaultSwap, Deposit,
    Equity, EquityOption, EquityTotalReturnSwap, FiIndexTotalReturnSwap, ForwardRateAgreement,
    FxBarrierOption, FxOption, FxSpot, FxSwap, InflationLinkedBond, InflationSwap,
    InterestRateFuture, InterestRateOption, InterestRateSwap, LookbackOption, LookbackType,
    PrivateMarketsFund, QuantoOption, RangeAccrual, RealizedVarMethod, Repo, RevolvingCredit,
    StructuredCredit, Swaption, TermLoan, VarianceSwap,
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

// Risk analysis functions
pub use valuations::risk::{cs01_ladder as cs01Ladder, krd_dv01_ladder as krdDv01Ladder};

// Statements exports
pub use statements::{
    JsAmountOrScalar as AmountOrScalar, JsCapitalStructureSpec as CapitalStructureSpec,
    JsCorkscrewExtension as CorkscrewExtension,
    JsCreditScorecardExtension as CreditScorecardExtension,
    JsDebtInstrumentSpec as DebtInstrumentSpec, JsEvaluator as Evaluator,
    JsExtensionMetadata as ExtensionMetadata, JsExtensionRegistry as ExtensionRegistry,
    JsExtensionResult as ExtensionResult, JsExtensionStatus as ExtensionStatus,
    JsFinancialModelSpec as FinancialModelSpec, JsForecastMethod as ForecastMethod,
    JsForecastSpec as ForecastSpec, JsMetricDefinition as MetricDefinition,
    JsMetricRegistry as StatementsMetricRegistry, JsModelBuilder as ModelBuilder,
    JsNodeSpec as NodeSpec, JsNodeType as NodeType, JsRegistry as Registry, JsResults as Results,
    JsResultsMeta as ResultsMeta, JsSeasonalMode as SeasonalMode, JsUnitType as UnitType,
};

// Scenarios exports
pub use scenarios::{
    JsApplicationReport as ApplicationReport, JsCurveKind as ScenarioCurveKind,
    JsExecutionContext as ExecutionContext, JsOperationSpec as OperationSpec,
    JsRollForwardReport as RollForwardReport, JsScenarioEngine as ScenarioEngine,
    JsScenarioSpec as ScenarioSpec, JsTenorMatchMode as TenorMatchMode,
    JsVolSurfaceKind as VolSurfaceKind,
};

// Portfolio exports
pub use portfolio::{
    js_aggregate_by_attribute as aggregateByAttribute, js_aggregate_metrics as aggregateMetrics,
    js_create_position_from_bond as createPositionFromBond,
    js_create_position_from_deposit as createPositionFromDeposit,
    js_group_by_attribute as groupByAttribute, js_value_portfolio as valuePortfolio,
    JsAggregatedMetric as AggregatedMetric, JsEntity as Entity, JsPortfolio as Portfolio,
    JsPortfolioBuilder as PortfolioBuilder, JsPortfolioMetrics as PortfolioMetrics,
    JsPortfolioResults as PortfolioResults, JsPortfolioValuation as PortfolioValuation,
    JsPosition as Position, JsPositionUnit as PositionUnit, JsPositionValue as PositionValue,
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
