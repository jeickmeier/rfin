// TypeScript declarations for the finstack-wasm bindings.
// These signatures mirror the wasm-bindgen exports implemented in src/lib.rs
// and enable downstream consumers to use the ported APIs with full intellisense.

export type InitInput =
  | RequestInfo
  | URL
  | Response
  | BufferSource
  | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
}

export interface InitOptions {
  /** Optional WebAssembly imports provided when instantiating synchronously. */
  readonly imports?: WebAssembly.Imports;
}

export default function init (
  module?: InitInput | Promise<InitInput>,
  maybeMemory?: WebAssembly.Memory
): Promise<InitOutput>;

export function initSync(
  module: BufferSource | WebAssembly.Module,
  maybeMemory?: WebAssembly.Memory
): InitOutput;

export enum RoundingMode {
  Bankers = 0,
  AwayFromZero = 1,
  TowardZero = 2,
  Floor = 3,
  Ceil = 4,
}

export enum BusinessDayConvention {
  Unadjusted = 0,
  Following = 1,
  ModifiedFollowing = 2,
  Preceding = 3,
  ModifiedPreceding = 4,
}

export enum InterpStyle {
  Linear = 0,
  LogLinear = 1,
  MonotoneConvex = 2,
  CubicHermite = 3,
  FlatFwd = 4,
}

export enum ExtrapolationPolicy {
  FlatZero = 0,
  FlatForward = 1,
}

export enum SeriesInterpolation {
  Step = 0,
  Linear = 1,
}

export enum FxConversionPolicy {
  CashflowDate = 0,
  PeriodEnd = 1,
  PeriodAverage = 2,
  Custom = 3,
}

export type CurrencyTuple = [code: string, numeric: number, decimals: number];
export type MoneyTuple = [amount: number, currency: Currency | string];

export class Currency {
  static fromNumeric(numeric: number): Currency;
  static all(): Currency[];
  constructor(code: string);
  free(): void;
  readonly code: string;
  readonly numeric: number;
  readonly decimals: number;
  toTuple(): CurrencyTuple;
}

export class FinstackConfig {
  constructor();
  free(): void;
  copy(): FinstackConfig;
  readonly roundingMode: RoundingMode;
  setRoundingMode(mode: RoundingMode): void;
  setRoundingModeLabel(label: string): void;
  ingestScale(currency: Currency): number;
  setIngestScale(currency: Currency, decimals: number): void;
  outputScale(currency: Currency): number;
  setOutputScale(currency: Currency, decimals: number): void;
}

export class Money {
  static zero(currency: Currency): Money;
  static fromTuple(value: MoneyTuple | Money): Money;
  static fromCode(amount: number, code: string): Money;
  static fromConfig(amount: number, currency: Currency, config: FinstackConfig): Money;
  constructor(amount: number, currency: Currency);
  free(): void;
  readonly amount: number;
  readonly currency: Currency;
  toTuple(): [number, Currency];
  format(): string;
}

export class Date {
  constructor(year: number, month: number, day: number);
  free(): void;
  readonly year: number;
  readonly month: number;
  readonly day: number;
  toString(): string;
  equals(other: Date): boolean;
  isWeekend(): boolean;
  quarter(): number;
  fiscalYear(): number;
  addWeekdays(offset: number): Date;
}

export class Calendar {
  constructor(code: string);
  free(): void;
  readonly code: string;
  readonly name: string;
  readonly ignoreWeekends: boolean;
  isBusinessDay(date: Date): boolean;
  isHoliday(date: Date): boolean;
  toString(): string;
}

export class Frequency {
  constructor(months: number);
  free(): void;
  static fromMonths(months: number): Frequency;
  static fromDays(days: number): Frequency;
  static fromPaymentsPerYear(paymentsPerYear: number): Frequency;
  static annual(): Frequency;
  static semiAnnual(): Frequency;
  static quarterly(): Frequency;
  static monthly(): Frequency;
  static biMonthly(): Frequency;
  static biWeekly(): Frequency;
  static weekly(): Frequency;
  static daily(): Frequency;
  readonly months: number | undefined;
  readonly days: number | undefined;
  toString(): string;
}

export class DayCountContext {
  constructor();
  free(): void;
  setCalendar(calendar: Calendar): void;
  setCalendarCode(code: string): void;
  clearCalendar(): void;
  setFrequency(frequency: Frequency): void;
  clearFrequency(): void;
  readonly calendarCode: string | undefined;
  readonly frequency: Frequency | undefined;
}

export class DayCount {
  static newFromName(name: string): DayCount;
  static act360(): DayCount;
  static act365f(): DayCount;
  static act365l(): DayCount;
  static thirty360(): DayCount;
  static thirtyE360(): DayCount;
  static actAct(): DayCount;
  static actActIsma(): DayCount;
  static bus252(): DayCount;
  constructor(name: string);
  free(): void;
  readonly name: string;
  yearFraction(start: Date, end: Date, context?: DayCountContext | null): number;
}

export class StubKind {
  static none(): StubKind;
  static shortFront(): StubKind;
  static shortBack(): StubKind;
  static longFront(): StubKind;
  static longBack(): StubKind;
  static fromName(name: string): StubKind;
  constructor(name: string);
  free(): void;
  name(): string;
}

export class ScheduleBuilder {
  constructor(start: Date, end: Date);
  free(): void;
  frequency(frequency: Frequency): ScheduleBuilder;
  stubRule(stub: StubKind): ScheduleBuilder;
  adjustWith(convention: BusinessDayConvention, calendar: Calendar): ScheduleBuilder;
  endOfMonth(enabled: boolean): ScheduleBuilder;
  cdsImm(): ScheduleBuilder;
  build(): Schedule;
  toString(): string;
}

export class Schedule {
  free(): void;
  readonly length: number;
  toArray(): Date[];
}

export class DiscountCurve {
  constructor(
    id: string,
    baseDate: Date,
    times: number[],
    discountFactors: number[],
    dayCount?: string | null,
    interp?: InterpStyle | string | null,
    extrapolation?: ExtrapolationPolicy | string | null,
    requireMonotonic?: boolean
  );
  free(): void;
  readonly id: string;
  readonly baseDate: Date;
  dayCount(): string;
  df(time: number): number;
  zero(time: number): number;
  forward(t1: number, t2: number): number;
  dfOnDate(date: Date, dayCount?: string | null): number;
}

export class ForwardCurve {
  constructor(
    id: string,
    baseDate: Date,
    tenorYears: number,
    times: number[],
    forwards: number[],
    dayCount?: string | null,
    resetLag?: number | null,
    interp?: InterpStyle | string | null
  );
  free(): void;
  readonly id: string;
  readonly baseDate: Date;
  readonly resetLag: number;
  readonly tenor: number;
  dayCount(): string;
  rate(time: number): number;
  ratePeriod(t1: number, t2: number): number;
}

export class HazardCurve {
  constructor(
    id: string,
    baseDate: Date,
    times: number[],
    hazardRates: number[],
    recoveryRate?: number | null,
    dayCount?: string | null,
    issuer?: string | null,
    seniority?: string | null,
    currency?: string | null,
    parTenors?: number[] | null,
    parSpreadsBp?: number[] | null
  );
  free(): void;
  readonly id: string;
  readonly baseDate: Date;
  recoveryRate(): number;
  dayCount(): string;
  sp(time: number): number;
  defaultProb(t1: number, t2: number): number;
}

export class InflationCurve {
  constructor(
    id: string,
    baseCpi: number,
    times: number[],
    cpiLevels: number[],
    interp?: InterpStyle | string | null
  );
  free(): void;
  readonly id: string;
  readonly baseCpi: number;
  cpi(time: number): number;
  inflationRate(t1: number, t2: number): number;
}

export class BaseCorrelationCurve {
  constructor(id: string, detachmentPoints: number[], correlations: number[]);
  free(): void;
  readonly id: string;
  correlation(detachmentPct: number): number;
  points(): Array<[number, number]>;
}

export class MarketScalar {
  static unitless(value: number): MarketScalar;
  static price(money: Money): MarketScalar;
  free(): void;
  readonly isUnitless: boolean;
  readonly isPrice: boolean;
  readonly value: number | Money;
}

export class ScalarTimeSeries {
  constructor(
    id: string,
    dates: Date[],
    values: number[],
    currency?: Currency | null,
    interpolation?: SeriesInterpolation | null
  );
  free(): void;
  setInterpolation(interpolation: SeriesInterpolation): void;
  readonly id: string;
  readonly currency?: Currency;
  readonly interpolation: SeriesInterpolation;
  valueOn(date: Date): number;
  valuesOn(dates: Date[]): number[];
}

export class VolSurface {
  constructor(id: string, expiries: number[], strikes: number[], vols: number[]);
  free(): void;
  readonly id: string;
  readonly expiries: number[];
  readonly strikes: number[];
  gridShape(): [number, number];
  value(expiry: number, strike: number): number;
  valueChecked(expiry: number, strike: number): number;
  valueClamped(expiry: number, strike: number): number;
}

export class FxConfig {
  constructor();
  free(): void;
  readonly pivotCurrency: Currency;
  setPivotCurrency(currency: Currency): void;
  readonly enableTriangulation: boolean;
  setEnableTriangulation(flag: boolean): void;
  readonly cacheCapacity: number;
  setCacheCapacity(capacity: number): void;
}

export class FxRateResult {
  private constructor();
  free(): void;
  readonly rate: number;
  readonly triangulated: boolean;
}

export class FxMatrix {
  constructor();
  static withConfig(config: FxConfig): FxMatrix;
  free(): void;
  setQuote(from: Currency, to: Currency, rate: number): void;
  setQuotes(quotes: Array<[string, string, number]>): void;
  rate(
    from: Currency,
    to: Currency,
    on: Date,
    policy?: FxConversionPolicy | string | number | null
  ): FxRateResult;
  cacheStats(): [number, number];
  clearCache(): void;
  clearExpired(): void;
}

export class DividendEvent {
  private constructor();
  free(): void;
  readonly date: Date;
  readonly kind: 'cash' | 'yield' | 'stock';
  readonly cashAmount?: Money;
  readonly dividendYield?: number;
  readonly stockRatio?: number;
}

export class DividendSchedule {
  private constructor();
  free(): void;
  readonly id: string;
  readonly underlying?: string;
  readonly currency?: Currency;
  readonly events: DividendEvent[];
  eventsBetween(start: Date, end: Date): DividendEvent[];
}

export class DividendScheduleBuilder {
  constructor(id: string);
  free(): void;
  underlying(name: string): void;
  currency(currency: Currency): void;
  cash(date: Date, amount: Money): void;
  yieldDividend(date: Date, dividendYield: number): void;
  stock(date: Date, ratio: number): void;
  build(): DividendSchedule;
}

export class CreditIndexData {
  constructor(
    numConstituents: number,
    recoveryRate: number,
    indexCurve: HazardCurve,
    baseCorrelationCurve: BaseCorrelationCurve,
    issuerIds?: string[] | null,
    issuerCurves?: HazardCurve[] | null
  );
  free(): void;
  readonly numConstituents: number;
  readonly recoveryRate: number;
  readonly indexCurve: HazardCurve;
  readonly baseCorrelationCurve: BaseCorrelationCurve;
  hasIssuerCurves(): boolean;
  issuerIds(): string[];
  issuerCurve(issuerId: string): HazardCurve | undefined;
}

export interface MarketContextStats {
  totalCurves: number;
  surfaceCount: number;
  priceCount: number;
  seriesCount: number;
  inflationIndexCount: number;
  creditIndexCount: number;
  dividendScheduleCount: number;
  collateralMappingCount: number;
  hasFx: boolean;
  curveCounts: Record<string, number>;
}

export class MarketContext {
  constructor();
  free(): void;
  clone(): MarketContext;
  insertDiscount(curve: DiscountCurve): void;
  insertForward(curve: ForwardCurve): void;
  insertHazard(curve: HazardCurve): void;
  insertInflation(curve: InflationCurve): void;
  insertBaseCorrelation(curve: BaseCorrelationCurve): void;
  insertSurface(surface: VolSurface): void;
  insertPrice(id: string, scalar: MarketScalar): void;
  insertSeries(series: ScalarTimeSeries): void;
  insertDividends(schedule: DividendSchedule): void;
  insertCreditIndex(id: string, data: CreditIndexData): void;
  insertFx(matrix: FxMatrix): void;
  mapCollateral(csaCode: string, curveId: string): void;
  discount(id: string): DiscountCurve;
  forward(id: string): ForwardCurve;
  hazard(id: string): HazardCurve;
  inflation(id: string): InflationCurve;
  baseCorrelation(id: string): BaseCorrelationCurve;
  surface(id: string): VolSurface;
  price(id: string): MarketScalar;
  series(id: string): ScalarTimeSeries;
  creditIndex(id: string): CreditIndexData;
  dividendSchedule(id: string): DividendSchedule | undefined;
  curveIds(): string[];
  curveIdsByType(curveType: string): string[];
  countByType(): Record<string, number>;
  stats(): MarketContextStats;
  isEmpty(): boolean;
  totalObjects(): number;
  hasFx(): boolean;
}

export class FiscalConfig {
  static calendarYear(): FiscalConfig;
  static usFederal(): FiscalConfig;
  static uk(): FiscalConfig;
  static japan(): FiscalConfig;
  static canada(): FiscalConfig;
  static australia(): FiscalConfig;
  static germany(): FiscalConfig;
  static france(): FiscalConfig;
  constructor(startMonth: number, startDay: number);
  free(): void;
  readonly startMonth: number;
  readonly startDay: number;
  toString(): string;
}

export class PeriodId {
  static parse(code: string): PeriodId;
  static quarter(year: number, quarter: number): PeriodId;
  static month(year: number, month: number): PeriodId;
  static week(year: number, week: number): PeriodId;
  static half(year: number, half: number): PeriodId;
  static annual(year: number): PeriodId;
  constructor(code: string);
  free(): void;
  readonly code: string;
  readonly year: number;
  readonly index: number;
  readonly kind: string;
  toString(): string;
}

export class Period {
  free(): void;
  readonly id: PeriodId;
  readonly start: Date;
  readonly end: Date;
  readonly isActual: boolean;
  toString(): string;
}

export class PeriodPlan {
  free(): void;
  readonly length: number;
  toArray(): Period[];
}

export function buildPeriods(range: string, actualsUntil?: string | null): PeriodPlan;
export function buildFiscalPeriods(
  range: string,
  config: FiscalConfig,
  actualsUntil?: string | null
): PeriodPlan;

export function availableCalendars(): Calendar[];
export function availableCalendarCodes(): string[];
export function getCalendar(code: string): Calendar;
export function adjust(date: Date, convention: BusinessDayConvention, calendar: Calendar): Date;
export function businessDayConventionFromName(name: string): BusinessDayConvention;
export function businessDayConventionName(convention: BusinessDayConvention): string;

export function addMonths(date: Date, months: number): Date;
export function lastDayOfMonth(date: Date): Date;
export function daysInMonth(year: number, month: number): number;
export function isLeapYear(year: number): boolean;
export function dateToDaysSinceEpoch(date: Date): number;
export function daysSinceEpochToDate(days: number): Date;

export function nextImm(date: Date): Date;
export function nextCdsDate(date: Date): Date;
export function nextImmOptionExpiry(date: Date): Date;
export function immOptionExpiry(year: number, month: number): Date;
export function nextEquityOptionExpiry(date: Date): Date;
export function thirdFriday(year: number, month: number): Date;
export function thirdWednesday(year: number, month: number): Date;
