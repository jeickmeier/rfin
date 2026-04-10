/**
 * Calibration State Types - Shared Primitives for the examples app
 *
 * This module provides:
 * 1. **Shared UI Primitives**
 *    - DateJson, SolverKindJson, CalibrationConfigJson, CalibrationTab
 *    - Consumed by component state types in `components/calibration/state-types.ts`
 *
 * For component-specific state types (with QuoteEditor integration),
 * see `components/calibration/state-types.ts`.
 */

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
