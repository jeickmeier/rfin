import React, { useState, useCallback, useMemo } from 'react';
import { CreditQuote, FsDate, MarketContext, executeCalibration } from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CalibrationResultPanel, StatusBadge } from './CurveChart';
import { CreditQuoteEditor, DEFAULT_CREDIT_QUOTES, type CdsQuoteData } from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type { HazardCurveCalibrationState } from './state-types';
import { buildWasmConfig, isoDate, toFsDate, useEffectiveQuotes } from './shared';

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

/** Convert quote data to WASM CreditQuote objects */
const buildWasmQuotes = (quotes: CdsQuoteData[]): CreditQuote[] => {
  return quotes.map((q) =>
    CreditQuote.cdsParSpread(
      `${q.entity}-${q.maturityYear}-${q.maturityMonth}-${q.maturityDay}`,
      q.entity,
      new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
      q.spreadBps,
      q.recoveryRate,
      q.currency,
      'CR14'
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
  const {
    curveId: _curveId,
    currency,
    entity,
    seniority,
    recoveryRate,
    discountCurveId,
    showChart,
    config,
  } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  const [quotes, setLocalQuotes] = useEffectiveQuotes(state.quotes, DEFAULT_CREDIT_QUOTES);

  const handleQuotesChange = (newQuotes: CdsQuoteData[]) => {
    if (onStateChange) {
      onStateChange({ ...state, quotes: newQuotes });
    } else {
      setLocalQuotes(newQuotes);
    }
  };

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

      const curveId = `${entity}-${seniority}`;
      const quoteSet = wasmQuotes.map((q) => q.toMarketQuote().toJSON());
      const envelope = {
        schema: 'finstack.calibration/2',
        initial_market: market.toJson(),
        plan: {
          id: `hazard:${curveId}`,
          quote_sets: {
            cds: quoteSet,
          },
          steps: [
            {
              id: 'haz',
              quote_set: 'cds',
              kind: 'hazard',
              curve_id: curveId,
              entity,
              seniority,
              currency,
              base_date: isoDate(baseDate),
              discount_curve_id: discountCurveId,
              recovery_rate: recoveryRate,
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

      const calibratedCurve = marketCtx.hazard(curveId) as unknown as CalibratedHazardCurve;

      const sampleTimes = [0.5, 1, 2, 3, 5, 7, 10];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: calibratedCurve.sp(t),
        label: `SP(${t}Y)`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId,
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
  }, [
    baseDate,
    currency,
    quotes,
    entity,
    seniority,
    recoveryRate,
    discountCurveId,
    config,
    market,
    onCalibrated,
  ]);

  const quotesSummary = useMemo(() => `${quotes.length} CDS quotes`, [quotes]);

  const exportState = useCallback((): HazardCurveCalibrationState => state, [state]);

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
              {entity} - {seniority} - Recovery: {(recoveryRate * 100).toFixed(0)}% -{' '}
              {quotesSummary}
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

        <CalibrationResultPanel
          result={result}
          showChart={showChart}
          chartConfig={{
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
              <span className="font-mono text-destructive">
                {(curve.defaultProb(0, 5) * 100).toFixed(2)}%
              </span>
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
