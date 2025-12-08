import React, { useEffect, useState, useMemo, useCallback } from 'react';
import {
  BaseCorrelationCurve,
  CalibrationConfig,
  CreditIndexData,
  DiscountCurve,
  DiscountCurveCalibrator,
  FsDate,
  Frequency,
  HazardCurve,
  MarketContext,
  RatesQuote,
  SolverKind,
} from 'finstack-wasm';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import {
  DiscountCurveCalibration,
  ForwardCurveCalibration,
  HazardCurveCalibration,
  InflationCurveCalibration,
  VolSurfaceCalibration,
  BaseCorrelationCalibration,
  type CalibrationResult,
} from './calibration';

/** Create default calibration configuration - must be called after WASM init */
const createDefaultConfig = () =>
  CalibrationConfig.multiCurve()
    .withSolverKind(SolverKind.Brent())
    .withMaxIterations(40)
    .withVerbose(false);

/** Quote factory for initial market setup */
const createDiscountQuotesForMarket = () => [
  RatesQuote.deposit(new FsDate(2024, 2, 1), 0.045, 'act_360'),
  RatesQuote.deposit(new FsDate(2024, 4, 2), 0.0465, 'act_360'),
  RatesQuote.swap(
    new FsDate(2025, 1, 2),
    0.0475,
    Frequency.annual(),
    Frequency.quarterly(),
    '30_360',
    'act_360',
    'USD-SOFR'
  ),
  RatesQuote.swap(
    new FsDate(2027, 1, 2),
    0.0485,
    Frequency.annual(),
    Frequency.quarterly(),
    '30_360',
    'act_360',
    'USD-SOFR'
  ),
];

export const CalibrationExample: React.FC = () => {
  const [results, setResults] = useState<Map<string, CalibrationResult>>(new Map());
  const [market, setMarket] = useState<MarketContext | null>(null);
  const [isReady, setIsReady] = useState(false);
  const [config, setConfig] = useState<CalibrationConfig | null>(null);

  const baseDate = useMemo(() => new FsDate(2024, 1, 2), []);

  // Initialize the base market with a discount curve and credit index for other calibrators to use
  useEffect(() => {
    const initializeMarket = async () => {
      try {
        const defaultConfig = createDefaultConfig();
        setConfig(defaultConfig);

        // Create fresh quotes for initial market calibration
        const quotesForInit = createDiscountQuotesForMarket();

        const calibrator = new DiscountCurveCalibrator('USD-OIS', baseDate, 'USD');
        const calibratorWithConfig = calibrator.withConfig(defaultConfig);
        const [curve] = calibratorWithConfig.calibrate(quotesForInit, null) as [
          DiscountCurve,
          unknown,
        ];

        const marketCtx = new MarketContext();
        marketCtx.insertDiscount(curve);

        // Create credit index data for base correlation calibration
        // Create index hazard curve (representing the underlying CDX spread curve)
        const indexHazardCurve = new HazardCurve(
          'CDX.NA.IG.42',
          baseDate,
          new Float64Array([1.0, 3.0, 5.0, 7.0, 10.0]),
          new Float64Array([0.01, 0.012, 0.015, 0.018, 0.02]), // hazard rates
          0.4, // recovery rate
          'act_360',
          null, // issuer
          null, // seniority
          'USD',
          new Float64Array([1.0, 3.0, 5.0, 7.0, 10.0]), // par tenors
          new Float64Array([50, 60, 75, 90, 110]) // par spreads in bp
        );

        // Create a placeholder base correlation curve (will be replaced by calibration)
        const placeholderBaseCorr = new BaseCorrelationCurve(
          'CDX.NA.IG.42_5Y',
          new Float64Array([3.0, 7.0, 10.0, 15.0, 30.0]),
          new Float64Array([0.2, 0.35, 0.45, 0.55, 0.7]) // initial placeholder correlations
        );

        // Create and insert credit index data
        const creditIndexData = new CreditIndexData(
          125, // num constituents
          0.4, // recovery rate
          indexHazardCurve,
          placeholderBaseCorr,
          null, // issuer_ids
          null // issuer_curves
        );
        marketCtx.insertCreditIndex('CDX.NA.IG.42', creditIndexData);

        setMarket(marketCtx);
        setIsReady(true);

        console.log('Base market initialized with discount curve and credit index');
      } catch (err) {
        console.warn('Failed to initialize base market:', err);
        // Still allow showing the discount calibrator
        setIsReady(true);
        setConfig(createDefaultConfig());
      }
    };

    initializeMarket();
  }, [baseDate]);

  const handleCalibrated = useCallback((result: CalibrationResult) => {
    setResults((prev) => {
      const next = new Map(prev);
      next.set(result.curveId, result);
      return next;
    });
  }, []);

  // Summary statistics
  const summaryStats = useMemo(() => {
    const all = Array.from(results.values());
    const successful = all.filter((r) => r.success).length;
    const failed = all.filter((r) => !r.success).length;
    const totalIterations = all.reduce((sum, r) => sum + r.iterations, 0);
    return { total: all.length, successful, failed, totalIterations };
  }, [results]);

  if (!isReady || !config) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="text-muted-foreground">Initializing market context...</div>
      </div>
    );
  }

  return (
    <section className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-primary mb-2">Curve Calibration Suite</h2>
        <p className="text-muted-foreground">
          Demonstrates all calibration types: discount curves, forward curves, credit hazard curves,
          inflation curves, and volatility surfaces. Each calibrator fits curves to market prices
          using numerical optimization with configurable solvers.
        </p>
      </div>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-lg">Calibration Summary</CardTitle>
          <CardDescription>Base Date: {baseDate.toString()} - Currency: USD</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-4 gap-4">
            <div className="text-center p-3 bg-muted/50 rounded-lg">
              <div className="text-2xl font-bold">{summaryStats.total}</div>
              <div className="text-xs text-muted-foreground uppercase tracking-wide">
                Total Curves
              </div>
            </div>
            <div className="text-center p-3 bg-success/10 rounded-lg">
              <div className="text-2xl font-bold text-success">{summaryStats.successful}</div>
              <div className="text-xs text-muted-foreground uppercase tracking-wide">Converged</div>
            </div>
            <div className="text-center p-3 bg-destructive/10 rounded-lg">
              <div className="text-2xl font-bold text-destructive">{summaryStats.failed}</div>
              <div className="text-xs text-muted-foreground uppercase tracking-wide">Failed</div>
            </div>
            <div className="text-center p-3 bg-muted/50 rounded-lg">
              <div className="text-2xl font-bold">{summaryStats.totalIterations}</div>
              <div className="text-xs text-muted-foreground uppercase tracking-wide">
                Total Iterations
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Tabs defaultValue="discount" className="w-full">
        <TabsList className="grid w-full grid-cols-6">
          <TabsTrigger value="discount">Discount</TabsTrigger>
          <TabsTrigger value="forward">Forward</TabsTrigger>
          <TabsTrigger value="hazard">Credit</TabsTrigger>
          <TabsTrigger value="inflation">Inflation</TabsTrigger>
          <TabsTrigger value="vol">Vol Surface</TabsTrigger>
          <TabsTrigger value="correlation">Correlation</TabsTrigger>
        </TabsList>

        <TabsContent value="discount" className="mt-4">
          <DiscountCurveCalibration
            baseDate={baseDate}
            curveId="USD-OIS"
            currency="USD"
            config={config}
            onCalibrated={handleCalibrated}
          />
        </TabsContent>

        <TabsContent value="forward" className="mt-4">
          <ForwardCurveCalibration
            baseDate={baseDate}
            curveId="USD-SOFR-3M"
            currency="USD"
            tenor={0.25}
            discountCurveId="USD-OIS"
            config={config}
            market={market}
            onCalibrated={handleCalibrated}
          />
        </TabsContent>

        <TabsContent value="hazard" className="mt-4">
          <HazardCurveCalibration
            baseDate={baseDate}
            curveId="ACME-Senior"
            currency="USD"
            entity="ACME"
            seniority="senior"
            recoveryRate={0.4}
            discountCurveId="USD-OIS"
            config={config}
            market={market}
            onCalibrated={handleCalibrated}
          />
        </TabsContent>

        <TabsContent value="inflation" className="mt-4">
          <InflationCurveCalibration
            baseDate={baseDate}
            curveId="US-CPI-U"
            currency="USD"
            indexName="US-CPI-U"
            baseCpi={300}
            discountCurveId="USD-OIS"
            config={config}
            market={market}
            onCalibrated={handleCalibrated}
          />
        </TabsContent>

        <TabsContent value="vol" className="mt-4">
          <VolSurfaceCalibration
            baseDate={baseDate}
            curveId="AAPL-VOL"
            currency="USD"
            underlying="AAPL"
            spotPrice={100}
            expiries={[0.5, 1]}
            strikes={[90, 100, 110]}
            discountCurveId="USD-OIS"
            config={config}
            market={market}
            onCalibrated={handleCalibrated}
          />
        </TabsContent>

        <TabsContent value="correlation" className="mt-4">
          <BaseCorrelationCalibration
            baseDate={baseDate}
            curveId="CDX-IG-BASECORR"
            indexId="CDX.NA.IG.42"
            series={42}
            maturityYears={5.0}
            discountCurveId="USD-OIS"
            config={config}
            market={market}
            onCalibrated={handleCalibrated}
          />
        </TabsContent>
      </Tabs>

      <Card className="bg-primary/5 border-primary/20">
        <CardHeader>
          <CardTitle className="text-lg">Calibration API</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-muted-foreground text-sm">
            The calibration module provides specialized calibrators for different market data types:
          </p>
          <ul className="space-y-2 text-sm">
            <li className="flex gap-2">
              <code className="bg-muted px-1.5 py-0.5 rounded text-xs">
                DiscountCurveCalibrator
              </code>
              <span className="text-muted-foreground">
                Bootstrap OIS/Treasury curves from deposits and swaps
              </span>
            </li>
            <li className="flex gap-2">
              <code className="bg-muted px-1.5 py-0.5 rounded text-xs">ForwardCurveCalibrator</code>
              <span className="text-muted-foreground">
                Calibrate LIBOR/SOFR forward curves from FRAs and swaps
              </span>
            </li>
            <li className="flex gap-2">
              <code className="bg-muted px-1.5 py-0.5 rounded text-xs">HazardCurveCalibrator</code>
              <span className="text-muted-foreground">
                Calibrate credit default probability curves from CDS spreads
              </span>
            </li>
            <li className="flex gap-2">
              <code className="bg-muted px-1.5 py-0.5 rounded text-xs">
                InflationCurveCalibrator
              </code>
              <span className="text-muted-foreground">
                Calibrate CPI projection curves from inflation swap quotes
              </span>
            </li>
            <li className="flex gap-2">
              <code className="bg-muted px-1.5 py-0.5 rounded text-xs">VolSurfaceCalibrator</code>
              <span className="text-muted-foreground">
                Calibrate implied volatility surfaces from option quotes
              </span>
            </li>
            <li className="flex gap-2">
              <code className="bg-muted px-1.5 py-0.5 rounded text-xs">
                BaseCorrelationCalibrator
              </code>
              <span className="text-muted-foreground">
                Calibrate base correlation curves from CDO tranche quotes
              </span>
            </li>
          </ul>
          <div className="bg-muted/50 border-l-2 border-primary p-3 rounded-r text-sm">
            <strong>Note:</strong> Use the editable quote tables to modify market inputs and
            re-calibrate curves. Production calibration requires 5-10+ quotes for reliable
            convergence.
          </div>
        </CardContent>
      </Card>
    </section>
  );
};

export default CalibrationExample;
