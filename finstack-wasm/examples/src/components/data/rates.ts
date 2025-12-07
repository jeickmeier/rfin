/**
 * Interest rates instruments fixture data.
 */

import {
  DiscountCurveData,
  ForwardCurveData,
  VolSurfaceData,
  DateData,
  MoneyData,
  DEFAULT_VALUATION_DATE,
} from './market-data';

// Interest Rate Swap Data
export interface SwapData {
  id: string;
  notional: MoneyData;
  fixedRate: number;
  startDate: DateData;
  endDate: DateData;
  discountCurveId: string;
  forwardCurveId: string;
  direction: 'receive_fixed' | 'pay_fixed';
  fixedDayCount: string;
  floatDayCount: string;
  fixedFrequency: number;
}

// FRA Data
export interface FraData {
  id: string;
  notional: MoneyData;
  fixedRate: number;
  fixingDate: DateData;
  settlementDate: DateData;
  maturityDate: DateData;
  discountCurveId: string;
  forwardCurveId: string;
  dayCount: string;
  compounding: number;
  payAtMaturity: boolean;
}

// Swaption Data
export interface SwaptionData {
  id: string;
  notional: MoneyData;
  strike: number;
  optionExpiry: DateData;
  swapStart: DateData;
  swapEnd: DateData;
  discountCurveId: string;
  forwardCurveId: string;
  volSurfaceId: string;
  optionType: 'payer' | 'receiver';
}

// Cap/Floor Data
export interface CapFloorData {
  id: string;
  notional: MoneyData;
  strike: number;
  startDate: DateData;
  endDate: DateData;
  discountCurveId: string;
  forwardCurveId: string;
  volSurfaceId: string;
  capOrFloor: 'cap' | 'floor';
  frequency: number;
  dayCount: string;
}

// Interest Rate Future Data
export interface FutureData {
  id: string;
  notional: MoneyData;
  price: number;
  lastTradeDate: DateData;
  settlementDate: DateData;
  accrualStart: DateData;
  accrualEnd: DateData;
  discountCurveId: string;
  forwardCurveId: string;
  direction: 'long' | 'short';
  dayCount: string;
}

export interface RatesInstrumentsProps {
  valuationDate?: DateData;
  discountCurve?: DiscountCurveData;
  forwardCurve?: ForwardCurveData;
  swaptionVolSurface?: VolSurfaceData;
  capVolSurface?: VolSurfaceData;
  swaps?: SwapData[];
  fras?: FraData[];
  swaptions?: SwaptionData[];
  capsFloors?: CapFloorData[];
  futures?: FutureData[];
}

// Default discount curve for rates
export const DEFAULT_RATES_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 0.5, 1, 2, 5],
  discountFactors: [1, 0.995, 0.99, 0.975, 0.94],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default forward curve for rates
export const DEFAULT_RATES_FORWARD_CURVE: ForwardCurveData = {
  id: 'USD-SOFR-3M',
  baseDate: DEFAULT_VALUATION_DATE,
  tenor: 0.25,
  tenors: [0, 1, 2, 5],
  rates: [0.03, 0.032, 0.034, 0.036],
  dayCount: 'act_360',
  compounding: 2,
  interpolation: 'linear',
};

// Default swaption volatility surface
export const DEFAULT_SWAPTION_VOL_SURFACE: VolSurfaceData = {
  id: 'SWAPTION-VOL',
  expiries: [1, 2, 5],
  strikes: [0.02, 0.03, 0.04],
  vols: [0.3, 0.29, 0.28, 0.28, 0.27, 0.26, 0.26, 0.25, 0.24],
};

// Default cap volatility surface
export const DEFAULT_CAP_VOL_SURFACE: VolSurfaceData = {
  id: 'IR-CAP-VOL',
  expiries: [0.5, 1, 2, 5],
  strikes: [0.01, 0.02, 0.03, 0.04],
  vols: [
    0.38, 0.36, 0.34, 0.32, 0.35, 0.33, 0.31, 0.3, 0.32, 0.31, 0.29, 0.28, 0.28, 0.27, 0.26, 0.25,
  ],
};

// Default swaps
export const DEFAULT_SWAPS: SwapData[] = [
  {
    id: 'irs_receive_fixed',
    notional: { amount: 10_000_000, currency: 'USD' },
    fixedRate: 0.0325,
    startDate: DEFAULT_VALUATION_DATE,
    endDate: { year: 2029, month: 1, day: 2 },
    discountCurveId: 'USD-OIS',
    forwardCurveId: 'USD-SOFR-3M',
    direction: 'receive_fixed',
    fixedDayCount: 'thirty360',
    floatDayCount: 'act360',
    fixedFrequency: 2,
  },
];

// Default FRAs
export const DEFAULT_FRAS: FraData[] = [
  {
    id: 'fra_3x6',
    notional: { amount: 10_000_000, currency: 'USD' },
    fixedRate: 0.036,
    fixingDate: { year: 2024, month: 4, day: 2 },
    settlementDate: { year: 2024, month: 4, day: 4 },
    maturityDate: { year: 2024, month: 7, day: 4 },
    discountCurveId: 'USD-OIS',
    forwardCurveId: 'USD-SOFR-3M',
    dayCount: 'act360',
    compounding: 2,
    payAtMaturity: true,
  },
];

// Default swaptions
export const DEFAULT_SWAPTIONS: SwaptionData[] = [
  {
    id: 'swaption_1y5y',
    notional: { amount: 10_000_000, currency: 'USD' },
    strike: 0.0325,
    optionExpiry: { year: 2025, month: 1, day: 2 },
    swapStart: { year: 2025, month: 1, day: 2 },
    swapEnd: { year: 2030, month: 1, day: 2 },
    discountCurveId: 'USD-OIS',
    forwardCurveId: 'USD-SOFR-3M',
    volSurfaceId: 'SWAPTION-VOL',
    optionType: 'payer',
  },
];

// Default caps/floors
export const DEFAULT_CAPS_FLOORS: CapFloorData[] = [
  {
    id: 'cap_5y',
    notional: { amount: 10_000_000, currency: 'USD' },
    strike: 0.04,
    startDate: DEFAULT_VALUATION_DATE,
    endDate: { year: 2029, month: 1, day: 2 },
    discountCurveId: 'USD-OIS',
    forwardCurveId: 'USD-SOFR-3M',
    volSurfaceId: 'IR-CAP-VOL',
    capOrFloor: 'cap',
    frequency: 4,
    dayCount: 'act360',
  },
];

// Default futures
export const DEFAULT_FUTURES: FutureData[] = [
  {
    id: 'sofr_fut_mar24',
    notional: { amount: 1_000_000, currency: 'USD' },
    price: 97.25,
    lastTradeDate: { year: 2024, month: 3, day: 16 },
    settlementDate: { year: 2024, month: 3, day: 18 },
    accrualStart: { year: 2024, month: 3, day: 18 },
    accrualEnd: { year: 2024, month: 6, day: 18 },
    discountCurveId: 'USD-OIS',
    forwardCurveId: 'USD-SOFR-3M',
    direction: 'long',
    dayCount: 'act360',
  },
];

// Complete props bundle
export const DEFAULT_RATES_PROPS: RatesInstrumentsProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  discountCurve: DEFAULT_RATES_DISCOUNT_CURVE,
  forwardCurve: DEFAULT_RATES_FORWARD_CURVE,
  swaptionVolSurface: DEFAULT_SWAPTION_VOL_SURFACE,
  capVolSurface: DEFAULT_CAP_VOL_SURFACE,
  swaps: DEFAULT_SWAPS,
  fras: DEFAULT_FRAS,
  swaptions: DEFAULT_SWAPTIONS,
  capsFloors: DEFAULT_CAPS_FLOORS,
  futures: DEFAULT_FUTURES,
};
