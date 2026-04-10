import {
  type FrequencyType,
  getSwapConventions,
} from './CurrencyConventions';

/** Base quote data that can be edited */
export interface DepositQuoteData {
  type: 'deposit';
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  dayCount: string;
}

export interface SwapQuoteData {
  type: 'swap';
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  /** Fixed leg payment frequency */
  fixedFrequency: FrequencyType;
  /** Float leg payment frequency */
  floatFrequency: FrequencyType;
  fixedDayCount: string;
  floatDayCount: string;
  index: string;
}

export interface FraQuoteData {
  type: 'fra';
  startYear: number;
  startMonth: number;
  startDay: number;
  endYear: number;
  endMonth: number;
  endDay: number;
  rate: number;
  dayCount: string;
}

export type DiscountQuoteData = DepositQuoteData | SwapQuoteData;
/** Forward curve quotes can include FRAs, deposits, and swaps (for multi-curve calibration) */
export type ForwardQuoteData = FraQuoteData | DepositQuoteData | SwapQuoteData;

/** Credit CDS quote data */
export interface CdsQuoteData {
  entity: string;
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  spreadBps: number;
  recoveryRate: number;
  currency: string;
}

/** Inflation swap quote data */
export interface InflationSwapQuoteData {
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  indexName: string;
}

/** Vol quote data */
export interface VolQuoteData {
  underlying: string;
  expiryYear: number;
  expiryMonth: number;
  expiryDay: number;
  strike: number;
  vol: number;
  optionType: 'Call' | 'Put';
}

/**
 * Generate default discount quotes relative to a base date.
 * Uses tenors (1M, 3M, 1Y, 3Y) instead of hardcoded dates.
 */
export function generateDefaultDiscountQuotes(
  baseYear: number,
  baseMonth: number,
  baseDay: number,
  currency: string = 'USD'
): DiscountQuoteData[] {
  const conventions = getSwapConventions(currency);
  // Generate maturity dates as offsets from base
  const addMonths = (y: number, m: number, d: number, months: number) => {
    const newMonth = m + months;
    const yearOffset = Math.floor((newMonth - 1) / 12);
    const finalMonth = ((newMonth - 1) % 12) + 1;
    return { y: y + yearOffset, m: finalMonth, d: Math.min(d, 28) }; // Safe day
  };

  const m1 = addMonths(baseYear, baseMonth, baseDay, 1);
  const m3 = addMonths(baseYear, baseMonth, baseDay, 3);
  const y1 = addMonths(baseYear, baseMonth, baseDay, 12);
  const y3 = addMonths(baseYear, baseMonth, baseDay, 36);

  return [
    {
      type: 'deposit',
      maturityYear: m1.y,
      maturityMonth: m1.m,
      maturityDay: m1.d,
      rate: 0.045,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'deposit',
      maturityYear: m3.y,
      maturityMonth: m3.m,
      maturityDay: m3.d,
      rate: 0.0465,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'swap',
      maturityYear: y1.y,
      maturityMonth: y1.m,
      maturityDay: y1.d,
      rate: 0.0475,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
    {
      type: 'swap',
      maturityYear: y3.y,
      maturityMonth: y3.m,
      maturityDay: y3.d,
      rate: 0.0485,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
  ];
}

/**
 * Generate default forward quotes relative to a base date.
 */
export function generateDefaultForwardQuotes(
  baseYear: number,
  baseMonth: number,
  baseDay: number,
  currency: string = 'USD'
): ForwardQuoteData[] {
  const conventions = getSwapConventions(currency);
  const addMonths = (y: number, m: number, d: number, months: number) => {
    const newMonth = m + months;
    const yearOffset = Math.floor((newMonth - 1) / 12);
    const finalMonth = ((newMonth - 1) % 12) + 1;
    return { y: y + yearOffset, m: finalMonth, d: Math.min(d, 28) };
  };

  const m1 = addMonths(baseYear, baseMonth, baseDay, 1);
  const m3 = addMonths(baseYear, baseMonth, baseDay, 3);
  const m6 = addMonths(baseYear, baseMonth, baseDay, 6);
  const m9 = addMonths(baseYear, baseMonth, baseDay, 9);
  const y2 = addMonths(baseYear, baseMonth, baseDay, 24);
  const y5 = addMonths(baseYear, baseMonth, baseDay, 60);

  return [
    // Short-end deposits
    {
      type: 'deposit',
      maturityYear: m1.y,
      maturityMonth: m1.m,
      maturityDay: m1.d,
      rate: 0.0535,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'deposit',
      maturityYear: m3.y,
      maturityMonth: m3.m,
      maturityDay: m3.d,
      rate: 0.054,
      dayCount: conventions.floatDayCount,
    },
    // FRAs for the near term
    {
      type: 'fra',
      startYear: m3.y,
      startMonth: m3.m,
      startDay: m3.d,
      endYear: m6.y,
      endMonth: m6.m,
      endDay: m6.d,
      rate: 0.052,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'fra',
      startYear: m6.y,
      startMonth: m6.m,
      startDay: m6.d,
      endYear: m9.y,
      endMonth: m9.m,
      endDay: m9.d,
      rate: 0.05,
      dayCount: conventions.floatDayCount,
    },
    // SOFR swaps for longer tenors
    {
      type: 'swap',
      maturityYear: y2.y,
      maturityMonth: y2.m,
      maturityDay: y2.d,
      rate: 0.0475,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
    {
      type: 'swap',
      maturityYear: y5.y,
      maturityMonth: y5.m,
      maturityDay: y5.d,
      rate: 0.045,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
  ];
}

export const DEFAULT_CREDIT_QUOTES: CdsQuoteData[] = [
  {
    entity: 'ACME',
    maturityYear: 2027,
    maturityMonth: 1,
    maturityDay: 2,
    spreadBps: 120,
    recoveryRate: 0.4,
    currency: 'USD',
  },
  {
    entity: 'ACME',
    maturityYear: 2029,
    maturityMonth: 1,
    maturityDay: 2,
    spreadBps: 135,
    recoveryRate: 0.4,
    currency: 'USD',
  },
];

export const DEFAULT_INFLATION_QUOTES: InflationSwapQuoteData[] = [
  { maturityYear: 2026, maturityMonth: 1, maturityDay: 2, rate: 0.021, indexName: 'US-CPI-U' },
  { maturityYear: 2029, maturityMonth: 1, maturityDay: 2, rate: 0.023, indexName: 'US-CPI-U' },
];

export const DEFAULT_VOL_QUOTES: VolQuoteData[] = [
  {
    underlying: 'AAPL',
    expiryYear: 2024,
    expiryMonth: 7,
    expiryDay: 1,
    strike: 90,
    vol: 0.24,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2024,
    expiryMonth: 7,
    expiryDay: 1,
    strike: 100,
    vol: 0.22,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2024,
    expiryMonth: 7,
    expiryDay: 1,
    strike: 110,
    vol: 0.23,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2025,
    expiryMonth: 1,
    expiryDay: 2,
    strike: 90,
    vol: 0.26,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2025,
    expiryMonth: 1,
    expiryDay: 2,
    strike: 100,
    vol: 0.24,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2025,
    expiryMonth: 1,
    expiryDay: 2,
    strike: 110,
    vol: 0.25,
    optionType: 'Call',
  },
];

/** CDS Tranche quote data for base correlation calibration */
export interface TrancheQuoteData {
  index: string;
  attachment: number;
  detachment: number;
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  upfrontPct: number;
  runningSpreadBp: number;
}

/** CDS Vol quote data for CDS option pricing */
export interface CdsVolQuoteData {
  expiryMonths: number;
  strikeBps: number;
  vol: number;
  optionType: 'payer' | 'receiver';
}

/**
 * Default tranche quotes for CDX.NA.IG (equity sub-tranches for base correlation calibration).
 * Base correlation calibration requires equity sub-tranches [0, D] for each detachment point D.
 * Upfront values are synthetic placeholders for demonstration purposes.
 */
export const DEFAULT_TRANCHE_QUOTES: TrancheQuoteData[] = [
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 3.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 25.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 7.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 15.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 10.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 10.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 15.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 6.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 30.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 2.5,
    runningSpreadBp: 500.0,
  },
];
