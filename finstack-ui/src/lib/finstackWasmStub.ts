/**
 * Test/dev fallback shim for the `finstack-wasm` package.
 *
 * CI does not build the WASM bundle (wasm-bindgen `pkg/` artifacts), but the UI
 * code imports `finstack-wasm`. Vitest mocks `finstack-wasm` in unit tests, yet
 * Vite still needs a resolvable module during import analysis. This file exists
 * purely to make that resolution succeed.
 *
 * NOTE: Most tests provide their own `vi.doMock("finstack-wasm", ...)` mocks, so
 * these exports are intentionally minimal and safe-by-default.
 */

export default async function (): Promise<void> {
  // no-op
}

export async function init(): Promise<void> {
  // no-op
}

export class Currency {
  constructor(public code: string) {}
}

export class FinstackConfig {
  // Match the shape used by UI code/tests.
  roundingMode: string = "nearest";
  setOutputScale() {}
  setRoundingModeLabel() {}
}

export class FsDate {
  constructor(
    public year: number,
    public month: number,
    public day: number,
  ) {}
}

export function addMonths<T>(date: T, _months: number): T {
  return date;
}

export class MarketContext {
  static fromJson(json: string) {
    return { fromJson: json };
  }
  static fromJSON(json: string) {
    return { fromJSON: json };
  }
}

export const Money = {
  fromCode: (amount: number, currency: string) => ({ amount, currency }),
};

export const Bond = {
  fromJson: (json: string) => ({ bondJson: json }),
  fixedSemiannual: (
    id: string,
    notional: unknown,
    couponRate: number,
    issue: unknown,
    maturity: unknown,
    discountCurve: string,
    quoted_clean_price?: unknown,
  ) => ({
    id,
    notional,
    couponRate,
    issue,
    maturity,
    discountCurve,
    quoted_clean_price,
  }),
};

export class PricerRegistry {
  priceBond() {
    throw new Error("finstack-wasm stub: pricing not available");
  }
}

export function createStandardRegistry() {
  return new PricerRegistry();
}
