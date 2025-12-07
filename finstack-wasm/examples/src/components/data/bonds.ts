/**
 * Bond instruments fixture data.
 */

import { DiscountCurveData, ForwardCurveData } from './market-data';

export interface BondData {
  id: string;
  name: string;
  notional: { amount: number; currency: string };
  issueDate: { year: number; month: number; day: number };
  maturityDate: { year: number; month: number; day: number };
  discountCurveId: string;
  quotedCleanPrice?: number;
  bondType:
    | { type: 'fixed'; couponRate: number }
    | { type: 'zero' }
    | { type: 'floating'; forwardCurveId: string; marginBps: number }
    | {
        type: 'amortizing';
        couponRate: number;
        finalNotional: { amount: number; currency: string };
      }
    | { type: 'callable'; couponRate: number; callSchedule: Array<[string, number]> }
    | {
        type: 'fixedToFloating';
        fixedRate: number;
        switchDate: { year: number; month: number; day: number };
        forwardCurveId: string;
        marginBps: number;
      }
    | { type: 'pikToggle'; couponRate: number; cashPct: number; pikPct: number };
}

export interface BondsValuationProps {
  valuationDate?: { year: number; month: number; day: number };
  discountCurve?: DiscountCurveData;
  forwardCurve?: ForwardCurveData;
  bonds?: BondData[];
}

// Default discount curve for bonds (starts at valuation date)
export const DEFAULT_BOND_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: { year: 2024, month: 3, day: 15 },
  tenors: [0, 0.25, 0.5, 1, 2, 3, 5],
  discountFactors: [1, 0.9975, 0.994, 0.985, 0.965, 0.945, 0.915],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default forward curve for floating rate bonds
export const DEFAULT_BOND_FORWARD_CURVE: ForwardCurveData = {
  id: 'USD-SOFR-3M',
  baseDate: { year: 2024, month: 3, day: 15 },
  tenor: 0.25,
  tenors: [0.25, 0.5, 1, 2, 3],
  rates: [0.053, 0.054, 0.055, 0.056, 0.057],
  dayCount: 'act_360',
  compounding: 2,
  interpolation: 'linear',
};

// Default bonds
export const DEFAULT_BONDS: BondData[] = [
  {
    id: 'corp_fixed_2029',
    name: '5Y Corporate Fixed',
    notional: { amount: 1_000_000, currency: 'USD' },
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2029, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
    quotedCleanPrice: 99.5,
    bondType: { type: 'fixed', couponRate: 0.045 },
  },
  {
    id: 'corp_zero_2027',
    name: '3Y Discount Note',
    notional: { amount: 1_000_000, currency: 'USD' },
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2027, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
    quotedCleanPrice: 95,
    bondType: { type: 'zero' },
  },
  {
    id: 'corp_frn_2027',
    name: '3Y Floating Rate Note',
    notional: { amount: 1_000_000, currency: 'USD' },
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2027, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
    quotedCleanPrice: 100.25,
    bondType: { type: 'floating', forwardCurveId: 'USD-SOFR-3M', marginBps: 150 },
  },
  {
    id: 'corp_amort_2029',
    name: '5Y Amortizing Bond',
    notional: { amount: 1_000_000, currency: 'USD' },
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2029, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
    quotedCleanPrice: 98.5,
    bondType: {
      type: 'amortizing',
      couponRate: 0.055,
      finalNotional: { amount: 200_000, currency: 'USD' },
    },
  },
  {
    id: 'corp_call_2029',
    name: '5Y Callable Bond',
    notional: { amount: 1_000_000, currency: 'USD' },
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2029, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
    quotedCleanPrice: 102,
    bondType: {
      type: 'callable',
      couponRate: 0.06,
      callSchedule: [
        ['2026-01-15', 103],
        ['2027-01-15', 102],
        ['2028-01-15', 101],
      ],
    },
  },
  {
    id: 'corp_fix2flt_2029',
    name: '5Y Fixed-to-Floating',
    notional: { amount: 1_000_000, currency: 'USD' },
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2029, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
    quotedCleanPrice: 99.75,
    bondType: {
      type: 'fixedToFloating',
      fixedRate: 0.05,
      switchDate: { year: 2026, month: 1, day: 15 },
      forwardCurveId: 'USD-SOFR-3M',
      marginBps: 100,
    },
  },
  {
    id: 'corp_pik_2029',
    name: '5Y PIK-Toggle Bond',
    notional: { amount: 1_000_000, currency: 'USD' },
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2029, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
    quotedCleanPrice: 97,
    bondType: { type: 'pikToggle', couponRate: 0.08, cashPct: 0.5, pikPct: 0.5 },
  },
];

// Complete props bundle for default bonds example
export const DEFAULT_BONDS_PROPS: BondsValuationProps = {
  valuationDate: { year: 2024, month: 3, day: 15 },
  discountCurve: DEFAULT_BOND_DISCOUNT_CURVE,
  forwardCurve: DEFAULT_BOND_FORWARD_CURVE,
  bonds: DEFAULT_BONDS,
};
