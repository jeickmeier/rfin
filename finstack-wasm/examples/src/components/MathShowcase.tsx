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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

type RequiredMathShowcaseProps = Required<MathShowcaseProps>;

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
  if (!Number.isFinite(value)) return 'NaN';
  const abs = Math.abs(value);
  if (abs !== 0 && (abs >= 1e4 || abs <= 1e-6)) {
    return value.toExponential(6);
  }
  return value.toFixed(10);
};

export const MathShowcaseExample: React.FC<MathShowcaseProps> = (props) => {
  const defaults = DEFAULT_MATH_SHOWCASE_PROPS as RequiredMathShowcaseProps;
  const {
    integrationExamples = defaults.integrationExamples,
    solverExamples = defaults.solverExamples,
    distributionExamples = defaults.distributionExamples,
  } = props;

  const [state, setState] = useState<MathShowcaseState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    const solversToFree: Array<NewtonSolver | BrentSolver | GaussHermiteQuadrature> = [];

    try {
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

        integrals.push({ label: example.label, value, reference: example.reference });
      }

      const solvers: SolverRow[] = [];
      for (const example of solverExamples) {
        let root = 0;

        switch (example.type) {
          case 'newton': {
            const newton = new NewtonSolver(example.tolerance, example.maxIterations, 1e-8);
            solversToFree.push(newton);
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
            if (example.label.includes('cos(x) − x')) {
              root = brent.solve((x: number) => Math.cos(x) - x, example.initialGuess);
            } else if (example.label.includes('x³ − x − 1')) {
              root = brent.solve((x: number) => x * x * x - x - 1, example.initialGuess);
            }
            break;
          }
        }

        solvers.push({ label: example.label, root, reference: example.reference });
      }

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

        distributions.push({ label: example.label, value });
      }

      if (mounted) {
        setState({ integrals, solvers, distributions });
      }
    } catch (err) {
      if (mounted) {
        setError((err as Error).message);
      }
    } finally {
      for (const solver of solversToFree) {
        solver.free();
      }
    }

    return () => {
      mounted = false;
    };
  }, [integrationExamples, solverExamples, distributionExamples]);

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (!state) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        <span className="ml-3 text-muted-foreground">Computing math showcases…</span>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Math Utilities</CardTitle>
        <CardDescription>
          Demonstrates numerical integration, probability helpers, and root-finding solvers exposed
          via the WASM bindings. These mirror the Python examples for consistent parity across
          runtimes.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-8">
        {/* Integration */}
        <div className="space-y-3">
          <h3 className="text-lg font-semibold">Integration</h3>
          <div className="rounded-lg border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Scenario</TableHead>
                  <TableHead className="text-right">Value</TableHead>
                  <TableHead className="text-right">Reference</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {state.integrals.map((row) => (
                  <TableRow key={row.label}>
                    <TableCell className="font-medium">{row.label}</TableCell>
                    <TableCell className="text-right font-mono">
                      {formatNumber(row.value)}
                    </TableCell>
                    <TableCell className="text-right text-muted-foreground">
                      {row.reference ?? '—'}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>

        {/* Solvers */}
        <div className="space-y-3">
          <h3 className="text-lg font-semibold">Solvers</h3>
          <div className="rounded-lg border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Problem</TableHead>
                  <TableHead className="text-right">Root</TableHead>
                  <TableHead className="text-right">Reference</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {state.solvers.map((row) => (
                  <TableRow key={row.label}>
                    <TableCell className="font-medium">{row.label}</TableCell>
                    <TableCell className="text-right font-mono">{formatNumber(row.root)}</TableCell>
                    <TableCell className="text-right text-muted-foreground">
                      {row.reference ?? '—'}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>

        {/* Distribution Helpers */}
        <div className="space-y-3">
          <h3 className="text-lg font-semibold">Distribution Helpers</h3>
          <div className="rounded-lg border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Metric</TableHead>
                  <TableHead className="text-right">Value</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {state.distributions.map((row) => (
                  <TableRow key={row.label}>
                    <TableCell className="font-medium">{row.label}</TableCell>
                    <TableCell className="text-right font-mono">
                      {formatNumber(row.value)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
