/**
 * Equity instruments fixture data.
 */

import { DiscountCurveData, VolSurfaceData, DEFAULT_USD_OIS_CURVE } from './market-data';

export interface EquityPositionData {
  id: string;
  ticker: string;
  currency: string;
  quantity: number;
  costBasis: number | null;
}

export interface EquityOptionData {
  id: string;
  ticker: string;
  strike: number;
  expiryDate: { year: number; month: number; day: number };
  spotPrice: { amount: number; currency: string };
  quantity: number;
  optionType: 'call' | 'put';
}

export interface EquityMarketData {
  ticker: string;
  spotPrice: { amount: number; currency: string };
  dividendYield: number;
}

export interface EquityInstrumentsProps {
  valuationDate?: { year: number; month: number; day: number };
  discountCurve?: DiscountCurveData;
  volSurface?: VolSurfaceData;
  marketData?: EquityMarketData[];
  positions?: EquityPositionData[];
  options?: EquityOptionData[];
}

// Default equity volatility surface
export const DEFAULT_EQUITY_VOL_SURFACE: VolSurfaceData = {
  id: 'EQUITY-VOL',
  expiries: [0.25, 0.5, 1.0, 2.0],
  strikes: [120.0, 140.0, 160.0, 180.0],
  // Flattened grid (row-major): 4 expiries x 4 strikes = 16 values
  vols: [
    0.28,
    0.26,
    0.25,
    0.24, // 3M expiry
    0.27,
    0.25,
    0.24,
    0.23, // 6M expiry
    0.26,
    0.24,
    0.23,
    0.22, // 1Y expiry
    0.25,
    0.23,
    0.22,
    0.21, // 2Y expiry
  ],
};

// Default market data for AAPL
export const DEFAULT_EQUITY_MARKET_DATA: EquityMarketData[] = [
  {
    ticker: 'AAPL',
    spotPrice: { amount: 150.0, currency: 'USD' },
    dividendYield: 0.015,
  },
];

// Default equity positions
export const DEFAULT_EQUITY_POSITIONS: EquityPositionData[] = [
  {
    id: 'aapl_position',
    ticker: 'AAPL',
    currency: 'USD',
    quantity: 1000.0,
    costBasis: null,
  },
];

// Default equity options
export const DEFAULT_EQUITY_OPTIONS: EquityOptionData[] = [
  {
    id: 'aapl_call_150',
    ticker: 'AAPL',
    strike: 150.0,
    expiryDate: { year: 2024, month: 12, day: 31 },
    spotPrice: { amount: 150.0, currency: 'USD' },
    quantity: 100.0,
    optionType: 'call',
  },
  {
    id: 'aapl_put_140',
    ticker: 'AAPL',
    strike: 140.0,
    expiryDate: { year: 2024, month: 9, day: 30 },
    spotPrice: { amount: 140.0, currency: 'USD' },
    quantity: 100.0,
    optionType: 'put',
  },
];

// Complete props bundle for default equity example
export const DEFAULT_EQUITY_PROPS: EquityInstrumentsProps = {
  valuationDate: { year: 2024, month: 1, day: 2 },
  discountCurve: DEFAULT_USD_OIS_CURVE,
  volSurface: DEFAULT_EQUITY_VOL_SURFACE,
  marketData: DEFAULT_EQUITY_MARKET_DATA,
  positions: DEFAULT_EQUITY_POSITIONS,
  options: DEFAULT_EQUITY_OPTIONS,
};
