/**
 * Inflation instruments fixture data.
 */

import { DiscountCurveData, DEFAULT_USD_OIS_CURVE } from './market-data';

export interface InflationCurveData {
  id: string;
  baseIndex: number;
  tenors: number[];
  indexLevels: number[];
  interpolation: string;
}

export interface InflationLinkedBondData {
  id: string;
  notional: { amount: number; currency: string };
  realCoupon: number;
  issueDate: { year: number; month: number; day: number };
  maturityDate: { year: number; month: number; day: number };
  baseIndex: number;
  discountCurveId: string;
  inflationCurveId: string;
  bondType: 'tips' | 'oat' | 'ukti';
  frequency: string;
}

export interface InflationSwapData {
  id: string;
  notional: { amount: number; currency: string };
  fixedRate: number;
  startDate: { year: number; month: number; day: number };
  endDate: { year: number; month: number; day: number };
  discountCurveId: string;
  inflationCurveId: string;
  direction: 'pay_fixed' | 'receive_fixed';
  dayCount: string;
}

export interface InflationInstrumentsProps {
  valuationDate?: { year: number; month: number; day: number };
  discountCurve?: DiscountCurveData;
  inflationCurve?: InflationCurveData;
  bonds?: InflationLinkedBondData[];
  swaps?: InflationSwapData[];
}

// Default inflation curve
export const DEFAULT_US_CPI_CURVE: InflationCurveData = {
  id: 'US-CPI',
  baseIndex: 300.0,
  tenors: [0.0, 1.0, 2.0, 5.0, 10.0],
  indexLevels: [300.0, 303.0, 306.5, 320.0, 345.0],
  interpolation: 'log_linear',
};

// Default inflation-linked bonds
export const DEFAULT_INFLATION_BONDS: InflationLinkedBondData[] = [
  {
    id: 'tips_2033',
    notional: { amount: 1_000_000, currency: 'USD' },
    realCoupon: 0.0125,
    issueDate: { year: 2024, month: 1, day: 2 },
    maturityDate: { year: 2034, month: 1, day: 15 },
    baseIndex: 300.0,
    discountCurveId: 'USD-OIS',
    inflationCurveId: 'US-CPI',
    bondType: 'tips',
    frequency: 'semi_annual',
  },
];

// Default inflation swaps
export const DEFAULT_INFLATION_SWAPS: InflationSwapData[] = [
  {
    id: 'zc_inflation_swap',
    notional: { amount: 5_000_000, currency: 'USD' },
    fixedRate: 0.025,
    startDate: { year: 2024, month: 1, day: 2 },
    endDate: { year: 2030, month: 1, day: 2 },
    discountCurveId: 'USD-OIS',
    inflationCurveId: 'US-CPI',
    direction: 'pay_fixed',
    dayCount: 'act_act',
  },
];

// Complete props bundle for default inflation example
export const DEFAULT_INFLATION_PROPS: InflationInstrumentsProps = {
  valuationDate: { year: 2024, month: 1, day: 2 },
  discountCurve: DEFAULT_USD_OIS_CURVE,
  inflationCurve: DEFAULT_US_CPI_CURVE,
  bonds: DEFAULT_INFLATION_BONDS,
  swaps: DEFAULT_INFLATION_SWAPS,
};
