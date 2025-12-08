/**
 * JSON-serializable state types for calibration components.
 * These types enable LLM chat control over calibration parameters.
 *
 * All state types are designed to be:
 * 1. Fully JSON-serializable (no functions, classes, or complex objects)
 * 2. Self-describing with clear field names
 * 3. Composable for building complete calibration scenarios
 */

import type {
  DiscountQuoteData,
  ForwardQuoteData,
  CdsQuoteData,
  InflationSwapQuoteData,
  VolQuoteData,
  TrancheQuoteData,
} from './QuoteEditor';

// ============================================================================
// Common Date Type (JSON-serializable)
// ============================================================================

/** JSON-serializable date representation */
export interface DateJson {
  year: number;
  month: number;
  day: number;
}

// ============================================================================
// Calibration Config (JSON-serializable)
// ============================================================================

/** Solver type for calibration */
export type SolverKindJson = 'Brent' | 'Newton';

/** JSON-serializable calibration configuration */
export interface CalibrationConfigJson {
  solverKind: SolverKindJson;
  maxIterations: number;
  tolerance: number;
  verbose: boolean;
}

/** Default calibration config */
export const DEFAULT_CALIBRATION_CONFIG: CalibrationConfigJson = {
  solverKind: 'Brent',
  maxIterations: 40,
  tolerance: 1e-8,
  verbose: false,
};

// ============================================================================
// Discount Curve Calibration State
// ============================================================================

/** Complete JSON state for discount curve calibration */
export interface DiscountCurveCalibrationState {
  /** Base date for the curve */
  baseDate: DateJson;
  /** Unique identifier for the curve */
  curveId: string;
  /** Currency code (ISO 4217) */
  currency: string;
  /** Market quotes (deposits and swaps) */
  quotes: DiscountQuoteData[];
  /** Calibration configuration */
  config: CalibrationConfigJson;
  /** Whether to show the chart */
  showChart: boolean;
}

/** Default state factory for discount curve calibration */
export function createDefaultDiscountCurveState(
  overrides?: Partial<DiscountCurveCalibrationState>
): DiscountCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
      day: now.getDate(),
    },
    curveId: 'USD-OIS',
    currency: 'USD',
    quotes: [],
    config: { ...DEFAULT_CALIBRATION_CONFIG },
    showChart: true,
    ...overrides,
  };
}

// ============================================================================
// Forward Curve Calibration State
// ============================================================================

/** Complete JSON state for forward curve calibration */
export interface ForwardCurveCalibrationState {
  /** Base date for the curve */
  baseDate: DateJson;
  /** Unique identifier for the curve */
  curveId: string;
  /** Currency code (ISO 4217) */
  currency: string;
  /** Tenor in years (e.g., 0.25 for 3M) */
  tenor: number;
  /** ID of the discount curve to use */
  discountCurveId: string;
  /** Market quotes (deposits, FRAs, and swaps) */
  quotes: ForwardQuoteData[];
  /** Calibration configuration */
  config: CalibrationConfigJson;
  /** Whether to show the chart */
  showChart: boolean;
}

/** Default state factory for forward curve calibration */
export function createDefaultForwardCurveState(
  overrides?: Partial<ForwardCurveCalibrationState>
): ForwardCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
      day: now.getDate(),
    },
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

// ============================================================================
// Hazard (Credit) Curve Calibration State
// ============================================================================

/** Complete JSON state for hazard curve calibration */
export interface HazardCurveCalibrationState {
  /** Base date for the curve */
  baseDate: DateJson;
  /** Unique identifier for the curve */
  curveId: string;
  /** Currency code (ISO 4217) */
  currency: string;
  /** Entity name (issuer) */
  entity: string;
  /** Seniority level (e.g., 'senior', 'subordinated') */
  seniority: string;
  /** Recovery rate assumption (0-1) */
  recoveryRate: number;
  /** ID of the discount curve to use */
  discountCurveId: string;
  /** CDS quotes */
  quotes: CdsQuoteData[];
  /** Calibration configuration */
  config: CalibrationConfigJson;
  /** Whether to show the chart */
  showChart: boolean;
}

/** Default state factory for hazard curve calibration */
export function createDefaultHazardCurveState(
  overrides?: Partial<HazardCurveCalibrationState>
): HazardCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
      day: now.getDate(),
    },
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

// ============================================================================
// Inflation Curve Calibration State
// ============================================================================

/** Complete JSON state for inflation curve calibration */
export interface InflationCurveCalibrationState {
  /** Base date for the curve */
  baseDate: DateJson;
  /** Unique identifier for the curve */
  curveId: string;
  /** Currency code (ISO 4217) */
  currency: string;
  /** Inflation index name (e.g., 'US-CPI-U') */
  indexName: string;
  /** Base CPI value at the base date */
  baseCpi: number;
  /** ID of the discount curve to use */
  discountCurveId: string;
  /** Inflation swap quotes */
  quotes: InflationSwapQuoteData[];
  /** Calibration configuration */
  config: CalibrationConfigJson;
  /** Whether to show the chart */
  showChart: boolean;
}

/** Default state factory for inflation curve calibration */
export function createDefaultInflationCurveState(
  overrides?: Partial<InflationCurveCalibrationState>
): InflationCurveCalibrationState {
  const now = new Date();
  return {
    baseDate: {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
      day: now.getDate(),
    },
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

// ============================================================================
// Vol Surface Calibration State
// ============================================================================

/** Complete JSON state for vol surface calibration */
export interface VolSurfaceCalibrationState {
  /** Base date for the surface */
  baseDate: DateJson;
  /** Unique identifier for the surface */
  curveId: string;
  /** Currency code (ISO 4217) */
  currency: string;
  /** Underlying asset identifier */
  underlying: string;
  /** Current spot price of the underlying */
  spotPrice: number;
  /** Expiry times in years for the surface grid */
  expiries: number[];
  /** Strike levels for the surface grid (as % of spot) */
  strikes: number[];
  /** ID of the discount curve to use */
  discountCurveId: string;
  /** Option vol quotes */
  quotes: VolQuoteData[];
  /** Calibration configuration */
  config: CalibrationConfigJson;
  /** Calibration tolerance (SABR needs higher than rates) */
  tolerance: number;
  /** Whether to show the chart */
  showChart: boolean;
}

/** Default state factory for vol surface calibration */
export function createDefaultVolSurfaceState(
  overrides?: Partial<VolSurfaceCalibrationState>
): VolSurfaceCalibrationState {
  const now = new Date();
  return {
    baseDate: {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
      day: now.getDate(),
    },
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

// ============================================================================
// Base Correlation Calibration State
// ============================================================================

/** Complete JSON state for base correlation calibration */
export interface BaseCorrelationCalibrationState {
  /** Base date for the curve */
  baseDate: DateJson;
  /** Unique identifier for the correlation curve */
  curveId: string;
  /** Credit index identifier (e.g., 'CDX.NA.IG.42') */
  indexId: string;
  /** Index series number */
  series: number;
  /** Maturity in years */
  maturityYears: number;
  /** ID of the discount curve to use */
  discountCurveId: string;
  /** Tranche quotes */
  quotes: TrancheQuoteData[];
  /** Calibration configuration */
  config: CalibrationConfigJson;
  /** Whether to show the chart */
  showChart: boolean;
}

/** Default state factory for base correlation calibration */
export function createDefaultBaseCorrelationState(
  overrides?: Partial<BaseCorrelationCalibrationState>
): BaseCorrelationCalibrationState {
  const now = new Date();
  return {
    baseDate: {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
      day: now.getDate(),
    },
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

// ============================================================================
// Complete Calibration Suite State
// ============================================================================

/** Complete state for the entire calibration suite */
export interface CalibrationSuiteState {
  /** Active tab in the calibration UI */
  activeTab: 'discount' | 'forward' | 'hazard' | 'inflation' | 'vol' | 'correlation';
  /** Discount curve calibration state */
  discount: DiscountCurveCalibrationState;
  /** Forward curve calibration state */
  forward: ForwardCurveCalibrationState;
  /** Hazard curve calibration state */
  hazard: HazardCurveCalibrationState;
  /** Inflation curve calibration state */
  inflation: InflationCurveCalibrationState;
  /** Vol surface calibration state */
  vol: VolSurfaceCalibrationState;
  /** Base correlation calibration state */
  correlation: BaseCorrelationCalibrationState;
}

/** Default state factory for the complete calibration suite */
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
// JSON Serialization/Deserialization Utilities
// ============================================================================

/**
 * Serialize calibration suite state to JSON string.
 * This can be used to export state for LLM processing or persistence.
 */
export function serializeCalibrationState(state: CalibrationSuiteState): string {
  return JSON.stringify(state, null, 2);
}

/**
 * Deserialize JSON string to calibration suite state.
 * Validates the structure and provides defaults for missing fields.
 */
export function deserializeCalibrationState(json: string): CalibrationSuiteState {
  const parsed = JSON.parse(json);
  // Merge with defaults to ensure all fields are present
  return createDefaultCalibrationSuiteState(parsed);
}

/**
 * Apply a partial state update (e.g., from LLM chat response).
 * Deeply merges the update into the current state.
 */
export function applyStateUpdate(
  currentState: CalibrationSuiteState,
  update: DeepPartial<CalibrationSuiteState>
): CalibrationSuiteState {
  return deepMerge(currentState, update) as CalibrationSuiteState;
}

// ============================================================================
// Type Utilities
// ============================================================================

/** Deep partial type for nested partial updates */
export type DeepPartial<T> = T extends object
  ? {
      [P in keyof T]?: DeepPartial<T[P]>;
    }
  : T;

/** Deep merge utility for state updates */
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
        result[key] = deepMerge(
          targetValue as object,
          sourceValue as DeepPartial<object>
        );
      } else if (sourceValue !== undefined) {
        result[key] = sourceValue;
      }
    }
  }

  return result as T;
}

// ============================================================================
// LLM Chat Interface Types
// ============================================================================

/** Command type for LLM chat interactions */
export type CalibrationCommand =
  | { type: 'SET_STATE'; state: DeepPartial<CalibrationSuiteState> }
  | { type: 'SET_QUOTES'; calibrationType: keyof Omit<CalibrationSuiteState, 'activeTab'>; quotes: unknown[] }
  | { type: 'SET_TAB'; tab: CalibrationSuiteState['activeTab'] }
  | { type: 'CALIBRATE'; calibrationType: keyof Omit<CalibrationSuiteState, 'activeTab'> }
  | { type: 'RESET'; calibrationType?: keyof Omit<CalibrationSuiteState, 'activeTab'> };

/** Response from calibration operations */
export interface CalibrationCommandResult {
  success: boolean;
  message: string;
  state?: CalibrationSuiteState;
  error?: string;
}

/**
 * Example JSON that an LLM might generate to control calibrations:
 *
 * ```json
 * {
 *   "type": "SET_STATE",
 *   "state": {
 *     "activeTab": "discount",
 *     "discount": {
 *       "currency": "EUR",
 *       "curveId": "EUR-OIS",
 *       "quotes": [
 *         { "type": "deposit", "maturityYear": 2025, "maturityMonth": 1, "maturityDay": 15, "rate": 0.035, "dayCount": "act_360" },
 *         { "type": "swap", "maturityYear": 2026, "maturityMonth": 1, "maturityDay": 15, "rate": 0.038, "fixedFrequency": "annual", "floatFrequency": "quarterly", "fixedDayCount": "30_360", "floatDayCount": "act_360", "index": "EUR-ESTR" }
 *       ]
 *     }
 *   }
 * }
 * ```
 */
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

