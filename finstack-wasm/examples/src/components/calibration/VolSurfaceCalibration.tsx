import React, { useState, useCallback, useMemo } from 'react';
import { MarketContext, MarketScalar, Money, VolQuote, executeCalibration } from 'finstack-wasm';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { CalibrationResultPanel, StatusBadge } from './CurveChart';
import { VolQuoteEditor, DEFAULT_VOL_QUOTES, type VolQuoteData } from './QuoteEditor';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type { VolSurfaceCalibrationState } from './state-types';
import { buildWasmConfig, isoDate, toFsDate, useEffectiveQuotes } from './shared';

interface CalibratedVolSurface {
  value: (t: number, k: number) => number;
  id: string;
}

const MIN_VOL_TOLERANCE = 0.01;

interface VolSurfaceCalibrationProps {
  /** Complete JSON state */
  state: VolSurfaceCalibrationState;
  /** Callback when state changes */
  onStateChange?: (state: VolSurfaceCalibrationState) => void;
  /** Market context containing discount curve */
  market: MarketContext | null;
  /** Callback when calibration completes */
  onCalibrated?: (result: CalibrationResult) => void;
  /** Additional CSS class name */
  className?: string;
}

/** Convert quote data to WASM VolQuote objects */
const buildWasmQuotes = (quotes: VolQuoteData[]): VolQuote[] => {
  return quotes.map((q) =>
    VolQuote.fromJSON({
      type: 'option_vol',
      underlying: q.underlying,
      expiry: `${q.expiryYear}-${String(q.expiryMonth).padStart(2, '0')}-${String(q.expiryDay).padStart(2, '0')}`,
      strike: q.strike,
      vol: q.vol,
      option_type: q.optionType.toLowerCase(),
      convention: 'USD-EQUITY',
    })
  );
};

export const VolSurfaceCalibration: React.FC<VolSurfaceCalibrationProps> = ({
  state,
  onStateChange,
  market,
  onCalibrated,
  className,
}) => {
  const {
    curveId,
    currency,
    underlying,
    spotPrice,
    expiries,
    strikes,
    discountCurveId,
    showChart,
    config,
    tolerance,
  } = state;
  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  const [quotes, setLocalQuotes] = useEffectiveQuotes(state.quotes, DEFAULT_VOL_QUOTES);

  const handleQuotesChange = (newQuotes: VolQuoteData[]) => {
    if (onStateChange) {
      onStateChange({ ...state, quotes: newQuotes });
    } else {
      setLocalQuotes(newQuotes);
    }
  };

  const [status, setStatus] = useState<CalibrationStatus>('idle');
  const [result, setResult] = useState<CalibrationResult | null>(null);
  const [surface, setSurface] = useState<CalibratedVolSurface | null>(null);
  const [error, setError] = useState<string | null>(null);

  const effectiveTolerance = Math.max(tolerance, MIN_VOL_TOLERANCE);

  const runCalibration = useCallback(() => {
    if (quotes.length === 0) {
      setError('No vol quotes provided');
      return;
    }
    if (!market) {
      setError('Market context with discount curve required');
      return;
    }

    setStatus('running');
    setError(null);

    try {
      market.insertPrice(underlying, MarketScalar.price(Money.fromCode(spotPrice, currency)));
      market.insertPrice(`${underlying}-DIVYIELD`, MarketScalar.unitless(0.015));

      const calibrationConfig = buildWasmConfig(config, effectiveTolerance);
      const wasmQuotes = buildWasmQuotes(quotes);

      const quoteSet = wasmQuotes.map((q) => q.toMarketQuote().toJSON());
      const envelope = {
        schema: 'finstack.calibration/2',
        initial_market: market.toJson(),
        plan: {
          id: `vol_surface:${curveId}`,
          quote_sets: {
            vol: quoteSet,
          },
          steps: [
            {
              id: 'vol',
              quote_set: 'vol',
              kind: 'vol_surface',
              surface_id: curveId,
              base_date: isoDate(baseDate),
              underlying_ticker: underlying,
              model: 'SABR',
              discount_curve_id: discountCurveId,
              target_expiries: expiries,
              target_strikes: strikes,
              spot_override: spotPrice,
              dividend_yield_override: 0.015,
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

      const calibratedSurface = marketCtx.getSurface(curveId) as unknown as CalibratedVolSurface;

      const atmStrike =
        strikes.find((s) => Math.abs(s - 100) < 5) || strikes[Math.floor(strikes.length / 2)];
      const sampleValues: CurveDataPoint[] = expiries.map((t) => ({
        time: t,
        value: calibratedSurface.value(t, atmStrike),
        label: `Vol(${t}Y, ${atmStrike})`,
      }));

      const calibrationResult: CalibrationResult = {
        curveId,
        curveType: 'Vol Surface',
        success: report.success,
        iterations: report.iterations,
        maxResidual: report.maxResidual,
        sampleValues,
      };

      setSurface(calibratedSurface);
      setResult(calibrationResult);
      setStatus(report.success ? 'success' : 'failed');
      onCalibrated?.(calibrationResult);
    } catch (err) {
      const errorMsg = (err as Error).message;
      setError(errorMsg);
      setStatus('failed');

      const failedResult: CalibrationResult = {
        curveId,
        curveType: 'Vol Surface',
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
    underlying,
    spotPrice,
    expiries,
    strikes,
    discountCurveId,
    config,
    market,
    onCalibrated,
    effectiveTolerance,
  ]);

  const quotesSummary = useMemo(() => `${quotes.length} vol quotes`, [quotes]);

  const exportState = useCallback((): VolSurfaceCalibrationState => state, [state]);

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              Volatility Surface
              <StatusBadge status={status} />
            </CardTitle>
            <CardDescription>
              {underlying} - Spot: {spotPrice} {currency} - {expiries.length}x{strikes.length} grid
              - {quotesSummary}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <VolQuoteEditor
          quotes={quotes}
          onChange={handleQuotesChange}
          onCalibrate={runCalibration}
          disabled={status === 'running' || !market}
          underlying={underlying}
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
            title: 'ATM Vol Term Structure',
            xLabel: 'Expiry',
            yLabel: 'Implied Vol',
            color: 'hsl(var(--chart-5))',
            yFormatter: (v) => `${(v * 100).toFixed(1)}%`,
          }}
        />

        {surface && result?.success && (
          <>
            <div className="text-sm font-medium text-muted-foreground mb-2">
              Vol Surface Grid (Expiry x Strike)
            </div>
            <div className="overflow-x-auto">
              <table className="text-xs w-full">
                <thead>
                  <tr>
                    <th className="text-left p-1 text-muted-foreground">Expiry</th>
                    {strikes.map((k) => (
                      <th key={`strike-${k}`} className="text-right p-1 text-muted-foreground">
                        {k}%
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {expiries.map((t) => (
                    <tr key={`expiry-${t}`}>
                      <td className="p-1 font-medium">{t}Y</td>
                      {strikes.map((k) => (
                        <td key={`cell-${t}-${k}`} className="text-right p-1 font-mono">
                          {(surface.value(t, k) * 100).toFixed(1)}%
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
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
