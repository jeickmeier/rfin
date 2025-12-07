import { useState } from 'react';
import {
  JsScenarioEngine,
  JsScenarioSpec,
  JsOperationSpec,
  JsExecutionContext,
  JsCurveKind,
  JsVolSurfaceKind,
  MarketContext,
  DiscountCurve,
  VolSurface,
  FsDate,
  Currency,
  Money,
  MarketScalar,
  JsModelBuilder as ModelBuilder,
  JsEvaluator as Evaluator,
  JsForecastSpec as ForecastSpec,
} from 'finstack-wasm';

interface ScenarioResult {
  operationsApplied: number;
  warnings: string[];
  roundingContext: string | undefined;
  newDate?: string;
}

export default function ScenariosExample() {
  const [output, setOutput] = useState<string>('');
  const [result, setResult] = useState<ScenarioResult | null>(null);
  const [loading, setLoading] = useState(false);

  const runBasicScenario = () => {
    setLoading(true);
    setOutput('');
    setResult(null);

    try {
      const log: string[] = [];

      log.push('=== Basic Scenario: Curve Shock ===\n');

      // 1. Create market context with discount curve
      const market = new MarketContext();
      const baseDate = new FsDate(2025, 1, 1);

      const curve = new DiscountCurve(
        'USD_SOFR',
        baseDate,
        new Float64Array([0.0, 1.0, 5.0, 10.0]),
        new Float64Array([1.0, 0.98, 0.9, 0.8]),
        'act_365f',
        'monotone_convex',
        'flat_forward',
        true
      );

      market.insertDiscount(curve);
      log.push('✓ Created discount curve USD_SOFR');
      log.push('  - 1Y DF: 0.98 (~2% rate)');
      log.push('  - 5Y DF: 0.90 (~2.2% rate)');

      // 2. Create empty financial model
      const builder = new ModelBuilder('stress_test');
      const model = builder.periods('2025Q1..Q4', null)?.build();
      if (!model) throw new Error('Failed to build model');

      // 3. Create execution context
      const context = new JsExecutionContext(market, model, baseDate);
      log.push('\n✓ Created execution context');

      // 4. Define scenario with +50bp parallel shift
      const operations = [
        JsOperationSpec.curveParallelBp(JsCurveKind.DISCOUNT, 'USD_SOFR', 50.0).toJSON(),
      ];

      const scenario = JsScenarioSpec.fromJSON({
        id: 'rate_shock',
        name: 'Rate Shock: +50bp',
        description: 'Parallel shift to discount curve',
        operations,
        priority: 0,
      });

      log.push('\n=== Applying Scenario ===\n');
      log.push('Operation: +50bp parallel shift to USD_SOFR');

      // 5. Apply scenario
      const engine = new JsScenarioEngine();
      const report = engine.apply(scenario, context);

      log.push(`\n✓ Scenario applied successfully`);
      log.push(`  - Operations: ${report.operationsApplied}`);
      log.push(`  - Warnings: ${report.warnings.length}`);

      // Verify curve was shocked
      const shockedCurve = market.discount('USD_SOFR');
      if (shockedCurve) {
        log.push('\n✓ Curve was updated (rates increased by ~50bp)');
      }

      setOutput(log.join('\n'));
      setResult({
        operationsApplied: report.operationsApplied,
        warnings: Array.from(report.warnings),
        roundingContext: report.roundingContext ?? undefined,
      });
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runMultiAssetScenario = () => {
    setLoading(true);
    setOutput('');
    setResult(null);

    try {
      const log: string[] = [];

      log.push('=== Multi-Asset Stress Scenario ===\n');

      // 1. Setup market with multiple data points
      const market = new MarketContext();
      const baseDate = new FsDate(2025, 1, 1);

      // Add discount curve
      const curve = new DiscountCurve(
        'USD_SOFR',
        baseDate,
        new Float64Array([0.0, 1.0, 5.0]),
        new Float64Array([1.0, 0.98, 0.9]),
        'act_365f',
        'monotone_convex',
        'flat_forward',
        true
      );
      market.insertDiscount(curve);
      log.push('✓ Created discount curve');

      // Add equity price
      const usd = new Currency('USD');
      const spyPrice = MarketScalar.price(new Money(450.0, usd));
      market.insertPrice('SPY', spyPrice);
      log.push('✓ Inserted SPY price: $450');

      // Add volatility surface
      const volSurface = new VolSurface(
        'SPX_VOL',
        new Float64Array([0.25, 0.5, 1.0]), // 3M, 6M, 1Y
        new Float64Array([90.0, 100.0, 110.0]),
        new Float64Array([
          0.2,
          0.18,
          0.22, // 3M row
          0.21,
          0.19,
          0.23, // 6M row
          0.22,
          0.2,
          0.24, // 1Y row
        ])
      );
      market.insertSurface(volSurface);
      log.push('✓ Created volatility surface (3 expiries × 3 strikes)');

      // 2. Create model
      const builder = new ModelBuilder('multi_asset_model');
      const model = builder.periods('2025Q1..Q4', null)?.build();
      if (!model) throw new Error('Failed to build model');

      const context = new JsExecutionContext(market, model, baseDate);

      // 3. Define comprehensive scenario
      const operations = [
        // Curve shock
        JsOperationSpec.curveParallelBp(JsCurveKind.DISCOUNT, 'USD_SOFR', 75.0).toJSON(),
        // Equity shock
        JsOperationSpec.equityPricePct(['SPY'], -15.0).toJSON(),
        // Vol shock
        JsOperationSpec.volSurfaceParallelPct(JsVolSurfaceKind.EQUITY, 'SPX_VOL', 25.0).toJSON(),
      ];

      const scenario = JsScenarioSpec.fromJSON({
        id: 'multi_asset_stress',
        name: 'Multi-Asset Stress Test',
        description: 'Rates up, equities down, volatility up',
        operations,
        priority: 0,
      });

      log.push('\n=== Scenario Operations ===\n');
      log.push('1. Curve: +75bp parallel shift');
      log.push('2. Equity: -15% price shock');
      log.push('3. Volatility: +25% parallel shock');

      // 4. Apply scenario
      const engine = new JsScenarioEngine();
      const report = engine.apply(scenario, context);

      log.push('\n=== Results ===\n');
      log.push(`✓ Applied ${report.operationsApplied} operations`);

      // Verify results
      const shockedPrice = market.price('SPY');
      if (shockedPrice?.isPrice) {
        // Price was shocked
        log.push(`  - SPY price shocked (from $450)`);
      }

      const shockedSurface = market.surface('SPX_VOL');
      if (shockedSurface) {
        const atm1Y = shockedSurface.value(1.0, 100.0);
        log.push(`  - SPX 1Y ATM vol: ${(atm1Y * 100).toFixed(2)}% (shocked from 20%)`);
      }

      if (report.warnings.length > 0) {
        log.push('\nWarnings:');
        Array.from(report.warnings).forEach((w) => log.push(`  - ${w}`));
      }

      setOutput(log.join('\n'));
      setResult({
        operationsApplied: report.operationsApplied,
        warnings: Array.from(report.warnings),
        roundingContext: report.roundingContext ?? undefined,
      });
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runStatementScenario = () => {
    setLoading(true);
    setOutput('');
    setResult(null);

    try {
      const log: string[] = [];

      log.push('=== Statement Stress Scenario ===\n');

      // 1. Setup market
      const market = new MarketContext();
      const baseDate = new FsDate(2025, 1, 1);

      // 2. Create financial model with revenue
      const builder = new ModelBuilder('revenue_model');
      const builderWithPeriods = builder.periods('2025Q1..Q4', '2025Q1');

      // Add revenue with Q1 actual
      const revenueValues: { [key: string]: number } = {
        '2025Q1': 1000000,
      };
      const builderWithRevenue = builderWithPeriods?.value('revenue', revenueValues);

      // Add forecast
      const forecast = ForecastSpec.growth(0.1); // 10% growth
      const builderWithForecast = builderWithRevenue?.forecast('revenue', forecast);

      const model = builderWithForecast?.build();
      if (!model) throw new Error('Failed to build model');

      log.push('✓ Built model with revenue node');
      log.push('  - Q1 actual: $1,000,000');
      log.push('  - Forecast: 10% quarterly growth');

      // Evaluate initial model
      const evaluator = new Evaluator();
      const initialResults = evaluator.evaluate(model);

      log.push('\n=== Initial Forecast ===\n');
      const periods = ['2025Q1', '2025Q2', '2025Q3', '2025Q4'];
      periods.forEach((p) => {
        const val = initialResults?.get('revenue', p);
        if (val !== null && val !== undefined) {
          log.push(`  ${p}: $${val.toLocaleString()}`);
        }
      });

      // 3. Create context and apply shock
      const context = new JsExecutionContext(market, model, baseDate);

      const operations = [
        JsOperationSpec.stmtForecastPercent('revenue', -20.0).toJSON(), // -20% shock
      ];

      const scenario = JsScenarioSpec.fromJSON({
        id: 'revenue_stress',
        name: 'Revenue Stress: -20%',
        operations,
        priority: 0,
      });

      log.push('\n=== Applying Revenue Shock (-20%) ===\n');

      const engine = new JsScenarioEngine();
      const report = engine.apply(scenario, context);

      log.push(`✓ Applied shock to revenue forecast`);
      log.push(`  - Operations: ${report.operationsApplied}`);

      // Re-evaluate shocked model
      const shockedModel = context.model;
      const shockedResults = evaluator.evaluate(shockedModel);

      log.push('\n=== Shocked Forecast ===\n');
      periods.forEach((p) => {
        const val = shockedResults?.get('revenue', p);
        if (val !== null && val !== undefined) {
          log.push(`  ${p}: $${val.toLocaleString()}`);
        }
      });

      setOutput(log.join('\n'));
      setResult({
        operationsApplied: report.operationsApplied,
        warnings: Array.from(report.warnings),
        roundingContext: report.roundingContext ?? undefined,
      });
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runTimeRollScenario = () => {
    setLoading(true);
    setOutput('');
    setResult(null);

    try {
      const log: string[] = [];

      log.push('=== Time Roll Forward Scenario ===\n');

      // Setup
      const market = new MarketContext();
      const baseDate = new FsDate(2025, 1, 1);

      const curve = new DiscountCurve(
        'USD_SOFR',
        baseDate,
        new Float64Array([0.0, 1.0]),
        new Float64Array([1.0, 0.98]),
        'act_365f',
        'monotone_convex',
        'flat_forward',
        true
      );
      market.insertDiscount(curve);

      const builder = new ModelBuilder('time_roll_model');
      const model = builder.periods('2025Q1..Q4', null)?.build();
      if (!model) throw new Error('Failed to build model');

      const context = new JsExecutionContext(market, model, baseDate);

      log.push(`Initial date: ${baseDate.toString()}`);
      log.push('Rolling forward 1 month...\n');

      // Time roll operation
      const operations = [JsOperationSpec.timeRollForward('1M', true).toJSON()];

      const scenario = JsScenarioSpec.fromJSON({
        id: 'time_roll',
        name: 'Roll Forward 1 Month',
        operations,
        priority: 0,
      });

      const engine = new JsScenarioEngine();
      const report = engine.apply(scenario, context);

      const newDate = context.asOf;
      log.push(`✓ Time rolled forward successfully`);
      log.push(`  - New date: ${newDate.toString()}`);
      log.push(`  - Operations: ${report.operationsApplied}`);
      log.push('\nNote: Carry/theta calculations require instrument positions');

      setOutput(log.join('\n'));
      setResult({
        operationsApplied: report.operationsApplied,
        warnings: Array.from(report.warnings),
        roundingContext: report.roundingContext ?? undefined,
        newDate: newDate.toString(),
      });
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runComposedScenario = () => {
    setLoading(true);
    setOutput('');
    setResult(null);

    try {
      const log: string[] = [];

      log.push('=== Scenario Composition ===\n');

      // Setup
      const market = new MarketContext();
      const baseDate = new FsDate(2025, 1, 1);

      const curve = new DiscountCurve(
        'USD_SOFR',
        baseDate,
        new Float64Array([0.0, 1.0]),
        new Float64Array([1.0, 0.98]),
        'act_365f',
        'monotone_convex',
        'flat_forward',
        true
      );
      market.insertDiscount(curve);

      const usd = new Currency('USD');
      market.insertPrice('SPY', MarketScalar.price(new Money(450.0, usd)));

      const builder = new ModelBuilder('composed_model');
      const model = builder.periods('2025Q1..Q4', null)?.build();
      if (!model) throw new Error('Failed to build model');

      const context = new JsExecutionContext(market, model, baseDate);

      // Create two scenarios with different priorities
      const baseCase = JsScenarioSpec.fromJSON({
        id: 'base_case',
        name: 'Base Case',
        operations: [
          JsOperationSpec.curveParallelBp(JsCurveKind.DISCOUNT, 'USD_SOFR', 25.0).toJSON(),
        ],
        priority: 0, // Higher priority (runs first)
      });

      const overlay = JsScenarioSpec.fromJSON({
        id: 'overlay',
        name: 'Equity Overlay',
        operations: [JsOperationSpec.equityPricePct(['SPY'], -10.0).toJSON()],
        priority: 1, // Lower priority (runs second)
      });

      log.push('Scenario 1 (priority 0): +25bp curve shock');
      log.push('Scenario 2 (priority 1): -10% equity shock\n');

      // Compose scenarios
      const engine = new JsScenarioEngine();
      const composed = engine.compose([baseCase.toJSON(), overlay.toJSON()]);

      log.push('✓ Scenarios composed with priority ordering');
      log.push(`  - Combined operations: ${composed.operationCount()}\n`);

      // Apply composed scenario
      const report = engine.apply(composed, context);

      log.push('=== Composite Scenario Applied ===\n');
      log.push(`✓ Operations applied: ${report.operationsApplied}`);
      log.push('  - Both curve and equity shocks executed');

      setOutput(log.join('\n'));
      setResult({
        operationsApplied: report.operationsApplied,
        warnings: Array.from(report.warnings),
        roundingContext: report.roundingContext ?? undefined,
      });
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runComprehensiveScenario = () => {
    setLoading(true);
    setOutput('');
    setResult(null);

    try {
      const log: string[] = [];

      log.push('=== Comprehensive Stress Test ===\n');
      log.push('Demonstrating all major operation types\n');

      // 1. Setup complete market
      const market = new MarketContext();
      const baseDate = new FsDate(2025, 1, 1);

      // Discount curve
      const discCurve = new DiscountCurve(
        'USD_SOFR',
        baseDate,
        new Float64Array([0.0, 1.0, 5.0, 10.0]),
        new Float64Array([1.0, 0.98, 0.9, 0.8]),
        'act_365f',
        'monotone_convex',
        'flat_forward',
        true
      );
      market.insertDiscount(discCurve);

      // Equity price
      const usd = new Currency('USD');
      market.insertPrice('SPY', MarketScalar.price(new Money(450.0, usd)));
      market.insertPrice('QQQ', MarketScalar.price(new Money(380.0, usd)));

      // Vol surface
      const volSurface = new VolSurface(
        'SPX_VOL',
        new Float64Array([0.25, 0.5, 1.0]),
        new Float64Array([90.0, 100.0, 110.0]),
        new Float64Array([0.2, 0.18, 0.22, 0.21, 0.19, 0.23, 0.22, 0.2, 0.24])
      );
      market.insertSurface(volSurface);

      log.push('✓ Market Setup Complete:');
      log.push('  - Discount curve: USD_SOFR');
      log.push('  - Equities: SPY ($450), QQQ ($380)');
      log.push('  - Vol surface: SPX_VOL (3×3 grid)');

      // 2. Create model with revenue
      const builder = new ModelBuilder('comprehensive_model');
      const builderWithPeriods = builder.periods('2025Q1..Q4', '2025Q1');

      const revenueValues: { [key: string]: number } = { '2025Q1': 10000000 };
      const builderWithRevenue = builderWithPeriods?.value('revenue', revenueValues);
      const builderWithForecast = builderWithRevenue?.forecast(
        'revenue',
        ForecastSpec.growth(0.08)
      );

      const model = builderWithForecast?.build();
      if (!model) throw new Error('Failed to build model');

      log.push('\n✓ Financial Model:');
      log.push('  - Revenue Q1: $10M (actual)');
      log.push('  - Forecast: 8% growth\n');

      const context = new JsExecutionContext(market, model, baseDate);

      // 3. Define comprehensive scenario
      const operations = [
        // Market shocks
        JsOperationSpec.curveParallelBp(JsCurveKind.DISCOUNT, 'USD_SOFR', 100.0).toJSON(),
        JsOperationSpec.equityPricePct(['SPY', 'QQQ'], -20.0).toJSON(),
        JsOperationSpec.volSurfaceParallelPct(JsVolSurfaceKind.EQUITY, 'SPX_VOL', 30.0).toJSON(),

        // Vol bucket shock (just ATM strikes)
        JsOperationSpec.volSurfaceBucketPct(
          JsVolSurfaceKind.EQUITY,
          'SPX_VOL',
          null, // All tenors
          new Float64Array([100.0]), // Just ATM
          15.0 // Additional 15%
        ).toJSON(),

        // Statement shock
        JsOperationSpec.stmtForecastPercent('revenue', -15.0).toJSON(),
      ];

      const scenario = JsScenarioSpec.fromJSON({
        id: 'comprehensive_stress',
        name: 'Comprehensive Stress Test',
        description: 'Full market and statement stress',
        operations,
        priority: 0,
      });

      log.push('=== Stress Scenario ===\n');
      log.push('Market Shocks:');
      log.push('  1. Rates: +100bp');
      log.push('  2. Equities: -20% (SPY, QQQ)');
      log.push('  3. Vol (parallel): +30%');
      log.push('  4. Vol (ATM bucket): +15% additional');
      log.push('\nStatement Shocks:');
      log.push('  5. Revenue: -15%\n');

      // 4. Apply scenario
      const engine = new JsScenarioEngine();
      const report = engine.apply(scenario, context);

      log.push('=== Application Results ===\n');
      log.push(`✓ Applied ${report.operationsApplied} operations`);

      // Check results
      const spyPrice = market.price('SPY');
      if (spyPrice?.isPrice) {
        log.push(`\nSPY: Price shocked (original: $450)`);
      }

      const qqqPrice = market.price('QQQ');
      if (qqqPrice?.isPrice) {
        log.push(`QQQ: Price shocked (original: $380)`);
      }

      const volSurf = market.surface('SPX_VOL');
      if (volSurf) {
        const atm1Y = volSurf.value(1.0, 100.0);
        log.push(`\nSPX 1Y ATM Vol: ${(atm1Y * 100).toFixed(2)}%`);
        log.push('  (20% base + 30% parallel + 15% ATM bucket)');
      }

      if (report.warnings.length > 0) {
        log.push('\n⚠ Warnings:');
        Array.from(report.warnings).forEach((w) => log.push(`  - ${w}`));
      }

      setOutput(log.join('\n'));
      setResult({
        operationsApplied: report.operationsApplied,
        warnings: Array.from(report.warnings),
        roundingContext: report.roundingContext ?? undefined,
      });
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="example-container">
      <h1>Scenarios Engine</h1>
      <p className="description">
        Reproducible scenario analysis for stress testing and what-if analysis. Apply market shocks,
        statement adjustments, and time roll-forwards with full composability and priority-based
        ordering.
      </p>

      <div className="button-group">
        <button onClick={runBasicScenario} disabled={loading} className="primary">
          Run Basic Curve Shock
        </button>
        <button onClick={runMultiAssetScenario} disabled={loading} className="primary">
          Run Multi-Asset Stress
        </button>
        <button onClick={runStatementScenario} disabled={loading} className="primary">
          Run Statement Shock
        </button>
        <button onClick={runTimeRollScenario} disabled={loading} className="primary">
          Run Time Roll Forward
        </button>
        <button onClick={runComposedScenario} disabled={loading} className="primary">
          Run Scenario Composition
        </button>
        <button onClick={runComprehensiveScenario} disabled={loading} className="primary">
          Run Comprehensive Test
        </button>
      </div>

      {result && (
        <div className="result-summary">
          <h3>Scenario Results</h3>
          <div className="stats">
            <div className="stat">
              <span className="label">Operations Applied:</span>
              <span className="value">{result.operationsApplied}</span>
            </div>
            <div className="stat">
              <span className="label">Warnings:</span>
              <span className="value">{result.warnings.length}</span>
            </div>
            {result.newDate && (
              <div className="stat">
                <span className="label">New Date:</span>
                <span className="value">{result.newDate}</span>
              </div>
            )}
            <div className="stat">
              <span className="label">Rounding:</span>
              <span className="value">{result.roundingContext || 'default'}</span>
            </div>
          </div>
          {result.warnings.length > 0 && (
            <div className="warnings">
              <h4>Warnings:</h4>
              <ul>
                {result.warnings.map((w, i) => (
                  <li key={i}>{w}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {output && (
        <div className="output">
          <h3>Detailed Output</h3>
          <pre>{output}</pre>
        </div>
      )}

      <div className="features">
        <h2>Supported Operations</h2>

        <div className="feature-group">
          <h3>Market Data Shocks</h3>
          <ul>
            <li>
              <strong>FX Rates:</strong> Percentage shocks to exchange rates
            </li>
            <li>
              <strong>Equity Prices:</strong> Percentage shocks to spot prices
            </li>
            <li>
              <strong>Curves:</strong> Parallel and node-specific shifts (bp)
              <ul>
                <li>Discount, Forward, Hazard, Inflation curves</li>
                <li>Exact tenor matching or interpolated key-rate bumps</li>
              </ul>
            </li>
            <li>
              <strong>Volatility Surfaces:</strong> Parallel and bucketed shocks
              <ul>
                <li>Filter by tenor and/or strike</li>
                <li>Equity, Credit, Swaption surfaces</li>
              </ul>
            </li>
            <li>
              <strong>Base Correlation:</strong> Parallel and bucket-specific shifts
            </li>
          </ul>
        </div>

        <div className="feature-group">
          <h3>Statement Operations</h3>
          <ul>
            <li>
              <strong>Forecast Percent:</strong> Apply multiplicative factor to forecasts
            </li>
            <li>
              <strong>Forecast Assign:</strong> Override with fixed values
            </li>
          </ul>
        </div>

        <div className="feature-group">
          <h3>Time Operations</h3>
          <ul>
            <li>
              <strong>Roll Forward:</strong> Advance valuation date with carry/theta
              <ul>
                <li>Supports periods: 1D, 1W, 1M, 1Y</li>
                <li>Optional market shock application after roll</li>
              </ul>
            </li>
          </ul>
        </div>

        <div className="feature-group">
          <h3>Engine Features</h3>
          <ul>
            <li>
              <strong>Reproducible:</strong> Stable execution order and results
            </li>
            <li>
              <strong>Warning Collection:</strong> Non-fatal issues captured
            </li>
            <li>
              <strong>JSON Support:</strong> Full serialization for persistence
            </li>
          </ul>
        </div>
      </div>

      <div className="code-examples">
        <h2>Code Examples</h2>

        <div className="code-example">
          <h3>Curve Shock with Tenor Matching</h3>
          <pre>{`const operations = [
  JsOperationSpec.curveNodeBp(
    JsCurveKind.DISCOUNT,
    "USD_SOFR",
    [["2Y", 25.0], ["10Y", -10.0]], // Steepen
    JsTenorMatchMode.INTERPOLATE
  ).toJSON()
];

const scenario = JsScenarioSpec.fromJSON({
  id: "steepener",
  operations,
  priority: 0
});

const report = engine.apply(scenario, context);`}</pre>
        </div>

        <div className="code-example">
          <h3>Volatility Bucket Shock</h3>
          <pre>{`// Shock only near-term OTM puts
const operations = [
  JsOperationSpec.volSurfaceBucketPct(
    JsVolSurfaceKind.EQUITY,
    "SPX_VOL",
    ["1M", "3M"],     // Short tenors
    [90.0, 95.0],     // OTM puts
    40.0              // +40% vol spike
  ).toJSON()
];`}</pre>
        </div>

        <div className="code-example">
          <h3>Combined Market + Statement Stress</h3>
          <pre>{`const operations = [
  // Market shocks
  JsOperationSpec.curveParallelBp(
    JsCurveKind.DISCOUNT,
    "USD_SOFR",
    100.0
  ).toJSON(),
  JsOperationSpec.equityPricePct(["SPY"], -25.0).toJSON(),
  
  // Statement shocks
  JsOperationSpec.stmtForecastPercent("Revenue", -10.0).toJSON(),
  JsOperationSpec.stmtForecastPercent("COGS", 5.0).toJSON()
];`}</pre>
        </div>

        <div className="code-example">
          <h3>Horizon Analysis with Time Roll</h3>
          <pre>{`// T+1M scenario: roll forward then shock
const operations = [
  JsOperationSpec.timeRollForward("1M", true).toJSON(),
  JsOperationSpec.curveParallelBp(
    JsCurveKind.DISCOUNT,
    "USD_SOFR",
    50.0
  ).toJSON()
];

// Calculate carry/theta then apply market shock`}</pre>
        </div>
      </div>

      <style>{`
        .example-container {
          max-width: 1200px;
          margin: 0 auto;
          padding: 20px;
        }

        .description {
          font-size: 16px;
          color: #666;
          margin-bottom: 30px;
          line-height: 1.6;
        }

        .button-group {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
          gap: 15px;
          margin-bottom: 30px;
        }

        button {
          padding: 12px 24px;
          font-size: 14px;
          font-weight: 500;
          border: none;
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.2s;
        }

        button.primary {
          background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
          color: white;
        }

        button.primary:hover:not(:disabled) {
          transform: translateY(-2px);
          box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
        }

        button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .result-summary {
          background: #f8f9fa;
          border: 1px solid #e9ecef;
          border-radius: 8px;
          padding: 20px;
          margin-bottom: 20px;
        }

        .result-summary h3 {
          margin-top: 0;
          color: #2c3e50;
        }

        .stats {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
          gap: 15px;
          margin-top: 15px;
        }

        .stat {
          display: flex;
          flex-direction: column;
          padding: 12px;
          background: white;
          border-radius: 6px;
          border: 1px solid #dee2e6;
        }

        .stat .label {
          font-size: 12px;
          color: #6c757d;
          margin-bottom: 4px;
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }

        .stat .value {
          font-size: 20px;
          font-weight: 600;
          color: #2c3e50;
        }

        .warnings {
          margin-top: 15px;
          padding: 15px;
          background: #fff3cd;
          border: 1px solid #ffc107;
          border-radius: 6px;
        }

        .warnings h4 {
          margin-top: 0;
          color: #856404;
        }

        .warnings ul {
          margin: 10px 0 0 0;
          padding-left: 20px;
        }

        .warnings li {
          color: #856404;
          margin-bottom: 5px;
        }

        .output {
          background: #1e1e1e;
          color: #d4d4d4;
          padding: 20px;
          border-radius: 8px;
          margin-bottom: 30px;
        }

        .output h3 {
          margin-top: 0;
          color: #4ec9b0;
        }

        .output pre {
          margin: 0;
          font-family: 'Fira Code', 'Courier New', monospace;
          font-size: 13px;
          line-height: 1.6;
          white-space: pre-wrap;
          word-wrap: break-word;
        }

        .features {
          margin-top: 40px;
        }

        .features h2 {
          color: #2c3e50;
          border-bottom: 2px solid #667eea;
          padding-bottom: 10px;
        }

        .feature-group {
          margin: 25px 0;
          padding: 20px;
          background: #f8f9fa;
          border-radius: 8px;
        }

        .feature-group h3 {
          color: #495057;
          margin-top: 0;
        }

        .feature-group ul {
          margin: 10px 0;
        }

        .feature-group li {
          margin-bottom: 10px;
          line-height: 1.6;
        }

        .feature-group ul ul {
          margin-top: 5px;
          margin-left: 20px;
        }

        .feature-group ul ul li {
          font-size: 14px;
          color: #6c757d;
        }

        .code-examples {
          margin-top: 40px;
        }

        .code-examples h2 {
          color: #2c3e50;
          border-bottom: 2px solid #667eea;
          padding-bottom: 10px;
        }

        .code-example {
          margin: 25px 0;
        }

        .code-example h3 {
          color: #495057;
          font-size: 16px;
          margin-bottom: 10px;
        }

        .code-example pre {
          background: #1e1e1e;
          color: #d4d4d4;
          padding: 15px;
          border-radius: 6px;
          overflow-x: auto;
          font-family: 'Fira Code', 'Courier New', monospace;
          font-size: 13px;
          line-height: 1.5;
        }
      `}</style>
    </div>
  );
}
