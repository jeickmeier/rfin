/**
 * Exotic FX derivatives fixture data.
 */

import { DateData, DiscountCurveData, VolSurfaceData, MoneyData } from './market-data';

// FX quote for matrix setup in exotic FX context
export interface ExoticFxQuoteData {
  baseCurrency: string;
  quoteCurrency: string;
  rate: number;
}

// FX Barrier option JSON data
export interface FxBarrierOptionJsonData {
  id: string;
  domestic_currency: string;
  foreign_currency: string;
  strike: MoneyData;
  barrier: MoneyData;
  option_type: 'call' | 'put';
  barrier_type: 'UpAndOut' | 'UpAndIn' | 'DownAndOut' | 'DownAndIn';
  expiry: string;
  notional: MoneyData;
  day_count: string;
  use_gobet_miri: boolean;
  domestic_discount_curve_id: string;
  foreign_discount_curve_id: string;
  fx_spot_id: string;
  fx_vol_id: string;
}

// Quanto option JSON data
export interface QuantoOptionJsonData {
  id: string;
  underlying_ticker: string;
  equity_strike: MoneyData;
  expiry: string;
  option_type: 'call' | 'put';
  notional: MoneyData;
  domestic_currency: string;
  foreign_currency: string;
  correlation: number;
  day_count: string;
  discount_curve_id: string;
  foreign_discount_curve_id: string;
  spot_id: string;
  vol_surface_id: string;
  fx_vol_id: string;
}

// Market setup for exotic FX
export interface ExoticFxMarketData {
  discountCurves: DiscountCurveData[];
  fxQuotes: ExoticFxQuoteData[];
  fxVolSurface: VolSurfaceData;
  equityVolSurface: VolSurfaceData;
  spotPrices: Array<{ id: string; price: MoneyData }>;
}

export interface ExoticFxDerivativesProps {
  valuationDate?: DateData;
  market?: ExoticFxMarketData;
  fxBarrierOptions?: FxBarrierOptionJsonData[];
  quantoOptions?: QuantoOptionJsonData[];
}

// Default valuation date
const DEFAULT_VALUATION_DATE: DateData = { year: 2024, month: 1, day: 2 };

// Default discount curves
const DEFAULT_USD_DISCOUNT: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 0.5, 1, 3, 5],
  discountFactors: [1, 0.9975, 0.9945, 0.972, 0.945],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

const DEFAULT_EUR_DISCOUNT: DiscountCurveData = {
  id: 'EUR-OIS',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 0.5, 1, 3, 5],
  discountFactors: [1, 0.998, 0.996, 0.98, 0.955],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

const DEFAULT_GBP_DISCOUNT: DiscountCurveData = {
  id: 'GBP-OIS',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 0.5, 1, 3, 5],
  discountFactors: [1, 0.9978, 0.9955, 0.978, 0.952],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default market data
const DEFAULT_EXOTIC_FX_MARKET: ExoticFxMarketData = {
  discountCurves: [DEFAULT_USD_DISCOUNT, DEFAULT_EUR_DISCOUNT, DEFAULT_GBP_DISCOUNT],
  fxQuotes: [
    { baseCurrency: 'EUR', quoteCurrency: 'USD', rate: 1.085 },
    { baseCurrency: 'GBP', quoteCurrency: 'USD', rate: 1.265 },
  ],
  fxVolSurface: {
    id: 'FX-VOL',
    expiries: [0.25, 0.5, 1, 2],
    strikes: [1.05, 1.1, 1.15],
    vols: [0.14, 0.13, 0.12, 0.13, 0.12, 0.11, 0.12, 0.11, 0.1, 0.11, 0.1, 0.095],
  },
  equityVolSurface: {
    id: 'EQUITY-VOL',
    expiries: [0.25, 0.5, 1, 2],
    strikes: [100, 120, 140, 160],
    vols: [
      0.28, 0.26, 0.25, 0.24, 0.27, 0.25, 0.24, 0.23, 0.26, 0.24, 0.23, 0.22, 0.25, 0.23, 0.22,
      0.21,
    ],
  },
  spotPrices: [
    { id: 'EURUSD-SPOT', price: { amount: 1.085, currency: 'USD' } },
    { id: 'GBPUSD-SPOT', price: { amount: 1.265, currency: 'USD' } },
    { id: 'EUR-EQUITY-SPOT', price: { amount: 150, currency: 'EUR' } },
    { id: 'GBP-EQUITY-SPOT', price: { amount: 140, currency: 'GBP' } },
  ],
};

// Default FX barrier options
const DEFAULT_FX_BARRIER_OPTIONS: FxBarrierOptionJsonData[] = [
  {
    id: 'fx_barrier_up_out',
    domestic_currency: 'USD',
    foreign_currency: 'EUR',
    strike: { amount: 1.1, currency: 'USD' },
    barrier: { amount: 1.15, currency: 'USD' },
    option_type: 'call',
    barrier_type: 'UpAndOut',
    expiry: '2024-12-31',
    notional: { amount: 1, currency: 'USD' },
    day_count: 'Act365F',
    use_gobet_miri: false,
    domestic_discount_curve_id: 'USD-OIS',
    foreign_discount_curve_id: 'EUR-OIS',
    fx_spot_id: 'EURUSD-SPOT',
    fx_vol_id: 'FX-VOL',
  },
  {
    id: 'fx_barrier_down_in',
    domestic_currency: 'USD',
    foreign_currency: 'GBP',
    strike: { amount: 1.25, currency: 'USD' },
    barrier: { amount: 1.2, currency: 'USD' },
    option_type: 'put',
    barrier_type: 'DownAndIn',
    expiry: '2024-12-31',
    notional: { amount: 1, currency: 'USD' },
    day_count: 'Act365F',
    use_gobet_miri: false,
    domestic_discount_curve_id: 'USD-OIS',
    foreign_discount_curve_id: 'GBP-OIS',
    fx_spot_id: 'GBPUSD-SPOT',
    fx_vol_id: 'FX-VOL',
  },
];

// Default quanto options
const DEFAULT_QUANTO_OPTIONS: QuantoOptionJsonData[] = [
  {
    id: 'quanto_call_eur_usd',
    underlying_ticker: 'EUR-EQUITY',
    equity_strike: { amount: 150, currency: 'EUR' },
    expiry: '2024-12-31',
    option_type: 'call',
    notional: { amount: 1, currency: 'USD' },
    domestic_currency: 'USD',
    foreign_currency: 'EUR',
    correlation: 0.3,
    day_count: 'Act365F',
    discount_curve_id: 'USD-OIS',
    foreign_discount_curve_id: 'EUR-OIS',
    spot_id: 'EUR-EQUITY-SPOT',
    vol_surface_id: 'EQUITY-VOL',
    fx_vol_id: 'FX-VOL',
  },
  {
    id: 'quanto_put_gbp_usd',
    underlying_ticker: 'GBP-EQUITY',
    equity_strike: { amount: 140, currency: 'GBP' },
    expiry: '2024-12-31',
    option_type: 'put',
    notional: { amount: 1, currency: 'USD' },
    domestic_currency: 'USD',
    foreign_currency: 'GBP',
    correlation: 0.3,
    day_count: 'Act365F',
    discount_curve_id: 'USD-OIS',
    foreign_discount_curve_id: 'GBP-OIS',
    spot_id: 'GBP-EQUITY-SPOT',
    vol_surface_id: 'EQUITY-VOL',
    fx_vol_id: 'FX-VOL',
  },
];

// Complete props bundle
export const DEFAULT_EXOTIC_FX_PROPS: ExoticFxDerivativesProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  market: DEFAULT_EXOTIC_FX_MARKET,
  fxBarrierOptions: DEFAULT_FX_BARRIER_OPTIONS,
  quantoOptions: DEFAULT_QUANTO_OPTIONS,
};
