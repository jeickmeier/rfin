/**
 * Calibration Component State Types
 *
 * This module provides:
 * 1. Component state interfaces (using QuoteEditor data types for form binding)
 * 2. Factory functions for creating default states
 * 3. State serialization/update utilities for LLM chat control
 *
 * For wire-format types used with WASM fromJSON/toJSON, see `../../types/calibration-state`.
 */

import type {
  DiscountQuoteData,
  ForwardQuoteData,
  CdsQuoteData,
  InflationSwapQuoteData,
  VolQuoteData,
  TrancheQuoteData,
} from './QuoteEditor';

// Re-export shared types from centralized location
export type {
  DateJson,
  SolverKindJson,
  CalibrationConfigJson,
  CalibrationTab,
} from '../../types/calibration-state';

// Local import for use in this file
import type { DateJson, CalibrationConfigJson } from '../../types/calibration-state';

// ============================================================================
// Shared Config Default
// ============================================================================

export const DEFAULT_CALIBRATION_CONFIG: CalibrationConfigJson = {
  solverKind: 'Brent',
  maxIterations: 40,
  tolerance: 1e-8,
  verbose: false,
};

// ============================================================================
// Component State Interfaces (using QuoteEditor types for form binding)
// ============================================================================

export interface DiscountCurveCalibrationState {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  quotes: DiscountQuoteData[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

export interface ForwardCurveCalibrationState {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  tenor: number;
  discountCurveId: string;
  quotes: ForwardQuoteData[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

export interface HazardCurveCalibrationState {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  entity: string;
  seniority: string;
  recoveryRate: number;
  discountCurveId: string;
  quotes: CdsQuoteData[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

export interface InflationCurveCalibrationState {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  indexName: string;
  baseCpi: number;
  discountCurveId: string;
  quotes: InflationSwapQuoteData[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

export interface VolSurfaceCalibrationState {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  underlying: string;
  spotPrice: number;
  expiries: number[];
  strikes: number[];
  discountCurveId: string;
  quotes: VolQuoteData[];
  config: CalibrationConfigJson;
  tolerance: number;
  showChart: boolean;
}

export interface BaseCorrelationCalibrationState {
  baseDate: DateJson;
  curveId: string;
  indexId: string;
  series: number;
  maturityYears: number;
  discountCurveId: string;
  quotes: TrancheQuoteData[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

export interface CalibrationSuiteState {
  activeTab: 'discount' | 'forward' | 'hazard' | 'inflation' | 'vol' | 'correlation';
  discount: DiscountCurveCalibrationState;
  forward: ForwardCurveCalibrationState;
  hazard: HazardCurveCalibrationState;
  inflation: InflationCurveCalibrationState;
  vol: VolSurfaceCalibrationState;
  correlation: BaseCorrelationCalibrationState;
}

// ============================================================================
// Default State Factories
// ============================================================================

export function createDefaultDiscountCurveState(
  overrides?: Partial<DiscountCurveCalibrationState>
): DiscountCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() },
    curveId: 'USD-OIS',
    currency: 'USD',
    quotes: [],
    config: { ...DEFAULT_CALIBRATION_CONFIG },
    showChart: true,
    ...overrides,
  };
}

export function createDefaultForwardCurveState(
  overrides?: Partial<ForwardCurveCalibrationState>
): ForwardCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() },
    curveId: 'USD-SOFR-3M',
    currency: 'USD',
    tenor: 0.25,
    discountCurveId: 'USD-OIS',
    quotes: [],
    config: { ...DEFAULT_CALIBRATION_CONFIG, maxIterations: 30 },
    showChart: true,
    ...overrides,
  };
}

export function createDefaultHazardCurveState(
  overrides?: Partial<HazardCurveCalibrationState>
): HazardCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() },
    curveId: 'ACME-Senior',
    currency: 'USD',
    entity: 'ACME',
    seniority: 'senior',
    recoveryRate: 0.4,
    discountCurveId: 'USD-OIS',
    quotes: [],
    config: { ...DEFAULT_CALIBRATION_CONFIG, maxIterations: 25 },
    showChart: true,
    ...overrides,
  };
}

export function createDefaultInflationCurveState(
  overrides?: Partial<InflationCurveCalibrationState>
): InflationCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() },
    curveId: 'US-CPI-U',
    currency: 'USD',
    indexName: 'US-CPI-U',
    baseCpi: 300,
    discountCurveId: 'USD-OIS',
    quotes: [],
    config: { ...DEFAULT_CALIBRATION_CONFIG, maxIterations: 25 },
    showChart: true,
    ...overrides,
  };
}

export function createDefaultVolSurfaceState(
  overrides?: Partial<VolSurfaceCalibrationState>
): VolSurfaceCalibrationState {
  const now = new Date();
  return {
    baseDate: { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() },
    curveId: 'SPY-VOL',
    currency: 'USD',
    underlying: 'SPY',
    spotPrice: 100,
    expiries: [0.5, 1],
    strikes: [90, 100, 110],
    discountCurveId: 'USD-OIS',
    quotes: [],
    config: { ...DEFAULT_CALIBRATION_CONFIG, maxIterations: 100 },
    tolerance: 0.5,
    showChart: true,
    ...overrides,
  };
}

export function createDefaultBaseCorrelationState(
  overrides?: Partial<BaseCorrelationCalibrationState>
): BaseCorrelationCalibrationState {
  const now = new Date();
  return {
    baseDate: { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() },
    curveId: 'CDX-IG-BASECORR',
    indexId: 'CDX.NA.IG.42',
    series: 42,
    maturityYears: 5,
    discountCurveId: 'USD-OIS',
    quotes: [],
    config: { ...DEFAULT_CALIBRATION_CONFIG, maxIterations: 50 },
    showChart: true,
    ...overrides,
  };
}

export function createDefaultCalibrationSuiteState(
  overrides?: Partial<CalibrationSuiteState>
): CalibrationSuiteState {
  return {
    activeTab: 'discount',
    discount: createDefaultDiscountCurveState(),
    forward: createDefaultForwardCurveState(),
    hazard: createDefaultHazardCurveState(),
    inflation: createDefaultInflationCurveState(),
    vol: createDefaultVolSurfaceState(),
    correlation: createDefaultBaseCorrelationState(),
    ...overrides,
  };
}

// ============================================================================
// State Utilities
// ============================================================================

export function serializeCalibrationState(state: CalibrationSuiteState): string {
  return JSON.stringify(state, null, 2);
}

export function deserializeCalibrationState(json: string): CalibrationSuiteState {
  const parsed = JSON.parse(json);
  return createDefaultCalibrationSuiteState(parsed);
}

export type DeepPartial<T> = T extends object ? { [P in keyof T]?: DeepPartial<T[P]> } : T;

function deepMerge<T extends object>(target: T, source: DeepPartial<T>): T {
  const result = { ...target } as Record<string, unknown>;
  for (const key in source) {
    if (Object.prototype.hasOwnProperty.call(source, key)) {
      const sourceValue = source[key];
      const targetValue = (target as Record<string, unknown>)[key];
      if (
        sourceValue !== undefined &&
        typeof sourceValue === 'object' &&
        sourceValue !== null &&
        !Array.isArray(sourceValue) &&
        typeof targetValue === 'object' &&
        targetValue !== null &&
        !Array.isArray(targetValue)
      ) {
        result[key] = deepMerge(targetValue, sourceValue as DeepPartial<object>);
      } else if (sourceValue !== undefined) {
        result[key] = sourceValue;
      }
    }
  }
  return result as T;
}

export function applyStateUpdate(
  currentState: CalibrationSuiteState,
  update: DeepPartial<CalibrationSuiteState>
): CalibrationSuiteState {
  return deepMerge(currentState, update);
}

// ============================================================================
// LLM Command Types
// ============================================================================

export type CalibrationCommand =
  | { type: 'SET_STATE'; state: DeepPartial<CalibrationSuiteState> }
  | { type: 'SET_QUOTES'; calibrationType: keyof Omit<CalibrationSuiteState, 'activeTab'>; quotes: unknown[] }
  | { type: 'SET_TAB'; tab: CalibrationSuiteState['activeTab'] }
  | { type: 'CALIBRATE'; calibrationType: keyof Omit<CalibrationSuiteState, 'activeTab'> }
  | { type: 'RESET'; calibrationType?: keyof Omit<CalibrationSuiteState, 'activeTab'> };

export interface CalibrationCommandResult {
  success: boolean;
  message: string;
  state?: CalibrationSuiteState;
  error?: string;
}

export const EXAMPLE_LLM_COMMAND = `{
  "type": "SET_STATE",
  "state": {
    "activeTab": "discount",
    "discount": {
      "currency": "EUR",
      "curveId": "EUR-OIS",
      "quotes": [
        { "type": "deposit", "maturityYear": 2025, "maturityMonth": 1, "maturityDay": 15, "rate": 0.035, "dayCount": "act_360" },
        { "type": "swap", "maturityYear": 2026, "maturityMonth": 1, "maturityDay": 15, "rate": 0.038, "fixedFrequency": "annual", "floatFrequency": "quarterly", "fixedDayCount": "30_360", "floatDayCount": "act_360", "index": "EUR-ESTR" }
      ]
    }
  }
}`;
