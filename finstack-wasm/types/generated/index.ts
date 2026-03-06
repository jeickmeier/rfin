/**
 * Generated TypeScript types from Finstack Rust structs via ts-rs.
 *
 * DO NOT EDIT individual type files - regenerate with:
 *   cargo test -p finstack-valuations --features ts_export export_calibration_types
 *
 * These types represent the **wire format** used by finstack-wasm JSON methods:
 * - `fromJSON()` accepts objects matching these types
 * - `toJSON()` returns objects matching these types
 *
 * Exported from the finstack-wasm package as `finstack-wasm/types`.
 */

// Calibration configuration types
export type { CalibrationConfig } from './CalibrationConfig';
export type { RateBounds } from './RateBounds';
export type { SolverKind } from './SolverKind';
export type { ValidationMode } from './ValidationMode';

// Quote types (wire format for calibration inputs)
export type { BondQuote } from './BondQuote';
export type { RatesQuote } from './RatesQuote';
export type { CreditQuote } from './CreditQuote';
export type { FxQuote } from './FxQuote';
export type { VolQuote } from './VolQuote';
export type { InflationQuote } from './InflationQuote';
export type { XccyQuote } from './XccyQuote';
export type { MarketQuote } from './MarketQuote';
export type { FutureSpecs } from './FutureSpecs';
