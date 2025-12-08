import React, { useState, useCallback, useMemo } from 'react';
import {
  CalibrationConfig,
  FsDate,
  MarketContext,
  MarketScalar,
  Money,
  SolverKind,
  VolQuote,
  VolSurfaceCalibrator,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CurveChart, StatusBadge, CalibrationMetrics } from './CurveChart';
import { VolQuoteEditor, DEFAULT_VOL_QUOTES, type VolQuoteData } from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';

interface CalibratedVolSurface {
  value: (t: number, k: number) => number;
  id: string;
}

interface VolSurfaceCalibrationProps {
  baseDate: FsDate;
  curveId: string;
  currency: string;
  underlying: string;
  spotPrice: number;
  expiries: number[];
  strikes: number[];
  discountCurveId: string;
  config?: CalibrationConfig;
  market: MarketContext | null;
  onCalibrated?: (result: CalibrationResult) => void;
  showChart?: boolean;
  className?: string;
  /** Initial quotes - if not provided, uses DEFAULT_VOL_QUOTES */
  initialQuotes?: VolQuoteData[];
  /**
   * Calibration tolerance for SABR fit residuals.
   * SABR model fitting typically has larger residuals than rate curve bootstrapping.
   * Minimum enforced is 0.01 (1%); default is 0.5 (50bp vol error).
   * Tighter tolerances may cause calibration failure.
   */
  tolerance?: number;
}

/** Minimum tolerance for SABR calibration (SABR residuals are larger than rate bootstraps) */
const MIN_VOL_TOLERANCE = 0.01;
/** Default tolerance for vol surface calibration */
const DEFAULT_VOL_TOLERANCE = 0.5;

/** Convert quote data to WASM VolQuote objects */
const buildWasmQuotes = (quotes: VolQuoteData[]): VolQuote[] => {
  return quotes.map((q) =>
    VolQuote.optionVol(
      q.underlying,
      new FsDate(q.expiryYear, q.expiryMonth, q.expiryDay),
      q.strike,
      q.vol,
      q.optionType
    )
  );
};

export const VolSurfaceCalibration: React.FC<VolSurfaceCalibrationProps> = ({
  baseDate,
  curveId,
  currency,
  underlying,
  spotPrice,
  expiries,
  strikes,
  discountCurveId,
  config,
  market,
  onCalibrated,
  showChart = true,
  className,
  initialQuotes,
  tolerance = DEFAULT_VOL_TOLERANCE,
}) => {
  // Local state for editable quotes
  const [quotes, setQuotes] = useState<VolQuoteData[]>(initialQuotes ?? DEFAULT_VOL_QUOTES);

  const [status, setStatus] = useState<CalibrationStatus>('idle');
  const [result, setResult] = useState<CalibrationResult | null>(null);
  const [surface, setSurface] = useState<CalibratedVolSurface | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Enforce minimum tolerance for SABR (too tight = calibration failure)
  const effectiveTolerance = Math.max(tolerance, MIN_VOL_TOLERANCE);

  const runCalibration = useCallback(() => {
    if (quotes.length === 0) {
      setError('No vol quotes provided');
      return;
    }

    if (!market) {
      setError('Market context with discount curve required');
      return;
    }

    setStatus('running');
    setError(null);

    try {
      // Ensure spot price and div yield are in market
      market.insertPrice(underlying, MarketScalar.price(Money.fromCode(spotPrice, currency)));
      market.insertPrice(`${underlying}-DIVYIELD`, MarketScalar.unitless(0.015));

      // Use user-provided tolerance (floored to MIN_VOL_TOLERANCE for SABR stability)
      const calibrationConfig = CalibrationConfig.multiCurve()
        .withSolverKind(SolverKind.Brent())
        .withMaxIterations(config?.maxIterations ?? 100)
        .withTolerance(effectiveTolerance)
        .withVerbose(false);

      // Build fresh WASM quotes from the editable data
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new VolSurfaceCalibrator(
        curveId,
        1,
        new Float64Array(expiries),
        new Float64Array(strikes)
      )
        .withBaseDate(baseDate)
        .withConfig(calibrationConfig)
        .withDiscountId(discountCurveId);

      const [calibratedSurface, report] = calibrator.calibrate(wasmQuotes, market) as [
        CalibratedVolSurface,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      // Generate ATM vol term structure for charting
      const atmStrike =
        strikes.find((s) => Math.abs(s - 100) < 5) || strikes[Math.floor(strikes.length / 2)];
      const sampleValues: CurveDataPoint[] = expiries.map((t) => ({
        time: t,
        value: calibratedSurface.value(t, atmStrike),
        label: `Vol(${t}Y, ${atmStrike})`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId,
        curveType: 'Vol Surface',
        success: report.success,
        iterations: report.iterations,
        maxResidual: report.maxResidual,
        sampleValues,
      };

      setSurface(calibratedSurface);
      setResult(calibrationResult);
      setStatus(report.success ? 'success' : 'failed');
      onCalibrated?.(calibrationResult);

      console.log(`Vol surface '${curveId}' calibrated:`, {
        atmVol6m: calibratedSurface.value(0.5, atmStrike),
        atmVol1y: calibratedSurface.value(1, atmStrike),
        iterations: report.iterations,
      });
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');
      console.warn(`Vol surface calibration failed: ${errorMsg}`);

      const failedResult: CalibrationResult = {
        curveId,
        curveType: 'Vol Surface',
        success: false,
        iterations: 0,
        maxResidual: 0,
        sampleValues: [],
      };
      setResult(failedResult);
      onCalibrated?.(failedResult);
    }
  }, [
    baseDate,
    curveId,
    currency,
    quotes,
    underlying,
    spotPrice,
    expiries,
    strikes,
    discountCurveId,
    config,
    market,
    onCalibrated,
    effectiveTolerance,
  ]);

  // Format quote summary for display
  const quotesSummary = useMemo(() => {
    return `${quotes.length} vol quotes`;
  }, [quotes]);

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              Volatility Surface
              <StatusBadge status={status} />
            </CardTitle>
            <CardDescription>
              {underlying} - Spot: {spotPrice} {currency} - {expiries.length}x{strikes.length} grid
              - {quotesSummary}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Editable Quote Table */}
        <VolQuoteEditor
          quotes={quotes}
          onChange={setQuotes}
          onCalibrate={runCalibration}
          disabled={status === 'running' || !market}
          underlying={underlying}
        />

        {!market && (
          <div className="text-sm text-amber-600 bg-amber-50 border border-amber-200 rounded-md p-3">
            Waiting for discount curve market context...
          </div>
        )}

        {error && (
          <div className="text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded-md p-3">
            {error}
          </div>
        )}

        {result && (
          <CalibrationMetrics
            iterations={result.iterations}
            maxResidual={result.maxResidual}
            success={result.success}
          />
        )}

        {showChart && result && result.sampleValues.length > 0 && (
          <CurveChart
            data={result.sampleValues}
            config={{
              title: 'ATM Vol Term Structure',
              xLabel: 'Expiry',
              yLabel: 'Implied Vol',
              color: 'hsl(var(--chart-5))',
              yFormatter: (v) => `${(v * 100).toFixed(1)}%`,
            }}
          />
        )}

        {surface && result?.success && (
          <>
            <div className="text-sm font-medium text-muted-foreground mb-2">
              Vol Surface Grid (Expiry x Strike)
            </div>
            <div className="overflow-x-auto">
              <table className="text-xs w-full">
                <thead>
                  <tr>
                    <th className="text-left p-1 text-muted-foreground">Expiry</th>
                    {strikes.map((k) => (
                      <th key={`strike-${k}`} className="text-right p-1 text-muted-foreground">
                        {k}%
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {expiries.map((t) => (
                    <tr key={`expiry-${t}`}>
                      <td className="p-1 font-medium">{t}Y</td>
                      {strikes.map((k) => (
                        <td key={`cell-${t}-${k}`} className="text-right p-1 font-mono">
                          {(surface.value(t, k) * 100).toFixed(1)}%
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
};
