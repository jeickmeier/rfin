import React, { useState, useCallback, useMemo } from 'react';
import {
  CalibrationConfig,
  ForwardCurveCalibrator,
  Frequency,
  FsDate,
  MarketContext,
  RatesQuote,
  SolverKind,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CurveChart, StatusBadge, CalibrationMetrics } from './CurveChart';
import { ForwardQuoteEditor, generateDefaultForwardQuotes, type ForwardQuoteData } from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type { FrequencyType } from './CurrencyConventions';

interface CalibratedForwardCurve {
  rate: (t: number) => number;
  id: string;
}

interface ForwardCurveCalibrationProps {
  baseDate: FsDate;
  curveId: string;
  currency: string;
  tenor: number;
  discountCurveId: string;
  config?: CalibrationConfig;
  market: MarketContext | null;
  onCalibrated?: (result: CalibrationResult) => void;
  showChart?: boolean;
  className?: string;
  /** Initial quotes - if not provided, generates dynamic defaults from baseDate */
  initialQuotes?: ForwardQuoteData[];
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

/** Convert quote data to WASM RatesQuote objects (supports deposits, FRAs, and swaps) */
const buildWasmQuotes = (quotes: ForwardQuoteData[]): RatesQuote[] => {
  return quotes.map((q) => {
    if (q.type === 'deposit') {
      return RatesQuote.deposit(
        new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
        q.rate,
        q.dayCount
      );
    } else if (q.type === 'swap') {
      // Use the frequency from the quote data
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
      // FRA quote
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
  baseDate,
  curveId,
  currency,
  tenor,
  discountCurveId,
  config,
  market,
  onCalibrated,
  showChart = true,
  className,
  initialQuotes,
}) => {
  // Generate dynamic defaults from baseDate and currency
  const defaultQuotes = useMemo(() => {
    return generateDefaultForwardQuotes(
      baseDate.year,
      baseDate.month,
      baseDate.day,
      currency
    );
  }, [baseDate, currency]);

  // Local state for editable quotes
  const [quotes, setQuotes] = useState<ForwardQuoteData[]>(initialQuotes ?? defaultQuotes);

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
      const calibrationConfig =
        config ||
        CalibrationConfig.multiCurve()
          .withSolverKind(SolverKind.Brent())
          .withMaxIterations(30)
          .withVerbose(false);

      // Build fresh WASM quotes from the editable data
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new ForwardCurveCalibrator(
        curveId,
        tenor,
        baseDate,
        currency,
        discountCurveId
      );
      const calibratorWithConfig = calibrator.withConfig(calibrationConfig);

      const [calibratedCurve, report] = calibratorWithConfig.calibrate(wasmQuotes, market) as [
        CalibratedForwardCurve,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      // Generate sample values
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

      console.log(`Forward curve '${curveId}' calibrated:`, {
        rate1y: calibratedCurve.rate(1),
        iterations: report.iterations,
      });
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');
      console.warn(`Forward curve calibration failed: ${errorMsg}`);

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

  // Format quote summary for display
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
        {/* Editable Quote Table */}
        <ForwardQuoteEditor
          currency={currency}
          quotes={quotes}
          onChange={setQuotes}
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
      </CardContent>
    </Card>
  );
};
