import React, { useState, useCallback, useMemo } from 'react';
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

interface CalibratedHazardCurve {
  sp: (t: number) => number;
  defaultProb: (t1: number, t2: number) => number;
  id: string;
}

interface HazardCurveCalibrationProps {
  baseDate: FsDate;
  curveId: string;
  currency: string;
  entity: string;
  seniority: string;
  recoveryRate: number;
  discountCurveId: string;
  config?: CalibrationConfig;
  market: MarketContext | null;
  onCalibrated?: (result: CalibrationResult) => void;
  showChart?: boolean;
  className?: string;
  /** Initial quotes - if not provided, uses DEFAULT_CREDIT_QUOTES */
  initialQuotes?: CdsQuoteData[];
}

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
  baseDate,
  curveId,
  currency,
  entity,
  seniority,
  recoveryRate,
  discountCurveId,
  config,
  market,
  onCalibrated,
  showChart = true,
  className,
  initialQuotes,
}) => {
  // Local state for editable quotes
  const [quotes, setQuotes] = useState<CdsQuoteData[]>(initialQuotes ?? DEFAULT_CREDIT_QUOTES);

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
      const calibrationConfig =
        config ||
        CalibrationConfig.multiCurve()
          .withSolverKind(SolverKind.Brent())
          .withMaxIterations(25)
          .withVerbose(false);

      // Build fresh WASM quotes from the editable data
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new HazardCurveCalibrator(
        entity,
        seniority,
        recoveryRate,
        baseDate,
        currency,
        discountCurveId
      );
      const calibratorWithConfig = calibrator.withConfig(calibrationConfig);

      const [calibratedCurve, report] = calibratorWithConfig.calibrate(wasmQuotes, market) as [
        CalibratedHazardCurve,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      // Generate survival probability curve
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

      console.log(`Hazard curve '${entity}' calibrated:`, {
        sp1y: calibratedCurve.sp(1),
        sp5y: calibratedCurve.sp(5),
        defaultProb5y: calibratedCurve.defaultProb(0, 5),
        iterations: report.iterations,
      });
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');
      console.warn(`Hazard curve calibration failed: ${errorMsg}`);

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
    curveId,
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

  // Format quote summary for display
  const quotesSummary = useMemo(() => {
    return `${quotes.length} CDS quotes`;
  }, [quotes]);

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
        {/* Editable Quote Table */}
        <CreditQuoteEditor
          quotes={quotes}
          onChange={setQuotes}
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
              <span className="font-mono text-destructive">
                {(curve.defaultProb(0, 5) * 100).toFixed(2)}%
              </span>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
};
