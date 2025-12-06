import { useState } from 'react';
import {
  JsModelBuilder as ModelBuilder,
  JsEvaluator as Evaluator,
  JsForecastSpec as ForecastSpec,
  JsRegistry as Registry,
} from 'finstack-wasm';

interface ResultDisplay {
  nodeId: string;
  periods: { [key: string]: number };
}

export default function StatementsModeling() {
  const [output, setOutput] = useState<string>('');
  const [results, setResults] = useState<ResultDisplay[]>([]);
  const [loading, setLoading] = useState(false);

  const runBasicModel = () => {
    setLoading(true);
    setOutput('');
    setResults([]);

    try {
      const log: string[] = [];

      // Create a simple P&L model
      log.push('=== Building Basic P&L Model ===\n');

      const builder = new ModelBuilder('Acme Corp P&L');

      // Define periods
      const builderWithPeriods = builder.periods('2025Q1..Q4', '2025Q1');
      if (!builderWithPeriods) {
        throw new Error('Failed to set periods');
      }
      log.push('✓ Defined periods: 2025Q1-Q4 (actuals through Q1)');

      // Add revenue (actual in Q1)
      const revenueValues: { [key: string]: number } = {
        '2025Q1': 1000000,
      };
      const builderWithRevenue = builderWithPeriods.value('revenue', revenueValues);
      if (!builderWithRevenue) {
        throw new Error('Failed to add revenue');
      }
      log.push('✓ Added revenue with Q1 actual: $1,000,000');

      // Add forecast for revenue (10% growth)
      const revenueForecast = ForecastSpec.growth(0.1);
      const builderWithRevenueForecast = builderWithRevenue.forecast('revenue', revenueForecast);
      if (!builderWithRevenueForecast) {
        throw new Error('Failed to add revenue forecast');
      }
      log.push('✓ Added revenue forecast: 10% growth');

      // Add COGS calculation
      const builderWithCogs = builderWithRevenueForecast.compute('cogs', 'revenue * 0.6');
      if (!builderWithCogs) {
        throw new Error('Failed to add COGS');
      }
      log.push('✓ Added COGS formula: revenue * 0.6');

      // Add gross profit calculation
      const builderWithGrossProfit = builderWithCogs.compute('gross_profit', 'revenue - cogs');
      if (!builderWithGrossProfit) {
        throw new Error('Failed to add gross_profit');
      }
      log.push('✓ Added gross_profit formula: revenue - cogs');

      // Add operating expenses
      const opexValues: { [key: string]: number } = {
        '2025Q1': 250000,
      };
      const builderWithOpex = builderWithGrossProfit.value('opex', opexValues);
      if (!builderWithOpex) {
        throw new Error('Failed to add opex');
      }
      log.push('✓ Added operating expenses: $250,000');

      // Add forecast for opex (5% growth)
      const opexForecast = ForecastSpec.growth(0.05);
      const builderWithOpexForecast = builderWithOpex.forecast('opex', opexForecast);
      if (!builderWithOpexForecast) {
        throw new Error('Failed to add opex forecast');
      }
      log.push('✓ Added opex forecast: 5% growth');

      // Add EBITDA calculation
      const builderWithEbitda = builderWithOpexForecast.compute('ebitda', 'gross_profit - opex');
      if (!builderWithEbitda) {
        throw new Error('Failed to add ebitda');
      }
      log.push('✓ Added EBITDA formula: gross_profit - opex');

      // Build the model
      const model = builderWithEbitda.build();
      if (!model) {
        throw new Error('Failed to build model');
      }
      log.push(`\n✓ Model built successfully!`);
      log.push(`  - ID: ${model.id}`);
      log.push(`  - Periods: ${model.periodCount()}`);
      log.push(`  - Nodes: ${model.nodeCount()}\n`);

      // Evaluate the model
      log.push('=== Evaluating Model ===\n');
      const evaluator = new Evaluator();
      const evalResults = evaluator.evaluate(model);
      if (!evalResults) {
        throw new Error('Failed to evaluate model');
      }

      const meta = evalResults.meta;
      log.push(`✓ Evaluation complete`);
      log.push(`  - Nodes evaluated: ${meta.numNodes}`);
      log.push(`  - Periods evaluated: ${meta.numPeriods}`);
      if (meta.evalTimeMs) {
        log.push(`  - Evaluation time: ${meta.evalTimeMs}ms`);
      }

      // Extract results
      log.push('\n=== Results (2025Q1) ===\n');
      const nodeIds = ['revenue', 'cogs', 'gross_profit', 'opex', 'ebitda'];
      const displayResults: ResultDisplay[] = [];

      for (const nodeId of nodeIds) {
        const periods: { [key: string]: number } = {};
        const q1Value = evalResults.get(nodeId, '2025Q1');

        if (q1Value !== null && q1Value !== undefined) {
          periods['2025Q1'] = q1Value;
          log.push(`${nodeId.padEnd(15)}: $${q1Value.toLocaleString()}`);
        }

        displayResults.push({ nodeId, periods });
      }

      setOutput(log.join('\n'));
      setResults(displayResults);
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runForecastModel = () => {
    setLoading(true);
    setOutput('');
    setResults([]);

    try {
      const log: string[] = [];

      log.push('=== Building Model with Forecasts ===\n');

      const builder = new ModelBuilder('Forecast Demo');
      const builderWithPeriods = builder.periods('2025Q1..Q4', '2025Q1');
      if (!builderWithPeriods) {
        throw new Error('Failed to set periods');
      }
      log.push('✓ Defined periods: 2025Q1-Q4');

      // Add revenue with growth forecast
      const revenueValues: { [key: string]: number } = {
        '2025Q1': 1000000,
      };
      const builderWithRevenue = builderWithPeriods.value('revenue', revenueValues);
      if (!builderWithRevenue) {
        throw new Error('Failed to add revenue');
      }

      // Add 5% annual growth forecast
      const growthForecast = ForecastSpec.growth(0.05);
      const builderWithForecast = builderWithRevenue.forecast('revenue', growthForecast);
      if (!builderWithForecast) {
        throw new Error('Failed to add forecast');
      }
      log.push('✓ Added revenue with 5% growth forecast');

      // Add expenses with curve forecast
      const expenseValues: { [key: string]: number } = {
        '2025Q1': 800000,
      };
      const builderWithExpenses = builderWithForecast.value('expenses', expenseValues);
      if (!builderWithExpenses) {
        throw new Error('Failed to add expenses');
      }

      // Different growth rates per quarter
      const curveForecast = ForecastSpec.curve(new Float64Array([0.02, 0.03, 0.04]));
      const builderWithCurveForecast = builderWithExpenses.forecast('expenses', curveForecast);
      if (!builderWithCurveForecast) {
        throw new Error('Failed to add curve forecast');
      }
      log.push('✓ Added expenses with curve forecast [2%, 3%, 4%]');

      // Calculate net income
      const builderWithNetIncome = builderWithCurveForecast.compute(
        'net_income',
        'revenue - expenses'
      );
      if (!builderWithNetIncome) {
        throw new Error('Failed to add net_income');
      }
      log.push('✓ Added net_income formula');

      const model = builderWithNetIncome.build();
      if (!model) {
        throw new Error('Failed to build model');
      }
      log.push(`\n✓ Model built: ${model.nodeCount()} nodes\n`);

      // Evaluate
      log.push('=== Evaluating with Forecasts ===\n');
      const evaluator = new Evaluator();
      const evalResults = evaluator.evaluate(model);
      if (!evalResults) {
        throw new Error('Failed to evaluate model');
      }

      // Show all quarters
      log.push('Results by Quarter:\n');
      const displayResults: ResultDisplay[] = [];

      const nodeIds = ['revenue', 'expenses', 'net_income'];
      const quarters = ['2025Q1', '2025Q2', '2025Q3', '2025Q4'];

      for (const nodeId of nodeIds) {
        const periods: { [key: string]: number } = {};

        log.push(`${nodeId.toUpperCase()}:`);
        for (const quarter of quarters) {
          const value = evalResults.get(nodeId, quarter);
          if (value !== null && value !== undefined) {
            periods[quarter] = value;
            log.push(
              `  ${quarter}: $${value.toLocaleString('en-US', { maximumFractionDigits: 0 })}`
            );
          }
        }
        log.push('');

        displayResults.push({ nodeId, periods });
      }

      setOutput(log.join('\n'));
      setResults(displayResults);
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runRegistryDemo = () => {
    setLoading(true);
    setOutput('');
    setResults([]);

    try {
      const log: string[] = [];

      log.push('=== Dynamic Metric Registry ===\n');

      const registry = new Registry();
      try {
        registry.loadBuiltins();
      } catch (error) {
        throw new Error(`Failed to load builtins: ${error}`);
      }

      log.push(`✓ Loaded built-in metrics: ${registry.metricCount()} total\n`);

      // List metrics in fin namespace
      const finMetrics = registry.listMetrics('fin');
      log.push(`Finance Metrics (fin.*):`);
      finMetrics.slice(0, 10).forEach((metricId: string) => {
        log.push(`  • ${metricId}`);
      });
      if (finMetrics.length > 10) {
        log.push(`  ... and ${finMetrics.length - 10} more\n`);
      } else {
        log.push('');
      }

      // Show details for specific metrics
      log.push('Metric Details:\n');
      const sampleMetrics = ['fin.gross_margin', 'fin.operating_margin', 'fin.net_margin'];

      for (const metricId of sampleMetrics) {
        if (registry.hasMetric(metricId)) {
          const metric = registry.get(metricId);
          log.push(`${metric.name}:`);
          log.push(`  ID: ${metric.id}`);
          log.push(`  Formula: ${metric.formula}`);
          if (metric.description) {
            log.push(`  Description: ${metric.description}`);
          }
          log.push('');
        }
      }

      setOutput(log.join('\n'));
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const runCompleteExample = () => {
    setLoading(true);
    setOutput('');
    setResults([]);

    try {
      const log: string[] = [];

      log.push('=== Complete Financial Model Example ===\n');

      const builder = new ModelBuilder('Complete Model');
      const builderWithPeriods = builder.periods('2024Q1..2025Q4', '2024Q4');
      if (!builderWithPeriods) {
        throw new Error('Failed to set periods');
      }
      log.push('✓ Periods: 2024Q1-2025Q4 (actuals through 2024Q4)\n');

      // Revenue with historical data
      const revenueValues: { [key: string]: number } = {
        '2024Q1': 900000,
        '2024Q2': 950000,
        '2024Q3': 975000,
        '2024Q4': 1000000,
      };
      const builderWithRevenue = builderWithPeriods.value('revenue', revenueValues);
      if (!builderWithRevenue) {
        throw new Error('Failed to add revenue');
      }

      // 8% annual growth forecast
      const revenueForecast = ForecastSpec.growth(0.08);
      const builderWithRevenueForecast = builderWithRevenue.forecast('revenue', revenueForecast);
      if (!builderWithRevenueForecast) {
        throw new Error('Failed to add revenue forecast');
      }
      log.push('✓ Revenue: historical data + 8% growth forecast');

      // COGS (60% of revenue)
      const builderWithCogs = builderWithRevenueForecast.compute('cogs', 'revenue * 0.60');
      if (!builderWithCogs) {
        throw new Error('Failed to add COGS');
      }

      // Gross profit
      const builderWithGrossProfit = builderWithCogs.compute('gross_profit', 'revenue - cogs');
      if (!builderWithGrossProfit) {
        throw new Error('Failed to add gross_profit');
      }
      log.push('✓ Calculated: COGS, Gross Profit');

      // Operating expenses
      const opexValues: { [key: string]: number } = {
        '2024Q1': 200000,
        '2024Q2': 210000,
        '2024Q3': 220000,
        '2024Q4': 230000,
      };
      const builderWithOpex = builderWithGrossProfit.value('opex', opexValues);
      if (!builderWithOpex) {
        throw new Error('Failed to add opex');
      }

      // OpEx grows at 4% annually
      const opexForecast = ForecastSpec.growth(0.04);
      const builderWithOpexForecast = builderWithOpex.forecast('opex', opexForecast);
      if (!builderWithOpexForecast) {
        throw new Error('Failed to add opex forecast');
      }
      log.push('✓ OpEx: historical data + 4% growth forecast');

      // EBITDA
      const builderWithEbitda = builderWithOpexForecast.compute('ebitda', 'gross_profit - opex');
      if (!builderWithEbitda) {
        throw new Error('Failed to add ebitda');
      }

      // Calculate margins
      const builderWithGrossMargin = builderWithEbitda.compute(
        'gross_margin',
        'gross_profit / revenue'
      );
      if (!builderWithGrossMargin) {
        throw new Error('Failed to add gross_margin');
      }
      const builderWithEbitdaMargin = builderWithGrossMargin.compute(
        'ebitda_margin',
        'ebitda / revenue'
      );
      if (!builderWithEbitdaMargin) {
        throw new Error('Failed to add ebitda_margin');
      }
      log.push('✓ Calculated: EBITDA, margins\n');

      const model = builderWithEbitdaMargin.build();
      if (!model) {
        throw new Error('Failed to build model');
      }
      log.push(`✓ Model built: ${model.nodeCount()} nodes across ${model.periodCount()} periods\n`);

      // Evaluate
      const evaluator = new Evaluator();
      const evalResults = evaluator.evaluate(model);
      if (!evalResults) {
        throw new Error('Failed to evaluate model');
      }

      log.push('=== Summary Results ===\n');

      // Show last historical and first forecast quarter
      const quarters = ['2024Q4', '2025Q1'];
      const displayResults: ResultDisplay[] = [];

      const keyMetrics = [
        'revenue',
        'cogs',
        'gross_profit',
        'opex',
        'ebitda',
        'gross_margin',
        'ebitda_margin',
      ];

      for (const metric of keyMetrics) {
        const periods: { [key: string]: number } = {};

        log.push(`${metric.toUpperCase()}:`);
        for (const quarter of quarters) {
          const value = evalResults.get(metric, quarter);
          if (value !== null && value !== undefined) {
            periods[quarter] = value;
            if (metric.includes('margin')) {
              log.push(`  ${quarter}: ${(value * 100).toFixed(1)}%`);
            } else {
              log.push(
                `  ${quarter}: $${value.toLocaleString('en-US', { maximumFractionDigits: 0 })}`
              );
            }
          }
        }
        log.push('');

        displayResults.push({ nodeId: metric, periods });
      }

      setOutput(log.join('\n'));
      setResults(displayResults);
    } catch (error) {
      setOutput(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="bg-white shadow rounded-lg p-6">
        <h2 className="text-2xl font-bold mb-4">Financial Statements Modeling</h2>
        <p className="text-gray-600 mb-6">
          Build and evaluate financial statement models with formulas, forecasts, and dynamic
          metrics. Supports period-by-period evaluation with deterministic results.
        </p>

        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <button
              onClick={runBasicModel}
              disabled={loading}
              className="px-4 py-3 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Basic P&L Model
            </button>

            <button
              onClick={runForecastModel}
              disabled={loading}
              className="px-4 py-3 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Model with Forecasts
            </button>

            <button
              onClick={runRegistryDemo}
              disabled={loading}
              className="px-4 py-3 bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Metric Registry
            </button>

            <button
              onClick={runCompleteExample}
              disabled={loading}
              className="px-4 py-3 bg-indigo-600 text-white rounded hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Complete Example
            </button>
          </div>
        </div>
      </div>

      {output && (
        <div className="bg-white shadow rounded-lg p-6">
          <h3 className="text-lg font-semibold mb-4">Output</h3>
          <pre className="bg-gray-50 p-4 rounded overflow-x-auto text-sm font-mono whitespace-pre-wrap">
            {output}
          </pre>
        </div>
      )}

      {results.length > 0 && (
        <div className="bg-white shadow rounded-lg p-6">
          <h3 className="text-lg font-semibold mb-4">Results Table</h3>
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Metric
                  </th>
                  {results[0] &&
                    Object.keys(results[0].periods).map((period) => (
                      <th
                        key={period}
                        className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
                      >
                        {period}
                      </th>
                    ))}
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {results.map((result) => (
                  <tr key={result.nodeId}>
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                      {result.nodeId}
                    </td>
                    {Object.entries(result.periods).map(([period, value]) => (
                      <td
                        key={period}
                        className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-500"
                      >
                        {result.nodeId.includes('margin')
                          ? `${(value * 100).toFixed(1)}%`
                          : `$${value.toLocaleString('en-US', { maximumFractionDigits: 0 })}`}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <div className="bg-blue-50 border border-blue-200 rounded-lg p-6">
        <h3 className="text-lg font-semibold mb-2 text-blue-900">Features Demonstrated</h3>
        <ul className="space-y-2 text-blue-800">
          <li>
            ✓ <strong>Builder Pattern:</strong> Fluent API for model construction
          </li>
          <li>
            ✓ <strong>Period Ranges:</strong> Define periods using string ranges (e.g.,
            &quot;2025Q1..Q4&quot;)
          </li>
          <li>
            ✓ <strong>Value Nodes:</strong> Explicit values for actuals and assumptions
          </li>
          <li>
            ✓ <strong>Calculated Nodes:</strong> Formula-based computations with DSL
          </li>
          <li>
            ✓ <strong>Forecast Methods:</strong> Growth rates, curves, and statistical distributions
          </li>
          <li>
            ✓ <strong>Evaluator:</strong> Deterministic period-by-period evaluation
          </li>
          <li>
            ✓ <strong>Dynamic Registry:</strong> 22+ built-in financial metrics
          </li>
          <li>
            ✓ <strong>Type Safety:</strong> Full TypeScript support
          </li>
        </ul>
      </div>
    </div>
  );
}
