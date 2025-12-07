/**
 * FX instruments fixture data.
 */

import {
  DiscountCurveData,
  VolSurfaceData,
  FxQuoteData,
  DEFAULT_USD_OIS_CURVE,
  DEFAULT_EUR_OIS_CURVE,
} from './market-data';

export interface FxSpotData {
  id: string;
  baseCurrency: string;
  quoteCurrency: string;
  settlementDate: { year: number; month: number; day: number };
  rate: number;
  notional: { amount: number; currency: string };
}

export interface FxOptionData {
  id: string;
  baseCurrency: string;
  quoteCurrency: string;
  strike: number;
  expiryDate: { year: number; month: number; day: number };
  notional: { amount: number; currency: string };
  domesticCurveId: string;
  foreignCurveId: string;
  volSurfaceId: string;
  optionType: 'call' | 'put';
}

export interface FxSwapData {
  id: string;
  baseCurrency: string;
  quoteCurrency: string;
  notional: { amount: number; currency: string };
  nearDate: { year: number; month: number; day: number };
  farDate: { year: number; month: number; day: number };
  domesticCurveId: string;
  foreignCurveId: string;
  nearRate: number;
  farRate: number;
}

export interface FxInstrumentsProps {
  valuationDate?: { year: number; month: number; day: number };
  discountCurves?: DiscountCurveData[];
  volSurface?: VolSurfaceData;
  fxQuotes?: FxQuoteData[];
  spots?: FxSpotData[];
  options?: FxOptionData[];
  swaps?: FxSwapData[];
}

// Default FX volatility surface
export const DEFAULT_FX_VOL_SURFACE: VolSurfaceData = {
  id: 'FX-VOL',
  expiries: [0.25, 0.5, 1.0, 2.0],
  strikes: [1.05, 1.1, 1.15],
  // Flattened grid (row-major): 4 expiries x 3 strikes = 12 values
  vols: [0.14, 0.13, 0.12, 0.13, 0.12, 0.11, 0.12, 0.11, 0.1, 0.11, 0.1, 0.095],
};

// Default FX quotes
export const DEFAULT_FX_QUOTES: FxQuoteData[] = [{ base: 'EUR', quote: 'USD', rate: 1.085 }];

// Default FX spot trades
export const DEFAULT_FX_SPOTS: FxSpotData[] = [
  {
    id: 'eurusd_spot',
    baseCurrency: 'EUR',
    quoteCurrency: 'USD',
    settlementDate: { year: 2024, month: 1, day: 4 }, // T+2
    rate: 1.086,
    notional: { amount: 1_000_000, currency: 'EUR' },
  },
];

// Default FX options
export const DEFAULT_FX_OPTIONS: FxOptionData[] = [
  {
    id: 'eurusd_call',
    baseCurrency: 'EUR',
    quoteCurrency: 'USD',
    strike: 1.1,
    expiryDate: { year: 2025, month: 1, day: 2 },
    notional: { amount: 2_000_000, currency: 'EUR' },
    domesticCurveId: 'USD-OIS',
    foreignCurveId: 'EUR-OIS',
    volSurfaceId: 'FX-VOL',
    optionType: 'call',
  },
  {
    id: 'eurusd_put',
    baseCurrency: 'EUR',
    quoteCurrency: 'USD',
    strike: 1.06,
    expiryDate: { year: 2024, month: 7, day: 2 },
    notional: { amount: 1_500_000, currency: 'EUR' },
    domesticCurveId: 'USD-OIS',
    foreignCurveId: 'EUR-OIS',
    volSurfaceId: 'FX-VOL',
    optionType: 'put',
  },
];

// Default FX swaps
export const DEFAULT_FX_SWAPS: FxSwapData[] = [
  {
    id: 'eurusd_swap',
    baseCurrency: 'EUR',
    quoteCurrency: 'USD',
    notional: { amount: 5_000_000, currency: 'EUR' },
    nearDate: { year: 2024, month: 1, day: 4 },
    farDate: { year: 2024, month: 7, day: 4 },
    domesticCurveId: 'USD-OIS',
    foreignCurveId: 'EUR-OIS',
    nearRate: 1.0865,
    farRate: 1.092,
  },
];

// Complete props bundle for default FX example
export const DEFAULT_FX_PROPS: FxInstrumentsProps = {
  valuationDate: { year: 2024, month: 1, day: 2 },
  discountCurves: [DEFAULT_USD_OIS_CURVE, DEFAULT_EUR_OIS_CURVE],
  volSurface: DEFAULT_FX_VOL_SURFACE,
  fxQuotes: DEFAULT_FX_QUOTES,
  spots: DEFAULT_FX_SPOTS,
  options: DEFAULT_FX_OPTIONS,
  swaps: DEFAULT_FX_SWAPS,
};
