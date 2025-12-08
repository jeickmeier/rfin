import React, { useState, useCallback, useMemo } from 'react';
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

interface CalibratedInflationCurve {
  cpi: (t: number) => number;
  id: string;
}

interface InflationCurveCalibrationProps {
  baseDate: FsDate;
  curveId: string;
  currency: string;
  indexName: string;
  baseCpi: number;
  discountCurveId: string;
  config?: CalibrationConfig;
  market: MarketContext | null;
  onCalibrated?: (result: CalibrationResult) => void;
  showChart?: boolean;
  className?: string;
  /** Initial quotes - if not provided, uses DEFAULT_INFLATION_QUOTES */
  initialQuotes?: InflationSwapQuoteData[];
}

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
  baseDate,
  curveId,
  currency,
  indexName,
  baseCpi,
  discountCurveId,
  config,
  market,
  onCalibrated,
  showChart = true,
  className,
  initialQuotes,
}) => {
  // Local state for editable quotes
  const [quotes, setQuotes] = useState<InflationSwapQuoteData[]>(
    initialQuotes ?? DEFAULT_INFLATION_QUOTES
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
      const calibrationConfig =
        config ||
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
    config,
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
          onChange={setQuotes}
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
      </CardContent>
    </Card>
  );
};
