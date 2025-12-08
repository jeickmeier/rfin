/**
 * Calibration State Types - Derived from WASM Bindings
 * 
 * This module provides JSON-serializable state types for LLM chat control
 * by re-using types from the WASM bindings where possible.
 * 
 * ## Architecture
 * 
 * The types here compose:
 * 1. **WASM-derived types**: Imported from 'finstack-wasm' (generated from Rust)
 * 2. **UI state extensions**: Additional fields for React component state
 * 
 * This approach minimizes maintenance by:
 * - Using the WASM-generated .d.ts as the source of truth for core types
 * - Only defining UI-specific extensions here
 * 
 * ## Usage with LLM Chat
 * 
 * All types are JSON-serializable, allowing LLMs to:
 * - Read current state via JSON export
 * - Generate state updates as JSON patches
 * - Control calibration parameters through natural language
 */

// Re-export core types from WASM bindings
// These are auto-generated from Rust structs via wasm-bindgen
export type {
  // Calibration types
  CalibrationConfig,
  DiscountCurveCalibrator,
  ForwardCurveCalibrator,
  HazardCurveCalibrator,
  InflationCurveCalibrator,
  VolSurfaceCalibrator,
  BaseCorrelationCalibrator,
  // Quote types
  RatesQuote,
  CreditQuote,
  VolQuote,
  InflationQuote,
  // Core types
  FsDate,
  Currency,
  Money,
  Frequency,
} from 'finstack-wasm';

// ============================================================================
// JSON-Serializable Primitives (for state persistence and LLM control)
// ============================================================================

/**
 * JSON-serializable date representation.
 * 
 * This mirrors FsDate but as a plain object for JSON serialization.
 * Use `new FsDate(date.year, date.month, date.day)` to convert back.
 */
export interface DateJson {
  year: number;
  month: number;
  day: number;
}

/**
 * Solver algorithm selection.
 * 
 * Maps to the Rust SolverKind enum. Available options:
 * - "Brent": Brent's method (root-finding, fast for 1D problems)
 * - "Newton": Newton-Raphson (requires derivatives, fast convergence)
 */
export type SolverKindJson = 'Brent' | 'Newton';

/**
 * JSON-serializable calibration configuration.
 * 
 * This mirrors CalibrationConfig from Rust but as a plain object.
 */
export interface CalibrationConfigJson {
  solverKind: SolverKindJson;
  maxIterations: number;
  tolerance: number;
  verbose: boolean;
}

// ============================================================================
// Quote Data Types (JSON-serializable mirrors of WASM quote types)
// ============================================================================

/**
 * Deposit rate quote for discount curve calibration.
 */
export interface DepositQuoteJson {
  type: 'deposit';
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  dayCount: string;
}

/**
 * Interest rate swap quote for discount/forward curve calibration.
 */
export interface SwapQuoteJson {
  type: 'swap';
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  fixedFrequency: 'annual' | 'semi_annual' | 'quarterly' | 'monthly';
  floatFrequency: 'annual' | 'semi_annual' | 'quarterly' | 'monthly';
  fixedDayCount: string;
  floatDayCount: string;
  index: string;
}

/**
 * Forward Rate Agreement (FRA) quote for forward curve calibration.
 */
export interface FraQuoteJson {
  type: 'fra';
  startYear: number;
  startMonth: number;
  startDay: number;
  endYear: number;
  endMonth: number;
  endDay: number;
  rate: number;
  dayCount: string;
}

/**
 * CDS (Credit Default Swap) quote for hazard curve calibration.
 */
export interface CdsQuoteJson {
  entity: string;
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  spreadBps: number;
  recoveryRate: number;
  currency: string;
}

/**
 * Inflation swap quote for inflation curve calibration.
 */
export interface InflationSwapQuoteJson {
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  indexName: string;
}

/**
 * Option volatility quote for vol surface calibration.
 */
export interface VolQuoteJson {
  underlying: string;
  expiryYear: number;
  expiryMonth: number;
  expiryDay: number;
  strike: number;
  vol: number;
  optionType: 'Call' | 'Put';
}

/**
 * CDO tranche quote for base correlation calibration.
 */
export interface TrancheQuoteJson {
  index: string;
  attachment: number;
  detachment: number;
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  upfrontPct: number;
  runningSpreadBp: number;
}

// ============================================================================
// Component State Types (JSON state for each calibration component)
// ============================================================================

/**
 * Complete state for discount curve calibration.
 * 
 * @example
 * ```json
 * {
 *   "baseDate": { "year": 2024, "month": 1, "day": 2 },
 *   "curveId": "USD-OIS",
 *   "currency": "USD",
 *   "quotes": [
 *     { "type": "deposit", "maturityYear": 2024, "maturityMonth": 2, "maturityDay": 1, "rate": 0.045, "dayCount": "act_360" }
 *   ],
 *   "config": { "solverKind": "Brent", "maxIterations": 40, "tolerance": 1e-8, "verbose": false },
 *   "showChart": true
 * }
 * ```
 */
export interface DiscountCurveStateJson {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  quotes: (DepositQuoteJson | SwapQuoteJson)[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

/**
 * Complete state for forward curve calibration.
 */
export interface ForwardCurveStateJson {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  tenor: number;
  discountCurveId: string;
  quotes: (DepositQuoteJson | SwapQuoteJson | FraQuoteJson)[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

/**
 * Complete state for hazard (credit) curve calibration.
 */
export interface HazardCurveStateJson {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  entity: string;
  seniority: string;
  recoveryRate: number;
  discountCurveId: string;
  quotes: CdsQuoteJson[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

/**
 * Complete state for inflation curve calibration.
 */
export interface InflationCurveStateJson {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  indexName: string;
  baseCpi: number;
  discountCurveId: string;
  quotes: InflationSwapQuoteJson[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

/**
 * Complete state for volatility surface calibration.
 */
export interface VolSurfaceStateJson {
  baseDate: DateJson;
  curveId: string;
  currency: string;
  underlying: string;
  spotPrice: number;
  expiries: number[];
  strikes: number[];
  discountCurveId: string;
  quotes: VolQuoteJson[];
  config: CalibrationConfigJson;
  tolerance: number;
  showChart: boolean;
}

/**
 * Complete state for base correlation calibration.
 */
export interface BaseCorrelationStateJson {
  baseDate: DateJson;
  curveId: string;
  indexId: string;
  series: number;
  maturityYears: number;
  discountCurveId: string;
  quotes: TrancheQuoteJson[];
  config: CalibrationConfigJson;
  showChart: boolean;
}

// ============================================================================
// Aggregate State Type
// ============================================================================

/**
 * Complete calibration suite state.
 * 
 * This is the top-level state object that an LLM can read/write
 * to control all calibration components.
 */
export interface CalibrationSuiteStateJson {
  activeTab: 'discount' | 'forward' | 'hazard' | 'inflation' | 'vol' | 'correlation';
  discount: DiscountCurveStateJson;
  forward: ForwardCurveStateJson;
  hazard: HazardCurveStateJson;
  inflation: InflationCurveStateJson;
  vol: VolSurfaceStateJson;
  correlation: BaseCorrelationStateJson;
}

// ============================================================================
// LLM Command Types
// ============================================================================

/**
 * Commands that an LLM can generate to control calibrations.
 */
export type CalibrationCommandJson =
  | { type: 'SET_STATE'; state: Partial<CalibrationSuiteStateJson> }
  | { type: 'SET_TAB'; tab: CalibrationSuiteStateJson['activeTab'] }
  | { type: 'ADD_QUOTE'; calibrationType: string; quote: unknown }
  | { type: 'CALIBRATE'; calibrationType?: string }
  | { type: 'RESET'; calibrationType?: string };

// ============================================================================
// Documentation for LLM Usage
// ============================================================================

/**
 * Example JSON command for LLM to set EUR discount curve:
 */
export const EXAMPLE_COMMAND = `{
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

