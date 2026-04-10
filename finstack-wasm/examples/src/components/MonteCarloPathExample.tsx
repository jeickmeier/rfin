import React, { useEffect, useState } from 'react';
import { MonteCarloPathProps, DEFAULT_MONTE_CARLO_PROPS } from './data/monte-carlo';

type RequiredMonteCarloPathProps = Required<MonteCarloPathProps>;

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

const normalFrom = (nextUniform: () => number) => {
  let u1 = 0;
  while (u1 <= Number.EPSILON) {
    u1 = nextUniform();
  }
  const u2 = nextUniform();
  return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
};

const createRng = (seed: bigint) => {
  let state = Number(seed % BigInt(2 ** 32)) >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t ^= t + Math.imul(t ^ (t >>> 7), 61 | t);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

export const MonteCarloPathExample: React.FC<MonteCarloPathProps> = (props) => {
  const defaults = DEFAULT_MONTE_CARLO_PROPS as RequiredMonteCarloPathProps;
  const { gbmParams = defaults.gbmParams, maxRowsToDisplay = defaults.maxRowsToDisplay } = props;

  const [stats, setStats] = useState<PathStats | null>(null);
  const [pathData, setPathData] = useState<PathDataRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        setLoading(true);
        const nextUniform = createRng(gbmParams.seed);
        const dt = gbmParams.timeToMaturity / gbmParams.numSteps;
        const drift =
          (gbmParams.riskFreeRate - gbmParams.dividendYield - 0.5 * gbmParams.volatility ** 2) * dt;
        const diffusion = gbmParams.volatility * Math.sqrt(dt);
        const numPathsCaptured =
          gbmParams.captureMode === 'all'
            ? gbmParams.numPaths
            : Math.min(gbmParams.sampleCount, gbmParams.numPaths);
        const terminalValues: number[] = [];
        const rows: PathDataRow[] = [];

        for (let pathId = 0; pathId < gbmParams.numPaths; pathId++) {
          let spot = gbmParams.initialSpot;
          for (let step = 0; step < gbmParams.numSteps; step++) {
            spot *= Math.exp(drift + diffusion * normalFrom(nextUniform));
          }
          terminalValues.push(spot);
          if (pathId < numPathsCaptured) {
            rows.push({
              pathId,
              terminalValue: spot,
              steps: gbmParams.numSteps,
            });
          }
        }

        const meanTerminal = terminalValues.reduce((a, b) => a + b, 0) / terminalValues.length;
        const variance =
          terminalValues.reduce((sum, val) => sum + Math.pow(val - meanTerminal, 2), 0) /
          terminalValues.length;
        const stdTerminal = Math.sqrt(variance);
        const minTerminal = Math.min(...terminalValues);
        const maxTerminal = Math.max(...terminalValues);

        const processParams = {
          processType: 'GBM',
          captureMode: gbmParams.captureMode,
          dt,
          seed: gbmParams.seed.toString(),
        };

        if (!cancelled) {
          setStats({
            numPathsTotal: gbmParams.numPaths,
            numPathsCaptured,
            samplingRatio: numPathsCaptured / gbmParams.numPaths,
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
