// Type declarations for the finstack-wasm namespaced facade.
// Shapes follow `wasm-bindgen` JS names in `src/api/**` (see Rust `js_name`).

export { default } from "./pkg/finstack_wasm";

// --- core -----------------------------------------------------------------

export interface Currency {
  readonly code: string;
  readonly numeric: number;
  readonly decimals: number;
  toString(): string;
  toJson(): string;
}

export interface CurrencyConstructor {
  new (code: string): Currency;
  fromJson(json: string): Currency;
}

export interface Money {
  readonly amount: number;
  readonly currency: Currency;
  add(other: Money): Money;
  sub(other: Money): Money;
  mulScalar(factor: number): Money;
  divScalar(divisor: number): Money;
  negate(): Money;
  toString(): string;
}

export interface MoneyConstructor {
  new (amount: number, currency: Currency): Money;
}

export interface Rate {
  readonly asDecimal: number;
  readonly asPercent: number;
  readonly asBps: number;
}

export interface RateConstructor {
  new (decimal: number): Rate;
  fromPercent(pct: number): Rate;
  fromBps(bps: number): Rate;
}

export interface Bps {
  asDecimal(): number;
  asBps(): number;
}

export interface BpsConstructor {
  new (value: number): Bps;
}

export interface Percentage {
  asDecimal(): number;
  asPercent(): number;
}

export interface PercentageConstructor {
  new (value: number): Percentage;
}

export interface DayCount {
  yearFraction(startEpochDays: number, endEpochDays: number): number;
  calendarDays(startEpochDays: number, endEpochDays: number): number;
  toString(): string;
}

export interface DayCountConstructor {
  new (name: string): DayCount;
  act360(): DayCount;
  act365f(): DayCount;
  thirty360(): DayCount;
  thirtyE360(): DayCount;
  actAct(): DayCount;
  actActIsma(): DayCount;
  bus252(): DayCount;
}

export interface Tenor {
  readonly count: number;
  toYearsSimple(): number;
  toString(): string;
}

export interface TenorConstructor {
  new (s: string): Tenor;
  daily(): Tenor;
  weekly(): Tenor;
  monthly(): Tenor;
  quarterly(): Tenor;
  semiAnnual(): Tenor;
  annual(): Tenor;
}

export interface DiscountCurve {
  readonly id: string;
  readonly baseDate: string;
  df(t: number): number;
  zero(t: number): number;
  forwardRate(t1: number, t2: number): number;
}

export interface DiscountCurveConstructor {
  new (
    id: string,
    baseDate: string,
    knots: number[],
    interp?: string,
    extrapolation?: string,
    dayCount?: string
  ): DiscountCurve;
}

export interface ForwardCurve {
  readonly id: string;
  readonly baseDate: string;
  rate(t: number): number;
}

export interface ForwardCurveConstructor {
  new (
    id: string,
    tenor: number,
    baseDate: string,
    knots: number[],
    dayCount?: string,
    interp?: string,
    extrapolation?: string
  ): ForwardCurve;
}

export interface FxMatrix {
  setQuote(base: string, quote: string, rate: number): void;
  rate(
    base: string,
    quote: string,
    date: string,
    policy?: string
  ): number;
}

export interface FxMatrixConstructor {
  new (): FxMatrix;
}

/** Monte Carlo European pricer result (JSON object from Rust). */
export interface MonteCarloEstimateJson {
  mean: number;
  currency: string;
  stderr: number;
  std_dev: number | null;
  ci_lower: number;
  ci_upper: number;
  num_paths: number;
}

/** Variation margin calculator result (JSON object from Rust). */
export interface VariationMarginJson {
  gross_exposure: number;
  net_exposure: number;
  delivery_amount: number;
  return_amount: number;
  net_margin: number;
  requires_call: boolean;
}

/** Forecast backtest metrics (JSON object from Rust). */
export interface BacktestForecastMetricsJson {
  mae: number;
  mape: number;
  rmse: number;
  n: number;
}

export interface CoreNamespace {
  Currency: CurrencyConstructor;
  Money: MoneyConstructor;
  Rate: RateConstructor;
  Bps: BpsConstructor;
  Percentage: PercentageConstructor;
  DayCount: DayCountConstructor;
  Tenor: TenorConstructor;
  createDate(year: number, month: number, day: number): number;
  dateFromEpochDays(days: number): number[];
  adjustBusinessDay(
    epochDays: number,
    convention: string,
    calendarCode: string
  ): number;
  availableCalendars(): string[];
  DiscountCurve: DiscountCurveConstructor;
  ForwardCurve: ForwardCurveConstructor;
  FxMatrix: FxMatrixConstructor;
  choleskyDecomposition(matrix: number[][]): number[][];
  choleskySolve(chol: number[][], b: number[]): number[];
  /** Validates a square correlation matrix passed as nested rows (core/math). */
  validateCorrelationMatrix(matrix: number[][]): void;
  mean(data: number[]): number;
  variance(data: number[]): number;
  populationVariance(data: number[]): number;
  correlation(x: number[], y: number[]): number;
  covariance(x: number[], y: number[]): number;
  quantile(data: number[], q: number): number;
  normCdf(x: number): number;
  normPdf(x: number): number;
  standardNormalInvCdf(p: number): number;
  erf(x: number): number;
  lnGamma(x: number): number;
  kahanSum(values: number[]): number;
  neumaierSum(values: number[]): number;
}

export declare const core: CoreNamespace;

// --- analytics ------------------------------------------------------------

export interface AnalyticsNamespace {
  // Risk metrics — return-based
  sharpe(annReturn: number, annVol: number, riskFreeRate: number): number;
  sortino(annReturn: number, downsideDev: number, riskFreeRate: number): number;
  volatility(returns: number[], periodsPerYear: number): number;
  meanReturn(returns: number[]): number;
  cagrFromPeriods(totalReturn: number, numPeriods: number, periodsPerYear: number): number;
  downsideDeviation(returns: number[], threshold: number, periodsPerYear: number): number;
  geometricMean(returns: number[]): number;
  omegaRatio(returns: number[], threshold: number): number;
  gainToPain(returns: number[]): number;
  modifiedSharpe(annReturn: number, annVol: number, skew: number, kurt: number, riskFreeRate: number): number;
  // Risk metrics — tail
  valueAtRisk(returns: number[], confidence: number): number;
  expectedShortfall(returns: number[], confidence: number): number;
  parametricVar(mean: number, std: number, confidence: number): number;
  cornishFisherVar(mean: number, std: number, skew: number, kurt: number, confidence: number): number;
  skewness(data: number[]): number;
  kurtosis(data: number[]): number;
  tailRatio(returns: number[], confidence: number): number;
  outlierWinRatio(returns: number[], threshold: number): number;
  outlierLossRatio(returns: number[], threshold: number): number;
  // Risk metrics — rolling
  rollingSharpeValues(returns: number[], window: number, periodsPerYear: number, riskFreeRate: number): number[];
  rollingSortinoValues(returns: number[], window: number, periodsPerYear: number, riskFreeRate: number, threshold: number): number[];
  rollingVolatilityValues(returns: number[], window: number, periodsPerYear: number): number[];
  // Returns
  simpleReturns(prices: number[]): number[];
  compSum(returns: number[]): number[];
  compTotal(returns: number[]): number;
  cleanReturns(returns: number[]): number[];
  convertToPrices(returns: number[], startPrice: number): number[];
  rebase(prices: number[], baseValue: number): number[];
  excessReturns(returns: number[], benchmark: number[]): number[];
  // Drawdown
  toDrawdownSeries(returns: number[]): number[];
  maxDrawdown(drawdownSeries: number[]): number;
  maxDrawdownFromReturns(returns: number[]): number;
  avgDrawdown(drawdownSeries: number[], count: number): number;
  averageDrawdown(drawdownSeries: number[]): number;
  cdar(drawdownSeries: number[], confidence: number): number;
  ulcerIndex(drawdownSeries: number[]): number;
  painIndex(drawdownSeries: number[]): number;
  calmar(cagr: number, maxDd: number): number;
  calmarFromReturns(returns: number[], periodsPerYear: number): number;
  recoveryFactor(totalReturn: number, maxDd: number): number;
  recoveryFactorFromReturns(returns: number[]): number;
  martinRatio(annReturn: number, ulcerIdx: number, riskFreeRate: number): number;
  martinRatioFromReturns(returns: number[], periodsPerYear: number, riskFreeRate: number): number;
  sterlingRatio(annReturn: number, avgDd: number, riskFreeRate: number): number;
  sterlingRatioFromReturns(returns: number[], periodsPerYear: number, riskFreeRate: number, numDrawdowns: number): number;
  burkeRatio(annReturn: number, drawdownSeries: number[], riskFreeRate: number): number;
  painRatio(annReturn: number, painIdx: number, riskFreeRate: number): number;
  painRatioFromReturns(returns: number[], periodsPerYear: number, riskFreeRate: number): number;
  // Benchmark
  trackingError(returns: number[], benchmark: number[], periodsPerYear: number): number;
  informationRatio(annReturn: number, benchAnnReturn: number, te: number): number;
  rSquared(returns: number[], benchmark: number[]): number;
  upCapture(returns: number[], benchmark: number[]): number;
  downCapture(returns: number[], benchmark: number[]): number;
  captureRatio(upCap: number, downCap: number): number;
  battingAverage(returns: number[], benchmark: number[]): number;
  treynor(annReturn: number, riskFreeRate: number, beta: number): number;
  mSquared(annReturn: number, annVol: number, benchVol: number, riskFreeRate: number): number;
  mSquaredFromReturns(returns: number[], benchmark: number[], periodsPerYear: number, riskFreeRate: number): number;
  // Consecutive
  countConsecutive(returns: number[]): number[];
}

export declare const analytics: AnalyticsNamespace;

// --- correlation ------------------------------------------------------------

export interface Copula {
  readonly numFactors: number;
  readonly modelName: string;
  conditionalDefaultProb(
    defaultThreshold: number,
    factorRealization: number[],
    correlation: number
  ): number;
  tailDependence(correlation: number): number;
}

export interface CopulaSpec {
  readonly isGaussian: boolean;
  readonly isStudentT: boolean;
  build(): Copula;
}

export interface CopulaSpecConstructor {
  gaussian(): CopulaSpec;
  studentT(df: number): CopulaSpec;
  randomFactorLoading(loadingVol: number): CopulaSpec;
  multiFactor(numFactors: number): CopulaSpec;
}

export interface RecoveryModel {
  readonly expectedRecovery: number;
  readonly lgd: number;
  readonly isStochastic: boolean;
  readonly modelName: string;
  conditionalRecovery(marketFactor: number): number;
}

export interface RecoverySpec {
  readonly expectedRecovery: number;
  build(): RecoveryModel;
}

export interface RecoverySpecConstructor {
  constant(rate: number): RecoverySpec;
  marketCorrelated(
    mean: number,
    vol: number,
    correlation: number
  ): RecoverySpec;
}

/** Exported class; construct instances via `CopulaSpec.build()` (no public `new`). */
export interface CopulaClass {
  readonly prototype: Copula;
}

/** Exported class; construct instances via `RecoverySpec.build()` (no public `new`). */
export interface RecoveryModelClass {
  readonly prototype: RecoveryModel;
}

export interface CorrelationNamespace {
  CopulaSpec: CopulaSpecConstructor;
  Copula: CopulaClass;
  RecoverySpec: RecoverySpecConstructor;
  RecoveryModel: RecoveryModelClass;
  correlationBounds(p1: number, p2: number): number[];
  jointProbabilities(p1: number, p2: number, correlation: number): number[];
  /**
   * Flat row-major correlation matrix with explicit dimension `n`
   * (finstack-correlation). Same wasm export name as core/math; if both are
   * linked, the generated binding is whichever the linker keeps last.
   */
  validateCorrelationMatrix(matrix: number[], n: number): void;
}

export declare const correlation: CorrelationNamespace;

// --- monte_carlo ----------------------------------------------------------

export interface MonteCarloNamespace {
  priceEuropeanCall(
    spot: number, strike: number, rate: number, divYield: number,
    vol: number, expiry: number, numPaths: number, seed: bigint,
    numSteps?: number, currency?: string
  ): MonteCarloEstimateJson;
  priceEuropeanPut(
    spot: number, strike: number, rate: number, divYield: number,
    vol: number, expiry: number, numPaths: number, seed: bigint,
    numSteps?: number, currency?: string
  ): MonteCarloEstimateJson;
  priceAsianCall(
    spot: number, strike: number, rate: number, divYield: number,
    vol: number, expiry: number, numPaths: number, seed: bigint,
    numSteps?: number, currency?: string
  ): MonteCarloEstimateJson;
  priceAsianPut(
    spot: number, strike: number, rate: number, divYield: number,
    vol: number, expiry: number, numPaths: number, seed: bigint,
    numSteps?: number, currency?: string
  ): MonteCarloEstimateJson;
  priceAmericanPut(
    spot: number, strike: number, rate: number, divYield: number,
    vol: number, expiry: number, numPaths: number, seed: bigint,
    numSteps?: number, currency?: string
  ): MonteCarloEstimateJson;
  blackScholesCall(
    spot: number, strike: number, rate: number, divYield: number,
    vol: number, expiry: number
  ): number;
  blackScholesPut(
    spot: number, strike: number, rate: number, divYield: number,
    vol: number, expiry: number
  ): number;
}

export declare const monte_carlo: MonteCarloNamespace;

// --- margin ----------------------------------------------------------------

export interface MarginNamespace {
  csaUsdRegulatory(): string;
  csaEurRegulatory(): string;
  validateCsaJson(json: string): string;
  calculateVm(
    csaJson: string,
    exposure: number,
    postedCollateral: number,
    currency: string,
    year: number,
    month: number,
    day: number
  ): VariationMarginJson;
}

export declare const margin: MarginNamespace;

// --- valuations ------------------------------------------------------------

export interface ValuationsNamespace {
  validateValuationResultJson(json: string): string;
  validateInstrumentJson(json: string): string;
  priceInstrument(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model: string
  ): string;
  priceInstrumentWithMetrics(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model: string,
    metrics: string[]
  ): string;
  listStandardMetrics(): string[];
  /** Run P&L attribution for a single instrument. */
  attributePnl(
    instrumentJson: string,
    marketT0Json: string,
    marketT1Json: string,
    asOfT0: string,
    asOfT1: string,
    methodJson: string,
    configJson?: string
  ): string;
  /** Run attribution from a full JSON AttributionEnvelope. */
  attributePnlFromSpec(specJson: string): string;
  /** Validate an attribution specification JSON. */
  validateAttributionJson(json: string): string;
  /** Return the default waterfall factor ordering. */
  defaultWaterfallOrder(): string[];
  /** Return the default metric IDs used by metrics-based attribution. */
  defaultAttributionMetrics(): string[];
  /** Compute first-order factor sensitivities. */
  computeFactorSensitivities(
    positionsJson: string,
    factorsJson: string,
    marketJson: string,
    asOf: string,
    bumpConfigJson?: string
  ): string;
  /** Compute scenario P&L profiles via full repricing. */
  computePnlProfiles(
    positionsJson: string,
    factorsJson: string,
    marketJson: string,
    asOf: string,
    bumpConfigJson?: string,
    nScenarioPoints?: number
  ): string;
  /** Decompose portfolio risk into factor and position contributions. */
  decomposeFactorRisk(
    sensitivitiesJson: string,
    covarianceJson: string,
    riskMeasureJson?: string
  ): string;
}

export declare const valuations: ValuationsNamespace;

// --- statements ------------------------------------------------------------

export interface StatementsNamespace {
  validateFinancialModelJson(json: string): string;
  modelNodeIds(json: string): string[];
}

export declare const statements: StatementsNamespace;

// --- statements_analytics -------------------------------------------------

export interface GoalSeekResult {
  driver_value: number;
  achieved_value: number;
  iterations: number;
  converged: boolean;
}

export interface StatementsAnalyticsNamespace {
  runSensitivity(modelJson: string, configJson: string): string;
  runVariance(
    baseJson: string,
    comparisonJson: string,
    configJson: string
  ): string;
  evaluateScenarioSet(modelJson: string, scenarioSetJson: string): string;
  backtestForecast(
    actual: number[],
    forecast: number[]
  ): BacktestForecastMetricsJson;
  generateTornadoEntries(
    resultJson: string,
    metricNode: string,
    period?: string
  ): string;
  runMonteCarlo(modelJson: string, configJson: string): string;
  goalSeek(
    modelJson: string,
    targetNode: string,
    driverNode: string,
    targetPeriod: string,
    driverPeriod: string,
    targetValue: number,
    lowerBound: number,
    upperBound: number
  ): GoalSeekResult;
  traceDependencies(modelJson: string, nodeId: string): string;
  explainFormula(
    modelJson: string,
    resultsJson: string,
    nodeId: string,
    period: string
  ): string;
  plSummaryReport(
    resultsJson: string,
    lineItems: string[],
    periods: string[]
  ): string;
  creditAssessmentReport(resultsJson: string, asOf: string): string;
}

export declare const statements_analytics: StatementsAnalyticsNamespace;

// --- portfolio -------------------------------------------------------------

export interface ScenarioRevalueResult {
  valuation: string;
  report: string;
}

export interface PortfolioNamespace {
  parsePortfolioSpec(jsonStr: string): string;
  buildPortfolioFromSpec(specJson: string): string;
  portfolioResultTotalValue(resultJson: string): number;
  portfolioResultGetMetric(
    resultJson: string,
    metricId: string
  ): number | undefined;
  aggregateMetrics(
    valuationJson: string,
    baseCcy: string,
    marketJson: string,
    asOf: string
  ): string;
  valuePortfolio(
    specJson: string,
    marketJson: string,
    strictRisk: boolean
  ): string;
  aggregateCashflows(specJson: string, marketJson: string): string;
  applyScenarioAndRevalue(
    specJson: string,
    scenarioJson: string,
    marketJson: string
  ): ScenarioRevalueResult;
  /** Optimize portfolio weights using the LP-based optimizer. */
  optimizePortfolio(specJson: string, marketJson: string): string;
}

export declare const portfolio: PortfolioNamespace;

// --- scenarios -------------------------------------------------------------

export interface ScenarioApplyResult {
  market_json: string;
  model_json: string;
  operations_applied: number;
  warnings: string[];
}

export interface ScenarioApplyMarketResult {
  market_json: string;
  operations_applied: number;
  warnings: string[];
}

export interface ScenariosNamespace {
  parseScenarioSpec(jsonStr: string): string;
  composeScenarios(specsJson: string): string;
  validateScenarioSpec(jsonStr: string): boolean;
  listBuiltinTemplates(): string[];
  listBuiltinTemplateMetadata(): string;
  buildFromTemplate(templateId: string): string;
  listTemplateComponents(templateId: string): string[];
  buildTemplateComponent(templateId: string, componentId: string): string;
  buildScenarioSpec(
    id: string,
    operationsJson: string,
    name?: string,
    description?: string,
    priority?: number
  ): string;
  applyScenario(
    scenarioJson: string,
    marketJson: string,
    modelJson: string,
    asOf: string
  ): ScenarioApplyResult;
  applyScenarioToMarket(
    scenarioJson: string,
    marketJson: string,
    asOf: string
  ): ScenarioApplyMarketResult;
}

export declare const scenarios: ScenariosNamespace;
