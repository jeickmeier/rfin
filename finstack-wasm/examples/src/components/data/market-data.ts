/**
 * Common market data fixtures used across multiple components.
 */

// Common date format for fixture data
export interface DateData {
  year: number;
  month: number;
  day: number;
}

// Common money format for fixture data
export interface MoneyData {
  amount: number;
  currency: string;
}

export interface DiscountCurveData {
  id: string;
  baseDate: { year: number; month: number; day: number };
  tenors: number[];
  discountFactors: number[];
  dayCount: string;
  interpolation: string;
  extrapolation: string;
  continuous: boolean;
}

export interface ForwardCurveData {
  id: string;
  baseDate: { year: number; month: number; day: number };
  tenor: number;
  tenors: number[];
  rates: number[];
  dayCount: string;
  compounding: number;
  interpolation: string;
}

export interface HazardCurveData {
  id: string;
  baseDate: { year: number; month: number; day: number };
  tenors: number[];
  hazardRates: number[];
  recoveryRate: number;
  dayCount: string;
}

export interface VolSurfaceData {
  id: string;
  expiries: number[];
  strikes: number[];
  vols: number[];
}

export interface FxQuoteData {
  base: string;
  quote: string;
  rate: number;
}

// Default USD OIS discount curve
export const DEFAULT_USD_OIS_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: { year: 2024, month: 1, day: 2 },
  tenors: [0.0, 0.5, 1.0, 3.0, 5.0],
  discountFactors: [1.0, 0.9975, 0.994, 0.9725, 0.948],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default EUR OIS discount curve
export const DEFAULT_EUR_OIS_CURVE: DiscountCurveData = {
  id: 'EUR-OIS',
  baseDate: { year: 2024, month: 1, day: 2 },
  tenors: [0.0, 0.5, 1.0, 3.0, 5.0],
  discountFactors: [1.0, 0.998, 0.996, 0.98, 0.955],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default USD SOFR 3M forward curve
export const DEFAULT_USD_SOFR_3M_CURVE: ForwardCurveData = {
  id: 'USD-SOFR-3M',
  baseDate: { year: 2024, month: 1, day: 2 },
  tenor: 0.25,
  tenors: [0.0, 1.0, 2.0, 5.0],
  rates: [0.03, 0.032, 0.034, 0.036],
  dayCount: 'act_360',
  compounding: 2,
  interpolation: 'linear',
};

// Default valuation date
export const DEFAULT_VALUATION_DATE = { year: 2024, month: 1, day: 2 };

