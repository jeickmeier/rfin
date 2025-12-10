import React, { useState, useCallback, useMemo, useEffect } from 'react';
import {
  CalibrationConfig,
  CreditQuote,
  FsDate,
  HazardCurveCalibrator,
  MarketContext,
  SolverKind,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CurveChart, StatusBadge, CalibrationMetrics } from './CurveChart';
import { CreditQuoteEditor, DEFAULT_CREDIT_QUOTES, type CdsQuoteData } from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type {
  HazardCurveCalibrationState,
  CalibrationConfigJson,
  DateJson,
} from './state-types';

interface CalibratedHazardCurve {
  sp: (t: number) => number;
  defaultProb: (t1: number, t2: number) => number;
  id: string;
}

interface HazardCurveCalibrationProps {
  /** Complete JSON state */
  state: HazardCurveCalibrationState;
  /** Callback when state changes */
  onStateChange?: (state: HazardCurveCalibrationState) => void;
  /** Market context containing discount curve */
  market: MarketContext | null;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;
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
const toFsDate = (date: DateJson): FsDate => new FsDate(date.year, date.month, date.day);

/** Convert quote data to WASM CreditQuote objects */
const buildWasmQuotes = (quotes: CdsQuoteData[]): CreditQuote[] => {
  return quotes.map((q) =>
    CreditQuote.cds(
      q.entity,
      new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
      q.spreadBps,
      q.recoveryRate,
      q.currency
    )
  );
};

export const HazardCurveCalibration: React.FC<HazardCurveCalibrationProps> = ({
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
}) => {
  const { curveId: _curveId, currency, entity, seniority, recoveryRate, discountCurveId, showChart, config } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  const [localQuotes, setLocalQuotes] = useState<CdsQuoteData[]>(
    state.quotes.length > 0 ? state.quotes : DEFAULT_CREDIT_QUOTES
  );

  useEffect(() => {
    if (state.quotes.length > 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLocalQuotes(state.quotes);
    }
  }, [state.quotes]);

  const quotes = state.quotes.length > 0 ? state.quotes : localQuotes;

  const handleQuotesChange = useCallback(
    (newQuotes: CdsQuoteData[]) => {
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
  const [curve, setCurve] = useState<CalibratedHazardCurve | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runCalibration = useCallback(() => {
    if (quotes.length === 0) {
      setError('No CDS quotes provided');
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

      const calibrator = new HazardCurveCalibrator(entity, seniority, recoveryRate, baseDate, currency, discountCurveId);
      const calibratorWithConfig = calibrator.withConfig(calibrationConfig);

      const [calibratedCurve, report] = calibratorWithConfig.calibrate(wasmQuotes, market) as [
        CalibratedHazardCurve,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      const sampleTimes = [0.5, 1, 2, 3, 5, 7, 10];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: calibratedCurve.sp(t),
        label: `SP(${t}Y)`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId: `${entity}-${seniority}`,
        curveType: 'Hazard (Credit)',
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
        curveId: `${entity}-${seniority}`,
        curveType: 'Hazard (Credit)',
        success: false,
        iterations: 0,
        maxResidual: 0,
        sampleValues: [],
      };
      setResult(failedResult);
      onCalibrated?.(failedResult);
    }
  }, [baseDate, currency, quotes, entity, seniority, recoveryRate, discountCurveId, config, market, onCalibrated]);

  const quotesSummary = useMemo(() => `${quotes.length} CDS quotes`, [quotes]);

  const exportState = useCallback(
    (): HazardCurveCalibrationState => state,
    [state],
  );

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              Hazard Curve (Credit)
              <StatusBadge status={status} />
            </CardTitle>
            <CardDescription>
              {entity} - {seniority} - Recovery: {(recoveryRate * 100).toFixed(0)}% - {quotesSummary}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <CreditQuoteEditor
          quotes={quotes}
          onChange={handleQuotesChange}
          onCalibrate={runCalibration}
          disabled={status === 'running' || !market}
          entity={entity}
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
              title: 'Survival Probability',
              xLabel: 'Time',
              yLabel: 'SP',
              color: 'hsl(var(--chart-4))',
              yFormatter: (v) => `${(v * 100).toFixed(1)}%`,
            }}
            showArea
            referenceLines={[
              { y: 1, label: '100%', stroke: 'hsl(var(--muted-foreground))' },
              { y: 0.5, label: '50%', stroke: 'hsl(var(--destructive))' },
            ]}
          />
        )}

        {curve && result?.success && (
          <div className="grid grid-cols-4 gap-2 text-sm">
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">SP(1Y)</span>
              <span className="font-mono">{(curve.sp(1) * 100).toFixed(2)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">SP(3Y)</span>
              <span className="font-mono">{(curve.sp(3) * 100).toFixed(2)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">SP(5Y)</span>
              <span className="font-mono">{(curve.sp(5) * 100).toFixed(2)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">PD(0-5Y)</span>
              <span className="font-mono text-destructive">{(curve.defaultProb(0, 5) * 100).toFixed(2)}%</span>
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
