/**
 * Deposit valuation fixture data.
 */

import { DiscountCurveData } from './market-data';

export interface DepositData {
  id: string;
  notional: { amount: number; currency: string };
  startDate: { year: number; month: number; day: number };
  endDate: { year: number; month: number; day: number };
  dayCount: string;
  discountCurveId: string;
  quoteRate: number;
}

export interface DepositValuationProps {
  valuationDate?: { year: number; month: number; day: number };
  deposit?: DepositData;
  discountCurve?: DiscountCurveData;
}

// Default deposit data
export const DEFAULT_DEPOSIT: DepositData = {
  id: 'usd_deposit_3m',
  notional: { amount: 5_000_000, currency: 'USD' },
  startDate: { year: 2024, month: 1, day: 15 },
  endDate: { year: 2024, month: 4, day: 15 },
  dayCount: 'act360',
  discountCurveId: 'USD-OIS',
  quoteRate: 0.0525,
};

// Discount curve for deposit valuation (starts at deposit start date)
export const DEFAULT_DEPOSIT_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: { year: 2024, month: 1, day: 15 },
  tenors: [0.0, 0.25, 0.5, 1.0],
  discountFactors: [1.0, 0.998, 0.9945, 0.9875],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

export const DEFAULT_DEPOSIT_VALUATION_DATE = { year: 2024, month: 2, day: 15 };

// Complete props bundle for default deposit example
export const DEFAULT_DEPOSIT_PROPS: DepositValuationProps = {
  valuationDate: DEFAULT_DEPOSIT_VALUATION_DATE,
  deposit: DEFAULT_DEPOSIT,
  discountCurve: DEFAULT_DEPOSIT_DISCOUNT_CURVE,
};

