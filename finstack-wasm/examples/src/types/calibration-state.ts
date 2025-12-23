/**
 * Calibration State Types - Shared Primitives and Wire Format Types
 *
 * This module provides:
 * 1. **Wire Format Types** (re-exported from `./generated/`)
 *    - Match the JSON accepted/returned by finstack-wasm `fromJSON`/`toJSON` methods
 *    - Generated from Rust via ts-rs; do not edit manually
 *
 * 2. **Shared UI Primitives**
 *    - DateJson, SolverKindJson, CalibrationConfigJson, CalibrationTab
 *    - Consumed by component state types in `components/calibration/state-types.ts`
 *
 * For component-specific state types (with QuoteEditor integration),
 * see `components/calibration/state-types.ts`.
 */

// ============================================================================
// Wire Format Types (generated from Rust - canonical JSON shapes)
// ============================================================================

export type {
  // Config types
  CalibrationConfig,
  RateBounds,
  SolverKind,
  ValidationMode,
  // Quote types (wire format)
  RatesQuote,
  CreditQuote,
  VolQuote,
  InflationQuote,
  MarketQuote,
  FutureSpecs,
} from './generated';

// ============================================================================
// Shared UI Primitives (used by component state types)
// ============================================================================

/**
 * JSON-serializable date for React form state.
 * Convert to FsDate: `new FsDate(date.year, date.month, date.day)`
 */
export interface DateJson {
  year: number;
  month: number;
  day: number;
}

/** Solver selection for UI dropdowns */
export type SolverKindJson = 'Brent' | 'Newton';

/** UI-friendly calibration config */
export interface CalibrationConfigJson {
  solverKind: SolverKindJson;
  maxIterations: number;
  tolerance: number;
  verbose: boolean;
}

/** Tab selection for calibration suite */
export type CalibrationTab =
  | 'discount'
  | 'forward'
  | 'hazard'
  | 'inflation'
  | 'vol'
  | 'correlation';
