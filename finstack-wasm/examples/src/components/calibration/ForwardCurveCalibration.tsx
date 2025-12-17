import React, { useState, useCallback, useMemo, useEffect } from 'react';
import {
  CalibrationConfig,
  Frequency,
  FsDate,
  MarketContext,
  RatesQuote,
  executeCalibrationV2,
  SolverKind,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CurveChart, StatusBadge, CalibrationMetrics } from './CurveChart';
import {
  ForwardQuoteEditor,
  generateDefaultForwardQuotes,
  type ForwardQuoteData,
} from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type { FrequencyType } from './CurrencyConventions';
import type { ForwardCurveCalibrationState, CalibrationConfigJson, DateJson } from './state-types';

interface CalibratedForwardCurve {
  rate: (t: number) => number;
  id: string;
}

interface ForwardCurveCalibrationProps {
  /** Complete JSON state */
  state: ForwardCurveCalibrationState;
  /** Callback when state changes */
  onStateChange?: (state: ForwardCurveCalibrationState) => void;
  /** Market context containing discount curve */
  market: MarketContext | null;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;
}

/** Map frequency type string to WASM Frequency object */
const mapFrequency = (freq: FrequencyType): ReturnType<typeof Frequency.annual> => {
  switch (freq) {
    case 'annual':
      return Frequency.annual();
    case 'semi_annual':
      return Frequency.semiAnnual();
    case 'quarterly':
      return Frequency.quarterly();
    case 'monthly':
      return Frequency.monthly();
    default:
      return Frequency.quarterly();
  }
};

/** Convert JSON config to WASM CalibrationConfig */
const buildWasmConfig = (config: CalibrationConfigJson): CalibrationConfig => {
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
    .withTolerance(config.tolerance)
    .withVerbose(config.verbose);
};

/** Convert DateJson to FsDate */
const toFsDate = (date: DateJson): FsDate => new FsDate(date.year, date.month, date.day);

const isoDate = (date: FsDate): string => {
  const y = String(date.year).padStart(4, '0');
  const m = String(date.month).padStart(2, '0');
  const d = String(date.day).padStart(2, '0');
  return `${y}-${m}-${d}`;
};

/** Convert quote data to WASM RatesQuote objects */
const buildWasmQuotes = (quotes: ForwardQuoteData[]): RatesQuote[] => {
  return quotes.map((q) => {
    if (q.type === 'deposit') {
      return RatesQuote.deposit(
        new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
        q.rate,
        q.dayCount
      );
    } else if (q.type === 'swap') {
      return RatesQuote.swap(
        new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
        q.rate,
        mapFrequency(q.fixedFrequency),
        mapFrequency(q.floatFrequency),
        q.fixedDayCount,
        q.floatDayCount,
        q.index
      );
    } else {
      return RatesQuote.fra(
        new FsDate(q.startYear, q.startMonth, q.startDay),
        new FsDate(q.endYear, q.endMonth, q.endDay),
        q.rate,
        q.dayCount
      );
    }
  });
};

export const ForwardCurveCalibration: React.FC<ForwardCurveCalibrationProps> = ({
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
}) => {
  const { curveId, currency, tenor, discountCurveId, showChart, config } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  const defaultQuotes = useMemo(
    () => generateDefaultForwardQuotes(baseDate.year, baseDate.month, baseDate.day, currency),
    [baseDate, currency]
  );

  const [localQuotes, setLocalQuotes] = useState<ForwardQuoteData[]>(
    state.quotes.length > 0 ? state.quotes : defaultQuotes
  );

  useEffect(() => {
    if (state.quotes.length > 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalQuotes(state.quotes);
    }
  }, [state.quotes]);

  const quotes = state.quotes.length > 0 ? state.quotes : localQuotes;

  const handleQuotesChange = useCallback(
    (newQuotes: ForwardQuoteData[]) => {
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
  const [curve, setCurve] = useState<CalibratedForwardCurve | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runCalibration = useCallback(() => {
    if (quotes.length === 0) {
      setError('No quotes provided');
      return;
    }
    if (!market) {
      setError('Market context with discount curve required');
      return;
    }

    setStatus('running');
    setError(null);

    try {
      const calibrationConfig = buildWasmConfig(config);
      const wasmQuotes = buildWasmQuotes(quotes);

      const quoteSet = wasmQuotes.map((q) => q.toMarketQuote().toJSON());
      const envelope = {
        schema: 'finstack.calibration/2',
        initial_market: market.toState(),
        plan: {
          id: `forward:${curveId}`,
          quote_sets: {
            fwd: quoteSet,
          },
          steps: [
            {
              id: 'fwd',
              quote_set: 'fwd',
              kind: 'forward',
              curve_id: curveId,
              currency,
              base_date: isoDate(baseDate),
              tenor_years: tenor,
              discount_curve_id: discountCurveId,
            },
          ],
          settings: calibrationConfig.toJSON(),
        },
      };

      const [marketCtx, report] = executeCalibrationV2(envelope) as [
        MarketContext,
        { success: boolean; iterations: number; maxResidual: number },
        Record<string, unknown>,
      ];

      const calibratedCurve = marketCtx.forward(curveId) as unknown as CalibratedForwardCurve;

      const sampleTimes = [0.25, 0.5, 1, 2, 3, 5, 7, 10];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: calibratedCurve.rate(t),
        label: `Fwd(${t}Y)`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId,
        curveType: 'Forward',
        success: report.success,
        iterations: report.iterations,
        maxResidual: report.maxResidual,
        sampleValues,
      };

      setCurve(calibratedCurve);
      setResult(calibrationResult);
      setStatus(report.success ? 'success' : 'failed');
      onCalibrated?.(calibrationResult);
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');

      const failedResult: CalibrationResult = {
        curveId,
        curveType: 'Forward',
        success: false,
        iterations: 0,
        maxResidual: 0,
        sampleValues: [],
      };
      setResult(failedResult);
      onCalibrated?.(failedResult);
    }
  }, [baseDate, curveId, currency, quotes, tenor, discountCurveId, config, market, onCalibrated]);

  const quotesSummary = useMemo(() => {
    const deposits = quotes.filter((q) => q.type === 'deposit').length;
    const fras = quotes.filter((q) => q.type === 'fra').length;
    const swaps = quotes.filter((q) => q.type === 'swap').length;
    const parts: string[] = [];
    if (deposits > 0) parts.push(`${deposits} deposits`);
    if (fras > 0) parts.push(`${fras} FRAs`);
    if (swaps > 0) parts.push(`${swaps} swaps`);
    return parts.join(', ') || 'No quotes';
  }, [quotes]);

  const exportState = useCallback((): ForwardCurveCalibrationState => state, [state]);

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              Forward Curve
              <StatusBadge status={status} />
            </CardTitle>
            <CardDescription>
              {curveId} - {currency} - Tenor: {tenor}Y - {quotesSummary}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <ForwardQuoteEditor
          currency={currency}
          quotes={quotes}
          onChange={handleQuotesChange}
          onCalibrate={runCalibration}
          disabled={status === 'running' || !market}
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
              title: 'Forward Rates',
              xLabel: 'Maturity',
              yLabel: 'Rate',
              color: 'hsl(var(--chart-2))',
              yFormatter: (v) => `${(v * 100).toFixed(2)}%`,
            }}
          />
        )}

        {curve && result?.success && (
          <div className="grid grid-cols-3 gap-2 text-sm">
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">Fwd(1Y)</span>
              <span className="font-mono">{(curve.rate(1) * 100).toFixed(3)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">Fwd(2Y)</span>
              <span className="font-mono">{(curve.rate(2) * 100).toFixed(3)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">Fwd(5Y)</span>
              <span className="font-mono">{(curve.rate(5) * 100).toFixed(3)}%</span>
            </div>
          </div>
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
