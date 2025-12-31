/**
 * Exotic equity options fixture data.
 */

import { DateData, DiscountCurveData, VolSurfaceData, MoneyData } from './market-data';

// Barrier option data
export interface BarrierOptionData {
  id: string;
  underlyingTicker: string;
  strike: number;
  barrier: number;
  optionType: 'call' | 'put';
  barrierType: 'up_and_out' | 'up_and_in' | 'down_and_out' | 'down_and_in';
  expiry: DateData;
  notional: MoneyData;
  discountCurveId: string;
  spotId: string;
  volId: string;
  divYieldId: string;
  useGobetMiri?: boolean;
}

// Asian option data
export interface AsianOptionData {
  id: string;
  underlyingTicker: string;
  strike: number;
  expiry: DateData;
  fixingDates: DateData[];
  notional: MoneyData;
  discountCurveId: string;
  spotId: string;
  volId: string;
  averagingMethod: 'arithmetic' | 'geometric';
  optionType: 'call' | 'put';
  divYieldId: string;
}

// Lookback option JSON data
export interface LookbackOptionJsonData {
  id: string;
  underlying_ticker: string;
  strike: MoneyData;
  expiry: string;
  lookback_type: 'FixedStrike' | 'FloatingStrike';
  option_type: 'call' | 'put';
  notional: MoneyData;
  day_count: string;
  discount_curve_id: string;
  spot_id: string;
  vol_surface_id: string;
  div_yield_id: string;
}

// Cliquet option JSON data
export interface CliquetOptionJsonData {
  id: string;
  underlying_ticker: string;
  reset_dates: string[];
  local_cap: number;
  local_floor: number;
  global_cap: number;
  global_floor: number;
  notional: MoneyData;
  day_count: string;
  discount_curve_id: string;
  spot_id: string;
  vol_surface_id: string;
  div_yield_id: string;
}

// Market setup for exotic equity
export interface ExoticEquityMarketData {
  discountCurve: DiscountCurveData;
  volSurface: VolSurfaceData;
  spotPrices: Array<{ id: string; price: MoneyData }>;
  divYields: Array<{ id: string; value: number }>;
}

export interface ExoticEquityOptionsProps {
  valuationDate?: DateData;
  market?: ExoticEquityMarketData;
  barrierOptions?: BarrierOptionData[];
  asianOptions?: AsianOptionData[];
  lookbackOptions?: LookbackOptionJsonData[];
  cliquetOptions?: CliquetOptionJsonData[];
}

// Default valuation date
const DEFAULT_VALUATION_DATE: DateData = { year: 2024, month: 1, day: 2 };

// Default market data
const DEFAULT_EXOTIC_EQUITY_MARKET: ExoticEquityMarketData = {
  discountCurve: {
    id: 'USD-OIS',
    baseDate: DEFAULT_VALUATION_DATE,
    tenors: [0, 0.5, 1, 3, 5],
    discountFactors: [1, 0.997, 0.994, 0.9725, 0.948],
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
    { id: 'AAPL', price: { amount: 150, currency: 'USD' } },
    { id: 'AAPL-SPOT', price: { amount: 150, currency: 'USD' } },
    { id: 'EQUITY-SPOT', price: { amount: 150, currency: 'USD' } },
  ],
  divYields: [
    { id: 'AAPL-DIVYIELD', value: 0.015 },
    { id: 'EQUITY-DIVYIELD', value: 0.015 },
  ],
};

// Default barrier options
const DEFAULT_BARRIER_OPTIONS: BarrierOptionData[] = [
  {
    id: 'barrier_up_out_call',
    underlyingTicker: 'AAPL',
    strike: 150,
    barrier: 180,
    optionType: 'call',
    barrierType: 'up_and_out',
    expiry: { year: 2024, month: 12, day: 31 },
    notional: { amount: 150, currency: 'USD' },
    discountCurveId: 'USD-OIS',
    spotId: 'AAPL-SPOT',
    volId: 'EQUITY-VOL',
    divYieldId: 'AAPL-DIVYIELD',
    useGobetMiri: false,
  },
  {
    id: 'barrier_down_in_put',
    underlyingTicker: 'AAPL',
    strike: 140,
    barrier: 130,
    optionType: 'put',
    barrierType: 'down_and_in',
    expiry: { year: 2024, month: 12, day: 31 },
    notional: { amount: 140, currency: 'USD' },
    discountCurveId: 'USD-OIS',
    spotId: 'AAPL-SPOT',
    volId: 'EQUITY-VOL',
    divYieldId: 'AAPL-DIVYIELD',
    useGobetMiri: false,
  },
];

// Generate monthly fixing dates for 2024
function generateMonthlyFixingDates(year: number): DateData[] {
  const dates: DateData[] = [];
  for (let i = 1; i <= 12; i++) {
    dates.push({ year, month: i, day: 15 });
  }
  return dates;
}

// Default Asian options
const DEFAULT_ASIAN_OPTIONS: AsianOptionData[] = [
  {
    id: 'asian_arithmetic_call',
    underlyingTicker: 'AAPL',
    strike: 150,
    expiry: { year: 2024, month: 12, day: 31 },
    fixingDates: generateMonthlyFixingDates(2024),
    notional: { amount: 150, currency: 'USD' },
    discountCurveId: 'USD-OIS',
    spotId: 'AAPL-SPOT',
    volId: 'EQUITY-VOL',
    averagingMethod: 'arithmetic',
    optionType: 'call',
    divYieldId: 'AAPL-DIVYIELD',
  },
  {
    id: 'asian_geometric_put',
    underlyingTicker: 'AAPL',
    strike: 145,
    expiry: { year: 2024, month: 12, day: 31 },
    fixingDates: generateMonthlyFixingDates(2024),
    notional: { amount: 145, currency: 'USD' },
    discountCurveId: 'USD-OIS',
    spotId: 'AAPL-SPOT',
    volId: 'EQUITY-VOL',
    averagingMethod: 'geometric',
    optionType: 'put',
    divYieldId: 'AAPL-DIVYIELD',
  },
];

// Default Lookback options
const DEFAULT_LOOKBACK_OPTIONS: LookbackOptionJsonData[] = [
  {
    id: 'lookback_fixed_strike',
    underlying_ticker: 'AAPL',
    strike: { amount: 150, currency: 'USD' },
    expiry: '2024-12-31',
    lookback_type: 'FixedStrike',
    option_type: 'call',
    notional: { amount: 1, currency: 'USD' },
    day_count: 'act_365f',
    discount_curve_id: 'USD-OIS',
    spot_id: 'AAPL-SPOT',
    vol_surface_id: 'EQUITY-VOL',
    div_yield_id: 'AAPL-DIVYIELD',
  },
];

// Default Cliquet options
const DEFAULT_CLIQUET_OPTIONS: CliquetOptionJsonData[] = [
  {
    id: 'cliquet_local_floor',
    underlying_ticker: 'AAPL',
    reset_dates: ['2024-04-01', '2024-07-01', '2024-10-01', '2024-12-31'],
    local_cap: 0.15,
    local_floor: -0.05,
    global_cap: 0.3,
    global_floor: 0.0,
    notional: { amount: 1_000_000, currency: 'USD' },
    day_count: 'act_365f',
    discount_curve_id: 'USD-OIS',
    spot_id: 'AAPL-SPOT',
    vol_surface_id: 'EQUITY-VOL',
    div_yield_id: 'AAPL-DIVYIELD',
  },
];

// Complete props bundle
export const DEFAULT_EXOTIC_EQUITY_PROPS: ExoticEquityOptionsProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  market: DEFAULT_EXOTIC_EQUITY_MARKET,
  barrierOptions: DEFAULT_BARRIER_OPTIONS,
  asianOptions: DEFAULT_ASIAN_OPTIONS,
  lookbackOptions: DEFAULT_LOOKBACK_OPTIONS,
  cliquetOptions: DEFAULT_CLIQUET_OPTIONS,
};
