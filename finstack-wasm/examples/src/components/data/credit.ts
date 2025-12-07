/**
 * Credit instruments fixture data.
 */

import {
  DiscountCurveData,
  VolSurfaceData,
  DateData,
  MoneyData,
  DEFAULT_VALUATION_DATE,
  HazardCurveData,
} from './market-data';

// Re-export HazardCurveData for convenience
export type { HazardCurveData } from './market-data';

// Base correlation curve data
export interface BaseCorrelationData {
  id: string;
  attachmentPoints: number[];
  correlations: number[];
}

// Credit index data
export interface CreditIndexDataSpec {
  indexFamily: string;
  hazardCurveId: string;
  constituents: number;
  recoveryRate: number;
}

// CDS Data
export interface CdsData {
  id: string;
  notional: MoneyData;
  spreadBps: number;
  effectiveDate: DateData;
  maturityDate: DateData;
  discountCurveId: string;
  hazardCurveId: string;
  direction: 'buy_protection' | 'sell_protection';
}

// CDS Index Instrument Data
export interface CdsIndexInstrumentData {
  id: string;
  indexFamily: string;
  series: number;
  version: number;
  notional: MoneyData;
  spreadBps: number;
  effectiveDate: DateData;
  maturityDate: DateData;
  discountCurveId: string;
  hazardCurveId: string;
  direction: 'pay_protection' | 'receive_protection';
  recoveryRate: number;
}

// CDS Tranche Data
export interface CdsTrancheData {
  id: string;
  indexFamily: string;
  series: number;
  attachmentPoint: number;
  detachmentPoint: number;
  notional: MoneyData;
  maturityDate: DateData;
  spreadBps: number;
  discountCurveId: string;
  /** Credit index ID - must match the indexFamily used in market.insertCreditIndex() */
  creditIndexId: string;
  direction: 'buy_protection' | 'sell_protection';
  frequency: number;
}

// CDS Option Data
export interface CdsOptionData {
  id: string;
  notional: MoneyData;
  strikeBps: number;
  expiryDate: DateData;
  underlyingMaturity: DateData;
  discountCurveId: string;
  hazardCurveId: string;
  volSurfaceId: string;
  optionType: 'call' | 'put';
  recoveryRate: number;
  knockedOut: boolean;
}

// Revolving Credit Data
export interface RevolvingCreditData {
  id: string;
  commitmentAmount: MoneyData;
  drawnAmount: MoneyData;
  commitmentDate: string;
  maturityDate: string;
  baseRateSpec: { Fixed: { rate: number } };
  dayCount: string;
  paymentFrequency: { Months: number };
  fees: {
    upfrontFee: MoneyData | null;
    commitmentFeeBp: number;
    usageFeeBp: number;
    facilityFeeBp: number;
  };
  drawRepaySpec:
    | { Deterministic: Array<{ date: string; amount: MoneyData; is_draw: boolean }> }
    | {
        Stochastic: {
          utilization_process: {
            MeanReverting: { target_rate: number; speed: number; volatility: number };
          };
          num_paths: number;
          seed: number;
        };
      };
  discountCurveId: string;
}

export interface CreditInstrumentsProps {
  valuationDate?: DateData;
  discountCurve?: DiscountCurveData;
  hazardCurves?: HazardCurveData[];
  baseCorrelation?: BaseCorrelationData;
  cdsVolSurface?: VolSurfaceData;
  creditIndexData?: CreditIndexDataSpec;
  cdsSwaps?: CdsData[];
  cdsIndices?: CdsIndexInstrumentData[];
  cdsTranches?: CdsTrancheData[];
  cdsOptions?: CdsOptionData[];
  revolvingCredits?: RevolvingCreditData[];
}

// Default discount curve for credit
export const DEFAULT_CREDIT_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 0.5, 1, 3, 5],
  discountFactors: [1, 0.998, 0.996, 0.985, 0.96],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default hazard curves
export const DEFAULT_HAZARD_CURVES: HazardCurveData[] = [
  {
    id: 'ACME-HZD',
    baseDate: DEFAULT_VALUATION_DATE,
    tenors: [0, 3, 5],
    hazardRates: [0.012, 0.018, 0.022],
    recoveryRate: 0.4,
    dayCount: 'act_365f',
  },
  {
    id: 'CDX-IG-HZD',
    baseDate: DEFAULT_VALUATION_DATE,
    tenors: [0, 3, 5, 7],
    hazardRates: [0.01, 0.016, 0.019, 0.021],
    recoveryRate: 0.4,
    dayCount: 'act_365f',
  },
];

// Default base correlation curve
// Note: attachment points are in PERCENTAGE format (3.0 = 3%), matching Rust internals
export const DEFAULT_BASE_CORRELATION: BaseCorrelationData = {
  id: 'CDX-IG-BC',
  attachmentPoints: [3, 7, 10, 15, 30, 100], // Percentage format: 3%, 7%, 10%, 15%, 30%, 100%
  correlations: [0.25, 0.45, 0.6, 0.75, 0.85, 0.95], // Realistic correlations for CDX.NA.IG
};

// Default CDS volatility surface
export const DEFAULT_CDS_VOL_SURFACE: VolSurfaceData = {
  id: 'CDS-VOL',
  expiries: [0.5, 1, 3, 5],
  strikes: [0.01, 0.02, 0.04],
  vols: [0.45, 0.4, 0.35, 0.42, 0.38, 0.33, 0.38, 0.35, 0.3, 0.35, 0.32, 0.28],
};

// Default credit index data
export const DEFAULT_CREDIT_INDEX_DATA: CreditIndexDataSpec = {
  indexFamily: 'CDX.NA.IG',
  hazardCurveId: 'CDX-IG-HZD',
  constituents: 125,
  recoveryRate: 0.4,
};

// Default CDS swaps
export const DEFAULT_CDS_SWAPS: CdsData[] = [
  {
    id: 'acme_cds',
    notional: { amount: 10_000_000, currency: 'USD' },
    spreadBps: 120,
    effectiveDate: { year: 2024, month: 1, day: 3 },
    maturityDate: { year: 2029, month: 1, day: 2 },
    discountCurveId: 'USD-OIS',
    hazardCurveId: 'ACME-HZD',
    direction: 'buy_protection',
  },
];

// Default CDS indices
export const DEFAULT_CDS_INDICES: CdsIndexInstrumentData[] = [
  {
    id: 'cdx_trad',
    indexFamily: 'CDX.NA.IG',
    series: 42,
    version: 1,
    notional: { amount: 25_000_000, currency: 'USD' },
    spreadBps: 100,
    effectiveDate: { year: 2024, month: 1, day: 3 },
    maturityDate: { year: 2029, month: 1, day: 2 },
    discountCurveId: 'USD-OIS',
    hazardCurveId: 'CDX-IG-HZD',
    direction: 'pay_protection',
    recoveryRate: 0.4,
  },
];

// Default CDS tranches
export const DEFAULT_CDS_TRANCHES: CdsTrancheData[] = [
  {
    id: 'cdx_mez_tranche',
    indexFamily: 'CDX.NA.IG',
    series: 42,
    attachmentPoint: 3, // 3% as percentage value
    detachmentPoint: 7, // 7% as percentage value
    notional: { amount: 10_000_000, currency: 'USD' },
    maturityDate: { year: 2029, month: 1, day: 2 },
    spreadBps: 500,
    discountCurveId: 'USD-OIS',
    creditIndexId: 'CDX.NA.IG', // Must match the key used in market.insertCreditIndex()
    direction: 'buy_protection',
    frequency: 4,
  },
];

// Default CDS options
export const DEFAULT_CDS_OPTIONS: CdsOptionData[] = [
  {
    id: 'acme_cdsopt',
    notional: { amount: 5_000_000, currency: 'USD' },
    strikeBps: 150,
    expiryDate: { year: 2025, month: 1, day: 2 },
    underlyingMaturity: { year: 2029, month: 1, day: 2 },
    discountCurveId: 'USD-OIS',
    hazardCurveId: 'ACME-HZD',
    volSurfaceId: 'CDS-VOL',
    optionType: 'call',
    recoveryRate: 0.4,
    knockedOut: false,
  },
];

// Default revolving credits
export const DEFAULT_REVOLVING_CREDITS: RevolvingCreditData[] = [
  {
    id: 'rc_facility_det',
    commitmentAmount: { amount: 10_000_000, currency: 'USD' },
    drawnAmount: { amount: 5_000_000, currency: 'USD' },
    commitmentDate: '2024-01-02',
    maturityDate: '2026-01-02',
    baseRateSpec: { Fixed: { rate: 0.05 } },
    dayCount: 'act360',
    paymentFrequency: { Months: 3 },
    fees: {
      upfrontFee: { amount: 50_000, currency: 'USD' },
      commitmentFeeBp: 25,
      usageFeeBp: 10,
      facilityFeeBp: 5,
    },
    drawRepaySpec: {
      Deterministic: [
        { date: '2024-07-01', amount: { amount: 2_000_000, currency: 'USD' }, is_draw: true },
        { date: '2025-01-01', amount: { amount: 1_000_000, currency: 'USD' }, is_draw: false },
      ],
    },
    discountCurveId: 'USD-OIS',
  },
];

// Complete props bundle
export const DEFAULT_CREDIT_PROPS: CreditInstrumentsProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  discountCurve: DEFAULT_CREDIT_DISCOUNT_CURVE,
  hazardCurves: DEFAULT_HAZARD_CURVES,
  baseCorrelation: DEFAULT_BASE_CORRELATION,
  cdsVolSurface: DEFAULT_CDS_VOL_SURFACE,
  creditIndexData: DEFAULT_CREDIT_INDEX_DATA,
  cdsSwaps: DEFAULT_CDS_SWAPS,
  cdsIndices: DEFAULT_CDS_INDICES,
  cdsTranches: DEFAULT_CDS_TRANCHES,
  cdsOptions: DEFAULT_CDS_OPTIONS,
  revolvingCredits: DEFAULT_REVOLVING_CREDITS,
};
