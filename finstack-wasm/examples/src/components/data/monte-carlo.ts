/**
 * Monte Carlo path generation fixture data.
 */

// GBM parameters
export interface GbmParamsData {
  initialSpot: number;
  riskFreeRate: number;
  dividendYield: number;
  volatility: number;
  timeToMaturity: number;
  numSteps: number;
  numPaths: number;
  captureMode: 'all' | 'sample';
  sampleCount: number;
  seed: bigint;
}

export interface MonteCarloPathProps {
  gbmParams?: GbmParamsData;
  maxRowsToDisplay?: number;
}

// Default GBM parameters
const DEFAULT_GBM_PARAMS: GbmParamsData = {
  initialSpot: 100,
  riskFreeRate: 0.05,
  dividendYield: 0.02,
  volatility: 0.25,
  timeToMaturity: 1,
  numSteps: 252, // Daily steps
  numPaths: 1000,
  captureMode: 'sample',
  sampleCount: 50,
  seed: BigInt(42),
};

// Complete props bundle
export const DEFAULT_MONTE_CARLO_PROPS: MonteCarloPathProps = {
  gbmParams: DEFAULT_GBM_PARAMS,
  maxRowsToDisplay: 20,
};

