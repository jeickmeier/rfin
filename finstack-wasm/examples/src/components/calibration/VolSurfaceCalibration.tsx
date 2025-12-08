import React, { useState, useCallback, useMemo, useEffect } from 'react';
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
import type {
  VolSurfaceCalibrationState,
  CalibrationConfigJson,
  DateJson,
} from './state-types';

interface CalibratedVolSurface {
  value: (t: number, k: number) => number;
  id: string;
}

/** Minimum tolerance for SABR calibration (SABR residuals are larger than rate bootstraps) */
const MIN_VOL_TOLERANCE = 0.01;
/** Default tolerance for vol surface calibration */
const DEFAULT_VOL_TOLERANCE = 0.5;

/**
 * Props for VolSurfaceCalibration component.
 * Supports both controlled (via state prop) and uncontrolled modes.
 */
interface VolSurfaceCalibrationProps {
  /** Complete JSON state for controlled mode */
  state?: VolSurfaceCalibrationState;
  /** Callback when state changes (for controlled mode) */
  onStateChange?: (state: VolSurfaceCalibrationState) => void;
  /** Market context containing discount curve */
  market: MarketContext | null;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;

  // Legacy props for backward compatibility
  /** @deprecated Use state.baseDate instead */
  baseDate?: FsDate;
  /** @deprecated Use state.curveId instead */
  curveId?: string;
  /** @deprecated Use state.currency instead */
  currency?: string;
  /** @deprecated Use state.underlying instead */
  underlying?: string;
  /** @deprecated Use state.spotPrice instead */
  spotPrice?: number;
  /** @deprecated Use state.expiries instead */
  expiries?: number[];
  /** @deprecated Use state.strikes instead */
  strikes?: number[];
  /** @deprecated Use state.discountCurveId instead */
  discountCurveId?: string;
  /** @deprecated Use state.config instead */
  config?: CalibrationConfig;
  /** @deprecated Use state.showChart instead */
  showChart?: boolean;
  /** @deprecated Use state.quotes instead */
  initialQuotes?: VolQuoteData[];
  /** @deprecated Use state.tolerance instead */
  tolerance?: number;
}

/** Convert JSON config to WASM CalibrationConfig */
const buildWasmConfig = (config: CalibrationConfigJson, tolerance: number): CalibrationConfig => {
  let wasmConfig = CalibrationConfig.multiCurve();

  switch (config.solverKind) {
    case 'Brent':
      wasmConfig = wasmConfig.withSolverKind(SolverKind.Brent());
      break;
    case 'Newton':
      wasmConfig = wasmConfig.withSolverKind(SolverKind.Newton());
      break;
  }

  return wasmConfig
    .withMaxIterations(config.maxIterations)
    .withTolerance(tolerance)
    .withVerbose(config.verbose);
};

/** Convert DateJson to FsDate */
const toFsDate = (date: DateJson): FsDate => {
  return new FsDate(date.year, date.month, date.day);
};

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
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
  // Legacy props
  baseDate: legacyBaseDate,
  curveId: legacyCurveId,
  currency: legacyCurrency,
  underlying: legacyUnderlying,
  spotPrice: legacySpotPrice,
  expiries: legacyExpiries,
  strikes: legacyStrikes,
  discountCurveId: legacyDiscountCurveId,
  config: legacyConfig,
  showChart: legacyShowChart,
  initialQuotes: legacyInitialQuotes,
  tolerance: legacyTolerance,
}) => {
  // Determine if we're in controlled mode
  const isControlled = state !== undefined;

  // Extract values from state or legacy props
  const baseDate = useMemo(() => {
    if (state) return toFsDate(state.baseDate);
    if (legacyBaseDate) return legacyBaseDate;
    return new FsDate(new Date().getFullYear(), new Date().getMonth() + 1, new Date().getDate());
  }, [state, legacyBaseDate]);

  const curveId = state?.curveId ?? legacyCurveId ?? 'SPY-VOL';
  const currency = state?.currency ?? legacyCurrency ?? 'USD';
  const underlying = state?.underlying ?? legacyUnderlying ?? 'SPY';
  const spotPrice = state?.spotPrice ?? legacySpotPrice ?? 100;
  const expiries = state?.expiries ?? legacyExpiries ?? [0.5, 1];
  const strikes = state?.strikes ?? legacyStrikes ?? [90, 100, 110];
  const discountCurveId = state?.discountCurveId ?? legacyDiscountCurveId ?? 'USD-OIS';
  const showChart = state?.showChart ?? legacyShowChart ?? true;
  const tolerance = state?.tolerance ?? legacyTolerance ?? DEFAULT_VOL_TOLERANCE;

  // Quote state - controlled or local
  const [localQuotes, setLocalQuotes] = useState<VolQuoteData[]>(
    legacyInitialQuotes ?? DEFAULT_VOL_QUOTES
  );

  // Sync quotes from state prop in controlled mode
  useEffect(() => {
    if (isControlled && state.quotes.length > 0) {
      setLocalQuotes(state.quotes);
    }
  }, [isControlled, state?.quotes]);

  const quotes = isControlled && state.quotes.length > 0 ? state.quotes : localQuotes;

  // Handle quote changes
  const handleQuotesChange = useCallback(
    (newQuotes: VolQuoteData[]) => {
      if (isControlled && onStateChange && state) {
        onStateChange({ ...state, quotes: newQuotes });
      } else {
        setLocalQuotes(newQuotes);
      }
    },
    [isControlled, onStateChange, state]
  );

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
      const calibrationConfig = state?.config
        ? buildWasmConfig(state.config, effectiveTolerance)
        : CalibrationConfig.multiCurve()
            .withSolverKind(SolverKind.Brent())
            .withMaxIterations(legacyConfig?.maxIterations ?? 100)
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
    state?.config,
    legacyConfig,
    market,
    onCalibrated,
    effectiveTolerance,
  ]);

  // Format quote summary for display
  const quotesSummary = useMemo(() => {
    return `${quotes.length} vol quotes`;
  }, [quotes]);

  // Export current state as JSON (for debugging/LLM integration)
  const exportState = useCallback((): VolSurfaceCalibrationState => {
    return {
      baseDate: { year: baseDate.year, month: baseDate.month, day: baseDate.day },
      curveId,
      currency,
      underlying,
      spotPrice,
      expiries,
      strikes,
      discountCurveId,
      quotes,
      config: state?.config ?? {
        solverKind: 'Brent',
        maxIterations: 100,
        tolerance: 1e-8,
        verbose: false,
      },
      tolerance,
      showChart,
    };
  }, [baseDate, curveId, currency, underlying, spotPrice, expiries, strikes, discountCurveId, quotes, state?.config, tolerance, showChart]);

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
          onChange={handleQuotesChange}
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

        {/* JSON State Export (for debugging/LLM integration) */}
        <details className="text-xs">
          <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
            View JSON State
          </summary>
          <pre className="mt-2 p-2 bg-muted/50 rounded overflow-x-auto text-[10px]">
            {JSON.stringify(exportState(), null, 2)}
          </pre>
        </details>
      </CardContent>
    </Card>
  );
};
