/**
 * Exotic rates derivatives fixture data.
 */

import { DateData, DiscountCurveData, ForwardCurveData, VolSurfaceData, MoneyData } from './market-data';

// CMS option JSON data
export interface CmsOptionJsonData {
  id: string;
  cms_tenor: number;
  strike_rate: number;
  fixing_dates: string[];
  payment_dates: string[];
  accrual_fractions: number[];
  option_type: 'call' | 'put';
  notional: MoneyData;
  day_count: string;
  swap_fixed_freq: { Months: number };
  swap_float_freq: { Months: number };
  swap_day_count: string;
  discount_curve_id: string;
  forward_curve_id: string;
  vol_surface_id: string;
}

// Range accrual JSON data
export interface RangeAccrualJsonData {
  id: string;
  underlying_ticker: string;
  observation_dates: string[];
  lower_bound: number;
  upper_bound: number;
  coupon_rate: number;
  notional: MoneyData;
  day_count: string;
  discount_curve_id: string;
  spot_id: string;
  vol_surface_id: string;
}

// Market setup for exotic rates
export interface ExoticRatesMarketData {
  discountCurve: DiscountCurveData;
  forwardCurve: ForwardCurveData;
  volSurface: VolSurfaceData;
  spotPrices: Array<{ id: string; price: MoneyData }>;
}

export interface ExoticRatesDerivativesProps {
  valuationDate?: DateData;
  notional?: MoneyData;
  market?: ExoticRatesMarketData;
  cmsOptions?: CmsOptionJsonData[];
  rangeAccruals?: RangeAccrualJsonData[];
}

// Default valuation date
const DEFAULT_VALUATION_DATE: DateData = { year: 2024, month: 1, day: 2 };

// Default notional
const DEFAULT_NOTIONAL: MoneyData = { amount: 10_000_000, currency: 'USD' };

// Default market data
const DEFAULT_EXOTIC_RATES_MARKET: ExoticRatesMarketData = {
  discountCurve: {
    id: 'USD-OIS',
    baseDate: DEFAULT_VALUATION_DATE,
    tenors: [0, 0.5, 1, 2, 5, 10],
    discountFactors: [1, 0.995, 0.99, 0.975, 0.94, 0.87],
    dayCount: 'act_365f',
    interpolation: 'monotone_convex',
    extrapolation: 'flat_forward',
    continuous: true,
  },
  forwardCurve: {
    id: 'USD-SOFR-3M',
    baseDate: DEFAULT_VALUATION_DATE,
    tenor: 0.25,
    tenors: [0, 1, 2, 5, 10],
    rates: [0.03, 0.032, 0.034, 0.036, 0.038],
    dayCount: 'act_360',
    compounding: 2,
    interpolation: 'linear',
  },
  volSurface: {
    id: 'SWAPTION-VOL',
    expiries: [1, 2, 5, 10],
    strikes: [0.02, 0.03, 0.04, 0.05],
    vols: [
      0.3, 0.29, 0.28, 0.27, 0.28, 0.27, 0.26, 0.25, 0.26, 0.25, 0.24, 0.23, 0.24, 0.23, 0.22, 0.21,
    ],
  },
  spotPrices: [{ id: 'USD-SOFR-3M-SPOT', price: { amount: 0.032, currency: 'USD' } }],
};

// Default CMS options
const DEFAULT_CMS_OPTIONS: CmsOptionJsonData[] = [
  {
    id: 'cms_call_10y',
    cms_tenor: 10,
    strike_rate: 0.035,
    fixing_dates: ['2025-01-02'],
    payment_dates: ['2025-01-02'],
    accrual_fractions: [1],
    option_type: 'call',
    notional: { amount: 10_000_000, currency: 'USD' },
    day_count: 'Act365F',
    swap_fixed_freq: { Months: 6 },
    swap_float_freq: { Months: 3 },
    swap_day_count: 'Act360',
    discount_curve_id: 'USD-OIS',
    forward_curve_id: 'USD-SOFR-3M',
    vol_surface_id: 'SWAPTION-VOL',
  },
  {
    id: 'cms_put_5y',
    cms_tenor: 5,
    strike_rate: 0.03,
    fixing_dates: ['2025-01-02'],
    payment_dates: ['2025-01-02'],
    accrual_fractions: [1],
    option_type: 'put',
    notional: { amount: 10_000_000, currency: 'USD' },
    day_count: 'Act365F',
    swap_fixed_freq: { Months: 6 },
    swap_float_freq: { Months: 3 },
    swap_day_count: 'Act360',
    discount_curve_id: 'USD-OIS',
    forward_curve_id: 'USD-SOFR-3M',
    vol_surface_id: 'SWAPTION-VOL',
  },
];

// Generate monthly observation dates
function generateMonthlyObservationDates(
  startYear: number,
  startMonth: number,
  startDay: number,
  endYear: number,
  endMonth: number,
  endDay: number
): string[] {
  const dates: string[] = [];
  const start = new Date(startYear, startMonth - 1, startDay);
  const end = new Date(endYear, endMonth - 1, endDay);
  const current = new Date(start);

  while (current <= end) {
    dates.push(
      `${current.getFullYear()}-${String(current.getMonth() + 1).padStart(2, '0')}-${String(current.getDate()).padStart(2, '0')}`
    );
    current.setMonth(current.getMonth() + 1);
  }
  return dates;
}

// Default range accruals
const DEFAULT_RANGE_ACCRUALS: RangeAccrualJsonData[] = [
  {
    id: 'range_accrual_1',
    underlying_ticker: 'USD-SOFR-3M',
    observation_dates: generateMonthlyObservationDates(2024, 1, 2, 2025, 1, 2),
    lower_bound: 0.02,
    upper_bound: 0.05,
    coupon_rate: 0.06,
    notional: { amount: 10_000_000, currency: 'USD' },
    day_count: 'Act365F',
    discount_curve_id: 'USD-OIS',
    spot_id: 'USD-SOFR-3M-SPOT',
    vol_surface_id: 'SWAPTION-VOL',
  },
];

// Complete props bundle
export const DEFAULT_EXOTIC_RATES_PROPS: ExoticRatesDerivativesProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  notional: DEFAULT_NOTIONAL,
  market: DEFAULT_EXOTIC_RATES_MARKET,
  cmsOptions: DEFAULT_CMS_OPTIONS,
  rangeAccruals: DEFAULT_RANGE_ACCRUALS,
};

