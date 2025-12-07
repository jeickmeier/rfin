import React, { useEffect, useState } from 'react';
import {
  GaussHermiteQuadrature,
  adaptiveSimpson,
  binomialProbability,
  gaussLegendreIntegrate,
  logBinomialCoefficient,
  logFactorial,
  NewtonSolver,
  BrentSolver,
} from 'finstack-wasm';
import { MathShowcaseProps, DEFAULT_MATH_SHOWCASE_PROPS } from './data/math-showcase';

interface IntegrationRow {
  label: string;
  value: number;
  reference?: string;
}

interface SolverRow {
  label: string;
  root: number;
  reference?: string;
}

interface DistributionRow {
  label: string;
  value: number;
}

interface MathShowcaseState {
  integrals: IntegrationRow[];
  solvers: SolverRow[];
  distributions: DistributionRow[];
}

const formatNumber = (value: number): string => {
  if (!Number.isFinite(value)) {
    return 'NaN';
  }
  const abs = Math.abs(value);
  if (abs !== 0 && (abs >= 1e4 || abs <= 1e-6)) {
    return value.toExponential(6);
  }
  return value.toFixed(10);
};

export const MathShowcaseExample: React.FC<MathShowcaseProps> = (props) => {
  const {
    integrationExamples = DEFAULT_MATH_SHOWCASE_PROPS.integrationExamples!,
    solverExamples = DEFAULT_MATH_SHOWCASE_PROPS.solverExamples!,
    distributionExamples = DEFAULT_MATH_SHOWCASE_PROPS.distributionExamples!,
  } = props;

  const [state, setState] = useState<MathShowcaseState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    const solversToFree: Array<NewtonSolver | BrentSolver | GaussHermiteQuadrature> = [];

    try {
      // Process integration examples
      const integrals: IntegrationRow[] = [];
      for (const example of integrationExamples) {
        let value = 0;

        switch (example.type) {
          case 'gauss_hermite': {
            const quad = GaussHermiteQuadrature.order7();
            solversToFree.push(quad);
            value = quad.integrate((x: number) => x * x);
            break;
          }
          case 'gauss_legendre': {
            value = gaussLegendreIntegrate(
              (x: number) => Math.cos(x),
              example.lowerBound ?? 0,
              example.upperBound ?? Math.PI / 2,
              example.order ?? 8
            );
            break;
          }
          case 'adaptive_simpson': {
            value = adaptiveSimpson(
              (x: number) => Math.sin(10 * x) / (1 + x * x),
              example.lowerBound ?? 0,
              example.upperBound ?? 1,
              example.tolerance ?? 1e-8,
              example.maxIterations ?? 12
            );
            break;
          }
        }

        integrals.push({
          label: example.label,
          value,
          reference: example.reference,
        });
      }

      // Process solver examples
      const solvers: SolverRow[] = [];
      for (const example of solverExamples) {
        let root = 0;

        switch (example.type) {
          case 'newton': {
            const newton = new NewtonSolver(
              example.tolerance,
              example.maxIterations,
              1e-8 // derivative step
            );
            solversToFree.push(newton);

            // Determine which equation based on label
            if (example.label.includes('x² − 2')) {
              root = newton.solve((x: number) => x * x - 2, example.initialGuess);
            }
            break;
          }
          case 'brent': {
            const brent = new BrentSolver(
              example.tolerance,
              example.maxIterations,
              example.bracketSize ?? 2,
              null
            );
            solversToFree.push(brent);

            // Determine which equation based on label
            if (example.label.includes('cos(x) − x')) {
              root = brent.solve((x: number) => Math.cos(x) - x, example.initialGuess);
            } else if (example.label.includes('x³ − x − 1')) {
              root = brent.solve((x: number) => x * x * x - x - 1, example.initialGuess);
            }
            break;
          }
        }

        solvers.push({
          label: example.label,
          root,
          reference: example.reference,
        });
      }

      // Process distribution examples
      const distributions: DistributionRow[] = [];
      for (const example of distributionExamples) {
        let value = 0;

        switch (example.type) {
          case 'binomial_probability': {
            const [n, k, p] = example.params;
            value = binomialProbability(n, k, p);
            break;
          }
          case 'log_binomial_coefficient': {
            const [n, k] = example.params;
            value = logBinomialCoefficient(n, k);
            break;
          }
          case 'log_factorial': {
            const [n] = example.params;
            value = logFactorial(n);
            break;
          }
        }

        distributions.push({
          label: example.label,
          value,
        });
      }

      if (mounted) {
        setState({ integrals, solvers, distributions });
      }
    } catch (err) {
      if (mounted) {
        setError((err as Error).message);
      }
    } finally {
      // Free all solvers
      for (const solver of solversToFree) {
        solver.free();
      }
    }

    return () => {
      mounted = false;
    };
  }, [integrationExamples, solverExamples, distributionExamples]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (!state) {
    return <p>Computing math showcases…</p>;
  }

  return (
    <section className="example-section">
      <h2>Math Utilities</h2>
      <p>
        Demonstrates numerical integration, probability helpers, and root-finding solvers exposed
        via the WASM bindings. These mirror the Python examples for consistent parity across
        runtimes.
      </p>

      <h3>Integration</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Scenario</th>
            <th>Value</th>
            <th>Reference</th>
          </tr>
        </thead>
        <tbody>
          {state.integrals.map((row) => (
            <tr key={row.label}>
              <td>{row.label}</td>
              <td>{formatNumber(row.value)}</td>
              <td>{row.reference ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>Solvers</h3>
      <table className="data-table compact">
        <thead>
          <tr>
            <th>Problem</th>
            <th>Root</th>
            <th>Reference</th>
          </tr>
        </thead>
        <tbody>
          {state.solvers.map((row) => (
            <tr key={row.label}>
              <td>{row.label}</td>
              <td>{formatNumber(row.root)}</td>
              <td>{row.reference ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>Distribution Helpers</h3>
      <table className="data-table compact">
        <thead>
          <tr>
            <th>Metric</th>
            <th>Value</th>
          </tr>
        </thead>
        <tbody>
          {state.distributions.map((row) => (
            <tr key={row.label}>
              <td>{row.label}</td>
              <td>{formatNumber(row.value)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
