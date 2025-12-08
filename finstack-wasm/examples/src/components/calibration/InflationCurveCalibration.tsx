import React, { useState, useCallback, useMemo, useEffect } from 'react';
import {
  CalibrationConfig,
  FsDate,
  InflationCurveCalibrator,
  InflationQuote,
  MarketContext,
  SolverKind,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CurveChart, StatusBadge, CalibrationMetrics } from './CurveChart';
import {
  InflationQuoteEditor,
  DEFAULT_INFLATION_QUOTES,
  type InflationSwapQuoteData,
} from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type {
  InflationCurveCalibrationState,
  CalibrationConfigJson,
  DateJson,
} from './state-types';

interface CalibratedInflationCurve {
  cpi: (t: number) => number;
  id: string;
}

/**
 * Props for InflationCurveCalibration component.
 * Supports both controlled (via state prop) and uncontrolled modes.
 */
interface InflationCurveCalibrationProps {
  /** Complete JSON state for controlled mode */
  state?: InflationCurveCalibrationState;
  /** Callback when state changes (for controlled mode) */
  onStateChange?: (state: InflationCurveCalibrationState) => void;
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
  /** @deprecated Use state.indexName instead */
  indexName?: string;
  /** @deprecated Use state.baseCpi instead */
  baseCpi?: number;
  /** @deprecated Use state.discountCurveId instead */
  discountCurveId?: string;
  /** @deprecated Use state.config instead */
  config?: CalibrationConfig;
  /** @deprecated Use state.showChart instead */
  showChart?: boolean;
  /** @deprecated Use state.quotes instead */
  initialQuotes?: InflationSwapQuoteData[];
}

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

/** Convert quote data to WASM InflationQuote objects */
const buildWasmQuotes = (quotes: InflationSwapQuoteData[]): InflationQuote[] => {
  return quotes.map((q) =>
    InflationQuote.inflationSwap(
      new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
      q.rate,
      q.indexName
    )
  );
};

export const InflationCurveCalibration: React.FC<InflationCurveCalibrationProps> = ({
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
  // Legacy props
  baseDate: legacyBaseDate,
  curveId: legacyCurveId,
  currency: legacyCurrency,
  indexName: legacyIndexName,
  baseCpi: legacyBaseCpi,
  discountCurveId: legacyDiscountCurveId,
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

  const curveId = state?.curveId ?? legacyCurveId ?? 'US-CPI-U';
  const currency = state?.currency ?? legacyCurrency ?? 'USD';
  const indexName = state?.indexName ?? legacyIndexName ?? 'US-CPI-U';
  const baseCpi = state?.baseCpi ?? legacyBaseCpi ?? 300;
  const discountCurveId = state?.discountCurveId ?? legacyDiscountCurveId ?? 'USD-OIS';
  const showChart = state?.showChart ?? legacyShowChart ?? true;

  // Quote state - controlled or local
  const [localQuotes, setLocalQuotes] = useState<InflationSwapQuoteData[]>(
    legacyInitialQuotes ?? DEFAULT_INFLATION_QUOTES
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
    (newQuotes: InflationSwapQuoteData[]) => {
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
  const [curve, setCurve] = useState<CalibratedInflationCurve | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runCalibration = useCallback(() => {
    if (quotes.length === 0) {
      setError('No inflation quotes provided');
      return;
    }

    if (!market) {
      setError('Market context with discount curve required');
      return;
    }

    setStatus('running');
    setError(null);

    try {
      const calibrationConfig = state?.config
        ? buildWasmConfig(state.config)
        : legacyConfig ||
          CalibrationConfig.multiCurve()
            .withSolverKind(SolverKind.Brent())
            .withMaxIterations(25)
            .withVerbose(false);

      // Build fresh WASM quotes from the editable data
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new InflationCurveCalibrator(
        indexName,
        baseDate,
        currency,
        baseCpi,
        discountCurveId
      );
      const calibratorWithConfig = calibrator.withConfig(calibrationConfig);

      const [calibratedCurve, report] = calibratorWithConfig.calibrate(wasmQuotes, market) as [
        CalibratedInflationCurve,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      // Generate CPI projection curve
      const sampleTimes = [1, 2, 3, 5, 7, 10];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: calibratedCurve.cpi(t),
        label: `CPI(${t}Y)`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId: indexName,
        curveType: 'Inflation',
        success: report.success,
        iterations: report.iterations,
        maxResidual: report.maxResidual,
        sampleValues,
      };

      setCurve(calibratedCurve);
      setResult(calibrationResult);
      setStatus(report.success ? 'success' : 'failed');
      onCalibrated?.(calibrationResult);

      console.log(`Inflation curve '${indexName}' calibrated:`, {
        cpi1y: calibratedCurve.cpi(1),
        cpi5y: calibratedCurve.cpi(5),
        iterations: report.iterations,
      });
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');
      console.warn(`Inflation curve calibration failed: ${errorMsg}`);

      const failedResult: CalibrationResult = {
        curveId: indexName,
        curveType: 'Inflation',
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
    indexName,
    baseCpi,
    discountCurveId,
    state?.config,
    legacyConfig,
    market,
    onCalibrated,
  ]);

  // Calculate implied inflation rates
  const getImpliedInflation = (t: number) => {
    if (!curve) return 0;
    const futureCpi = curve.cpi(t);
    return Math.pow(futureCpi / baseCpi, 1 / t) - 1;
  };

  // Format quote summary for display
  const quotesSummary = useMemo(() => {
    return `${quotes.length} inflation swaps`;
  }, [quotes]);

  // Export current state as JSON (for debugging/LLM integration)
  const exportState = useCallback((): InflationCurveCalibrationState => {
    return {
      baseDate: { year: baseDate.year, month: baseDate.month, day: baseDate.day },
      curveId,
      currency,
      indexName,
      baseCpi,
      discountCurveId,
      quotes,
      config: state?.config ?? {
        solverKind: 'Brent',
        maxIterations: 25,
        tolerance: 1e-8,
        verbose: false,
      },
      showChart,
    };
  }, [baseDate, curveId, currency, indexName, baseCpi, discountCurveId, quotes, state?.config, showChart]);

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              Inflation Curve
              <StatusBadge status={status} />
            </CardTitle>
            <CardDescription>
              {indexName} - Base CPI: {baseCpi.toFixed(1)} - {quotesSummary}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Editable Quote Table */}
        <InflationQuoteEditor
          quotes={quotes}
          onChange={handleQuotesChange}
          onCalibrate={runCalibration}
          disabled={status === 'running' || !market}
          indexName={indexName}
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
              title: 'CPI Projection',
              xLabel: 'Time',
              yLabel: 'CPI Index',
              color: 'hsl(var(--chart-3))',
              yFormatter: (v) => v.toFixed(1),
            }}
            referenceLines={[
              { y: baseCpi, label: `Base: ${baseCpi}`, stroke: 'hsl(var(--muted-foreground))' },
            ]}
          />
        )}

        {curve && result?.success && (
          <>
            <div className="grid grid-cols-3 gap-2 text-sm">
              <div className="p-2 bg-muted/50 rounded">
                <span className="text-muted-foreground text-xs block">CPI(1Y)</span>
                <span className="font-mono">{curve.cpi(1).toFixed(2)}</span>
              </div>
              <div className="p-2 bg-muted/50 rounded">
                <span className="text-muted-foreground text-xs block">CPI(3Y)</span>
                <span className="font-mono">{curve.cpi(3).toFixed(2)}</span>
              </div>
              <div className="p-2 bg-muted/50 rounded">
                <span className="text-muted-foreground text-xs block">CPI(5Y)</span>
                <span className="font-mono">{curve.cpi(5).toFixed(2)}</span>
              </div>
            </div>
            <div className="grid grid-cols-3 gap-2 text-sm">
              <div className="p-2 bg-warning/10 border border-warning/20 rounded">
                <span className="text-muted-foreground text-xs block">Implied Infl. (1Y)</span>
                <span className="font-mono">{(getImpliedInflation(1) * 100).toFixed(2)}%</span>
              </div>
              <div className="p-2 bg-warning/10 border border-warning/20 rounded">
                <span className="text-muted-foreground text-xs block">Implied Infl. (3Y)</span>
                <span className="font-mono">{(getImpliedInflation(3) * 100).toFixed(2)}%</span>
              </div>
              <div className="p-2 bg-warning/10 border border-warning/20 rounded">
                <span className="text-muted-foreground text-xs block">Implied Infl. (5Y)</span>
                <span className="font-mono">{(getImpliedInflation(5) * 100).toFixed(2)}%</span>
              </div>
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
