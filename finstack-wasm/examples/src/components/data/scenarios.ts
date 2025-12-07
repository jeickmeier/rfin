/**
 * Scenarios example fixture data.
 */

import { DiscountCurveData, VolSurfaceData, DateData } from './market-data';

// Scenario operation specification
export interface ScenarioOperationData {
  type:
    | 'curve_parallel_bp'
    | 'curve_node_bp'
    | 'equity_price_pct'
    | 'vol_surface_parallel_pct'
    | 'vol_surface_bucket_pct'
    | 'stmt_forecast_percent'
    | 'time_roll_forward';
  params: Record<string, unknown>;
}

// Scenario specification
export interface ScenarioSpecData {
  id: string;
  name: string;
  description?: string;
  operations: ScenarioOperationData[];
  priority: number;
}

// Market setup for scenarios
export interface ScenariosMarketData {
  discountCurves?: DiscountCurveData[];
  equityPrices?: Array<{ symbol: string; price: number; currency: string }>;
  volSurfaces?: VolSurfaceData[];
}

// Model setup for scenarios
export interface ScenariosModelData {
  name: string;
  periodRange: string;
  actualsThrough: string | null;
  revenueValues?: { [period: string]: number };
  revenueForecastGrowth?: number;
}

export interface ScenariosExampleProps {
  baseDate?: DateData;
  market?: ScenariosMarketData;
  model?: ScenariosModelData;
  scenarios?: ScenarioSpecData[];
}

// Default base date
export const DEFAULT_SCENARIO_BASE_DATE: DateData = { year: 2025, month: 1, day: 1 };

// Default discount curve for scenarios
export const DEFAULT_SCENARIO_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD_SOFR',
  baseDate: DEFAULT_SCENARIO_BASE_DATE,
  tenors: [0, 0.25, 0.5, 1, 2, 5],
  discountFactors: [1, 0.9975, 0.995, 0.98, 0.96, 0.92],
  dayCount: 'act_365f',
  interpolation: 'monotone_convex',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Default equity prices
export const DEFAULT_EQUITY_PRICES = [
  { symbol: 'SPY', price: 450, currency: 'USD' },
  { symbol: 'QQQ', price: 380, currency: 'USD' },
];

// Default vol surface
export const DEFAULT_SCENARIO_VOL_SURFACE: VolSurfaceData = {
  id: 'SPX_VOL',
  expiries: [0.25, 0.5, 1],
  strikes: [90, 100, 110],
  vols: [0.2, 0.18, 0.22, 0.21, 0.19, 0.23, 0.22, 0.2, 0.24],
};

// Default market setup
export const DEFAULT_SCENARIO_MARKET: ScenariosMarketData = {
  discountCurves: [DEFAULT_SCENARIO_DISCOUNT_CURVE],
  equityPrices: DEFAULT_EQUITY_PRICES,
  volSurfaces: [DEFAULT_SCENARIO_VOL_SURFACE],
};

// Default model setup
export const DEFAULT_SCENARIO_MODEL: ScenariosModelData = {
  name: 'revenue_model',
  periodRange: '2025Q1..Q4',
  actualsThrough: '2025Q1',
  revenueValues: { '2025Q1': 1_000_000 },
  revenueForecastGrowth: 0.1,
};

// Basic curve shock scenario
export const BASIC_CURVE_SHOCK: ScenarioSpecData = {
  id: 'rate_shock',
  name: 'Rate Shock: +50bp',
  description: 'Parallel shift to discount curve',
  operations: [
    {
      type: 'curve_parallel_bp',
      params: { curveKind: 'discount', curveId: 'USD_SOFR', bpShift: 50 },
    },
  ],
  priority: 0,
};

// Multi-asset stress scenario
export const MULTI_ASSET_STRESS: ScenarioSpecData = {
  id: 'multi_asset_stress',
  name: 'Multi-Asset Stress Test',
  description: 'Rates up, equities down, volatility up',
  operations: [
    {
      type: 'curve_parallel_bp',
      params: { curveKind: 'discount', curveId: 'USD_SOFR', bpShift: 75 },
    },
    {
      type: 'equity_price_pct',
      params: { symbols: ['SPY'], pctShift: -15 },
    },
    {
      type: 'vol_surface_parallel_pct',
      params: { surfaceKind: 'equity', surfaceId: 'SPX_VOL', pctShift: 25 },
    },
  ],
  priority: 0,
};

// Statement stress scenario
export const STATEMENT_STRESS: ScenarioSpecData = {
  id: 'revenue_stress',
  name: 'Revenue Stress: -20%',
  description: 'Apply -20% shock to revenue forecast',
  operations: [
    {
      type: 'stmt_forecast_percent',
      params: { nodeId: 'revenue', pctShift: -20 },
    },
  ],
  priority: 0,
};

// Time roll scenario
export const TIME_ROLL: ScenarioSpecData = {
  id: 'time_roll',
  name: 'Roll Forward 1 Month',
  description: 'Advance valuation date by 1 month',
  operations: [
    {
      type: 'time_roll_forward',
      params: { period: '1M', applyCarry: true },
    },
  ],
  priority: 0,
};

// Comprehensive stress scenario
export const COMPREHENSIVE_STRESS: ScenarioSpecData = {
  id: 'comprehensive_stress',
  name: 'Comprehensive Stress Test',
  description: 'Full market and statement stress',
  operations: [
    {
      type: 'curve_parallel_bp',
      params: { curveKind: 'discount', curveId: 'USD_SOFR', bpShift: 100 },
    },
    {
      type: 'equity_price_pct',
      params: { symbols: ['SPY', 'QQQ'], pctShift: -20 },
    },
    {
      type: 'vol_surface_parallel_pct',
      params: { surfaceKind: 'equity', surfaceId: 'SPX_VOL', pctShift: 30 },
    },
    {
      type: 'vol_surface_bucket_pct',
      params: {
        surfaceKind: 'equity',
        surfaceId: 'SPX_VOL',
        tenors: null,
        strikes: [100],
        pctShift: 15,
      },
    },
    {
      type: 'stmt_forecast_percent',
      params: { nodeId: 'revenue', pctShift: -15 },
    },
  ],
  priority: 0,
};

// Default scenarios
export const DEFAULT_SCENARIOS: ScenarioSpecData[] = [
  BASIC_CURVE_SHOCK,
  MULTI_ASSET_STRESS,
  STATEMENT_STRESS,
  TIME_ROLL,
  COMPREHENSIVE_STRESS,
];

// Complete props bundle
export const DEFAULT_SCENARIOS_PROPS: ScenariosExampleProps = {
  baseDate: DEFAULT_SCENARIO_BASE_DATE,
  market: DEFAULT_SCENARIO_MARKET,
  model: DEFAULT_SCENARIO_MODEL,
  scenarios: DEFAULT_SCENARIOS,
};
