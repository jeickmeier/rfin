import React, { useEffect, useState } from 'react';
import { MonteCarloPathGenerator, SimulatedPath } from 'finstack-wasm';
import { MonteCarloPathProps, DEFAULT_MONTE_CARLO_PROPS } from './data/monte-carlo';

type PathStats = {
  numPathsTotal: number;
  numPathsCaptured: number;
  samplingRatio: number;
  meanTerminalValue: number;
  stdTerminalValue: number;
  minTerminalValue: number;
  maxTerminalValue: number;
  processParams: any;
};

type PathDataRow = {
  pathId: number;
  terminalValue: number;
  steps: number;
};

export const MonteCarloPathExample: React.FC<MonteCarloPathProps> = (props) => {
  const {
    gbmParams = DEFAULT_MONTE_CARLO_PROPS.gbmParams!,
    maxRowsToDisplay = DEFAULT_MONTE_CARLO_PROPS.maxRowsToDisplay!,
  } = props;

  const [stats, setStats] = useState<PathStats | null>(null);
  const [pathData, setPathData] = useState<PathDataRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        setLoading(true);

        // Generate GBM paths
        const generator = new MonteCarloPathGenerator();
        const paths = generator.generateGbmPaths(
          gbmParams.initialSpot,
          gbmParams.riskFreeRate,
          gbmParams.dividendYield,
          gbmParams.volatility,
          gbmParams.timeToMaturity,
          gbmParams.numSteps,
          gbmParams.numPaths,
          gbmParams.captureMode,
          gbmParams.sampleCount,
          gbmParams.seed
        );

        // Extract statistics
        const numPaths = paths.numPaths;
        const capturedPaths: SimulatedPath[] = [];
        const terminalValues: number[] = [];

        // Iterate through captured paths
        for (let i = 0; i < numPaths; i++) {
          const path = paths.getPath(i);
          if (path) {
            capturedPaths.push(path);
            const terminal = path.terminalPoint();
            if (terminal) {
              const terminalSpot = terminal.spot();
              if (terminalSpot !== null && terminalSpot !== undefined) {
                terminalValues.push(terminalSpot);
              }
            }
          }
        }

        // Calculate statistics
        const meanTerminal = terminalValues.reduce((a, b) => a + b, 0) / terminalValues.length;
        const variance =
          terminalValues.reduce((sum, val) => sum + Math.pow(val - meanTerminal, 2), 0) /
          terminalValues.length;
        const stdTerminal = Math.sqrt(variance);
        const minTerminal = Math.min(...terminalValues);
        const maxTerminal = Math.max(...terminalValues);

        // Get process parameters
        const processParams = paths.processParams();

        // Build path data rows
        const rows: PathDataRow[] = capturedPaths.map((path) => ({
          pathId: path.pathId,
          terminalValue: path.finalValue,
          steps: path.numSteps(),
        }));

        if (!cancelled) {
          setStats({
            numPathsTotal: gbmParams.numPaths,
            numPathsCaptured: numPaths,
            samplingRatio: numPaths / gbmParams.numPaths,
            meanTerminalValue: meanTerminal,
            stdTerminalValue: stdTerminal,
            minTerminalValue: minTerminal,
            maxTerminalValue: maxTerminal,
            processParams,
          });
          setPathData(rows);
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Monte Carlo path generation error:', err);
          setError((err as Error).message);
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [gbmParams]);

  const exportToCSV = () => {
    if (!pathData.length) return;

    const headers = ['Path ID', 'Terminal Value', 'Steps'];
    const rows = pathData.map((row) => [
      row.pathId.toString(),
      row.terminalValue.toFixed(4),
      row.steps.toString(),
    ]);

    const csvContent = [headers.join(','), ...rows.map((row) => row.join(','))].join('\n');

    const blob = new Blob([csvContent], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'monte_carlo_paths.csv';
    link.click();
    URL.revokeObjectURL(url);
  };

  const exportToJSON = () => {
    if (!pathData.length) return;

    const jsonContent = JSON.stringify(pathData, null, 2);
    const blob = new Blob([jsonContent], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'monte_carlo_paths.json';
    link.click();
    URL.revokeObjectURL(url);
  };

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (loading || !stats) {
    return <p>Generating Monte Carlo paths…</p>;
  }

  return (
    <section className="example-section">
      <h2>Monte Carlo Path Generation</h2>
      <p>
        Generate and analyze Monte Carlo paths using Geometric Brownian Motion (GBM). This example
        generates {gbmParams.numPaths.toLocaleString()} paths but captures only{' '}
        {gbmParams.sampleCount} for detailed analysis, demonstrating efficient sampling strategies.
      </p>

      <div style={{ marginBottom: '2rem' }}>
        <h3>Simulation Parameters</h3>
        <table>
          <tbody>
            <tr>
              <td>
                <strong>Process Type:</strong>
              </td>
              <td>{stats.processParams.processType || 'GBM'}</td>
            </tr>
            <tr>
              <td>
                <strong>Initial Spot:</strong>
              </td>
              <td>{gbmParams.initialSpot.toFixed(2)}</td>
            </tr>
            <tr>
              <td>
                <strong>Risk-Free Rate:</strong>
              </td>
              <td>{(gbmParams.riskFreeRate * 100).toFixed(2)}%</td>
            </tr>
            <tr>
              <td>
                <strong>Dividend Yield:</strong>
              </td>
              <td>{(gbmParams.dividendYield * 100).toFixed(2)}%</td>
            </tr>
            <tr>
              <td>
                <strong>Volatility:</strong>
              </td>
              <td>{(gbmParams.volatility * 100).toFixed(2)}%</td>
            </tr>
            <tr>
              <td>
                <strong>Time to Maturity:</strong>
              </td>
              <td>{gbmParams.timeToMaturity} year(s)</td>
            </tr>
            <tr>
              <td>
                <strong>Steps:</strong>
              </td>
              <td>{gbmParams.numSteps}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div style={{ marginBottom: '2rem' }}>
        <h3>Path Statistics</h3>
        <table>
          <thead>
            <tr>
              <th>Metric</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Total Paths Generated</td>
              <td>{stats.numPathsTotal.toLocaleString()}</td>
            </tr>
            <tr>
              <td>Paths Captured</td>
              <td>{stats.numPathsCaptured.toLocaleString()}</td>
            </tr>
            <tr>
              <td>Sampling Ratio</td>
              <td>{(stats.samplingRatio * 100).toFixed(1)}%</td>
            </tr>
            <tr>
              <td>Mean Terminal Value</td>
              <td>{stats.meanTerminalValue.toFixed(2)}</td>
            </tr>
            <tr>
              <td>Std Dev Terminal Value</td>
              <td>{stats.stdTerminalValue.toFixed(2)}</td>
            </tr>
            <tr>
              <td>Min Terminal Value</td>
              <td>{stats.minTerminalValue.toFixed(2)}</td>
            </tr>
            <tr>
              <td>Max Terminal Value</td>
              <td>{stats.maxTerminalValue.toFixed(2)}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div style={{ marginBottom: '2rem' }}>
        <h3>Captured Paths</h3>
        <div style={{ marginBottom: '1rem' }}>
          <button onClick={exportToCSV} style={{ marginRight: '0.5rem' }}>
            Export to CSV
          </button>
          <button onClick={exportToJSON}>Export to JSON</button>
        </div>
        <table>
          <thead>
            <tr>
              <th>Path ID</th>
              <th>Terminal Value</th>
              <th>Steps</th>
            </tr>
          </thead>
          <tbody>
            {pathData.slice(0, maxRowsToDisplay).map((row) => (
              <tr key={row.pathId}>
                <td>{row.pathId}</td>
                <td>{row.terminalValue.toFixed(4)}</td>
                <td>{row.steps}</td>
              </tr>
            ))}
            {pathData.length > maxRowsToDisplay && (
              <tr>
                <td colSpan={3} style={{ textAlign: 'center', fontStyle: 'italic' }}>
                  ... and {pathData.length - maxRowsToDisplay} more paths
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div>
        <h3>Usage Notes</h3>
        <ul>
          <li>
            <strong>Capture Modes:</strong> Use &apos;all&apos; to capture all paths (for small
            simulations) or &apos;sample&apos; with a count to capture only a subset (for large
            simulations).
          </li>
          <li>
            <strong>Reproducibility:</strong> The seed parameter ensures deterministic path
            generation.
          </li>
          <li>
            <strong>Performance:</strong> Sampling reduces memory usage while maintaining
            statistical accuracy for visualization purposes.
          </li>
          <li>
            <strong>Path Access:</strong> Use getPath(index) to access individual paths and their
            points for detailed analysis.
          </li>
        </ul>
      </div>
    </section>
  );
};
