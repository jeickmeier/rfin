import { ComponentType } from 'react';

import { CashflowBasicsExample } from './CashflowBasics';
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
];

export const getExampleBySlug = (slug: string): ExampleDefinition | undefined =>
  EXAMPLES.find((example) => example.slug === slug);
