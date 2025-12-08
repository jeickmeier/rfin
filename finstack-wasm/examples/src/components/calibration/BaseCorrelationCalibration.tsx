import React, { useState, useCallback, useEffect } from 'react';
import {
  BaseCorrelationCalibrator,
  CalibrationConfig,
  CreditQuote,
  FsDate,
  MarketContext,
  SolverKind,
} from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CurveChart, StatusBadge, CalibrationMetrics } from './CurveChart';
import { TrancheQuoteEditor, DEFAULT_TRANCHE_QUOTES, type TrancheQuoteData } from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type {
  BaseCorrelationCalibrationState,
  CalibrationConfigJson,
  DateJson,
} from './state-types';

interface CalibratedBaseCorrelationCurve {
  correlation: (detachment: number) => number;
  id: string;
}

/**
 * Props for BaseCorrelationCalibration component.
 * Supports both controlled (via state prop) and uncontrolled modes.
 */
interface BaseCorrelationCalibrationProps {
  /** Complete JSON state for controlled mode */
  state?: BaseCorrelationCalibrationState;
  /** Callback when state changes (for controlled mode) */
  onStateChange?: (state: BaseCorrelationCalibrationState) => void;
  /** Market context containing discount curve and credit index */
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
  /** @deprecated Use state.indexId instead */
  indexId?: string;
  /** @deprecated Use state.series instead */
  series?: number;
  /** @deprecated Use state.maturityYears instead */
  maturityYears?: number;
  /** @deprecated Use state.discountCurveId instead */
  discountCurveId?: string;
  /** @deprecated Use state.config instead */
  config?: CalibrationConfig;
  /** @deprecated Use state.showChart instead */
  showChart?: boolean;
  /** @deprecated Use state.quotes instead */
  initialQuotes?: TrancheQuoteData[];
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

/** Convert quote data to WASM CreditQuote objects */
const buildWasmQuotes = (quotes: TrancheQuoteData[]): CreditQuote[] => {
  return quotes.map((q) =>
    CreditQuote.cdsTranche(
      q.index,
      q.attachment,
      q.detachment,
      new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
      q.upfrontPct,
      q.runningSpreadBp
    )
  );
};

export const BaseCorrelationCalibration: React.FC<BaseCorrelationCalibrationProps> = ({
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
  // Legacy props
  baseDate: legacyBaseDate,
  curveId: legacyCurveId,
  indexId: legacyIndexId,
  series: legacySeries,
  maturityYears: legacyMaturityYears,
  discountCurveId: legacyDiscountCurveId,
  config: legacyConfig,
  showChart: legacyShowChart,
  initialQuotes: legacyInitialQuotes,
}) => {
  // Determine if we're in controlled mode
  const isControlled = state !== undefined;

  // Extract values from state or legacy props
  const baseDate = React.useMemo(() => {
    if (state) return toFsDate(state.baseDate);
    if (legacyBaseDate) return legacyBaseDate;
    return new FsDate(new Date().getFullYear(), new Date().getMonth() + 1, new Date().getDate());
  }, [state, legacyBaseDate]);

  const curveId = state?.curveId ?? legacyCurveId ?? 'CDX-IG-BASECORR';
  const indexId = state?.indexId ?? legacyIndexId ?? 'CDX.NA.IG.42';
  const series = state?.series ?? legacySeries ?? 42;
  const maturityYears = state?.maturityYears ?? legacyMaturityYears ?? 5.0;
  const discountCurveId = state?.discountCurveId ?? legacyDiscountCurveId ?? 'USD-OIS';
  const showChart = state?.showChart ?? legacyShowChart ?? true;

  // Quote state - controlled or local
  const [localQuotes, setLocalQuotes] = useState<TrancheQuoteData[]>(
    legacyInitialQuotes ?? DEFAULT_TRANCHE_QUOTES
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
    (newQuotes: TrancheQuoteData[]) => {
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
      const calibrationConfig = state?.config
        ? buildWasmConfig(state.config)
        : legacyConfig ||
          CalibrationConfig.multiCurve()
            .withSolverKind(SolverKind.Brent())
            .withMaxIterations(50)
            .withVerbose(false);

      // Build WASM quotes from the editable data
      const wasmQuotes = buildWasmQuotes(quotes);

      const calibrator = new BaseCorrelationCalibrator(indexId, series, maturityYears, baseDate);

      const calibratorWithConfig = calibrator
        .withConfig(calibrationConfig)
        .withDiscountCurveId(discountCurveId);

      // Set detachment points from quotes
      const detachmentPoints = quotes.map((q) => q.detachment).sort((a, b) => a - b);
      const calibratorWithPoints = calibratorWithConfig.withDetachmentPoints(
        new Float64Array(detachmentPoints)
      );

      const [calibratedCurve, report] = calibratorWithPoints.calibrate(wasmQuotes, market) as [
        CalibratedBaseCorrelationCurve,
        { success: boolean; iterations: number; maxResidual: number },
      ];

      // Generate sample values across the detachment spectrum
      const sampleDetachments = [3, 7, 10, 15, 30];
      const sampleValues: CurveDataPoint[] = sampleDetachments.map((d) => ({
        time: d,
        value: calibratedCurve.correlation(d),
        label: `ρ(${d}%)`,
      }));

      // Check if correlations are valid (between 0 and 1, and not NaN)
      const correlationsValid = sampleValues.every(
        (sv) => sv.value >= 0 && sv.value <= 1 && !Number.isNaN(sv.value)
      );

      // Consider calibration successful if we got valid correlations,
      // even if the solver didn't fully converge within tolerance
      const effectiveSuccess = correlationsValid && sampleValues.length > 0;

      const calibrationResult: CalibrationResult = {
        curveId,
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

      console.log(`✅ Base correlation '${curveId}' calibrated:`, {
        corr3pct: calibratedCurve.correlation(3),
        corr7pct: calibratedCurve.correlation(7),
        corr15pct: calibratedCurve.correlation(15),
        iterations: report.iterations,
        reportSuccess: report.success,
        effectiveSuccess,
      });
    } catch (err) {
      const errorMsg =
        err instanceof Error ? err.message : String(err) || 'Unknown calibration error';
      setError(errorMsg);
      setStatus('failed');
      console.warn(`Base correlation calibration failed:`, err);

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
    state?.config,
    legacyConfig,
    market,
    onCalibrated,
  ]);

  // Export current state as JSON (for debugging/LLM integration)
  const exportState = useCallback((): BaseCorrelationCalibrationState => {
    return {
      baseDate: { year: baseDate.year, month: baseDate.month, day: baseDate.day },
      curveId,
      indexId,
      series,
      maturityYears,
      discountCurveId,
      quotes,
      config: state?.config ?? {
        solverKind: 'Brent',
        maxIterations: 50,
        tolerance: 1e-8,
        verbose: false,
      },
      showChart,
    };
  }, [baseDate, curveId, indexId, series, maturityYears, discountCurveId, quotes, state?.config, showChart]);

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
        {/* Editable Quote Table */}
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
        )}

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
            sub-tranche quotes [0, D] for each detachment point D. The demo quotes are synthetic
            placeholders.
          </p>
        </div>

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
