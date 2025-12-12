/**
 * Currency-specific market conventions for interest rate swaps.
 *
 * These conventions follow market standards for each currency:
 * - USD SOFR: Semi-Annual fixed / Quarterly float
 * - EUR €STR: Annual fixed / Annual float
 * - GBP SONIA: Annual fixed / Annual float (Quarterly for some markets)
 * - JPY TONA: Semi-Annual fixed / Semi-Annual float
 * - CHF SARON: Annual fixed / Annual float
 * - AUD AONIA: Semi-Annual fixed / Quarterly float
 * - CAD CORRA: Semi-Annual fixed / Quarterly float
 */

export type FrequencyType = 'annual' | 'semi_annual' | 'quarterly' | 'monthly';

export interface SwapConventions {
  /** Fixed leg payment frequency */
  fixedFrequency: FrequencyType;
  /** Float leg payment frequency */
  floatFrequency: FrequencyType;
  /** Fixed leg day count convention */
  fixedDayCount: string;
  /** Float leg day count convention */
  floatDayCount: string;
  /** Default rate index name */
  defaultIndex: string;
}

/** Rate input validation bounds per currency */
export interface RateBounds {
  /** Minimum allowed rate (decimal, e.g., -0.05 for -5%) */
  minRate: number;
  /** Maximum allowed rate (decimal, e.g., 0.50 for 50%) */
  maxRate: number;
}

/** CDS spread validation bounds */
export interface SpreadBounds {
  /** Minimum spread in basis points */
  minBps: number;
  /** Maximum spread in basis points */
  maxBps: number;
}

/** Volatility validation bounds */
export interface VolBounds {
  /** Minimum volatility (decimal) */
  minVol: number;
  /** Maximum volatility (decimal) */
  maxVol: number;
}

/**
 * Market-standard swap conventions by currency.
 */
export const SWAP_CONVENTIONS: Record<string, SwapConventions> = {
  // USD - SOFR swaps
  USD: {
    fixedFrequency: 'semi_annual',
    floatFrequency: 'quarterly',
    fixedDayCount: '30_360',
    floatDayCount: 'act_360',
    defaultIndex: 'USD-SOFR',
  },
  // EUR - €STR swaps (formerly EONIA, now ESTER)
  EUR: {
    fixedFrequency: 'annual',
    floatFrequency: 'annual',
    fixedDayCount: 'act_360',
    floatDayCount: 'act_360',
    defaultIndex: 'EUR-ESTR',
  },
  // GBP - SONIA swaps
  GBP: {
    fixedFrequency: 'annual',
    floatFrequency: 'annual',
    fixedDayCount: 'act_365f',
    floatDayCount: 'act_365f',
    defaultIndex: 'GBP-SONIA',
  },
  // JPY - TONA swaps
  JPY: {
    fixedFrequency: 'semi_annual',
    floatFrequency: 'semi_annual',
    fixedDayCount: 'act_365f',
    floatDayCount: 'act_365f',
    defaultIndex: 'JPY-TONA',
  },
  // CHF - SARON swaps
  CHF: {
    fixedFrequency: 'annual',
    floatFrequency: 'annual',
    fixedDayCount: 'act_360',
    floatDayCount: 'act_360',
    defaultIndex: 'CHF-SARON',
  },
  // AUD - AONIA swaps
  AUD: {
    fixedFrequency: 'semi_annual',
    floatFrequency: 'quarterly',
    fixedDayCount: 'act_365f',
    floatDayCount: 'act_365f',
    defaultIndex: 'AUD-AONIA',
  },
  // CAD - CORRA swaps
  CAD: {
    fixedFrequency: 'semi_annual',
    floatFrequency: 'quarterly',
    fixedDayCount: 'act_365f',
    floatDayCount: 'act_365f',
    defaultIndex: 'CAD-CORRA',
  },
};

/**
 * Rate validation bounds by currency.
 * These reflect typical market ranges and known negative rate environments.
 */
export const RATE_BOUNDS: Record<string, RateBounds> = {
  // Deep negative rate environments
  EUR: { minRate: -0.05, maxRate: 0.3 },
  JPY: { minRate: -0.05, maxRate: 0.2 },
  CHF: { minRate: -0.05, maxRate: 0.2 },
  // Standard developed markets
  USD: { minRate: -0.02, maxRate: 0.5 },
  GBP: { minRate: -0.03, maxRate: 0.5 },
  CAD: { minRate: -0.02, maxRate: 0.5 },
  AUD: { minRate: -0.02, maxRate: 0.5 },
  // Emerging markets (allow higher rates)
  BRL: { minRate: -0.05, maxRate: 2.0 },
  TRY: { minRate: -0.05, maxRate: 2.0 },
  ZAR: { minRate: -0.05, maxRate: 1.0 },
  MXN: { minRate: -0.05, maxRate: 1.0 },
};

/** Default rate bounds for unknown currencies */
export const DEFAULT_RATE_BOUNDS: RateBounds = { minRate: -0.05, maxRate: 0.5 };

/** CDS spread bounds (in basis points) */
export const CDS_SPREAD_BOUNDS: SpreadBounds = { minBps: 0, maxBps: 10000 };

/** Implied volatility bounds */
export const VOL_BOUNDS: VolBounds = { minVol: 0.001, maxVol: 5.0 };

/** Recovery rate bounds */
export const RECOVERY_BOUNDS = { min: 0, max: 1 };

/**
 * Get swap conventions for a currency, with USD fallback.
 */
export function getSwapConventions(currency: string): SwapConventions {
  return SWAP_CONVENTIONS[currency.toUpperCase()] ?? SWAP_CONVENTIONS.USD;
}

/**
 * Get rate bounds for a currency.
 */
export function getRateBounds(currency: string): RateBounds {
  return RATE_BOUNDS[currency.toUpperCase()] ?? DEFAULT_RATE_BOUNDS;
}

/**
 * Convert frequency type to display label.
 */
export function frequencyLabel(freq: FrequencyType): string {
  const labels: Record<FrequencyType, string> = {
    annual: 'Annual',
    semi_annual: 'Semi-Annual',
    quarterly: 'Quarterly',
    monthly: 'Monthly',
  };
  return labels[freq];
}

/**
 * All available frequency options for dropdowns.
 */
export const FREQUENCY_OPTIONS: { value: FrequencyType; label: string }[] = [
  { value: 'annual', label: 'Annual' },
  { value: 'semi_annual', label: 'Semi-Annual' },
  { value: 'quarterly', label: 'Quarterly' },
  { value: 'monthly', label: 'Monthly' },
];

/**
 * Validate a rate is within bounds.
 */
export function isValidRate(rate: number, currency: string): boolean {
  const bounds = getRateBounds(currency);
  return rate >= bounds.minRate && rate <= bounds.maxRate && Number.isFinite(rate);
}

/**
 * Validate a CDS spread is within bounds.
 */
export function isValidSpread(spreadBps: number): boolean {
  return (
    spreadBps >= CDS_SPREAD_BOUNDS.minBps &&
    spreadBps <= CDS_SPREAD_BOUNDS.maxBps &&
    Number.isFinite(spreadBps)
  );
}

/**
 * Validate implied volatility is within bounds.
 */
export function isValidVol(vol: number): boolean {
  return vol >= VOL_BOUNDS.minVol && vol <= VOL_BOUNDS.maxVol && Number.isFinite(vol);
}

/**
 * Validate recovery rate is between 0 and 1.
 */
export function isValidRecovery(recovery: number): boolean {
  return (
    recovery >= RECOVERY_BOUNDS.min && recovery <= RECOVERY_BOUNDS.max && Number.isFinite(recovery)
  );
}
