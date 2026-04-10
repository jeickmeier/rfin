import React, { useState, useCallback, useMemo } from 'react';
import { CreditQuote, MarketContext, executeCalibration } from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CalibrationResultPanel, StatusBadge } from './CurveChart';
import { TrancheQuoteEditor, DEFAULT_TRANCHE_QUOTES, type TrancheQuoteData } from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type { BaseCorrelationCalibrationState } from './state-types';
import { buildWasmConfig, isoDate, toFsDate, useEffectiveQuotes } from './shared';

interface CalibratedBaseCorrelationCurve {
  correlation: (detachment: number) => number;
  id: string;
}

interface BaseCorrelationCalibrationProps {
  /** Complete JSON state */
  state: BaseCorrelationCalibrationState;
  /** Callback when state changes */
  onStateChange?: (state: BaseCorrelationCalibrationState) => void;
  /** Market context containing discount curve and credit index */
  market: MarketContext | null;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;
}

/** Convert quote data to WASM CreditQuote objects */
const buildWasmQuotes = (quotes: TrancheQuoteData[]): ReturnType<CreditQuote['toJSON']>[] => {
  return quotes.map((q) =>
    CreditQuote.fromJSON({
      type: 'cds_tranche',
      id: `${q.index}-${q.attachment}-${q.detachment}`,
      index: q.index,
      attachment: q.attachment / 100,
      detachment: q.detachment / 100,
      maturity: `${q.maturityYear}-${String(q.maturityMonth).padStart(2, '0')}-${String(q.maturityDay).padStart(2, '0')}`,
      upfront_pct: q.upfrontPct / 100,
      running_spread_bp: q.runningSpreadBp,
      convention: 'USD-CR14',
    }).toJSON()
  );
};

export const BaseCorrelationCalibration: React.FC<BaseCorrelationCalibrationProps> = ({
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
}) => {
  const { curveId, indexId, series, maturityYears, discountCurveId, showChart, config } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  const [quotes, setLocalQuotes] = useEffectiveQuotes(state.quotes, DEFAULT_TRANCHE_QUOTES);

  const handleQuotesChange = (newQuotes: TrancheQuoteData[]) => {
    if (onStateChange) {
      onStateChange({ ...state, quotes: newQuotes });
    } else {
      setLocalQuotes(newQuotes);
    }
  };

  const [status, setStatus] = useState<CalibrationStatus>('idle');
  const [result, setResult] = useState<CalibrationResult | null>(null);
  const [curve, setCurve] = useState<CalibratedBaseCorrelationCurve | null>(null);
  const [error, setError] = useState<string | null>(null);

  const runCalibration = useCallback(() => {
    if (quotes.length < 2) {
      setError('At least 2 tranche quotes required');
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

      const detachmentPoints = quotes.map((q) => q.detachment).sort((a, b) => a - b);
      const quoteSet = wasmQuotes;
      const envelope = {
        schema: 'finstack.calibration/2',
        initial_market: market.toJson(),
        plan: {
          id: `base_correlation:${indexId}`,
          quote_sets: {
            tranches: quoteSet,
          },
          steps: [
            {
              id: 'corr',
              quote_set: 'tranches',
              kind: 'base_correlation',
              index_id: indexId,
              series,
              maturity_years: maturityYears,
              base_date: isoDate(baseDate),
              discount_curve_id: discountCurveId,
              detachment_points: detachmentPoints.map((point) => point / 100),
              use_imm_dates: false,
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

      const outputCurveId = `${indexId}_CORR`;
      const calibratedCurve = marketCtx.baseCorrelation(
        outputCurveId
      ) as unknown as CalibratedBaseCorrelationCurve;

      const sampleDetachments = [3, 7, 10, 15, 30];
      const sampleValues: CurveDataPoint[] = sampleDetachments.map((d) => ({
        time: d,
        value: calibratedCurve.correlation(d),
        label: `ρ(${d}%)`,
      }));

      const correlationsValid = sampleValues.every(
        (sv) => sv.value >= 0 && sv.value <= 1 && !Number.isNaN(sv.value)
      );
      const effectiveSuccess = correlationsValid && sampleValues.length > 0;

      const calibrationResult: CalibrationResult = {
        curveId: outputCurveId,
        curveType: 'Base Correlation',
        success: effectiveSuccess,
        iterations: report.iterations,
        maxResidual: report.maxResidual,
        sampleValues,
      };

      setCurve(calibratedCurve);
      setResult(calibrationResult);
      setStatus(effectiveSuccess ? 'success' : 'failed');
      onCalibrated?.(calibrationResult);
    } catch (err) {
      const errorMsg =
        err instanceof Error ? err.message : String(err) || 'Unknown calibration error';
      setError(errorMsg);
      setStatus('failed');

      const failedResult: CalibrationResult = {
        curveId,
        curveType: 'Base Correlation',
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
    indexId,
    series,
    maturityYears,
    quotes,
    discountCurveId,
    config,
    market,
    onCalibrated,
  ]);

  const exportState = useCallback((): BaseCorrelationCalibrationState => state, [state]);

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              Base Correlation
              <StatusBadge status={status} />
            </CardTitle>
            <CardDescription>
              {indexId} Series {series} • {maturityYears}Y • {quotes.length} tranches
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <TrancheQuoteEditor
          quotes={quotes}
          onChange={handleQuotesChange}
          onCalibrate={runCalibration}
          disabled={status === 'running' || !market}
          indexId={indexId}
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
            title: 'Base Correlation Curve',
            xLabel: 'Detachment (%)',
            yLabel: 'Correlation',
            color: 'hsl(var(--chart-5))',
            yFormatter: (v) => `${(v * 100).toFixed(1)}%`,
            xFormatter: (v) => `${v}%`,
          }}
          showArea
          referenceLines={[{ y: 0.5, label: '50%', stroke: 'hsl(var(--muted-foreground))' }]}
        />

        {curve && result?.success && (
          <div className="grid grid-cols-5 gap-2 text-sm">
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">ρ(3%)</span>
              <span className="font-mono">{(curve.correlation(3) * 100).toFixed(1)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">ρ(7%)</span>
              <span className="font-mono">{(curve.correlation(7) * 100).toFixed(1)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">ρ(10%)</span>
              <span className="font-mono">{(curve.correlation(10) * 100).toFixed(1)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">ρ(15%)</span>
              <span className="font-mono">{(curve.correlation(15) * 100).toFixed(1)}%</span>
            </div>
            <div className="p-2 bg-muted/50 rounded">
              <span className="text-muted-foreground text-xs block">ρ(30%)</span>
              <span className="font-mono">{(curve.correlation(30) * 100).toFixed(1)}%</span>
            </div>
          </div>
        )}

        <div className="bg-muted/30 border-l-2 border-primary/50 p-3 rounded-r text-xs text-muted-foreground space-y-2">
          <p>
            <strong>Base Correlation:</strong> Models CDO tranche pricing using a single correlation
            parameter per detachment point. Higher correlation for senior tranches reflects
            systematic risk.
          </p>
          <p className="text-amber-600">
            <strong>Note:</strong> This is an advanced calibration requiring properly configured
            credit index data (hazard curve, recovery rate, num constituents) and consistent equity
            sub-tranche quotes [0, D] for each detachment point D.
          </p>
        </div>

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
