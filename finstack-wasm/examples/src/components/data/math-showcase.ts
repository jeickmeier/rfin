/**
 * Math showcase fixture data.
 */

// Integration example data
export interface IntegrationExampleData {
  label: string;
  type: 'gauss_hermite' | 'gauss_legendre' | 'adaptive_simpson';
  order?: number;
  lowerBound?: number;
  upperBound?: number;
  tolerance?: number;
  maxIterations?: number;
  reference?: string;
}

// Solver example data
export interface SolverExampleData {
  label: string;
  type: 'newton' | 'brent';
  tolerance: number;
  maxIterations: number;
  initialGuess: number;
  bracketSize?: number;
  reference?: string;
}

// Distribution example data
export interface DistributionExampleData {
  label: string;
  type: 'binomial_probability' | 'log_binomial_coefficient' | 'log_factorial';
  params: number[];
}

export interface MathShowcaseProps {
  integrationExamples?: IntegrationExampleData[];
  solverExamples?: SolverExampleData[];
  distributionExamples?: DistributionExampleData[];
}

// Default integration examples
const DEFAULT_INTEGRATION_EXAMPLES: IntegrationExampleData[] = [
  {
    label: 'E[X²] under N(0,1) (Gauss-Hermite order 7)',
    type: 'gauss_hermite',
    order: 7,
    reference: '≈ 1.0',
  },
  {
    label: '∫₀^{π/2} cos(x) dx (Gauss-Legendre order 8)',
    type: 'gauss_legendre',
    order: 8,
    lowerBound: 0,
    upperBound: Math.PI / 2,
    reference: '= 1.0',
  },
  {
    label: 'Adaptive Simpson ∫ sin(10x)/(1+x²) dx on [0,1]',
    type: 'adaptive_simpson',
    lowerBound: 0,
    upperBound: 1,
    tolerance: 1e-8,
    maxIterations: 12,
    reference: '≈ 0.4363',
  },
];

// Default solver examples
const DEFAULT_SOLVER_EXAMPLES: SolverExampleData[] = [
  {
    label: 'Newton solve x² − 2 = 0',
    type: 'newton',
    tolerance: 1e-12,
    maxIterations: 50,
    initialGuess: 1,
    reference: '√2 ≈ 1.4142',
  },
  {
    label: 'Brent solve cos(x) − x = 0',
    type: 'brent',
    tolerance: 1e-12,
    maxIterations: 100,
    initialGuess: 0.5,
    bracketSize: 2,
    reference: '≈ 0.7391',
  },
  {
    label: 'Brent solve x³ − x − 1 = 0',
    type: 'brent',
    tolerance: 1e-12,
    maxIterations: 100,
    initialGuess: 1,
    bracketSize: 2,
    reference: '≈ 1.3247',
  },
];

// Default distribution examples
const DEFAULT_DISTRIBUTION_EXAMPLES: DistributionExampleData[] = [
  {
    label: 'Binomial P(X=3; n=10, p=0.5)',
    type: 'binomial_probability',
    params: [10, 3, 0.5], // n, k, p
  },
  {
    label: 'log C(5, 2)',
    type: 'log_binomial_coefficient',
    params: [5, 2], // n, k
  },
  {
    label: 'log(10!)',
    type: 'log_factorial',
    params: [10], // n
  },
];

// Complete props bundle
export const DEFAULT_MATH_SHOWCASE_PROPS: MathShowcaseProps = {
  integrationExamples: DEFAULT_INTEGRATION_EXAMPLES,
  solverExamples: DEFAULT_SOLVER_EXAMPLES,
  distributionExamples: DEFAULT_DISTRIBUTION_EXAMPLES,
};
