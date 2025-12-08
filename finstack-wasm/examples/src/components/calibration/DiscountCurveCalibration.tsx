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
import type {
  DiscountCurveCalibrationState,
  CalibrationConfigJson,
  DateJson,
} from './state-types';

interface CalibratedCurve {
  df: (t: number) => number;
  zero: (t: number) => number;
  id: string;
}

/**
 * Props for DiscountCurveCalibration component.
 * Supports both controlled (via state prop) and uncontrolled modes.
 */
interface DiscountCurveCalibrationProps {
  /** Complete JSON state for controlled mode */
  state?: DiscountCurveCalibrationState;
  /** Callback when state changes (for controlled mode) */
  onStateChange?: (state: DiscountCurveCalibrationState) => void;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;

  // Legacy props for backward compatibility (used when state is not provided)
  /** @deprecated Use state.baseDate instead */
  baseDate?: FsDate;
  /** @deprecated Use state.curveId instead */
  curveId?: string;
  /** @deprecated Use state.currency instead */
  currency?: string;
  /** @deprecated Use state.config instead */
  config?: CalibrationConfig;
  /** @deprecated Use state.showChart instead */
  showChart?: boolean;
  /** @deprecated Use state.quotes instead */
  initialQuotes?: DiscountQuoteData[];
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
const toFsDate = (date: DateJson): FsDate => {
  return new FsDate(date.year, date.month, date.day);
};

/** Convert quote data to WASM RatesQuote objects using quote frequencies */
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
  // Legacy props
  baseDate: legacyBaseDate,
  curveId: legacyCurveId,
  currency: legacyCurrency,
  config: legacyConfig,
  showChart: legacyShowChart,
  initialQuotes: legacyInitialQuotes,
}) => {
  // Determine if we're in controlled mode
  const isControlled = state !== undefined;

  // Extract values from state or legacy props
  const baseDate = useMemo(() => {
    if (state) return toFsDate(state.baseDate);
    if (legacyBaseDate) return legacyBaseDate;
    return new FsDate(new Date().getFullYear(), new Date().getMonth() + 1, new Date().getDate());
  }, [state, legacyBaseDate]);

  const curveId = state?.curveId ?? legacyCurveId ?? 'USD-OIS';
  const currency = state?.currency ?? legacyCurrency ?? 'USD';
  const showChart = state?.showChart ?? legacyShowChart ?? true;

  // Generate dynamic defaults from baseDate and currency
  const defaultQuotes = useMemo(() => {
    return generateDefaultDiscountQuotes(baseDate.year, baseDate.month, baseDate.day, currency);
  }, [baseDate, currency]);

  // Quote state - controlled or local
  const [localQuotes, setLocalQuotes] = useState<DiscountQuoteData[]>(
    legacyInitialQuotes ?? defaultQuotes
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
    (newQuotes: DiscountQuoteData[]) => {
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
      // Build calibration config
      const calibrationConfig = state?.config
        ? buildWasmConfig(state.config)
        : legacyConfig ||
          CalibrationConfig.multiCurve()
            .withSolverKind(SolverKind.Brent())
            .withMaxIterations(40)
            .withVerbose(false);

      // Build fresh WASM quotes from the editable data
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new DiscountCurveCalibrator(curveId, baseDate, currency);
      const calibratorWithConfig = calibrator.withConfig(calibrationConfig);

      const [calibratedCurve, report] = calibratorWithConfig.calibrate(wasmQuotes, null) as [
        CalibratedCurve,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      // Generate sample values across the curve
      const sampleTimes = [0.25, 0.5, 1, 2, 3, 5, 7, 10];
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

      console.log(`✅ Discount curve '${curveId}' calibrated:`, {
        df1y: calibratedCurve.df(1),
        zero1y: calibratedCurve.zero(1),
        iterations: report.iterations,
      });
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');
      console.warn(`Discount curve calibration failed: ${errorMsg}`);

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
  }, [baseDate, curveId, currency, quotes, state?.config, legacyConfig, onCalibrated]);

  // Format quote for display
  const quotesSummary = useMemo(() => {
    const deposits = quotes.filter((q) => q.type === 'deposit').length;
    const swaps = quotes.filter((q) => q.type === 'swap').length;
    return `${deposits} deposits, ${swaps} swaps`;
  }, [quotes]);

  // Export current state as JSON (for debugging/LLM integration)
  const exportState = useCallback((): DiscountCurveCalibrationState => {
    return {
      baseDate: { year: baseDate.year, month: baseDate.month, day: baseDate.day },
      curveId,
      currency,
      quotes,
      config: state?.config ?? {
        solverKind: 'Brent',
        maxIterations: 40,
        tolerance: 1e-8,
        verbose: false,
      },
      showChart,
    };
  }, [baseDate, curveId, currency, quotes, state?.config, showChart]);

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
        {/* Editable Quote Table */}
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
