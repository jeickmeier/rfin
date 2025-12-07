/**
 * Portfolio example fixture data.
 */

import { DateData, DiscountCurveData, MoneyData } from './market-data';

// Entity data
export interface EntityData {
  id: string;
  name: string;
  tags: Record<string, string>;
}

// Bond instrument data for portfolio
export interface PortfolioBondData {
  id: string;
  notional: MoneyData;
  couponRate: number;
  issueDate: DateData;
  maturityDate: DateData;
  discountCurveId: string;
}

// Deposit instrument data for portfolio
export interface PortfolioDepositData {
  id: string;
  notional: MoneyData;
  startDate: DateData;
  endDate: DateData;
  dayCount: string;
  discountCurveId: string;
  quoteRate: number;
}

// Position data
export interface PositionData {
  positionId: string;
  entityId: string;
  instrumentType: 'bond' | 'deposit';
  instrumentRef: string; // Reference to bond or deposit by id
  quantity: number;
  unit: 'units' | 'notional';
}

// Portfolio configuration
export interface PortfolioConfigData {
  id: string;
  name: string;
  baseCurrency: string;
  tags: Record<string, string>;
}

export interface PortfolioExampleProps {
  valuationDate?: DateData;
  entities?: EntityData[];
  bonds?: PortfolioBondData[];
  deposits?: PortfolioDepositData[];
  positions?: PositionData[];
  portfolio?: PortfolioConfigData;
  discountCurve?: DiscountCurveData;
}

// Default valuation date
const DEFAULT_VALUATION_DATE: DateData = { year: 2024, month: 1, day: 2 };

// Default entities
const DEFAULT_ENTITIES: EntityData[] = [
  {
    id: 'CORP_A',
    name: 'Corporate A',
    tags: { sector: 'Finance' },
  },
  {
    id: 'FUND_B',
    name: 'Fund B',
    tags: { sector: 'Technology' },
  },
];

// Default bonds
const DEFAULT_BONDS: PortfolioBondData[] = [
  {
    id: 'BOND_CORP_A',
    notional: { amount: 5_000_000, currency: 'USD' },
    couponRate: 0.045,
    issueDate: { year: 2024, month: 1, day: 15 },
    maturityDate: { year: 2029, month: 1, day: 15 },
    discountCurveId: 'USD-OIS',
  },
];

// Default deposits
const DEFAULT_DEPOSITS: PortfolioDepositData[] = [
  {
    id: 'DEPOSIT_MM',
    notional: { amount: 2_000_000, currency: 'USD' },
    startDate: { year: 2024, month: 1, day: 2 },
    endDate: { year: 2024, month: 7, day: 2 },
    dayCount: 'act_360',
    discountCurveId: 'USD-OIS',
    quoteRate: 0.0525,
  },
];

// Default positions
const DEFAULT_POSITIONS: PositionData[] = [
  {
    positionId: 'POS_BOND_001',
    entityId: 'CORP_A',
    instrumentType: 'bond',
    instrumentRef: 'BOND_CORP_A',
    quantity: 1,
    unit: 'units',
  },
  {
    positionId: 'POS_DEP_001',
    entityId: 'FUND_B',
    instrumentType: 'deposit',
    instrumentRef: 'DEPOSIT_MM',
    quantity: 1,
    unit: 'units',
  },
];

// Default portfolio configuration
const DEFAULT_PORTFOLIO_CONFIG: PortfolioConfigData = {
  id: 'MULTI_ASSET_FUND',
  name: 'Multi-Asset Investment Fund',
  baseCurrency: 'USD',
  tags: {
    strategy: 'balanced',
    risk_profile: 'moderate',
  },
};

// Default discount curve for portfolio
const DEFAULT_PORTFOLIO_DISCOUNT_CURVE: DiscountCurveData = {
  id: 'USD-OIS',
  baseDate: DEFAULT_VALUATION_DATE,
  tenors: [0, 0.5, 1, 3, 5, 10],
  discountFactors: [1, 0.9975, 0.995, 0.975, 0.95, 0.9],
  dayCount: 'act_365f',
  interpolation: 'linear',
  extrapolation: 'flat_forward',
  continuous: true,
};

// Complete props bundle
export const DEFAULT_PORTFOLIO_PROPS: PortfolioExampleProps = {
  valuationDate: DEFAULT_VALUATION_DATE,
  entities: DEFAULT_ENTITIES,
  bonds: DEFAULT_BONDS,
  deposits: DEFAULT_DEPOSITS,
  positions: DEFAULT_POSITIONS,
  portfolio: DEFAULT_PORTFOLIO_CONFIG,
  discountCurve: DEFAULT_PORTFOLIO_DISCOUNT_CURVE,
};

