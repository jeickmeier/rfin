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
import StatementsModeling from './StatementsModeling';
import ScenariosExample from './ScenariosExample';
import PortfolioExample from './PortfolioExample';

export type ExampleDefinition = {
  slug: string;
  title: string;
  description: string;
  group: string;
  Component: ComponentType;
};

export const EXAMPLES: ExampleDefinition[] = [
  {
    slug: 'date-construction',
    title: 'Date Construction & Properties',
    description: 'Create finstack dates and inspect fiscal, quarter, and weekday metadata.',
    group: 'Dates & Calendars',
    Component: DateConstructionExample,
  },
  {
    slug: 'date-utilities',
    title: 'Date Utilities',
    description: 'Apply month arithmetic, leap-year checks, and epoch conversions.',
    group: 'Dates & Calendars',
    Component: DateUtilitiesExample,
  },
  {
    slug: 'calendar-business-days',
    title: 'Calendars & Business Days',
    description: 'Browse calendar codes and adjust dates with different conventions.',
    group: 'Dates & Calendars',
    Component: CalendarExample,
  },
  {
    slug: 'day-counts',
    title: 'Day Count Fractions',
    description: 'Compare ACT/365F, 30/360, and other day count calculations.',
    group: 'Dates & Calendars',
    Component: DayCountExample,
  },
  {
    slug: 'schedule-builder',
    title: 'Schedule Builder',
    description: 'Generate coupon schedules using stub rules and business-day conventions.',
    group: 'Dates & Calendars',
    Component: ScheduleBuilderExample,
  },
  {
    slug: 'period-plans',
    title: 'Period Plans',
    description: 'Build fiscal periods and custom accrual plans for reporting.',
    group: 'Dates & Calendars',
    Component: PeriodPlansExample,
  },
  {
    slug: 'imm-dates',
    title: 'IMM & Derivatives Dates',
    description: 'Produce IMM, CDS, and options expiry dates from core utilities.',
    group: 'Dates & Calendars',
    Component: IMMDatesExample,
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
  },
  {
    slug: 'cashflow-basics',
    title: 'Cashflow Primitives',
    description: 'Construct fixed, floating, fee, and principal cashflows with tuple views.',
    group: 'Cashflows',
    Component: CashflowBasicsExample,
  },
  {
    slug: 'cashflow-builder',
    title: 'Cashflow Builder',
    description: 'Composable builder for complex coupon structures: fixed/floating, cash/PIK, amortization, step-ups.',
    group: 'Cashflows',
    Component: CashflowBuilderExample,
  },
  {
    slug: 'math-showcase',
    title: 'Math Utilities',
    description: 'Evaluate integrals, solve equations, and inspect discrete distributions.',
    group: 'Math',
    Component: MathShowcaseExample,
  },
  {
    slug: 'bond-valuations',
    title: 'Bond Instruments & Metrics',
    description: 'Create fixed, zero, floating, and callable bonds; compute PV, duration, and DV01.',
    group: 'Valuations',
    Component: BondsValuationExample,
  },
  {
    slug: 'deposit-valuations',
    title: 'Deposit Valuation',
    description: 'Accrue interest on money-market deposits and compare quoted vs curve-implied rates.',
    group: 'Valuations',
    Component: DepositValuationExample,
  },
  {
    slug: 'rates-instruments',
    title: 'Interest Rate Derivatives',
    description: 'IRS, FRA, swaptions, basis swaps, caps/floors, and IR futures.',
    group: 'Valuations',
    Component: RatesInstrumentsExample,
  },
  {
    slug: 'fx-instruments',
    title: 'FX Instruments',
    description: 'FX spot, options, and swaps with multi-currency pricing.',
    group: 'Valuations',
    Component: FxInstrumentsExample,
  },
  {
    slug: 'credit-instruments',
    title: 'Credit Derivatives',
    description: 'CDS, CDS indices, tranches, and options on credit spreads.',
    group: 'Valuations',
    Component: CreditInstrumentsExample,
  },
  {
    slug: 'equity-instruments',
    title: 'Equity Instruments',
    description: 'Equity positions and European-style equity options.',
    group: 'Valuations',
    Component: EquityInstrumentsExample,
  },
  {
    slug: 'inflation-instruments',
    title: 'Inflation Instruments',
    description: 'Inflation-linked bonds (TIPS) and zero-coupon inflation swaps.',
    group: 'Valuations',
    Component: InflationInstrumentsExample,
  },
  {
    slug: 'structured-products',
    title: 'Structured Products',
    description: 'Baskets, ABS, CLO, and private markets funds with JSON definitions.',
    group: 'Valuations',
    Component: StructuredProductsExample,
  },
  {
    slug: 'structured-credit',
    title: 'Structured Credit Securities',
    description: 'CLO, ABS, RMBS, and CMBS with tranching, waterfalls, and prepayment models.',
    group: 'Valuations',
    Component: StructuredCreditExample,
  },
  {
    slug: 'calibration',
    title: 'Curve Calibration',
    description: 'Calibrate discount and forward curves from market quotes using numerical optimization.',
    group: 'Calibration',
    Component: CalibrationExample,
  },
  {
    slug: 'statements-modeling',
    title: 'Financial Statements Modeling',
    description: 'Build and evaluate financial statement models with formulas, forecasts, and dynamic metrics.',
    group: 'Statements',
    Component: StatementsModeling,
  },
  {
    slug: 'scenarios-stress-testing',
    title: 'Scenarios & Stress Testing',
    description: 'Apply deterministic market shocks, statement adjustments, and time roll-forwards with full composability.',
    group: 'Scenarios',
    Component: ScenariosExample,
  },
  {
    slug: 'portfolio-management',
    title: 'Portfolio Management',
    description: 'Create entities, build portfolios, value positions, and aggregate metrics with cross-currency support.',
    group: 'Portfolio',
    Component: PortfolioExample,
  },
];

export const getExampleBySlug = (slug: string): ExampleDefinition | undefined =>
  EXAMPLES.find((example) => example.slug === slug);
