/**
 * TypeScript type definitions for P&L Attribution
 * 
 * Multi-period P&L attribution for financial instruments.
 */

/**
 * Attribution methodology selector.
 */
export class AttributionMethod {
  /**
   * Create parallel attribution method.
   * 
   * Independent factor isolation (may not sum due to cross-effects).
   */
  constructor();
  
  /**
   * Create waterfall attribution method with custom factor order.
   * 
   * Sequential application (guarantees sum = total, order matters).
   * 
   * @param factors - Array of factor names: "carry", "rates_curves", "credit_curves",
   *                  "inflation_curves", "correlations", "fx", "volatility", 
   *                  "model_parameters", "market_scalars"
   */
  static waterfall(factors: string[]): AttributionMethod;
  
  /**
   * Create metrics-based attribution method.
   * 
   * Fast linear approximation using existing metrics.
   */
  static metricsBased(): AttributionMethod;
  
  toString(): string;
}

/**
 * Attribution metadata.
 */
export class AttributionMeta {
  /** Instrument identifier */
  readonly instrumentId: string;
  
  /** Number of repricings performed */
  readonly numRepricings: number;
  
  /** Residual as percentage of total P&L */
  readonly residualPct: number;
  
  /** Tolerance for residual validation */
  readonly tolerance: number;
  
  /** Attribution method used ("Parallel", "Waterfall", or "MetricsBased") */
  readonly method: string;
  
  /** Start date (T₀) as ISO string */
  readonly t0: string;
  
  /** End date (T₁) as ISO string */
  readonly t1: string;
}

/**
 * Detailed attribution for interest rate curves.
 */
export class RatesCurvesAttribution {
  /** Total discount curves P&L */
  readonly discountTotal: number;
  
  /** Total forward curves P&L */
  readonly forwardTotal: number;
  
  /**
   * Get P&L by curve as JSON object.
   * 
   * @returns JSON string mapping curve ID to P&L amount
   */
  byCurveToJson(): string;
}

/**
 * Detailed attribution for model-specific parameters.
 */
export class ModelParamsAttribution {
  /** Prepayment speed changes P&L (for MBS/ABS) */
  readonly prepayment?: number;
  
  /** Default rate changes P&L (for structured credit) */
  readonly defaultRate?: number;
  
  /** Recovery rate changes P&L (for credit instruments) */
  readonly recoveryRate?: number;
  
  /** Conversion ratio changes P&L (for convertible bonds) */
  readonly conversionRatio?: number;
}

/**
 * P&L attribution result for a single instrument.
 */
export class PnlAttribution {
  /** Total P&L (val_t1 - val_t0) */
  readonly totalPnl: number;
  
  /** Carry P&L (theta + accruals) */
  readonly carry: number;
  
  /** Interest rate curves P&L */
  readonly ratesCurvesPnl: number;
  
  /** Credit hazard curves P&L */
  readonly creditCurvesPnl: number;
  
  /** Inflation curves P&L */
  readonly inflationCurvesPnl: number;
  
  /** Base correlation curves P&L */
  readonly correlationsPnl: number;
  
  /** FX rate changes P&L */
  readonly fxPnl: number;
  
  /** Implied volatility changes P&L */
  readonly volPnl: number;
  
  /** Model parameters P&L */
  readonly modelParamsPnl: number;
  
  /** Market scalars P&L */
  readonly marketScalarsPnl: number;
  
  /** Residual P&L */
  readonly residual: number;
  
  /** Attribution metadata */
  readonly meta: AttributionMeta;
  
  /** Detailed rates curves attribution (if available) */
  readonly ratesDetail?: RatesCurvesAttribution;
  
  /** Detailed model parameters attribution (if available) */
  readonly modelParamsDetail?: ModelParamsAttribution;
  
  /**
   * Export attribution as CSV string.
   * 
   * @returns CSV string with headers and data row
   */
  toCsv(): string;
  
  /**
   * Export attribution as JSON string.
   * 
   * Requires serde feature enabled.
   * 
   * @returns JSON string with complete attribution data
   */
  toJson(): string;
  
  /**
   * Export rates curves detail as CSV.
   * 
   * @returns CSV string with curve-by-curve breakdown, or undefined if no detail
   */
  ratesDetailToCsv(): string | undefined;
  
  /**
   * Generate structured tree explanation.
   * 
   * @returns Multi-line string with tree structure showing factor breakdown
   * 
   * @example
   * ```typescript
   * console.log(attr.explain());
   * // Total P&L: USD 125,430
   * //   ├─ Carry: USD 45,000 (35.8%)
   * //   ├─ Rates Curves: USD 65,000 (51.7%)
   * //   ├─ FX: USD 12,000 (9.5%)
   * //   └─ Residual: USD -1,570 (-1.2%)
   * ```
   */
  explain(): string;
  
  /**
   * Check if residual is within acceptable tolerance.
   * 
   * @param pctTolerance - Percentage tolerance (e.g., 0.1 for 0.1%)
   * @param absTolerance - Absolute tolerance (e.g., 100.0 for $100)
   * @returns true if residual is within tolerance
   */
  residualWithinTolerance(pctTolerance: number, absTolerance: number): boolean;
}

/**
 * Portfolio-level P&L attribution result.
 */
export class PortfolioAttribution {
  /** Total portfolio P&L in base currency */
  readonly totalPnl: number;
  
  /** Carry P&L */
  readonly carry: number;
  
  /** Interest rate curves P&L */
  readonly ratesCurvesPnl: number;
  
  /** Credit hazard curves P&L */
  readonly creditCurvesPnl: number;
  
  /** Inflation curves P&L */
  readonly inflationCurvesPnl: number;
  
  /** Base correlation curves P&L */
  readonly correlationsPnl: number;
  
  /** FX rate changes P&L */
  readonly fxPnl: number;
  
  /** Implied volatility changes P&L */
  readonly volPnl: number;
  
  /** Model parameters P&L */
  readonly modelParamsPnl: number;
  
  /** Market scalars P&L */
  readonly marketScalarsPnl: number;
  
  /** Residual P&L */
  readonly residual: number;
  
  /**
   * Get position breakdown as JSON.
   * 
   * @returns JSON string mapping position ID to total P&L
   */
  byPositionToJson(): string;
  
  /**
   * Export portfolio attribution summary as CSV.
   */
  toCsv(): string;
  
  /**
   * Export position-by-position detail as CSV.
   */
  positionDetailToCsv(): string;
  
  /**
   * Generate explanation tree for portfolio attribution.
   */
  explain(): string;
}

/**
 * Note: Attribution functions (attributePnl, attributePortfolioPnl) require
 * instrument-specific implementations. In WASM, attribution is performed by:
 * 
 * 1. Importing the attribution types above
 * 2. Using instrument-specific pricing with different markets
 * 3. Manually constructing PnlAttribution from the results
 * 
 * For Python/Rust, use the generic attribute_pnl() function which handles
 * all instrument types automatically.
 * 
 * Example usage pattern in TypeScript:
 * ```typescript
 * import * as finstack from './finstack_wasm';
 * 
 * // Method 1: Use attribution classes directly (types only, no function yet)
 * const method = new finstack.AttributionMethod(); // Parallel
 * const waterfall = finstack.AttributionMethod.waterfall([
 *   "carry", "rates_curves", "fx"
 * ]);
 * 
 * // Method 2: Manual attribution calculation
 * // Price at T₀ and T₁, then analyze difference
 * const val_t0 = priceBond(bond, market_t0);
 * const val_t1 = priceBond(bond, market_t1);
 * // Attribution analysis would be done manually or server-side
 * ```
 */

