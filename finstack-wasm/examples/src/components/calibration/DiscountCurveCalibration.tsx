import React, { useState, useCallback, useMemo, useEffect } from 'react';
import {
  CalibrationConfig,
  DiscountCurveCalibrator,
  FsDate,
  Frequency,
  RatesQuote,
  SolverKind,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CurveChart, StatusBadge, CalibrationMetrics } from './CurveChart';
import {
  DiscountQuoteEditor,
  generateDefaultDiscountQuotes,
  type DiscountQuoteData,
} from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type { FrequencyType } from './CurrencyConventions';
import type { DiscountCurveCalibrationState, CalibrationConfigJson, DateJson } from './state-types';

interface CalibratedCurve {
  df: (t: number) => number;
  zero: (t: number) => number;
  id: string;
}

interface DiscountCurveCalibrationProps {
  /** Complete JSON state */
  state: DiscountCurveCalibrationState;
  /** Callback when state changes */
  onStateChange?: (state: DiscountCurveCalibrationState) => void;
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

/** Convert quote data to WASM RatesQuote objects */
const buildWasmQuotes = (quotes: DiscountQuoteData[]): RatesQuote[] => {
  return quotes.map((q) => {
    if (q.type === 'deposit') {
      return RatesQuote.deposit(
        new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
        q.rate,
        q.dayCount
      );
    } else {
      return RatesQuote.swap(
        new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
        q.rate,
        mapFrequency(q.fixedFrequency),
        mapFrequency(q.floatFrequency),
        q.fixedDayCount,
        q.floatDayCount,
        q.index
      );
    }
  });
};

export const DiscountCurveCalibration: React.FC<DiscountCurveCalibrationProps> = ({
  state,
  onStateChange,
  onCalibrated,
  className,
}) => {
  const { curveId, currency, showChart, config } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  // Generate dynamic defaults from baseDate and currency
  const defaultQuotes = useMemo(
    () => generateDefaultDiscountQuotes(baseDate.year, baseDate.month, baseDate.day, currency),
    [baseDate, currency]
  );

  // Local quotes state (synced with state.quotes)
  const [localQuotes, setLocalQuotes] = useState<DiscountQuoteData[]>(
    state.quotes.length > 0 ? state.quotes : defaultQuotes
  );

  // Sync quotes from state prop
  useEffect(() => {
    if (state.quotes.length > 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalQuotes(state.quotes);
    }
  }, [state.quotes]);

  const quotes = state.quotes.length > 0 ? state.quotes : localQuotes;

  const handleQuotesChange = useCallback(
    (newQuotes: DiscountQuoteData[]) => {
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
  const [curve, setCurve] = useState<CalibratedCurve | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runCalibration = useCallback(() => {
    if (quotes.length === 0) {
      setError('No quotes provided');
      return;
    }

    setStatus('running');
    setError(null);

    try {
      const calibrationConfig = buildWasmConfig(config);
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new DiscountCurveCalibrator(curveId, baseDate, currency);
      const calibratorWithConfig = calibrator.withConfig(calibrationConfig);

      const [calibratedCurve, report] = calibratorWithConfig.calibrate(wasmQuotes, null) as [
        CalibratedCurve,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      const sampleTimes = [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: calibratedCurve.df(t),
        label: `DF(${t}Y)`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId,
        curveType: 'Discount',
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
        curveType: 'Discount',
        success: false,
        iterations: 0,
        maxResidual: 0,
        sampleValues: [],
      };
      setResult(failedResult);
      onCalibrated?.(failedResult);
    }
  }, [baseDate, curveId, currency, quotes, config, onCalibrated]);

  const quotesSummary = useMemo(() => {
    const deposits = quotes.filter((q) => q.type === 'deposit').length;
    const swaps = quotes.filter((q) => q.type === 'swap').length;
    return `${deposits} deposits, ${swaps} swaps`;
  }, [quotes]);

  const exportState = useCallback((): DiscountCurveCalibrationState => state, [state]);

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              Discount Curve
              <StatusBadge status={status} />
            </CardTitle>
            <CardDescription>
              {curveId} • {currency} • {quotesSummary}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <DiscountQuoteEditor
          quotes={quotes}
          onChange={handleQuotesChange}
          onCalibrate={runCalibration}
          disabled={status === 'running'}
          currency={currency}
        />

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
              title: 'Discount Factors',
              xLabel: 'Maturity',
              yLabel: 'DF',
              color: 'hsl(var(--chart-1))',
              yFormatter: (v) => v.toFixed(4),
            }}
            showArea
            referenceLines={[{ y: 1, label: 'Par', stroke: 'hsl(var(--muted-foreground))' }]}
          />
        )}

        {curve && result?.success && (
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">DF(1Y)</span>
              <span className="font-mono">{curve.df(1).toFixed(6)}</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">Zero(1Y)</span>
              <span className="font-mono">{(curve.zero(1) * 100).toFixed(3)}%</span>
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
