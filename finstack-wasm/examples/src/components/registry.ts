import { ComponentType } from 'react';

import { CalibrationExample } from './CalibrationExample';
import { CashflowBasicsExample } from './CashflowBasics';
import { CashflowBuilderExample } from './CashflowBuilderExample';
import { MarketDataExample } from './DatesAndMarketData';
import { BondsValuationExample } from './BondsValuation';
import {
  CalendarExample,
  DateConstructionExample,
  DateUtilitiesExample,
  DayCountExample,
  FrequencyExample,
  IMMDatesExample,
  PeriodPlansExample,
  ScheduleBuilderExample,
} from './DatesShowcase';
import { DepositValuationExample } from './DepositsValuation';
import { MathShowcaseExample } from './MathShowcase';
import { RatesInstrumentsExample } from './RatesInstruments';
import { FxInstrumentsExample } from './FxInstruments';
import { CreditInstrumentsExample } from './CreditInstruments';
import { EquityInstrumentsExample } from './EquityInstruments';
import { InflationInstrumentsExample } from './InflationInstruments';
import { StructuredProductsExample } from './StructuredProducts';
import { StructuredCreditExample } from './StructuredCreditExample';
import { ExoticEquityOptionsExample } from './ExoticEquityOptions';
import { ExoticFxDerivativesExample } from './ExoticFxDerivatives';
import { ExoticRatesDerivativesExample } from './ExoticRatesDerivatives';
import { MonteCarloPathExample } from './MonteCarloPathExample';
import StatementsModeling from './StatementsModeling';
import ScenariosExample from './ScenariosExample';
import PortfolioExample from './PortfolioExample';

// Import default props for components that support them
import { DEFAULT_DEPOSIT_PROPS } from './data/deposits';
import { DEFAULT_EQUITY_PROPS } from './data/equity';
import { DEFAULT_FX_PROPS } from './data/fx';
import { DEFAULT_INFLATION_PROPS } from './data/inflation';
import { DEFAULT_CASHFLOW_PROPS } from './data/cashflows';
import { DEFAULT_BONDS_PROPS } from './data/bonds';
import { DEFAULT_RATES_PROPS } from './data/rates';
import { DEFAULT_CREDIT_PROPS } from './data/credit';
import { DEFAULT_CASHFLOW_BUILDER_PROPS } from './data/cashflow-builder';
import { DEFAULT_MARKET_DATA_EXAMPLE_PROPS } from './data/market-data-example';
import {
  DEFAULT_DATE_CONSTRUCTION,
  DEFAULT_DATE_UTILITIES,
  DEFAULT_CALENDAR_EXAMPLE,
  DEFAULT_DAY_COUNT_EXAMPLE,
  DEFAULT_SCHEDULE_BUILDER,
  DEFAULT_PERIOD_PLANS,
  DEFAULT_IMM_DATES,
} from './data/dates-showcase';
import { DEFAULT_EXOTIC_EQUITY_PROPS } from './data/exotic-equity';
import { DEFAULT_EXOTIC_FX_PROPS } from './data/exotic-fx';
import { DEFAULT_EXOTIC_RATES_PROPS } from './data/exotic-rates';
import { DEFAULT_STRUCTURED_PRODUCTS_PROPS } from './data/structured-products';
import { DEFAULT_MATH_SHOWCASE_PROPS } from './data/math-showcase';
import { DEFAULT_MONTE_CARLO_PROPS } from './data/monte-carlo';
import { DEFAULT_PORTFOLIO_PROPS } from './data/portfolio';

export type ExampleDefinition<P = any> = {
  slug: string;
  title: string;
  description: string;
  group: string;
  Component: ComponentType<P>;
  defaultProps?: P;
};

export const EXAMPLES: ExampleDefinition[] = [
  {
    slug: 'date-construction',
    title: 'Date Construction & Properties',
    description: 'Create finstack dates and inspect fiscal, quarter, and weekday metadata.',
    group: 'Dates & Calendars',
    Component: DateConstructionExample,
    defaultProps: DEFAULT_DATE_CONSTRUCTION,
  },
  {
    slug: 'date-utilities',
    title: 'Date Utilities',
    description: 'Apply month arithmetic, leap-year checks, and epoch conversions.',
    group: 'Dates & Calendars',
    Component: DateUtilitiesExample,
    defaultProps: DEFAULT_DATE_UTILITIES,
  },
  {
    slug: 'calendar-business-days',
    title: 'Calendars & Business Days',
    description: 'Browse calendar codes and adjust dates with different conventions.',
    group: 'Dates & Calendars',
    Component: CalendarExample,
    defaultProps: DEFAULT_CALENDAR_EXAMPLE,
  },
  {
    slug: 'day-counts',
    title: 'Day Count Fractions',
    description: 'Compare ACT/365F, 30/360, and other day count calculations.',
    group: 'Dates & Calendars',
    Component: DayCountExample,
    defaultProps: DEFAULT_DAY_COUNT_EXAMPLE,
  },
  {
    slug: 'schedule-builder',
    title: 'Schedule Builder',
    description: 'Generate coupon schedules using stub rules and business-day conventions.',
    group: 'Dates & Calendars',
    Component: ScheduleBuilderExample,
    defaultProps: DEFAULT_SCHEDULE_BUILDER,
  },
  {
    slug: 'period-plans',
    title: 'Period Plans',
    description: 'Build fiscal periods and custom accrual plans for reporting.',
    group: 'Dates & Calendars',
    Component: PeriodPlansExample,
    defaultProps: DEFAULT_PERIOD_PLANS,
  },
  {
    slug: 'imm-dates',
    title: 'IMM & Derivatives Dates',
    description: 'Produce IMM, CDS, and options expiry dates from core utilities.',
    group: 'Dates & Calendars',
    Component: IMMDatesExample,
    defaultProps: DEFAULT_IMM_DATES,
  },
  {
    slug: 'frequency-helpers',
    title: 'Frequency Helpers',
    description: 'Translate frequency codes into periods and payment intervals.',
    group: 'Dates & Calendars',
    Component: FrequencyExample,
  },
  {
    slug: 'market-data',
    title: 'Market Data Snapshot',
    description: 'Assemble discount curves, CPI series, FX matrices, and equity prices.',
    group: 'Market Data',
    Component: MarketDataExample,
    defaultProps: DEFAULT_MARKET_DATA_EXAMPLE_PROPS,
  },
  {
    slug: 'cashflow-basics',
    title: 'Cashflow Primitives',
    description: 'Construct fixed, floating, fee, and principal cashflows with tuple views.',
    group: 'Cashflows',
    Component: CashflowBasicsExample,
    defaultProps: DEFAULT_CASHFLOW_PROPS,
  },
  {
    slug: 'cashflow-builder',
    title: 'Cashflow Builder',
    description:
      'Composable builder for complex coupon structures: fixed/floating, cash/PIK, amortization, step-ups.',
    group: 'Cashflows',
    Component: CashflowBuilderExample,
    defaultProps: DEFAULT_CASHFLOW_BUILDER_PROPS,
  },
  {
    slug: 'math-showcase',
    title: 'Math Utilities',
    description: 'Evaluate integrals, solve equations, and inspect discrete distributions.',
    group: 'Math',
    Component: MathShowcaseExample,
    defaultProps: DEFAULT_MATH_SHOWCASE_PROPS,
  },
  {
    slug: 'bond-valuations',
    title: 'Bond Instruments & Metrics',
    description:
      'Create fixed, zero, floating, and callable bonds; compute PV, duration, and DV01.',
    group: 'Valuations',
    Component: BondsValuationExample,
    defaultProps: DEFAULT_BONDS_PROPS,
  },
  {
    slug: 'deposit-valuations',
    title: 'Deposit Valuation',
    description:
      'Accrue interest on money-market deposits and compare quoted vs curve-implied rates.',
    group: 'Valuations',
    Component: DepositValuationExample,
    defaultProps: DEFAULT_DEPOSIT_PROPS,
  },
  {
    slug: 'rates-instruments',
    title: 'Interest Rate Derivatives',
    description: 'IRS, FRA, swaptions, basis swaps, caps/floors, and IR futures.',
    group: 'Valuations',
    Component: RatesInstrumentsExample,
    defaultProps: DEFAULT_RATES_PROPS,
  },
  {
    slug: 'fx-instruments',
    title: 'FX Instruments',
    description: 'FX spot, options, and swaps with multi-currency pricing.',
    group: 'Valuations',
    Component: FxInstrumentsExample,
    defaultProps: DEFAULT_FX_PROPS,
  },
  {
    slug: 'credit-instruments',
    title: 'Credit Derivatives',
    description: 'CDS, CDS indices, tranches, and options on credit spreads.',
    group: 'Valuations',
    Component: CreditInstrumentsExample,
    defaultProps: DEFAULT_CREDIT_PROPS,
  },
  {
    slug: 'equity-instruments',
    title: 'Equity Instruments',
    description: 'Equity positions and European-style equity options.',
    group: 'Valuations',
    Component: EquityInstrumentsExample,
    defaultProps: DEFAULT_EQUITY_PROPS,
  },
  {
    slug: 'inflation-instruments',
    title: 'Inflation Instruments',
    description: 'Inflation-linked bonds (TIPS) and zero-coupon inflation swaps.',
    group: 'Valuations',
    Component: InflationInstrumentsExample,
    defaultProps: DEFAULT_INFLATION_PROPS,
  },
  {
    slug: 'structured-products',
    title: 'Structured Products',
    description: 'Baskets, ABS, CLO, and private markets funds with JSON definitions.',
    group: 'Valuations',
    Component: StructuredProductsExample,
    defaultProps: DEFAULT_STRUCTURED_PRODUCTS_PROPS,
  },
  {
    slug: 'structured-credit',
    title: 'Structured Credit Securities',
    description: 'CLO, ABS, RMBS, and CMBS with tranching, waterfalls, and prepayment models.',
    group: 'Valuations',
    Component: StructuredCreditExample,
  },
  {
    slug: 'exotic-equity-options',
    title: 'Exotic Equity Options',
    description: 'Barrier, Asian, Lookback, and Cliquet options with Monte Carlo pricing.',
    group: 'Valuations',
    Component: ExoticEquityOptionsExample,
    defaultProps: DEFAULT_EXOTIC_EQUITY_PROPS,
  },
  {
    slug: 'exotic-fx-derivatives',
    title: 'Exotic FX Derivatives',
    description: 'FX barrier options and quanto options with multi-currency setup.',
    group: 'Valuations',
    Component: ExoticFxDerivativesExample,
    defaultProps: DEFAULT_EXOTIC_FX_PROPS,
  },
  {
    slug: 'exotic-rates-derivatives',
    title: 'Exotic Rates Derivatives',
    description: 'CMS options and range accrual notes.',
    group: 'Valuations',
    Component: ExoticRatesDerivativesExample,
    defaultProps: DEFAULT_EXOTIC_RATES_PROPS,
  },
  {
    slug: 'monte-carlo-paths',
    title: 'Monte Carlo Path Generation',
    description: 'Generate and visualize stochastic paths with GBM and other processes.',
    group: 'Valuations',
    Component: MonteCarloPathExample,
    defaultProps: DEFAULT_MONTE_CARLO_PROPS,
  },
  {
    slug: 'calibration',
    title: 'Curve Calibration',
    description:
      'Calibrate discount and forward curves from market quotes using numerical optimization.',
    group: 'Calibration',
    Component: CalibrationExample,
  },
  {
    slug: 'statements-modeling',
    title: 'Financial Statements Modeling',
    description:
      'Build and evaluate financial statement models with formulas, forecasts, and dynamic metrics.',
    group: 'Statements',
    Component: StatementsModeling,
  },
  {
    slug: 'scenarios-stress-testing',
    title: 'Scenarios & Stress Testing',
    description:
      'Apply deterministic market shocks, statement adjustments, and time roll-forwards with full composability.',
    group: 'Scenarios',
    Component: ScenariosExample,
  },
  {
    slug: 'portfolio-management',
    title: 'Portfolio Management',
    description:
      'Create entities, build portfolios, value positions, and aggregate metrics with cross-currency support.',
    group: 'Portfolio',
    Component: PortfolioExample,
    defaultProps: DEFAULT_PORTFOLIO_PROPS,
  },
];

export const getExampleBySlug = (slug: string): ExampleDefinition | undefined =>
  EXAMPLES.find((example) => example.slug === slug);
