import React, { useState, useCallback, useMemo, useEffect } from 'react';
import {
  CalibrationConfig,
  FsDate,
  InflationQuote,
  MarketContext,
  executeCalibration,
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

interface InflationCurveCalibrationProps {
  /** Complete JSON state */
  state: InflationCurveCalibrationState;
  /** Callback when state changes */
  onStateChange?: (state: InflationCurveCalibrationState) => void;
  /** Market context containing discount curve */
  market: MarketContext | null;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;
}

/** Convert JSON config to WASM CalibrationConfig */
const buildWasmConfig = (config: CalibrationConfigJson): CalibrationConfig => {
  let wasmConfig = new CalibrationConfig();
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
}) => {
  const { curveId, currency, indexName, baseCpi, discountCurveId, showChart, config } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  const [localQuotes, setLocalQuotes] = useState<InflationSwapQuoteData[]>(
    state.quotes.length > 0 ? state.quotes : DEFAULT_INFLATION_QUOTES
  );

  useEffect(() => {
    if (state.quotes.length > 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalQuotes(state.quotes);
    }
  }, [state.quotes]);

  const quotes = state.quotes.length > 0 ? state.quotes : localQuotes;

  const handleQuotesChange = useCallback(
    (newQuotes: InflationSwapQuoteData[]) => {
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
  const [curve, setCurve] = useState<CalibratedInflationCurve | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runCalibration = () => {
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
      const calibrationConfig = buildWasmConfig(config);
      const wasmQuotes = buildWasmQuotes(quotes);

      const quoteSet = wasmQuotes.map((q) => q.toMarketQuote().toJSON());
      const envelope = {
        schema: 'finstack.calibration/2',
        initial_market: market.toState(),
        plan: {
          id: `inflation:${curveId}`,
          quote_sets: {
            infl: quoteSet,
          },
          steps: [
            {
              id: 'infl',
              quote_set: 'infl',
              kind: 'inflation',
              curve_id: curveId,
              currency,
              base_date: isoDate(baseDate),
              discount_curve_id: discountCurveId,
              index: indexName,
              observation_lag: '3M',
              base_cpi: baseCpi,
            },
          ],
          settings: calibrationConfig.toJSON(),
        },
      };

      const [marketCtx, report] = executeCalibration(envelope) as [
        MarketContext,
        { success: boolean; iterations: number; maxResidual: number },
        Record<string, unknown>,
      ];

      const calibratedCurve = marketCtx.inflation(curveId) as unknown as CalibratedInflationCurve;

      const sampleTimes = [1, 2, 3, 5, 7, 10];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: calibratedCurve.cpi(t),
        label: `CPI(${t}Y)`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId,
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
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');

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
  };

  const getImpliedInflation = (t: number) => {
    if (!curve) return 0;
    const futureCpi = curve.cpi(t);
    return Math.pow(futureCpi / baseCpi, 1 / t) - 1;
  };

  const quotesSummary = useMemo(() => `${quotes.length} inflation swaps`, [quotes]);

  const exportState = useCallback((): InflationCurveCalibrationState => state, [state]);

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
