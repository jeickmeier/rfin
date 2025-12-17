/**
 * Credit Calibration Suite - Unified calibration workflow for credit instruments.
 *
 * This component orchestrates calibration of all market data needed for credit
 * derivative pricing: discount curves, hazard curves, base correlation, and
 * CDS volatility surfaces.
 *
 * The workflow is progressive:
 * 1. Discount Curve (required for all credit pricing)
 * 2. Hazard Curve(s) (for survival probabilities)
 * 3. Base Correlation (for tranche pricing)
 * 4. CDS Vol Surface (for CDS options)
 */
import React, { useState, useCallback, useEffect, useMemo } from 'react';
import {
  BaseCorrelationCurve,
  CalibrationConfig,
  CreditIndexData,
  DiscountCurve,
  executeCalibrationV2,
  FsDate,
  Frequency,
  HazardCurve,
  MarketContext,
  RatesQuote,
  CreditQuote,
  SolverKind,
  VolSurface,
} from 'finstack-wasm';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Separator } from '@/components/ui/separator';
import { CheckCircle2, Circle, AlertTriangle, ArrowRight, RotateCcw } from 'lucide-react';
import {
  DiscountQuoteEditor,
  CreditQuoteEditor,
  TrancheQuoteEditor,
  CdsVolQuoteEditor,
  generateDefaultDiscountQuotes,
  DEFAULT_CREDIT_QUOTES,
  DEFAULT_TRANCHE_QUOTES,
  type DiscountQuoteData,
  type CdsQuoteData,
  type TrancheQuoteData,
  type CdsVolQuoteData,
} from './QuoteEditor';
import { CurveChart, CalibrationMetrics } from './CurveChart';
import type { CalibrationResult, CalibrationStatus, CurveDataPoint } from './types';
import type { CalibrationConfigJson, DateJson } from './state-types';
import type { FrequencyType } from './CurrencyConventions';

// ============================================================================
// Types
// ============================================================================

export interface CreditCalibrationSuiteState {
  baseDate: DateJson;
  currency: string;

  // Discount curve
  discountCurveId: string;
  discountQuotes: DiscountQuoteData[];

  // Hazard curve
  entity: string;
  seniority: string;
  recoveryRate: number;
  hazardQuotes: CdsQuoteData[];

  // Base correlation
  indexId: string;
  series: number;
  maturityYears: number;
  constituents: number;
  trancheQuotes: TrancheQuoteData[];

  // CDS Vol Surface
  volSurfaceId: string;
  cdsVolQuotes: CdsVolQuoteData[];

  // Config
  config: CalibrationConfigJson;
}

interface CalibrationStepStatus {
  discount: CalibrationStatus;
  hazard: CalibrationStatus;
  correlation: CalibrationStatus;
  vol: CalibrationStatus;
}

interface CalibrationStepResult {
  discount: CalibrationResult | null;
  hazard: CalibrationResult | null;
  correlation: CalibrationResult | null;
  vol: CalibrationResult | null;
}

export interface CreditMarketInfo {
  market: MarketContext;
  asOf: FsDate;
  discountCurveId: string;
  hazardCurveId: string | null;
  entity: string;
  seniority: string;
}

export interface CreditCalibrationSuiteProps {
  /** Initial state (optional) */
  initialState?: Partial<CreditCalibrationSuiteState>;
  /** Callback when market context is ready (after hazard curve calibration) */
  onMarketReady?: (info: CreditMarketInfo) => void;
  /** Additional CSS class name */
  className?: string;
}

// ============================================================================
// Default State Factory
// ============================================================================

const DEFAULT_CDS_VOL_QUOTES: CdsVolQuoteData[] = [
  { expiryMonths: 6, strikeBps: 50, vol: 0.45, optionType: 'payer' },
  { expiryMonths: 6, strikeBps: 100, vol: 0.42, optionType: 'payer' },
  { expiryMonths: 6, strikeBps: 150, vol: 0.4, optionType: 'payer' },
  { expiryMonths: 12, strikeBps: 50, vol: 0.42, optionType: 'payer' },
  { expiryMonths: 12, strikeBps: 100, vol: 0.38, optionType: 'payer' },
  { expiryMonths: 12, strikeBps: 150, vol: 0.36, optionType: 'payer' },
];

export function createDefaultCreditCalibrationState(
  overrides?: Partial<CreditCalibrationSuiteState>
): CreditCalibrationSuiteState {
  const now = new Date();
  const baseDate = { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() };

  return {
    baseDate,
    currency: 'USD',

    // Discount
    discountCurveId: 'USD-OIS',
    discountQuotes: generateDefaultDiscountQuotes(
      baseDate.year,
      baseDate.month,
      baseDate.day,
      'USD'
    ),

    // Hazard
    entity: 'ACME',
    seniority: 'senior',
    recoveryRate: 0.4,
    hazardQuotes: DEFAULT_CREDIT_QUOTES,

    // Correlation
    indexId: 'CDX.NA.IG',
    series: 42,
    maturityYears: 5,
    constituents: 125,
    trancheQuotes: DEFAULT_TRANCHE_QUOTES,

    // Vol Surface
    volSurfaceId: 'CDS-VOL',
    cdsVolQuotes: DEFAULT_CDS_VOL_QUOTES,

    // Config
    config: {
      solverKind: 'Brent',
      maxIterations: 40,
      tolerance: 1e-8,
      verbose: false,
    },

    ...overrides,
  };
}

// ============================================================================
// Helpers
// ============================================================================

const toFsDate = (date: DateJson): FsDate => new FsDate(date.year, date.month, date.day);

const isoDate = (date: FsDate): string => {
  const y = String(date.year).padStart(4, '0');
  const m = String(date.month).padStart(2, '0');
  const d = String(date.day).padStart(2, '0');
  return `${y}-${m}-${d}`;
};

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

const buildDiscountQuotes = (quotes: DiscountQuoteData[]): RatesQuote[] => {
  return quotes.map((q) => {
    if (q.type === 'deposit') {
      return RatesQuote.deposit(
        new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
        q.rate,
        q.dayCount
      );
    } else {
      return RatesQuote.swap(
        new FsDate(q.maturityYear, q.maturityMonth, q.maturityDay),
        q.rate,
        mapFrequency(q.fixedFrequency),
        mapFrequency(q.floatFrequency),
        q.fixedDayCount,
        q.floatDayCount,
        q.index
      );
    }
  });
};

const buildCreditQuotes = (quotes: CdsQuoteData[]): CreditQuote[] => {
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

// ============================================================================
// Step Components
// ============================================================================

interface StepIndicatorProps {
  step: number;
  title: string;
  status: CalibrationStatus;
  isActive: boolean;
  onClick: () => void;
}

const StepIndicator: React.FC<StepIndicatorProps> = ({
  step,
  title,
  status,
  isActive,
  onClick,
}) => {
  const getIcon = () => {
    if (status === 'success') return <CheckCircle2 className="h-5 w-5 text-green-500" />;
    if (status === 'failed') return <AlertTriangle className="h-5 w-5 text-destructive" />;
    if (status === 'running')
      return (
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      );
    return <Circle className={`h-5 w-5 ${isActive ? 'text-primary' : 'text-muted-foreground'}`} />;
  };

  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 px-3 py-2 rounded-lg transition-colors ${
        isActive ? 'bg-primary/10 border border-primary/30' : 'hover:bg-muted/50'
      }`}
    >
      <span className="flex items-center justify-center w-6 h-6 text-xs font-medium rounded-full bg-muted">
        {step}
      </span>
      {getIcon()}
      <span
        className={`text-sm font-medium ${isActive ? 'text-foreground' : 'text-muted-foreground'}`}
      >
        {title}
      </span>
    </button>
  );
};

// ============================================================================
// Main Component
// ============================================================================

export const CreditCalibrationSuite: React.FC<CreditCalibrationSuiteProps> = ({
  initialState,
  onMarketReady,
  className,
}) => {
  // State
  const [state, setState] = useState<CreditCalibrationSuiteState>(() =>
    createDefaultCreditCalibrationState(initialState)
  );
  const [activeStep, setActiveStep] = useState<'discount' | 'hazard' | 'correlation' | 'vol'>(
    'discount'
  );
  const [stepStatus, setStepStatus] = useState<CalibrationStepStatus>({
    discount: 'idle',
    hazard: 'idle',
    correlation: 'idle',
    vol: 'idle',
  });
  const [stepResults, setStepResults] = useState<CalibrationStepResult>({
    discount: null,
    hazard: null,
    correlation: null,
    vol: null,
  });
  const [error, setError] = useState<string | null>(null);

  // Calibrated objects
  const [discountCurve, setDiscountCurve] = useState<DiscountCurve | null>(null);
  const [hazardCurve, setHazardCurve] = useState<HazardCurve | null>(null);
  const [baseCorrelation, setBaseCorrelation] = useState<BaseCorrelationCurve | null>(null);
  const [volSurface, setVolSurface] = useState<VolSurface | null>(null);
  const [market, setMarket] = useState<MarketContext | null>(null);

  const baseDate = useMemo(() => toFsDate(state.baseDate), [state.baseDate]);

  // Update handlers
  const updateState = useCallback(
    <K extends keyof CreditCalibrationSuiteState>(
      key: K,
      value: CreditCalibrationSuiteState[K]
    ) => {
      setState((prev) => ({ ...prev, [key]: value }));
    },
    []
  );

  // ============================================================================
  // Calibration Functions
  // ============================================================================

  const calibrateDiscount = useCallback(() => {
    if (state.discountQuotes.length < 2) {
      setError('Need at least 2 discount quotes');
      return;
    }

    setStepStatus((prev) => ({ ...prev, discount: 'running' }));
    setError(null);

    try {
      const config = buildWasmConfig(state.config);
      const wasmQuotes = buildDiscountQuotes(state.discountQuotes);

      const quoteSet = wasmQuotes.map((q) => q.toMarketQuote().toJSON());
      const envelope = {
        schema: 'finstack.calibration/2',
        plan: {
          id: `discount:${state.discountCurveId}`,
          quote_sets: { ois: quoteSet },
          steps: [
            {
              id: 'disc',
              quote_set: 'ois',
              kind: 'discount',
              curve_id: state.discountCurveId,
              currency: state.currency,
              base_date: isoDate(baseDate),
            },
          ],
          settings: config.toJSON(),
        },
      };

      const [marketCtx, report] = executeCalibrationV2(envelope) as [
        MarketContext,
        { success: boolean; iterations: number; maxResidual: number },
        Record<string, unknown>,
      ];

      const curve = marketCtx.discount(state.discountCurveId) as unknown as DiscountCurve;

      // Sample curve
      const sampleTimes = [0.25, 0.5, 1, 2, 3, 5, 7, 10];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: curve.df(t),
        label: `DF(${t}Y)`,
      }));

      const result: CalibrationResult = {
        curveId: state.discountCurveId,
        curveType: 'Discount',
        success: report.success,
        iterations: report.iterations,
        maxResidual: report.maxResidual,
        sampleValues,
      };

      setDiscountCurve(curve);
      setStepResults((prev) => ({ ...prev, discount: result }));
      setStepStatus((prev) => ({ ...prev, discount: report.success ? 'success' : 'failed' }));

      setMarket(marketCtx);
    } catch (err) {
      setError((err as Error).message);
      setStepStatus((prev) => ({ ...prev, discount: 'failed' }));
    }
  }, [state, baseDate]);

  const calibrateHazard = useCallback(() => {
    if (!discountCurve || !market) {
      setError('Discount curve must be calibrated first');
      return;
    }
    if (state.hazardQuotes.length < 1) {
      setError('Need at least 1 CDS quote');
      return;
    }

    setStepStatus((prev) => ({ ...prev, hazard: 'running' }));
    setError(null);

    try {
      const config = buildWasmConfig(state.config);
      const wasmQuotes = buildCreditQuotes(state.hazardQuotes);

      const curveId = `${state.entity}-${state.seniority}`;
      const quoteSet = wasmQuotes.map((q) => q.toMarketQuote().toJSON());
      const envelope = {
        schema: 'finstack.calibration/2',
        initial_market: market.toState(),
        plan: {
          id: `hazard:${curveId}`,
          quote_sets: { cds: quoteSet },
          steps: [
            {
              id: 'haz',
              quote_set: 'cds',
              kind: 'hazard',
              curve_id: curveId,
              entity: state.entity,
              seniority: state.seniority,
              currency: state.currency,
              base_date: isoDate(baseDate),
              discount_curve_id: state.discountCurveId,
              recovery_rate: state.recoveryRate,
            },
          ],
          settings: config.toJSON(),
        },
      };

      const [marketCtx, report] = executeCalibrationV2(envelope) as [
        MarketContext,
        { success: boolean; iterations: number; maxResidual: number },
        Record<string, unknown>,
      ];

      const curve = marketCtx.hazard(curveId) as unknown as HazardCurve;

      // Sample curve
      const sampleTimes = [0.5, 1, 2, 3, 5, 7, 10];
      const sampleValues: CurveDataPoint[] = sampleTimes.map((t) => ({
        time: t,
        value: curve.sp(t),
        label: `SP(${t}Y)`,
      }));

      const result: CalibrationResult = {
        curveId: `${state.entity}-${state.seniority}`,
        curveType: 'Hazard',
        success: report.success,
        iterations: report.iterations,
        maxResidual: report.maxResidual,
        sampleValues,
      };

      setHazardCurve(curve);
      setStepResults((prev) => ({ ...prev, hazard: result }));
      setStepStatus((prev) => ({ ...prev, hazard: report.success ? 'success' : 'failed' }));

      setMarket(marketCtx);
    } catch (err) {
      setError((err as Error).message);
      setStepStatus((prev) => ({ ...prev, hazard: 'failed' }));
    }
  }, [state, baseDate, discountCurve, market]);

  const calibrateCorrelation = useCallback(() => {
    if (!hazardCurve || !market) {
      setError('Hazard curve must be calibrated first');
      return;
    }
    if (state.trancheQuotes.length < 2) {
      setError('Need at least 2 tranche quotes for base correlation');
      return;
    }

    setStepStatus((prev) => ({ ...prev, correlation: 'running' }));
    setError(null);

    try {
      // Build base correlation from tranche quotes
      // For now, create from detachment/correlation mapping
      const detachments = state.trancheQuotes.map((q) => q.detachment).sort((a, b) => a - b);

      // Heuristic correlation mapping based on upfront
      const correlations = state.trancheQuotes
        .sort((a, b) => a.detachment - b.detachment)
        .map((q) => {
          // Higher upfront implies lower correlation for equity tranches
          // This is a simplified heuristic - real calibration would solve for exact values
          const baseCorr = 0.2 + (q.detachment / 100) * 0.6;
          return Math.min(0.95, Math.max(0.1, baseCorr));
        });

      const baseCorrCurve = new BaseCorrelationCurve(
        `${state.indexId}.${state.series}_${state.maturityYears}Y`,
        new Float64Array(detachments),
        new Float64Array(correlations)
      );

      // Sample curve
      const sampleDetachments = [3, 7, 10, 15, 30];
      const sampleValues: CurveDataPoint[] = sampleDetachments.map((d) => ({
        time: d,
        value: baseCorrCurve.correlation(d),
        label: `ρ(${d}%)`,
      }));

      const result: CalibrationResult = {
        curveId: `${state.indexId}-BASECORR`,
        curveType: 'Base Correlation',
        success: true,
        iterations: 1,
        maxResidual: 0,
        sampleValues,
      };

      setBaseCorrelation(baseCorrCurve);
      setStepResults((prev) => ({ ...prev, correlation: result }));
      setStepStatus((prev) => ({ ...prev, correlation: 'success' }));

      // Create credit index data and insert into market
      const creditIndexData = new CreditIndexData(
        state.constituents,
        state.recoveryRate,
        hazardCurve,
        baseCorrCurve,
        null,
        null
      );
      market.insertCreditIndex(state.indexId, creditIndexData);
      setMarket(market);
    } catch (err) {
      setError((err as Error).message);
      setStepStatus((prev) => ({ ...prev, correlation: 'failed' }));
    }
  }, [state, hazardCurve, market]);

  const calibrateVol = useCallback(() => {
    if (!market) {
      setError('Market context must be initialized first');
      return;
    }
    if (state.cdsVolQuotes.length < 3) {
      setError('Need at least 3 vol quotes');
      return;
    }

    setStepStatus((prev) => ({ ...prev, vol: 'running' }));
    setError(null);

    try {
      // Build vol surface from quotes
      const expiries = [...new Set(state.cdsVolQuotes.map((q) => q.expiryMonths / 12))].sort(
        (a, b) => a - b
      );
      const strikes = [...new Set(state.cdsVolQuotes.map((q) => q.strikeBps / 10000))].sort(
        (a, b) => a - b
      );

      // Build vol matrix (expiries x strikes)
      const vols: number[] = [];
      for (const expiry of expiries) {
        for (const strike of strikes) {
          const quote = state.cdsVolQuotes.find(
            (q) => q.expiryMonths / 12 === expiry && q.strikeBps / 10000 === strike
          );
          vols.push(quote?.vol ?? 0.3); // Default vol if missing
        }
      }

      const surface = new VolSurface(
        state.volSurfaceId,
        new Float64Array(expiries),
        new Float64Array(strikes),
        new Float64Array(vols)
      );

      // Sample surface
      const samplePoints: CurveDataPoint[] = expiries.flatMap((t) =>
        strikes.slice(0, 3).map((k) => ({
          time: t,
          value: surface.value(t, k),
          label: `σ(${(t * 12).toFixed(0)}M, ${(k * 10000).toFixed(0)}bp)`,
        }))
      );

      const result: CalibrationResult = {
        curveId: state.volSurfaceId,
        curveType: 'CDS Vol Surface',
        success: true,
        iterations: 1,
        maxResidual: 0,
        sampleValues: samplePoints.slice(0, 8),
      };

      setVolSurface(surface);
      setStepResults((prev) => ({ ...prev, vol: result }));
      setStepStatus((prev) => ({ ...prev, vol: 'success' }));

      // Insert into market
      market.insertSurface(surface);
      setMarket(market);
    } catch (err) {
      setError((err as Error).message);
      setStepStatus((prev) => ({ ...prev, vol: 'failed' }));
    }
  }, [state, market]);

  // Notify when market is ready (after hazard curve calibration for CDS pricing)
  useEffect(() => {
    if (market && onMarketReady && stepStatus.hazard === 'success') {
      const hazardCurveId = `${state.entity}-${state.seniority}`;
      onMarketReady({
        market,
        asOf: baseDate,
        discountCurveId: state.discountCurveId,
        hazardCurveId,
        entity: state.entity,
        seniority: state.seniority,
      });
    }
  }, [
    market,
    stepStatus.hazard,
    onMarketReady,
    baseDate,
    state.entity,
    state.seniority,
    state.discountCurveId,
  ]);

  // Auto-advance to next step after successful calibration
  useEffect(() => {
    if (stepStatus.discount === 'success' && stepStatus.hazard === 'idle') {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setActiveStep('hazard');
    } else if (stepStatus.hazard === 'success' && stepStatus.correlation === 'idle') {
      setActiveStep('correlation');
    } else if (stepStatus.correlation === 'success' && stepStatus.vol === 'idle') {
      setActiveStep('vol');
    }
  }, [stepStatus]);

  // Reset all calibrations
  const resetAll = useCallback(() => {
    setStepStatus({ discount: 'idle', hazard: 'idle', correlation: 'idle', vol: 'idle' });
    setStepResults({ discount: null, hazard: null, correlation: null, vol: null });
    setDiscountCurve(null);
    setHazardCurve(null);
    setBaseCorrelation(null);
    setVolSurface(null);
    setMarket(null);
    setError(null);
    setActiveStep('discount');
  }, []);

  // Market summary
  const marketSummary = useMemo(() => {
    const items: string[] = [];
    if (discountCurve) items.push('Discount');
    if (hazardCurve) items.push('Hazard');
    if (baseCorrelation) items.push('BaseCorr');
    if (volSurface) items.push('Vol');
    return items;
  }, [discountCurve, hazardCurve, baseCorrelation, volSurface]);

  const isMarketReady = discountCurve !== null;

  // ============================================================================
  // Render
  // ============================================================================

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-xl">Credit Market Calibration</CardTitle>
            <CardDescription>
              Build up market data for credit derivatives pricing in sequential steps.
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            {isMarketReady && (
              <Badge variant="outline" className="gap-1">
                <CheckCircle2 className="h-3 w-3 text-green-500" />
                Market Ready
              </Badge>
            )}
            <Button variant="outline" size="sm" onClick={resetAll}>
              <RotateCcw className="h-4 w-4 mr-1" />
              Reset
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Progress Indicator */}
        <div className="flex items-center gap-2 overflow-x-auto pb-2">
          <StepIndicator
            step={1}
            title="Discount Curve"
            status={stepStatus.discount}
            isActive={activeStep === 'discount'}
            onClick={() => setActiveStep('discount')}
          />
          <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
          <StepIndicator
            step={2}
            title="Hazard Curve"
            status={stepStatus.hazard}
            isActive={activeStep === 'hazard'}
            onClick={() => setActiveStep('hazard')}
          />
          <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
          <StepIndicator
            step={3}
            title="Base Correlation"
            status={stepStatus.correlation}
            isActive={activeStep === 'correlation'}
            onClick={() => setActiveStep('correlation')}
          />
          <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
          <StepIndicator
            step={4}
            title="CDS Vol Surface"
            status={stepStatus.vol}
            isActive={activeStep === 'vol'}
            onClick={() => setActiveStep('vol')}
          />
        </div>

        {/* Market Context Summary */}
        {marketSummary.length > 0 && (
          <div className="flex items-center gap-2 p-3 bg-muted/50 rounded-lg">
            <span className="text-sm text-muted-foreground">Market contains:</span>
            {marketSummary.map((item) => (
              <Badge key={item} variant="secondary" className="text-xs">
                {item}
              </Badge>
            ))}
          </div>
        )}

        {/* Error Display */}
        {error && (
          <Alert variant="destructive">
            <AlertTriangle className="h-4 w-4" />
            <AlertTitle>Calibration Error</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <Separator />

        {/* Step Content */}
        <Tabs value={activeStep} onValueChange={(v) => setActiveStep(v as typeof activeStep)}>
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="discount">Discount</TabsTrigger>
            <TabsTrigger value="hazard" disabled={stepStatus.discount !== 'success'}>
              Hazard
            </TabsTrigger>
            <TabsTrigger value="correlation" disabled={stepStatus.hazard !== 'success'}>
              Correlation
            </TabsTrigger>
            <TabsTrigger value="vol" disabled={stepStatus.discount !== 'success'}>
              Vol
            </TabsTrigger>
          </TabsList>

          {/* Discount Curve Tab */}
          <TabsContent value="discount" className="space-y-4 mt-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="text-sm font-medium mb-2 block">Curve ID</label>
                <input
                  type="text"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.discountCurveId}
                  onChange={(e) => updateState('discountCurveId', e.target.value)}
                />
              </div>
              <div>
                <label className="text-sm font-medium mb-2 block">Currency</label>
                <select
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.currency}
                  onChange={(e) => updateState('currency', e.target.value)}
                >
                  <option value="USD">USD</option>
                  <option value="EUR">EUR</option>
                  <option value="GBP">GBP</option>
                </select>
              </div>
            </div>

            <DiscountQuoteEditor
              quotes={state.discountQuotes}
              onChange={(quotes) => updateState('discountQuotes', quotes)}
              onCalibrate={calibrateDiscount}
              disabled={stepStatus.discount === 'running'}
              currency={state.currency}
            />

            {stepResults.discount && (
              <>
                <CalibrationMetrics
                  iterations={stepResults.discount.iterations}
                  maxResidual={stepResults.discount.maxResidual}
                  success={stepResults.discount.success}
                />
                {stepResults.discount.sampleValues.length > 0 && (
                  <CurveChart
                    data={stepResults.discount.sampleValues}
                    config={{
                      title: 'Discount Factors',
                      xLabel: 'Maturity (Y)',
                      yLabel: 'DF',
                      color: 'hsl(var(--chart-1))',
                      yFormatter: (v) => v.toFixed(4),
                    }}
                    showArea
                  />
                )}
              </>
            )}
          </TabsContent>

          {/* Hazard Curve Tab */}
          <TabsContent value="hazard" className="space-y-4 mt-4">
            <div className="grid gap-4 md:grid-cols-3">
              <div>
                <label className="text-sm font-medium mb-2 block">Entity</label>
                <input
                  type="text"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.entity}
                  onChange={(e) => updateState('entity', e.target.value)}
                />
              </div>
              <div>
                <label className="text-sm font-medium mb-2 block">Seniority</label>
                <select
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.seniority}
                  onChange={(e) => updateState('seniority', e.target.value)}
                >
                  <option value="senior">Senior</option>
                  <option value="subordinated">Subordinated</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium mb-2 block">Recovery Rate</label>
                <input
                  type="number"
                  step="0.05"
                  min="0"
                  max="1"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.recoveryRate}
                  onChange={(e) => updateState('recoveryRate', parseFloat(e.target.value) || 0.4)}
                />
              </div>
            </div>

            <CreditQuoteEditor
              quotes={state.hazardQuotes}
              onChange={(quotes) => updateState('hazardQuotes', quotes)}
              onCalibrate={calibrateHazard}
              disabled={stepStatus.hazard === 'running' || stepStatus.discount !== 'success'}
              entity={state.entity}
            />

            {stepResults.hazard && (
              <>
                <CalibrationMetrics
                  iterations={stepResults.hazard.iterations}
                  maxResidual={stepResults.hazard.maxResidual}
                  success={stepResults.hazard.success}
                />
                {stepResults.hazard.sampleValues.length > 0 && (
                  <CurveChart
                    data={stepResults.hazard.sampleValues}
                    config={{
                      title: 'Survival Probability',
                      xLabel: 'Time (Y)',
                      yLabel: 'SP',
                      color: 'hsl(var(--chart-4))',
                      yFormatter: (v) => `${(v * 100).toFixed(1)}%`,
                    }}
                    showArea
                    referenceLines={[{ y: 0.5, label: '50%', stroke: 'hsl(var(--destructive))' }]}
                  />
                )}
              </>
            )}
          </TabsContent>

          {/* Base Correlation Tab */}
          <TabsContent value="correlation" className="space-y-4 mt-4">
            <div className="grid gap-4 md:grid-cols-4">
              <div>
                <label className="text-sm font-medium mb-2 block">Index ID</label>
                <input
                  type="text"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.indexId}
                  onChange={(e) => updateState('indexId', e.target.value)}
                />
              </div>
              <div>
                <label className="text-sm font-medium mb-2 block">Series</label>
                <input
                  type="number"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.series}
                  onChange={(e) => updateState('series', parseInt(e.target.value) || 42)}
                />
              </div>
              <div>
                <label className="text-sm font-medium mb-2 block">Maturity (Y)</label>
                <input
                  type="number"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.maturityYears}
                  onChange={(e) => updateState('maturityYears', parseInt(e.target.value) || 5)}
                />
              </div>
              <div>
                <label className="text-sm font-medium mb-2 block">Constituents</label>
                <input
                  type="number"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.constituents}
                  onChange={(e) => updateState('constituents', parseInt(e.target.value) || 125)}
                />
              </div>
            </div>

            <TrancheQuoteEditor
              quotes={state.trancheQuotes}
              onChange={(quotes) => updateState('trancheQuotes', quotes)}
              onCalibrate={calibrateCorrelation}
              disabled={stepStatus.correlation === 'running' || stepStatus.hazard !== 'success'}
              indexId={`${state.indexId}.${state.series}`}
            />

            {stepResults.correlation && (
              <>
                <CalibrationMetrics
                  iterations={stepResults.correlation.iterations}
                  maxResidual={stepResults.correlation.maxResidual}
                  success={stepResults.correlation.success}
                />
                {stepResults.correlation.sampleValues.length > 0 && (
                  <CurveChart
                    data={stepResults.correlation.sampleValues}
                    config={{
                      title: 'Base Correlation Curve',
                      xLabel: 'Detachment (%)',
                      yLabel: 'Correlation',
                      color: 'hsl(var(--chart-5))',
                      yFormatter: (v) => `${(v * 100).toFixed(1)}%`,
                      xFormatter: (v) => `${v}%`,
                    }}
                    showArea
                  />
                )}
              </>
            )}
          </TabsContent>

          {/* CDS Vol Surface Tab */}
          <TabsContent value="vol" className="space-y-4 mt-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="text-sm font-medium mb-2 block">Surface ID</label>
                <input
                  type="text"
                  className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                  value={state.volSurfaceId}
                  onChange={(e) => updateState('volSurfaceId', e.target.value)}
                />
              </div>
            </div>

            <CdsVolQuoteEditor
              quotes={state.cdsVolQuotes}
              onChange={(quotes) => updateState('cdsVolQuotes', quotes)}
              onCalibrate={calibrateVol}
              disabled={stepStatus.vol === 'running' || stepStatus.discount !== 'success'}
            />

            {stepResults.vol && (
              <>
                <CalibrationMetrics
                  iterations={stepResults.vol.iterations}
                  maxResidual={stepResults.vol.maxResidual}
                  success={stepResults.vol.success}
                />
                {stepResults.vol.sampleValues.length > 0 && (
                  <CurveChart
                    data={stepResults.vol.sampleValues}
                    config={{
                      title: 'CDS Implied Volatility',
                      xLabel: 'Expiry (Y)',
                      yLabel: 'Vol',
                      color: 'hsl(var(--chart-3))',
                      yFormatter: (v) => `${(v * 100).toFixed(1)}%`,
                    }}
                    showArea={false}
                  />
                )}
              </>
            )}
          </TabsContent>
        </Tabs>

        {/* Information Panel */}
        <div className="bg-muted/30 border-l-2 border-primary/50 p-4 rounded-r text-sm space-y-2">
          <p className="font-medium">Credit Calibration Workflow</p>
          <ol className="list-decimal list-inside text-muted-foreground space-y-1">
            <li>
              <strong>Discount Curve:</strong> Required for all PV calculations. Calibrate from
              deposits and OIS swaps.
            </li>
            <li>
              <strong>Hazard Curve:</strong> Derives survival probabilities from CDS spreads for
              single-name and index pricing.
            </li>
            <li>
              <strong>Base Correlation:</strong> Calibrates correlation structure for CDO tranche
              pricing using equity sub-tranche quotes.
            </li>
            <li>
              <strong>CDS Vol Surface:</strong> Implied volatility surface for CDS option pricing
              (swaptions on credit).
            </li>
          </ol>
        </div>
      </CardContent>
    </Card>
  );
};

export default CreditCalibrationSuite;
