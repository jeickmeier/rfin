/**
 * Cashflow builder fixture data.
 */

import { DateData, MoneyData, DiscountCurveData, ForwardCurveData } from './market-data';

// Schedule parameters
export interface ScheduleParamsData {
  type: 'quarterlyAct360' | 'semiannual30360' | 'annualActAct';
}

// Coupon type specification
export interface CouponTypeData {
  type: 'cash' | 'pik' | 'split';
  cashPct?: number;
  pikPct?: number;
}

// Fixed coupon specification
export interface FixedCouponData {
  rate: number;
  schedule: ScheduleParamsData;
  couponType: CouponTypeData;
}

// Floating coupon specification
export interface FloatingCouponData {
  indexId: string;
  marginBps: number;
  gearing: number;
  resetLagDays: number;
  schedule: ScheduleParamsData;
  couponType: CouponTypeData;
}

// Amortization specification
export interface AmortizationData {
  type: 'linearTo';
  finalNotional: MoneyData;
}

// Step-up program entry
export type StepUpEntry = [string, number]; // [date, rate]

// Payment split program entry
export type PaymentSplitEntry = [string, string]; // [date, 'cash' | 'pik' | 'split:x:y']

// Example schedule configuration
export interface CashflowExampleData {
  title: string;
  description: string;
  notional: MoneyData;
  issueDate: DateData;
  maturityDate: DateData;
  fixedCoupon?: FixedCouponData;
  floatingCoupon?: FloatingCouponData;
  amortization?: AmortizationData;
  stepUpProgram?: StepUpEntry[];
  paymentSplitProgram?: PaymentSplitEntry[];
  useMarketCurves?: boolean;
  marketData?: {
    discountCurve: DiscountCurveData;
    forwardCurve: ForwardCurveData;
  };
}

export interface CashflowBuilderProps {
  examples?: CashflowExampleData[];
}

// Default notional and dates
const DEFAULT_NOTIONAL: MoneyData = { amount: 1_000_000, currency: 'USD' };
const DEFAULT_ISSUE: DateData = { year: 2025, month: 1, day: 15 };
const DEFAULT_MATURITY: DateData = { year: 2030, month: 1, day: 15 };

// Default market data for floating rate examples
const DEFAULT_CASHFLOW_MARKET_DATA = {
  discountCurve: {
    id: 'USD-OIS',
    baseDate: { year: 2025, month: 1, day: 2 },
    tenors: [0, 1, 2, 3],
    discountFactors: [1, 0.995, 0.988, 0.98],
    dayCount: 'act_365f',
    interpolation: 'monotone_convex',
    extrapolation: 'flat_forward',
    continuous: true,
  },
  forwardCurve: {
    id: 'USD-SOFR-3M',
    baseDate: { year: 2025, month: 1, day: 2 },
    tenor: 0.25,
    tenors: [0, 0.5, 1, 2],
    rates: [0.03, 0.0325, 0.035, 0.04],
    dayCount: 'act_360',
    compounding: 2,
    interpolation: 'linear',
  },
};

// Default examples
export const DEFAULT_CASHFLOW_EXAMPLES: CashflowExampleData[] = [
  {
    title: 'Simple Fixed Coupon (5% Quarterly)',
    description: 'Standard quarterly coupons paid in cash with Act/360 day count',
    notional: DEFAULT_NOTIONAL,
    issueDate: DEFAULT_ISSUE,
    maturityDate: DEFAULT_MATURITY,
    fixedCoupon: {
      rate: 0.05,
      schedule: { type: 'quarterlyAct360' },
      couponType: { type: 'cash' },
    },
  },
  {
    title: 'PIK Toggle Bond (70% Cash / 30% PIK)',
    description: 'Semi-annual coupons split between cash payment and capitalization',
    notional: DEFAULT_NOTIONAL,
    issueDate: DEFAULT_ISSUE,
    maturityDate: DEFAULT_MATURITY,
    fixedCoupon: {
      rate: 0.08,
      schedule: { type: 'semiannual30360' },
      couponType: { type: 'split', cashPct: 0.7, pikPct: 0.3 },
    },
  },
  {
    title: 'Floating Rate Note - Margin Only (No Curves)',
    description: 'Uses only margin (150 bps): coupon = outstanding × 0.0150 × year_fraction',
    notional: DEFAULT_NOTIONAL,
    issueDate: DEFAULT_ISSUE,
    maturityDate: DEFAULT_MATURITY,
    floatingCoupon: {
      indexId: 'USD-SOFR-3M',
      marginBps: 150,
      gearing: 1,
      resetLagDays: 2,
      schedule: { type: 'quarterlyAct360' },
      couponType: { type: 'cash' },
    },
    useMarketCurves: false,
  },
  {
    title: 'Floating Rate Note - With Forward Rates (Market Curves)',
    description: 'Uses forward_rate × gearing + margin: coupon = outstanding × (fwd_rate + 0.0150) × yf',
    notional: DEFAULT_NOTIONAL,
    issueDate: DEFAULT_ISSUE,
    maturityDate: DEFAULT_MATURITY,
    floatingCoupon: {
      indexId: 'USD-SOFR-3M',
      marginBps: 150,
      gearing: 1,
      resetLagDays: 2,
      schedule: { type: 'quarterlyAct360' },
      couponType: { type: 'cash' },
    },
    useMarketCurves: true,
    marketData: DEFAULT_CASHFLOW_MARKET_DATA,
  },
  {
    title: 'Amortizing Loan (Linear to $200K)',
    description: 'Quarterly coupons with linear amortization from $1M to $200K',
    notional: DEFAULT_NOTIONAL,
    issueDate: DEFAULT_ISSUE,
    maturityDate: DEFAULT_MATURITY,
    fixedCoupon: {
      rate: 0.06,
      schedule: { type: 'quarterlyAct360' },
      couponType: { type: 'cash' },
    },
    amortization: {
      type: 'linearTo',
      finalNotional: { amount: 200_000, currency: 'USD' },
    },
  },
  {
    title: 'Step-Up Coupon Structure (4% → 5% → 6%)',
    description: 'Semi-annual coupons with step-up rates: 4% for 2 years, then 5%, then 6%',
    notional: DEFAULT_NOTIONAL,
    issueDate: DEFAULT_ISSUE,
    maturityDate: DEFAULT_MATURITY,
    stepUpProgram: [
      ['2027-01-15', 0.04],
      ['2029-01-15', 0.05],
      ['2030-01-15', 0.06],
    ],
  },
  {
    title: 'Payment Split Program (Cash → PIK Transition)',
    description: 'Quarterly coupons transitioning from 100% cash to 50/50 split to 100% PIK',
    notional: DEFAULT_NOTIONAL,
    issueDate: DEFAULT_ISSUE,
    maturityDate: DEFAULT_MATURITY,
    fixedCoupon: {
      rate: 0.07,
      schedule: { type: 'quarterlyAct360' },
      couponType: { type: 'cash' },
    },
    paymentSplitProgram: [
      ['2027-01-15', 'cash'],
      ['2028-01-15', 'split:0.5:0.5'],
      ['2030-01-15', 'pik'],
    ],
  },
];

// Complete props bundle
export const DEFAULT_CASHFLOW_BUILDER_PROPS: CashflowBuilderProps = {
  examples: DEFAULT_CASHFLOW_EXAMPLES,
};

