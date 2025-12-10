/**
 * Shared hook for building credit market context.
 */
import { useMemo } from 'react';
import {
  BaseCorrelationCurve,
  CreditIndexData,
  DiscountCurve,
  FsDate,
  HazardCurve,
  MarketContext,
  VolSurface,
} from 'finstack-wasm';
import type { BaseCorrelationData, CreditIndexDataSpec, HazardCurveData } from '../data/credit';
import type { DiscountCurveData, VolSurfaceData, DateData } from '../data/market-data';

export interface CreditMarketConfig {
  valuationDate: DateData;
  discountCurve: DiscountCurveData;
  hazardCurves: HazardCurveData[];
  baseCorrelation?: BaseCorrelationData;
  cdsVolSurface?: VolSurfaceData;
  creditIndexData?: CreditIndexDataSpec;
}

export interface CreditMarketResult {
  market: MarketContext;
  asOf: FsDate;
  hazardCurveMap: Map<string, HazardCurve>;
  baseCorr: BaseCorrelationCurve | null;
}

/**
 * Build credit market context from configuration.
 * Returns null if construction fails.
 */
export function buildCreditMarket(config: CreditMarketConfig): CreditMarketResult | null {
  try {
    const {
      valuationDate,
      discountCurve,
      hazardCurves,
      baseCorrelation,
      cdsVolSurface,
      creditIndexData,
    } = config;

    const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

    // Build discount curve
    const curveBaseDate = new FsDate(
      discountCurve.baseDate.year,
      discountCurve.baseDate.month,
      discountCurve.baseDate.day
    );
    const discCurve = new DiscountCurve(
      discountCurve.id,
      curveBaseDate,
      new Float64Array(discountCurve.tenors),
      new Float64Array(discountCurve.discountFactors),
      discountCurve.dayCount,
      discountCurve.interpolation,
      discountCurve.extrapolation,
      discountCurve.continuous
    );

    const market = new MarketContext();
    market.insertDiscount(discCurve);

    // Build hazard curves
    const hazardCurveMap = new Map<string, HazardCurve>();
    for (const hzData of hazardCurves) {
      const hzBaseDate = new FsDate(
        hzData.baseDate.year,
        hzData.baseDate.month,
        hzData.baseDate.day
      );
      const hzCurve = new HazardCurve(
        hzData.id,
        hzBaseDate,
        new Float64Array(hzData.tenors),
        new Float64Array(hzData.hazardRates),
        hzData.recoveryRate,
        hzData.dayCount,
        null,
        null,
        null,
        null,
        null
      );
      market.insertHazard(hzCurve);
      hazardCurveMap.set(hzData.id, hzCurve);
    }

    // Build base correlation curve (optional)
    let baseCorr: BaseCorrelationCurve | null = null;
    if (baseCorrelation) {
      baseCorr = new BaseCorrelationCurve(
        baseCorrelation.id,
        new Float64Array(baseCorrelation.attachmentPoints),
        new Float64Array(baseCorrelation.correlations)
      );
    }

    // Build credit index data (optional)
    if (creditIndexData && baseCorr) {
      const indexHazard = hazardCurveMap.get(creditIndexData.hazardCurveId);
      if (indexHazard) {
        const indexData = new CreditIndexData(
          creditIndexData.constituents,
          creditIndexData.recoveryRate,
          indexHazard,
          baseCorr,
          null,
          null
        );
        market.insertCreditIndex(creditIndexData.indexFamily, indexData);
      }
    }

    // Build CDS volatility surface (optional)
    if (cdsVolSurface) {
      const cdsVol = new VolSurface(
        cdsVolSurface.id,
        new Float64Array(cdsVolSurface.expiries),
        new Float64Array(cdsVolSurface.strikes),
        new Float64Array(cdsVolSurface.vols)
      );
      market.insertSurface(cdsVol);
    }

    return { market, asOf, hazardCurveMap, baseCorr };
  } catch (err) {
    console.warn('Failed to build credit market:', err);
    return null;
  }
}

/**
 * Hook version that memoizes market construction.
 */
export function useCreditMarket(config: CreditMarketConfig): CreditMarketResult | null {
  return useMemo(
    () => buildCreditMarket(config),
    [config]
  );
}

/** Currency formatter for display */
export const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

/** Common row type for instrument results */
export interface InstrumentRow {
  name: string;
  type: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
}
