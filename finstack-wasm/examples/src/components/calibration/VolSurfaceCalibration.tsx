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

const MIN_VOL_TOLERANCE = 0.01;

interface VolSurfaceCalibrationProps {
  /** Complete JSON state */
  state: VolSurfaceCalibrationState;
  /** Callback when state changes */
  onStateChange?: (state: VolSurfaceCalibrationState) => void;
  /** Market context containing discount curve */
  market: MarketContext | null;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;
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
const toFsDate = (date: DateJson): FsDate => new FsDate(date.year, date.month, date.day);

/** Convert quote data to WASM VolQuote objects */
const buildWasmQuotes = (quotes: VolQuoteData[]): VolQuote[] => {
  return quotes.map((q) =>
    VolQuote.optionVol(q.underlying, new FsDate(q.expiryYear, q.expiryMonth, q.expiryDay), q.strike, q.vol, q.optionType)
  );
};

export const VolSurfaceCalibration: React.FC<VolSurfaceCalibrationProps> = ({
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
}) => {
  const { curveId, currency, underlying, spotPrice, expiries, strikes, discountCurveId, showChart, config, tolerance } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  const [localQuotes, setLocalQuotes] = useState<VolQuoteData[]>(
    state.quotes.length > 0 ? state.quotes : DEFAULT_VOL_QUOTES
  );

  useEffect(() => {
    if (state.quotes.length > 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalQuotes(state.quotes);
    }
  }, [state.quotes]);

  const quotes = state.quotes.length > 0 ? state.quotes : localQuotes;

  const handleQuotesChange = useCallback(
    (newQuotes: VolQuoteData[]) => {
      if (onStateChange) {
        onStateChange({ ...state, quotes: newQuotes });
      } else {
        setLocalQuotes(newQuotes);
      }
    },
    [onStateChange, state]
  );

  const [status, setStatus] = useState<CalibrationStatus>('idle');
  const [result, setResult] = useState<CalibrationResult | null>(null);
  const [surface, setSurface] = useState<CalibratedVolSurface | null>(null);
  const [error, setError] = useState<string | null>(null);

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
      market.insertPrice(underlying, MarketScalar.price(Money.fromCode(spotPrice, currency)));
      market.insertPrice(`${underlying}-DIVYIELD`, MarketScalar.unitless(0.015));

      const calibrationConfig = buildWasmConfig(config, effectiveTolerance);
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new VolSurfaceCalibrator(curveId, 1, new Float64Array(expiries), new Float64Array(strikes))
        .withBaseDate(baseDate)
        .withConfig(calibrationConfig)
        .withDiscountId(discountCurveId);

      const [calibratedSurface, report] = calibrator.calibrate(wasmQuotes, market) as [
        CalibratedVolSurface,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      const atmStrike = strikes.find((s) => Math.abs(s - 100) < 5) || strikes[Math.floor(strikes.length / 2)];
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
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');

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
  }, [baseDate, curveId, currency, quotes, underlying, spotPrice, expiries, strikes, discountCurveId, config, market, onCalibrated, effectiveTolerance]);

  const quotesSummary = useMemo(() => `${quotes.length} vol quotes`, [quotes]);

  const exportState = useCallback((): VolSurfaceCalibrationState => state, [state]);

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
              {underlying} - Spot: {spotPrice} {currency} - {expiries.length}x{strikes.length} grid - {quotesSummary}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
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
            <div className="text-sm font-medium text-muted-foreground mb-2">Vol Surface Grid (Expiry x Strike)</div>
            <div className="overflow-x-auto">
              <table className="text-xs w-full">
                <thead>
                  <tr>
                    <th className="text-left p-1 text-muted-foreground">Expiry</th>
                    {strikes.map((k) => (
                      <th key={`strike-${k}`} className="text-right p-1 text-muted-foreground">{k}%</th>
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
