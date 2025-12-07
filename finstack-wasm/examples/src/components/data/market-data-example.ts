/**
 * Market data example fixture data.
 */

import { DateData, DiscountCurveData, MoneyData } from './market-data';

// CPI series data
export interface CpiSeriesData {
  id: string;
  currency: string;
  dates: DateData[];
  values: number[];
}

// FX quote data for market example
export interface FxQuoteExampleData {
  baseCurrency: string;
  quoteCurrency: string;
  rate: number;
}

// Equity price data
export interface EquityPriceData {
  symbol: string;
  price: MoneyData;
}

export interface MarketDataExampleProps {
  baseDate?: DateData;
  discountCurve?: DiscountCurveData;
  cpiSeries?: CpiSeriesData;
  fxQuote?: FxQuoteExampleData;
  equityPrice?: EquityPriceData;
  cpiLookupDate?: DateData;
}

// Default base date
export const DEFAULT_MARKET_BASE_DATE: DateData = { year: 2024, month: 1, day: 2 };

// Default discount curve
export const DEFAULT_MARKET_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: DEFAULT_MARKET_BASE_DATE,
  tenors: [0, 0.5, 1, 2],
  discountFactors: [1, 0.9905, 0.979, 0.955],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default CPI series
export const DEFAULT_CPI_SERIES: CpiSeriesData = {
  id: 'US-CPI',
  currency: 'USD',
  dates: [
    { year: 2023, month: 12, day: 31 },
    { year: 2024, month: 3, day: 31 },
  ],
  values: [300.1, 302.8],
};

// Default FX quote
export const DEFAULT_FX_QUOTE: FxQuoteExampleData = {
  baseCurrency: 'USD',
  quoteCurrency: 'EUR',
  rate: 0.92,
};

// Default equity price
export const DEFAULT_EQUITY_PRICE: EquityPriceData = {
  symbol: 'AAPL',
  price: { amount: 102.45, currency: 'USD' },
};

// Default CPI lookup date
export const DEFAULT_CPI_LOOKUP_DATE: DateData = { year: 2024, month: 2, day: 15 };

// Complete props bundle
export const DEFAULT_MARKET_DATA_EXAMPLE_PROPS: MarketDataExampleProps = {
  baseDate: DEFAULT_MARKET_BASE_DATE,
  discountCurve: DEFAULT_MARKET_DISCOUNT_CURVE,
  cpiSeries: DEFAULT_CPI_SERIES,
  fxQuote: DEFAULT_FX_QUOTE,
  equityPrice: DEFAULT_EQUITY_PRICE,
  cpiLookupDate: DEFAULT_CPI_LOOKUP_DATE,
};

