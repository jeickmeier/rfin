/**
 * Structured products fixture data.
 */

import { DateData, DiscountCurveData, VolSurfaceData, MoneyData } from './market-data';

// Basket constituent data
export interface BasketConstituentData {
  id: string;
  priceId: string;
  assetType: string;
  weight: number;
  ticker: string;
}

// Basket JSON data
export interface BasketJsonData {
  id: string;
  currency: string;
  discount_curve_id: string;
  expense_ratio: number;
  constituents: BasketConstituentData[];
}

// Private markets fund event
export interface FundEventData {
  date: string;
  amount: MoneyData;
  kind: 'contribution' | 'distribution';
}

// Private markets fund JSON data
export interface PrivateMarketsFundJsonData {
  id: string;
  currency: string;
  discount_curve_id: string;
  spec: {
    style: 'european' | 'american';
    catchup_mode: 'full' | 'none';
    irr_basis: string;
    tranches: Array<string | { preferred_irr: { irr: number } }>;
  };
  events: FundEventData[];
}

// Autocallable JSON data
export interface AutocallableJsonData {
  id: string;
  underlying_ticker: string;
  notional: MoneyData;
  observation_dates: string[];
  autocall_barriers: number[];
  coupons: number[];
  final_barrier: number;
  final_payoff_type: { CapitalProtection: { floor: number } } | 'PutSpread' | 'Digital';
  participation_rate: number;
  cap_level: number;
  day_count: string;
  discount_curve_id: string;
  spot_id: string;
  vol_surface_id: string;
}

// Market setup for structured products
export interface StructuredProductsMarketData {
  discountCurve: DiscountCurveData;
  volSurface: VolSurfaceData;
  spotPrices: Array<{ id: string; price: MoneyData }>;
}

export interface StructuredProductsProps {
  valuationDate?: DateData;
  market?: StructuredProductsMarketData;
  baskets?: BasketJsonData[];
  privateMarketsFunds?: PrivateMarketsFundJsonData[];
  autocallables?: AutocallableJsonData[];
}

// Default valuation date
const DEFAULT_VALUATION_DATE: DateData = { year: 2024, month: 1, day: 2 };

// Default market data
const DEFAULT_STRUCTURED_PRODUCTS_MARKET: StructuredProductsMarketData = {
  discountCurve: {
    id: 'USD-OIS',
    baseDate: DEFAULT_VALUATION_DATE,
    tenors: [0, 1, 3, 5],
    discountFactors: [1, 0.995, 0.98, 0.96],
    dayCount: 'act_365f',
    interpolation: 'monotone_convex',
    extrapolation: 'flat_forward',
    continuous: true,
  },
  volSurface: {
    id: 'EQUITY-VOL',
    expiries: [0.25, 0.5, 1, 2],
    strikes: [120, 140, 160, 180],
    vols: [
      0.28, 0.26, 0.25, 0.24, 0.27, 0.25, 0.24, 0.23, 0.26, 0.24, 0.23, 0.22, 0.25, 0.23, 0.22,
      0.21,
    ],
  },
  spotPrices: [
    { id: 'AAPL-SPOT', price: { amount: 150, currency: 'USD' } },
    { id: 'MSFT-SPOT', price: { amount: 380, currency: 'USD' } },
  ],
};

// Default baskets
const DEFAULT_BASKETS: BasketJsonData[] = [
  {
    id: 'multi_asset_basket',
    currency: 'USD',
    discount_curve_id: 'USD-OIS',
    expense_ratio: 0.0025,
    constituents: [
      { id: 'AAPL', priceId: 'AAPL-SPOT', assetType: 'equity', weight: 0.5, ticker: 'AAPL' },
      { id: 'MSFT', priceId: 'MSFT-SPOT', assetType: 'equity', weight: 0.5, ticker: 'MSFT' },
    ],
  },
];

// Default private markets funds
const DEFAULT_PRIVATE_MARKETS_FUNDS: PrivateMarketsFundJsonData[] = [
  {
    id: 'pe_fund_1',
    currency: 'USD',
    discount_curve_id: 'USD-OIS',
    spec: {
      style: 'european',
      catchup_mode: 'full',
      irr_basis: 'act_365f',
      tranches: ['return_of_capital', { preferred_irr: { irr: 0.08 } }],
    },
    events: [
      { date: '2024-01-02', amount: { amount: 2_000_000, currency: 'USD' }, kind: 'contribution' },
      { date: '2028-12-31', amount: { amount: 3_000_000, currency: 'USD' }, kind: 'distribution' },
    ],
  },
];

// Default autocallables
const DEFAULT_AUTOCALLABLES: AutocallableJsonData[] = [
  {
    id: 'autocallable_simple',
    underlying_ticker: 'AAPL',
    notional: { amount: 1_000_000, currency: 'USD' },
    observation_dates: ['2025-01-02', '2026-01-02'],
    autocall_barriers: [1.2, 1.2],
    coupons: [0.08, 0.1],
    final_barrier: 0.75,
    final_payoff_type: { CapitalProtection: { floor: 0.9 } },
    participation_rate: 1,
    cap_level: 2,
    day_count: 'act_365f',
    discount_curve_id: 'USD-OIS',
    spot_id: 'AAPL-SPOT',
    vol_surface_id: 'EQUITY-VOL',
  },
];

// Complete props bundle
export const DEFAULT_STRUCTURED_PRODUCTS_PROPS: StructuredProductsProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  market: DEFAULT_STRUCTURED_PRODUCTS_MARKET,
  baskets: DEFAULT_BASKETS,
  privateMarketsFunds: DEFAULT_PRIVATE_MARKETS_FUNDS,
  autocallables: DEFAULT_AUTOCALLABLES,
};

