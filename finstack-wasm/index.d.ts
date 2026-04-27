// Type declarations for the finstack-wasm namespaced facade.
// Shapes follow `wasm-bindgen` JS names in `src/api/**` (see Rust `js_name`).

export { default } from './pkg/finstack_wasm';

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
  yearFractionWithContext(
    startEpochDays: number,
    endEpochDays: number,
    ctx: DayCountContext
  ): number;
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

export interface DayCountContext {
  withCalendar(calendarCode: string): DayCountContext;
  withFrequency(frequency: Tenor): DayCountContext;
  withBusBasis(busBasis: number): DayCountContext;
}

export interface DayCountContextConstructor {
  new (): DayCountContext;
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

export interface VolCube {
  readonly id: string;
  vol(expiry: number, tenor: number, strike: number): number;
  volClamped(expiry: number, tenor: number, strike: number): number;
}

export interface VolCubeConstructor {
  new (
    id: string,
    expiries: number[],
    tenors: number[],
    paramsFlat: number[],
    forwards: number[]
  ): VolCube;
}

export interface FxConversionPolicy {
  getName(): string;
  toString(): string;
}

export interface FxConversionPolicyConstructor {
  cashflowDate(): FxConversionPolicy;
  periodEnd(): FxConversionPolicy;
  periodAverage(): FxConversionPolicy;
  custom(): FxConversionPolicy;
  fromName(name: string): FxConversionPolicy;
}

export interface FxRateResult {
  getRate(): number;
  getTriangulated(): boolean;
  getPolicy(): FxConversionPolicy;
}

export interface FxMatrix {
  setQuote(base: string, quote: string, rate: number): void;
  rate(base: string, quote: string, date: string, policy?: FxConversionPolicy): FxRateResult;
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
  /** Number of independent path estimators; equals `num_simulated_paths` without variance reduction, half of it with antithetic pairing. */
  num_paths: number;
  /** Total number of simulated sample paths; `2 * num_paths` with antithetic variates, otherwise equals `num_paths`. */
  num_simulated_paths: number;
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
  DayCountContext: DayCountContextConstructor;
  Tenor: TenorConstructor;
  createDate(year: number, month: number, day: number): number;
  dateFromEpochDays(days: number): number[];
  adjustBusinessDay(epochDays: number, convention: string, calendarCode: string): number;
  availableCalendars(): string[];
  DiscountCurve: DiscountCurveConstructor;
  ForwardCurve: ForwardCurveConstructor;
  VolCube: VolCubeConstructor;
  FxConversionPolicy: FxConversionPolicyConstructor;
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
  countConsecutive(values: number[]): number;
}

export declare const core: CoreNamespace;

// --- analytics ------------------------------------------------------------

/** Breach indicator from VaR backtesting: "Hit" or "Miss". */
export type Breach = 'Hit' | 'Miss';
export type VarMethod = 'Historical' | 'Parametric' | 'CornishFisher';

/** Kupiec Proportion of Failures (POF) unconditional coverage test result. */
export interface KupiecResultJson {
  lr_statistic: number;
  p_value: number;
  breach_count: number;
  expected_count: number;
  total_observations: number;
  observed_rate: number;
  reject_h0_5pct: boolean;
}

/** Christoffersen conditional coverage test result. */
export interface ChristoffersenResultJson {
  lr_uc: number;
  lr_ind: number;
  lr_cc: number;
  p_value_uc: number;
  p_value_ind: number;
  p_value_cc: number;
  transition_counts: [number, number, number, number];
  reject_h0_5pct: boolean;
}

/** Basel traffic-light zone: "Green", "Yellow", or "Red". */
export type TrafficLightZone = 'Green' | 'Yellow' | 'Red';

/** Basel traffic-light classification result. */
export interface TrafficLightResultJson {
  zone: TrafficLightZone;
  exceptions: number;
  capital_multiplier: number;
  window_size: number;
  confidence: number;
}

/** FRTB-style P&L explanation diagnostics. */
export interface PnlExplanationJson {
  explanation_ratio: number;
  aggregate_explanation_ratio: number;
  mean_abs_unexplained: number;
  std_unexplained: number;
  n: number;
}

/** Full VaR backtest result aggregating all statistical tests. */
export interface BacktestResultJson {
  kupiec: KupiecResultJson;
  christoffersen: ChristoffersenResultJson;
  traffic_light: TrafficLightResultJson;
  breaches: Breach[];
  confidence: number;
}

/** Side-by-side VaR backtest comparison across multiple model methods. */
export interface MultiModelComparisonJson {
  results: [VarMethod, BacktestResultJson][];
}

/** GARCH model parameter estimates (serde-serialized from Rust). */
export interface GarchParamsJson {
  omega: number;
  alpha: number;
  beta: number;
  gamma: number | null;
  dist: { Gaussian: null } | { StudentT: number };
  mean: number;
  family: string;
}

/** Complete GARCH model fit result (serde-serialized from Rust). */
export interface GarchFitJson {
  model: string;
  params: GarchParamsJson;
  std_errors: number[] | null;
  log_likelihood: number;
  n_obs: number;
  n_params: number;
  aic: number;
  bic: number;
  hqic: number;
  conditional_variances: number[];
  standardized_residuals: number[];
  terminal_variance: number;
  converged: boolean;
  iterations: number;
}

/** A single variance forecast returned by `forecastGarchFit`. */
export interface VarianceForecastJson {
  horizon: number;
  variance: number;
  annualized_vol: number;
}

/** Descriptive statistics returned by `peerStats`. */
export interface PeerStatsJson {
  count: number;
  mean: number;
  median: number;
  std_dev: number;
  min: number;
  max: number;
  q1: number;
  q3: number;
}

/** Single-factor OLS regression result returned by `regressionFairValue`. */
export interface RegressionResultJson {
  intercept: number;
  slope: number;
  r_squared: number;
  fitted_value: number;
  residual: number;
  n: number;
}

/** Per-dimension decomposition in a relative value score. */
export interface DimensionScoreJson {
  label: string;
  percentile: number;
  z_score: number;
  regression_residual: number | null;
  r_squared: number | null;
  weight: number;
}

/** Composite relative value result returned by `scoreRelativeValue`. */
export interface RelativeValueResultJson {
  company_id: string;
  composite_score: number;
  dimensions: DimensionScoreJson[];
  confidence: number;
  peer_count: number;
}

/** A single drawdown episode returned by `drawdownDetails`. */
export interface DrawdownEpisode {
  start: string;
  valley: string;
  end: string | null;
  duration_days: number;
  max_drawdown: number;
  near_recovery_threshold: number;
}

/** Aggregate statistics for grouped periodic returns. */
export interface PeriodStats {
  best: number;
  worst: number;
  consecutive_wins: number;
  consecutive_losses: number;
  win_rate: number;
  avg_return: number;
  avg_win: number;
  avg_loss: number;
  payoff_ratio: number;
  profit_factor: number;
  cpc_ratio: number;
  kelly_criterion: number;
}

/** Dated rolling Sharpe result returned by `rollingSharpe`. */
export interface RollingSharpe {
  values: number[];
  dates: string[];
}

/** Dated rolling Sortino result returned by `rollingSortino`. */
export interface RollingSortino {
  values: number[];
  dates: string[];
}

/** Dated rolling volatility result returned by `rollingVolatility`. */
export interface RollingVolatility {
  values: number[];
  dates: string[];
}

/** OLS beta result with standard error and 95% confidence interval. */
export interface BetaResult {
  beta: number;
  std_err: number;
  ci_lower: number;
  ci_upper: number;
}

/** Single-factor greeks (alpha, beta, R², adjusted R²). */
export interface GreeksResult {
  alpha: number;
  beta: number;
  r_squared: number;
  adjusted_r_squared: number;
}

/** Rolling greeks output aligned with rolling-window end dates. */
export interface RollingGreeksResult {
  dates: string[];
  alphas: number[];
  betas: number[];
}

/** Multi-factor regression result. */
export interface MultiFactorResult {
  alpha: number;
  betas: number[];
  r_squared: number;
  adjusted_r_squared: number;
  residual_vol: number;
}

/** Opaque CAGR annualization basis used by the analytics namespace. */
export interface CagrBasis {}

export interface CagrBasisConstructor {
  factor(annFactor: number): CagrBasis;
  dates(start: string, end: string, convention?: string): CagrBasis;
}

export interface BenchmarkAlignmentPolicyConstructor {
  zeroOnMissing(): BenchmarkAlignmentPolicy;
  errorOnMissing(): BenchmarkAlignmentPolicy;
}

export interface BenchmarkAlignmentPolicy {}

export interface RuinDefinitionConstructor {
  wealthFloor(floorFraction: number): RuinDefinition;
  terminalFloor(floorFraction: number): RuinDefinition;
  drawdownBreach(maxDrawdown: number): RuinDefinition;
}

export interface RuinDefinition {}

export interface RuinModelConstructor {
  new (
    horizonPeriods?: number,
    nPaths?: number,
    blockSize?: number,
    seed?: number,
    confidenceLevel?: number
  ): RuinModel;
}

export interface RuinModel {}

export interface RuinEstimateJson {
  probability: number;
  std_err: number;
  ci_lower: number;
  ci_upper: number;
}

export interface AnalyticsNamespace {
  /**
   * The WASM analytics namespace intentionally exposes pure functions and
   * typed value objects rather than the stateful Rust `Performance` panel API.
   * Use `finstack.analytics.Performance` in Python when you need the panel
   * facade, or compose the pure-function analytics exports directly in JS.
   */
  // Risk metrics — return-based
  CagrBasis: CagrBasisConstructor;
  BenchmarkAlignmentPolicy: BenchmarkAlignmentPolicyConstructor;
  RuinDefinition: RuinDefinitionConstructor;
  RuinModel: RuinModelConstructor;
  sharpe(annReturn: number, annVol: number, riskFreeRate: number): number;
  sortino(returns: number[], annualize: boolean, annFactor: number, mar: number): number;
  volatility(returns: number[], annualize: boolean, annFactor: number): number;
  meanReturn(returns: number[], annualize: boolean, annFactor: number): number;
  cagr(returns: number[], basis: CagrBasis): number;
  downsideDeviation(
    returns: number[],
    threshold: number,
    annualize: boolean,
    annFactor: number
  ): number;
  geometricMean(returns: number[]): number;
  omegaRatio(returns: number[], threshold: number): number;
  gainToPain(returns: number[]): number;
  modifiedSharpe(
    returns: number[],
    riskFreeRate: number,
    confidence: number,
    annFactor: number
  ): number;
  estimateRuin(returns: number[], definition: RuinDefinition, model: RuinModel): RuinEstimateJson;
  // Risk metrics — tail
  valueAtRisk(returns: number[], confidence: number): number;
  expectedShortfall(returns: number[], confidence: number): number;
  parametricVar(returns: number[], confidence: number, annFactor?: number): number;
  cornishFisherVar(returns: number[], confidence: number, annFactor?: number): number;
  skewness(returns: number[]): number;
  kurtosis(returns: number[]): number;
  tailRatio(returns: number[], confidence: number): number;
  outlierWinRatio(returns: number[], confidence: number): number;
  outlierLossRatio(returns: number[], confidence: number): number;
  // Risk metrics — rolling (dated structs)
  rollingSharpe(
    returns: number[],
    dates: string[],
    window: number,
    annFactor: number,
    riskFreeRate: number
  ): RollingSharpe;
  rollingSortino(
    returns: number[],
    dates: string[],
    window: number,
    annFactor: number
  ): RollingSortino;
  rollingVolatility(
    returns: number[],
    dates: string[],
    window: number,
    annFactor: number
  ): RollingVolatility;
  // Returns
  simpleReturns(prices: number[]): number[];
  compSum(returns: number[]): number[];
  compTotal(returns: number[]): number;
  cleanReturns(returns: number[]): number[];
  convertToPrices(returns: number[], startPrice: number): number[];
  rebase(prices: number[], baseValue: number): number[];
  excessReturns(returns: number[], rf: number[], nperiods?: number): number[];
  // Drawdown
  toDrawdownSeries(returns: number[]): number[];
  maxDrawdown(drawdownSeries: number[]): number;
  meanEpisodeDrawdown(drawdownSeries: number[], count: number): number;
  meanDrawdown(drawdownSeries: number[]): number;
  drawdownDetails(drawdownSeries: number[], dates: string[], n: number): DrawdownEpisode[];
  maxDrawdownDuration(drawdownSeries: number[], dates: string[]): number;
  cdar(drawdownSeries: number[], confidence: number): number;
  ulcerIndex(drawdownSeries: number[]): number;
  painIndex(drawdownSeries: number[]): number;
  calmar(cagr: number, maxDd: number): number;
  recoveryFactor(totalReturn: number, maxDd: number): number;
  martinRatio(cagr: number, ulcer: number): number;
  sterlingRatio(annReturn: number, avgDd: number, riskFreeRate: number): number;
  burkeRatio(annReturn: number, drawdownSeries: number[], riskFreeRate: number): number;
  painRatio(annReturn: number, painIdx: number, riskFreeRate: number): number;
  // Benchmark
  alignBenchmark(
    benchReturns: number[],
    benchDates: string[],
    targetDates: string[],
    policy: BenchmarkAlignmentPolicy
  ): number[];
  trackingError(
    returns: number[],
    benchmark: number[],
    annualize: boolean,
    annFactor: number
  ): number;
  informationRatio(
    returns: number[],
    benchmark: number[],
    annualize: boolean,
    annFactor: number
  ): number;
  rSquared(returns: number[], benchmark: number[]): number;
  upCapture(returns: number[], benchmark: number[]): number;
  downCapture(returns: number[], benchmark: number[]): number;
  captureRatio(returns: number[], benchmark: number[]): number;
  battingAverage(returns: number[], benchmark: number[]): number;
  treynor(annReturn: number, riskFreeRate: number, beta: number): number;
  mSquared(annReturn: number, annVol: number, benchVol: number, riskFreeRate: number): number;
  beta(portfolio: number[], benchmark: number[]): BetaResult;
  greeks(returns: number[], benchmark: number[], annFactor: number): GreeksResult;
  rollingGreeks(
    returns: number[],
    benchmark: number[],
    dates: string[],
    window: number,
    annFactor: number
  ): RollingGreeksResult;
  multiFactorGreeks(returns: number[], factors: number[][], annFactor: number): MultiFactorResult;
  // Aggregation
  groupByPeriod(dates: string[], returns: number[], periodKind: string): [string, number][];
  periodStats(returns: number[]): PeriodStats;
  // Lookback selectors
  mtdSelect(dates: string[], asOf: string, offsetDays: number): [number, number];
  qtdSelect(dates: string[], asOf: string, offsetDays: number): [number, number];
  ytdSelect(dates: string[], asOf: string, offsetDays: number): [number, number];
  fytdSelect(
    dates: string[],
    asOf: string,
    offsetDays: number,
    fiscalStartMonth: number,
    fiscalStartDay: number
  ): [number, number];
  // Backtesting
  classifyBreaches(varForecasts: number[], realizedPnl: number[]): boolean[];
  kupiecTest(breachCount: number, n: number, confidence: number): KupiecResultJson;
  christoffersenTest(breachIndicators: boolean[], confidence: number): ChristoffersenResultJson;
  trafficLight(exceptions: number, n: number, confidence: number): TrafficLightResultJson;
  runBacktest(
    varForecasts: number[],
    realizedPnl: number[],
    confidence: number,
    windowSize: number
  ): BacktestResultJson;
  rollingVarForecasts(
    returns: number[],
    lookback: number,
    confidence: number,
    method: string
  ): [number[], number[]];
  /**
   * Batched rolling-VaR forecasts across multiple `(lookback, confidence, method)`
   * configurations over the same return series. Amortizes the JS↔WASM serde
   * round-trip paid per call by `rollingVarForecasts`.
   */
  rollingVarBatch(returns: number[], configs: [number, number, string][]): [number[], number[]][];
  compareVarBacktests(
    models: [string, number[]][],
    realizedPnl: number[],
    confidence: number,
    windowSize: number
  ): MultiModelComparisonJson;
  pnlExplanation(
    hypotheticalPnl: number[],
    riskTheoreticalPnl: number[],
    varForecasts: number[]
  ): PnlExplanationJson;
  // GARCH volatility models
  fitGarch11(returns: number[], distribution: string): GarchFitJson;
  fitEgarch11(returns: number[], distribution: string): GarchFitJson;
  fitGjrGarch11(returns: number[], distribution: string): GarchFitJson;
  forecastGarchFit(
    fit: GarchFitJson,
    horizons: number[],
    tradingDaysPerYear?: number,
    terminalResidual?: number | null
  ): VarianceForecastJson[];
  ljungBox(residuals: number[], lags: number): [number, number];
  archLm(residuals: number[], lags: number): [number, number];
  aic(logLikelihood: number, nParams: number): number;
  bic(logLikelihood: number, nParams: number, nObs: number): number;
  hqic(logLikelihood: number, nParams: number, nObs: number): number;
}

export declare const analytics: AnalyticsNamespace;

// --- valuations.creditFactorHierarchy ----------------------------------------

/**
 * Calibrated credit factor hierarchy artifact.
 *
 * Produced by `CreditCalibrator` or deserialized from JSON via `fromJson`.
 * Immutable once constructed.
 */
export declare class CreditFactorModel {
  private constructor();
  /** Deserialize and validate a `CreditFactorModel` from JSON. */
  static fromJson(s: string): CreditFactorModel;
  /** Serialize to pretty-printed JSON. */
  toJson(): string;
  free(): void;
}

/**
 * Deterministic calibrator that produces a `CreditFactorModel`.
 *
 * Configuration and inputs are passed as JSON strings.
 */
export declare class CreditCalibrator {
  /** Construct a calibrator from a JSON-serialized `CreditCalibrationConfig`. */
  constructor(configJson: string);
  /** Run the calibration pipeline and return a `CreditFactorModel`. */
  calibrate(inputsJson: string): CreditFactorModel;
  free(): void;
}

/**
 * Snapshot of all hierarchy-level factor values at a single date.
 *
 * Produced by `decomposeLevels`. Pass to `decomposePeriod` to compute
 * period-over-period changes.
 */
export declare class LevelsAtDate {
  private constructor();
  /** Serialize the snapshot to pretty-printed JSON. */
  toJson(): string;
  free(): void;
}

/**
 * Component-wise difference between two `LevelsAtDate` snapshots.
 *
 * Produced by `decomposePeriod`.
 */
export declare class PeriodDecomposition {
  private constructor();
  /** Serialize the decomposition to pretty-printed JSON. */
  toJson(): string;
  free(): void;
}

/**
 * Vol-forecast view over a calibrated `CreditFactorModel`.
 *
 * `VolHorizon::Custom` is intentionally **not** exposed.
 *
 * Horizon strings accepted by `covarianceAt`, `idiosyncraticVol`, and
 * `factorModelAt`:
 * - `"one_step"` — calibrated annualized variance unchanged.
 * - `"unconditional"` — long-run.
 * - `'{"n_steps": N}'` — variance scaled by `N`.
 */
export declare class FactorCovarianceForecast {
  constructor(model: CreditFactorModel);
  /**
   * Build the factor covariance matrix at the requested horizon.
   * Returns pretty-printed JSON of a `FactorCovarianceMatrix`.
   */
  covarianceAt(horizonJson: string): string;
  /** Idiosyncratic vol (std dev) for a specific issuer at the requested horizon. */
  idiosyncraticVol(issuerId: string, horizonJson: string): number;
  /**
   * Build a portfolio-level `FactorModelConfig` JSON at the given horizon and
   * risk measure.
   */
  factorModelAt(horizonJson: string, riskMeasureJson: string): string;
  free(): void;
}

/**
 * Decompose observed issuer spreads at a point in time into per-level factor
 * values and per-issuer residual adders.
 *
 * @param model                Calibrated `CreditFactorModel`.
 * @param observedSpreadsJson  JSON `{issuer_id: spread}` map.
 * @param observedGeneric      Generic (PC) factor value at `asOf`.
 * @param asOf                 ISO 8601 date string.
 * @param runtimeTagsJson      Optional JSON `{issuer_id: {dim_key: tag}}` for
 *                             issuers not present in the model artifact.
 */
export declare function decomposeLevels(
  model: CreditFactorModel,
  observedSpreadsJson: string,
  observedGeneric: number,
  asOf: string,
  runtimeTagsJson?: string
): LevelsAtDate;

/**
 * Difference two `LevelsAtDate` snapshots component-wise.
 *
 * Output is restricted to buckets and issuers present in **both** snapshots.
 */
export declare function decomposePeriod(
  fromLevels: LevelsAtDate,
  toLevels: LevelsAtDate
): PeriodDecomposition;

// --- valuations.correlation -------------------------------------------------

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
  marketCorrelated(mean: number, vol: number, correlation: number): RecoverySpec;
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
   * (finstack_valuations::correlation). Same wasm export name as core/math; if
   * both are linked, the generated binding is whichever the linker keeps last.
   */
  validateCorrelationMatrix(matrix: number[], n: number): void;
  /**
   * Nearest correlation matrix (Higham 2002) for a near-PSD input.
   *
   * Projects a symmetric, near-unit-diagonal, near-PSD matrix onto the set of
   * valid correlation matrices in Frobenius norm. Gross input violations
   * (asymmetry > 1e-6 or diagonal far from 1) throw rather than being silently
   * reshaped.
   */
  nearestCorrelation(matrix: number[], n: number, maxIter?: number, tol?: number): number[];
}

// --- monte_carlo ----------------------------------------------------------
// GBM convenience subset of finstack-monte-carlo. Advanced Rust process,
// discretization, RNG, payoff, and Greeks types are not standalone WASM types.

export interface MonteCarloNamespace {
  priceEuropeanCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  priceEuropeanPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  priceAsianCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  priceAsianPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string
  ): MonteCarloEstimateJson;
  priceAmericanPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string,
    useParallel?: boolean,
    basis?: string,
    basisDegree?: number
  ): MonteCarloEstimateJson;
  priceAmericanCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number,
    numPaths: number,
    seed: bigint,
    numSteps?: number,
    currency?: string,
    useParallel?: boolean,
    basis?: string,
    basisDegree?: number
  ): MonteCarloEstimateJson;
  blackScholesCall(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number
  ): number;
  blackScholesPut(
    spot: number,
    strike: number,
    rate: number,
    divYield: number,
    vol: number,
    expiry: number
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

// --- cashflows -------------------------------------------------------------

/**
 * JSON bridge to the Rust `finstack-cashflows` crate.
 *
 * All methods accept and return JSON strings that mirror the canonical Rust
 * serde model. Refer to `api/cashflows.rs` for parameter and return-shape
 * details; the docstrings there are kept in sync with the underlying Rust
 * implementation.
 */
export interface CashflowsNamespace {
  /**
   * Build a cashflow schedule from a `CashflowScheduleBuildSpec` JSON string.
   *
   * @param specJson    JSON-encoded `CashflowScheduleBuildSpec`.
   * @param marketJson  Optional JSON-encoded market context for floating-rate lookups.
   * @returns           JSON-encoded `CashFlowSchedule`.
   * @throws            If the spec or market JSON is malformed, or schedule construction fails.
   */
  buildCashflowSchedule(specJson: string, marketJson?: string | null): string;

  /**
   * Validate a cashflow schedule JSON string and return it canonicalized.
   *
   * @param scheduleJson JSON-encoded `CashFlowSchedule`.
   * @returns            Canonicalized JSON-encoded `CashFlowSchedule`.
   * @throws             If the schedule JSON is malformed or fails validation.
   */
  validateCashflowSchedule(scheduleJson: string): string;

  /**
   * Extract dated flows from a cashflow schedule.
   *
   * @param scheduleJson JSON-encoded `CashFlowSchedule`.
   * @returns            JSON array of `{date, amount}` entries, where `amount`
   *                     is itself `{amount, currency}`. `CFKind` and accrual
   *                     metadata are intentionally omitted.
   * @throws             If the schedule JSON is malformed.
   */
  datedFlows(scheduleJson: string): string;

  /**
   * Compute accrued interest for a schedule as of a given date.
   *
   * @param scheduleJson JSON-encoded `CashFlowSchedule`.
   * @param asOf         ISO-8601 date (YYYY-MM-DD) for the accrual snapshot.
   * @param configJson   Optional JSON-encoded `AccrualConfig` overriding defaults.
   * @returns            Accrued interest in the schedule's settlement currency.
   * @throws             If any JSON input is malformed or the accrual computation fails.
   */
  accruedInterest(scheduleJson: string, asOf: string, configJson?: string | null): number;

  /**
   * Construct a tagged Bond instrument JSON from a cashflow schedule.
   *
   * Convenience wrapper that crosses crates: it materializes a
   * `finstack_valuations::instruments::fixed_income::bond::Bond` from the
   * supplied schedule and wraps it in the tagged `InstrumentJson` envelope.
   *
   * @param instrumentId    Identifier for the Bond instrument.
   * @param scheduleJson    JSON-encoded `CashFlowSchedule`.
   * @param discountCurveId Identifier of the discount curve used for pricing.
   * @param quotedClean     Optional clean quoted price used to calibrate yield on construction.
   * @returns               JSON-encoded tagged `InstrumentJson::Bond`.
   * @throws                If the schedule JSON is malformed or bond construction fails.
   */
  bondFromCashflows(
    instrumentId: string,
    scheduleJson: string,
    discountCurveId: string,
    quotedClean?: number | null
  ): string;
}

export declare const cashflows: CashflowsNamespace;

// --- valuations ------------------------------------------------------------

export interface ValuationInstrumentsNamespace {
  validateInstrumentJson(json: string): string;
  priceInstrument(instrumentJson: string, marketJson: string, asOf: string, model: string): string;
  priceInstrumentWithMetrics(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model: string,
    metrics: string[],
    pricingOptions?: string | null
  ): string;
  listStandardMetrics(): string[];
  listStandardMetricsGrouped(): Record<string, string[]>;
}

export type FxInstrumentSpec = Record<string, unknown> | string;

export interface FxInstrument {
  toJSON(): string;
  price(marketJson: string, asOf: string, model?: string | null): string;
  priceWithMetrics(
    marketJson: string,
    asOf: string,
    metrics: string[],
    model?: string | null,
    pricingOptions?: string | null
  ): string;
}

export interface FxOptionInstrument extends FxInstrument {
  delta(marketJson: string, asOf: string, model?: string | null): number;
  gamma(marketJson: string, asOf: string, model?: string | null): number;
  vega(marketJson: string, asOf: string, model?: string | null): number;
  theta(marketJson: string, asOf: string, model?: string | null): number;
  rho(marketJson: string, asOf: string, model?: string | null): number;
  foreignRho(marketJson: string, asOf: string, model?: string | null): number;
  vanna(marketJson: string, asOf: string, model?: string | null): number;
  volga(marketJson: string, asOf: string, model?: string | null): number;
  greeks(marketJson: string, asOf: string, model?: string | null): Record<string, number>;
}

export interface FxInstrumentConstructor<T extends FxInstrument> {
  new (spec: FxInstrumentSpec): T;
  fromJSON(json: string): T;
}

export interface FxNamespace {
  FxSpot: FxInstrumentConstructor<FxInstrument>;
  FxForward: FxInstrumentConstructor<FxInstrument>;
  FxSwap: FxInstrumentConstructor<FxInstrument>;
  Ndf: FxInstrumentConstructor<FxInstrument>;
  FxOption: FxInstrumentConstructor<FxOptionInstrument>;
  FxDigitalOption: FxInstrumentConstructor<FxOptionInstrument>;
  FxTouchOption: FxInstrumentConstructor<FxOptionInstrument>;
  FxBarrierOption: FxInstrumentConstructor<FxOptionInstrument>;
  FxVarianceSwap: FxInstrumentConstructor<FxInstrument>;
  QuantoOption: FxInstrumentConstructor<FxOptionInstrument>;
}

// --- SABR (Stochastic Alpha Beta Rho) volatility -------------------------

export interface SabrParameters {
  readonly alpha: number;
  readonly beta: number;
  readonly nu: number;
  readonly rho: number;
  readonly shift: number | undefined;
  isShifted(): boolean;
}

export interface SabrParametersConstructor {
  new (alpha: number, beta: number, nu: number, rho: number, shift?: number): SabrParameters;
  /** Equity-standard defaults `(alpha=0.20, beta=1.0, nu=0.30, rho=-0.20)`. */
  equityDefault(): SabrParameters;
  /** Rates-standard defaults `(alpha=0.02, beta=0.5, nu=0.30, rho=0.0)`. */
  ratesDefault(): SabrParameters;
}

export interface SabrModel {
  impliedVol(forward: number, strike: number, t: number): number;
  supportsNegativeRates(): boolean;
}

export interface SabrModelConstructor {
  new (params: SabrParameters): SabrModel;
}

export interface SabrSmileArbitrageResult {
  arbitrageFree: boolean;
  butterflyViolations: Array<{
    strike: number;
    butterfly_value: number;
    severity_pct: number;
  }>;
  monotonicityViolations: Array<{
    strike_low: number;
    strike_high: number;
    price_low: number;
    price_high: number;
  }>;
}

export interface SabrSmile {
  atmVol(): number;
  impliedVol(strike: number): number;
  generateSmile(strikes: number[]): number[];
  arbitrageDiagnostics(strikes: number[], r?: number, q?: number): SabrSmileArbitrageResult;
}

export interface SabrSmileConstructor {
  new (params: SabrParameters, forward: number, t: number): SabrSmile;
}

export interface SabrCalibrator {
  calibrate(
    forward: number,
    strikes: number[],
    marketVols: number[],
    t: number,
    beta: number
  ): SabrParameters;
}

export interface SabrCalibratorConstructor {
  new (): SabrCalibrator;
  /** Tighter tolerance for production fits. */
  highPrecision(): SabrCalibrator;
}

export interface ValuationCreditNamespace {
  mertonModelJson(
    assetValue: number,
    assetVol: number,
    debtBarrier: number,
    riskFreeRate: number
  ): string;
  creditGradesModelJson(
    equityValue: number,
    equityVol: number,
    totalDebt: number,
    riskFreeRate: number,
    barrierUncertainty: number,
    meanRecovery: number
  ): string;
  mertonDefaultProbability(modelJson: string, horizon: number): number;
  dynamicRecoveryConstantJson(recovery: number): string;
  endogenousHazardPowerLawJson(baseHazard: number, baseLeverage: number, exponent: number): string;
  creditStateJson(
    hazardRate: number,
    leverage: number,
    accretedNotional: number,
    couponDue: number,
    distanceToDefault?: number | null,
    assetValue?: number | null
  ): string;
  toggleExerciseThresholdJson(
    variable: 'hazard_rate' | 'distance_to_default' | 'leverage',
    threshold: number,
    direction: 'above' | 'below'
  ): string;
  toggleExerciseOptimalJson(
    nestedPaths: number,
    equityDiscountRate: number,
    assetVol: number,
    riskFreeRate: number,
    horizon: number
  ): string;
}

export interface CreditDerivativesNamespace {
  creditDefaultSwapExampleJson(): string;
  cdsIndexExampleJson(): string;
  cdsTrancheExampleJson(): string;
  cdsOptionExampleJson(): string;
  validate(instrumentJson: string): string;
  priceInstrument(instrumentJson: string, marketJson: string, asOf: string, model: string): string;
  priceInstrumentWithMetrics(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model: string,
    metrics: string[],
    pricingOptions?: string | null
  ): string;
}

export interface ValuationsNamespace {
  /** Credit-correlation infrastructure (copulas, recovery, factor models). */
  correlation: CorrelationNamespace;
  /** Structural credit models and toggle-exercise helpers. */
  credit: ValuationCreditNamespace;
  /** CDS-family JSON wrappers and pricing helpers. */
  creditDerivatives: CreditDerivativesNamespace;
  /** Direct FX instrument wrappers. */
  fx: FxNamespace;
  /** Instrument JSON validation and pricing helpers. */
  instruments: ValuationInstrumentsNamespace;
  // --- Credit factor hierarchy ---
  /** Calibrated credit factor hierarchy artifact class. */
  CreditFactorModel: typeof CreditFactorModel;
  /** Deterministic credit factor calibrator class. */
  CreditCalibrator: typeof CreditCalibrator;
  /** Snapshot of hierarchy factor values at a date class (opaque handle). */
  LevelsAtDate: typeof LevelsAtDate;
  /** Period-over-period decomposition class (opaque handle). */
  PeriodDecomposition: typeof PeriodDecomposition;
  /** Vol-forecast view over a calibrated `CreditFactorModel` class. */
  FactorCovarianceForecast: typeof FactorCovarianceForecast;
  /** Decompose spreads at a point in time into per-level factor values. */
  decomposeLevels(
    model: CreditFactorModel,
    observedSpreadsJson: string,
    observedGeneric: number,
    asOf: string,
    runtimeTagsJson?: string
  ): LevelsAtDate;
  /** Difference two `LevelsAtDate` snapshots component-wise. */
  decomposePeriod(fromLevels: LevelsAtDate, toLevels: LevelsAtDate): PeriodDecomposition;
  validateValuationResultJson(json: string): string;
  validateInstrumentJson(json: string): string;
  priceInstrument(instrumentJson: string, marketJson: string, asOf: string, model: string): string;
  priceInstrumentWithMetrics(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model: string,
    metrics: string[],
    pricingOptions?: string | null
  ): string;
  /**
   * Per-flow cashflow envelope (DF / survival / PV) for a discountable
   * instrument. `model` must be `"discounting"` or `"hazard_rate"`; the
   * envelope's `total_pv` reconciles with `base_value` for supported pairs.
   */
  instrumentCashflowsJson(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    model: string
  ): string;
  listStandardMetrics(): string[];
  /** List all standard metrics organized by group.
   *  Returns an object mapping group name to sorted metric ID arrays. */
  listStandardMetricsGrouped(): Record<string, string[]>;
  /** Per-unit Black-Scholes / Garman-Kohlhagen price of a European option.
   *  All rates are continuously compounded decimals; `sigma` is annualized vol;
   *  `t` is years to expiry. */
  bsPrice(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    isCall: boolean
  ): number;
  /** Black-Scholes / Garman-Kohlhagen Greeks as a dict
   *  `{delta, gamma, vega, theta, rho, rhoQ}`. `vega` and both rho values are
   *  per 1% move; `theta` is per-day under the `thetaDays` day-count. */
  bsGreeks(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    isCall: boolean,
    thetaDays?: number
  ): {
    delta: number;
    gamma: number;
    vega: number;
    theta: number;
    rho: number;
    rhoQ: number;
  };
  /** Solve for Black-Scholes implied volatility given a target price. */
  bsImpliedVol(
    spot: number,
    strike: number,
    r: number,
    q: number,
    t: number,
    price: number,
    isCall: boolean
  ): number;
  /** Solve for Black-76 (forward-based) implied volatility given a target price. */
  black76ImpliedVol(
    forward: number,
    strike: number,
    df: number,
    t: number,
    price: number,
    isCall: boolean
  ): number;
  /** Reiner-Rubinstein continuous-monitoring barrier call.
   *  `direction` is `"up"`/`"down"`, `knock` is `"in"`/`"out"`. */
  barrierCall(
    spot: number,
    strike: number,
    barrier: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    direction: 'up' | 'down',
    knock: 'in' | 'out'
  ): number;
  /** Arithmetic (Turnbull-Wakeman) or geometric (Kemna-Vorst) Asian option. */
  asianOptionPrice(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    numFixings: number,
    averaging?: 'arithmetic' | 'geometric',
    isCall?: boolean
  ): number;
  /** Conze-Viswanathan lookback option. */
  lookbackOptionPrice(
    spot: number,
    strike: number,
    r: number,
    q: number,
    sigma: number,
    t: number,
    extremum: number,
    strikeType?: 'fixed' | 'floating',
    isCall?: boolean
  ): number;
  /** Quanto (FX-adjusted cross-currency) option price in domestic currency. */
  quantoOptionPrice(
    spot: number,
    strike: number,
    t: number,
    rateDomestic: number,
    rateForeign: number,
    divYield: number,
    volAsset: number,
    volFx: number,
    correlation: number,
    isCall?: boolean
  ): number;
  /** SABR parameters `(alpha, beta, nu, rho)` with optional `shift`. */
  SabrParameters: SabrParametersConstructor;
  /** Hagan-2002 SABR volatility model. */
  SabrModel: SabrModelConstructor;
  /** SABR smile generator for a fixed `(forward, t)` pair. */
  SabrSmile: SabrSmileConstructor;
  /** Levenberg-Marquardt SABR calibrator (beta fixed). */
  SabrCalibrator: SabrCalibratorConstructor;
  /** Black-Scholes European option price via the Fang-Oosterlee COS method. */
  bsCosPrice(
    spot: number,
    strike: number,
    rate: number,
    dividend: number,
    vol: number,
    maturity: number,
    isCall: boolean,
    nTerms?: number
  ): number;
  /** Variance Gamma European option price via the COS method. */
  vgCosPrice(
    spot: number,
    strike: number,
    rate: number,
    dividend: number,
    sigma: number,
    theta: number,
    nu: number,
    maturity: number,
    isCall: boolean,
    nTerms?: number
  ): number;
  /** Merton (1976) jump-diffusion European option price via the COS method. */
  mertonJumpCosPrice(
    spot: number,
    strike: number,
    rate: number,
    dividend: number,
    sigma: number,
    muJump: number,
    sigmaJump: number,
    lambda: number,
    maturity: number,
    isCall: boolean,
    nTerms?: number
  ): number;
  /** Simulated TARN coupon profile. Returns `{coupons_paid, cumulative, redemption_index, redeemed_early}`. */
  tarnCouponProfile(
    fixedRate: number,
    couponFloor: number,
    floatingFixings: number[],
    targetCoupon: number,
    dayCountFraction: number
  ): {
    coupons_paid: number[];
    cumulative: number[];
    redemption_index: number | null;
    redeemed_early: boolean;
  };
  /** Snowball / inverse-floater coupon schedule. */
  snowballCouponProfile(
    initialCoupon: number,
    fixedRate: number,
    floatingFixings: number[],
    floor: number,
    cap: number,
    isInverseFloater: boolean,
    leverage?: number
  ): number[];
  /** Intrinsic (undiscounted) payoff of a CMS spread option. */
  cmsSpreadOptionIntrinsic(
    longCms: number,
    shortCms: number,
    strike: number,
    isCall: boolean,
    notional: number
  ): number;
  /** Accrued coupon on a range-accrual leg given observed rates. */
  callableRangeAccrualAccrued(
    lower: number,
    upper: number,
    observations: number[],
    couponRate: number,
    dayCountFraction: number
  ): number;
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
  validateCheckSuiteSpec(json: string): string;
  validateCapitalStructureSpec(json: string): string;
  validateWaterfallSpec(json: string): string;
  validateEcfSweepSpec(json: string): string;
  validatePikToggleSpec(json: string): string;
  evaluateModel(modelJson: string): string;
  evaluateModelWithMarket(modelJson: string, marketJson: string, asOf: string): string;
  parseFormula(formula: string): string;
  validateFormula(formula: string): boolean;
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
  runVariance(baseJson: string, comparisonJson: string, configJson: string): string;
  evaluateScenarioSet(modelJson: string, scenarioSetJson: string): string;
  backtestForecast(actual: number[], forecast: number[]): BacktestForecastMetricsJson;
  generateTornadoEntries(resultJson: string, metricNode: string, period?: string): string;
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
  explainFormula(modelJson: string, resultsJson: string, nodeId: string, period: string): string;
  plSummaryReport(resultsJson: string, lineItems: string[], periods: string[]): string;
  creditAssessmentReport(resultsJson: string, asOf: string): string;
  // Comps — comparable company analysis
  percentileRank(value: number, data: number[]): number | null;
  zScore(value: number, data: number[]): number | null;
  peerStats(data: number[]): PeerStatsJson | null;
  regressionFairValue(
    xValues: number[],
    yValues: number[],
    subjectX: number,
    subjectY: number
  ): RegressionResultJson | null;
  computeMultiple(companyMetrics: unknown, multiple: string): number | null;
  scoreRelativeValue(peerSet: unknown, dimensions: unknown[]): RelativeValueResultJson;
}

export declare const statements_analytics: StatementsAnalyticsNamespace;

// --- portfolio -------------------------------------------------------------

export interface ScenarioRevalueResult {
  valuation: string;
  report: string;
}

/**
 * Typed handle to a built portfolio. Construct once via
 * `Portfolio.fromSpec` and reuse it across cashflow / valuation calls to
 * skip the per-call `PortfolioSpec` parse + rebuild cost.
 */
export declare class Portfolio {
  private constructor();
  static fromSpec(specJson: string): Portfolio;
  readonly id: string;
  readonly asOf: string;
  readonly baseCcy: string;
  numPositions(): number;
  toSpecJson(): string;
  free(): void;
}

export interface PortfolioNamespace {
  /** Typed handle for cached portfolio builds. */
  Portfolio: typeof Portfolio;
  parsePortfolioSpec(jsonStr: string): string;
  buildPortfolioFromSpec(specJson: string): string;
  portfolioResultTotalValue(resultJson: string): number;
  portfolioResultGetMetric(resultJson: string, metricId: string): number | undefined;
  aggregateMetrics(
    valuationJson: string,
    baseCcy: string,
    marketJson: string,
    asOf: string
  ): string;
  valuePortfolio(specJson: string, marketJson: string, strictRisk: boolean): string;
  aggregateFullCashflows(specJson: string, marketJson: string): string;
  /**
   * Fast-path cashflow aggregation that reuses a built `Portfolio` handle.
   * Skips the `PortfolioSpec` parse + `Portfolio::from_spec` rebuild cost.
   */
  aggregateFullCashflowsBuilt(portfolio: Portfolio, marketJson: string): string;
  applyScenarioAndRevalue(
    specJson: string,
    scenarioJson: string,
    marketJson: string
  ): ScenarioRevalueResult;
  /** Optimize portfolio weights using the LP-based optimizer. */
  optimizePortfolio(specJson: string, marketJson: string): string;
  replayPortfolio(specJson: string, snapshotsJson: string, configJson: string): string;
  parametricVarDecomposition(
    positionIdsJson: string,
    weightsJson: string,
    covarianceJson: string,
    confidence: number
  ): string;
  parametricEsDecomposition(
    positionIdsJson: string,
    weightsJson: string,
    covarianceJson: string,
    confidence: number
  ): string;
  historicalVarDecomposition(
    positionIdsJson: string,
    positionPnlsJson: string,
    confidence: number
  ): string;
  evaluateRiskBudget(
    positionIdsJson: string,
    actualVarJson: string,
    targetVarPctJson: string,
    portfolioVar: number,
    utilizationThreshold: number
  ): string;
  rollEffectiveSpread(returnsJson: string): number;
  amihudIlliquidity(returnsJson: string, volumesJson: string): number;
  daysToLiquidate(positionValue: number, avgDailyVolume: number, participationRate: number): number;
  liquidityTier(daysToLiquidate: number): string;
  lvarBangia(
    varValue: number,
    spreadMean: number,
    spreadVol: number,
    confidence: number,
    positionValue: number
  ): string;
  almgrenChrissImpact(
    positionSize: number,
    avgDailyVolume: number,
    volatility: number,
    executionHorizonDays: number,
    permanentImpactCoef: number,
    temporaryImpactCoef: number,
    referencePrice?: number | null
  ): string;
  kyleLambda(volumesJson: string, returnsJson: string): number;
}

export declare const portfolio: PortfolioNamespace;

// --- scenarios -------------------------------------------------------------

export interface ScenarioApplyResult {
  market_json: string;
  model_json: string;
  operations_applied: number;
  user_operations: number;
  expanded_operations: number;
  warnings: string[];
}

export interface ScenarioApplyMarketResult {
  market_json: string;
  operations_applied: number;
  user_operations: number;
  expanded_operations: number;
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
  computeHorizonReturn(
    instrumentJson: string,
    market: unknown,
    asOf: string,
    scenarioJson: string,
    method?: string,
    config?: string
  ): string;
}

export declare const scenarios: ScenariosNamespace;
