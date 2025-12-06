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
  HybridSolver,
} from 'finstack-wasm';

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

export const MathShowcaseExample: React.FC = () => {
  const [state, setState] = useState<MathShowcaseState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    let quad: GaussHermiteQuadrature | null = null;
    let newton: NewtonSolver | null = null;
    let brent: BrentSolver | null = null;
    let hybrid: HybridSolver | null = null;

    try {
      quad = GaussHermiteQuadrature.order7();
      const normalMoment = quad.integrate((x: number) => x * x);
      const legendre = gaussLegendreIntegrate((x: number) => Math.cos(x), 0.0, Math.PI / 2.0, 8);
      const adaptive = adaptiveSimpson(
        (x: number) => Math.sin(10.0 * x) / (1.0 + x * x),
        0.0,
        1.0,
        1e-8,
        12
      );

      newton = new NewtonSolver(1e-12, 50, 1e-8);
      const sqrtTwo = newton.solve((x: number) => x * x - 2.0, 1.0);

      brent = new BrentSolver(1e-12, 100, 2.0, null);
      const cosMinusX = brent.solve((x: number) => Math.cos(x) - x, 0.5);

      hybrid = new HybridSolver(1e-12, 100);
      const cubicRoot = hybrid.solve((x: number) => x * x * x - x - 1.0, 1.0);

      const integrals: IntegrationRow[] = [
        {
          label: 'E[X²] under N(0,1) (Gauss-Hermite order 7)',
          value: normalMoment,
          reference: '≈ 1.0',
        },
        {
          label: '∫₀^{π/2} cos(x) dx (Gauss-Legendre order 8)',
          value: legendre,
          reference: '= 1.0',
        },
        {
          label: 'Adaptive Simpson ∫ sin(10x)/(1+x²) dx on [0,1]',
          value: adaptive,
          reference: '≈ 0.4363',
        },
      ];

      const solvers: SolverRow[] = [
        {
          label: 'Newton solve x² − 2 = 0',
          root: sqrtTwo,
          reference: '√2 ≈ 1.4142',
        },
        {
          label: 'Brent solve cos(x) − x = 0',
          root: cosMinusX,
          reference: '≈ 0.7391',
        },
        {
          label: 'Hybrid solve x³ − x − 1 = 0',
          root: cubicRoot,
          reference: '≈ 1.3247',
        },
      ];

      const distributions: DistributionRow[] = [
        {
          label: 'Binomial P(X=3; n=10, p=0.5)',
          value: binomialProbability(10, 3, 0.5),
        },
        {
          label: 'log C(5, 2)',
          value: logBinomialCoefficient(5, 2),
        },
        {
          label: 'log(10!)',
          value: logFactorial(10),
        },
      ];

      if (mounted) {
        setState({ integrals, solvers, distributions });
      }
    } catch (err) {
      if (mounted) {
        setError((err as Error).message);
      }
    } finally {
      quad?.free();
      newton?.free();
      brent?.free();
      hybrid?.free();
    }

    return () => {
      mounted = false;
    };
  }, []);

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
