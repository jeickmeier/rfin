/**
 * Statements modeling fixture data.
 */

 

// Node value specification
export interface NodeValueData {
  nodeId: string;
  values: { [period: string]: number };
}

// Forecast specification
export interface NodeForecastData {
  nodeId: string;
  type: 'growth' | 'curve' | 'constant';
  value: number | number[];
}

// Formula specification
export interface NodeFormulaData {
  nodeId: string;
  formula: string;
}

// Model specification
export interface StatementsModelData {
  name: string;
  periodRange: string;
  actualsThrough: string | null;
  values: NodeValueData[];
  forecasts: NodeForecastData[];
  formulas: NodeFormulaData[];
}

export interface StatementsModeingProps {
  models?: StatementsModelData[];
}

// Default basic P&L model
export const BASIC_PNL_MODEL: StatementsModelData = {
  name: 'Acme Corp P&L',
  periodRange: '2025Q1..Q4',
  actualsThrough: '2025Q1',
  values: [
    { nodeId: 'revenue', values: { '2025Q1': 1_000_000 } },
    { nodeId: 'opex', values: { '2025Q1': 250_000 } },
  ],
  forecasts: [
    { nodeId: 'revenue', type: 'growth', value: 0.1 },
    { nodeId: 'opex', type: 'growth', value: 0.05 },
  ],
  formulas: [
    { nodeId: 'cogs', formula: 'revenue * 0.6' },
    { nodeId: 'gross_profit', formula: 'revenue - cogs' },
    { nodeId: 'ebitda', formula: 'gross_profit - opex' },
  ],
};

// Forecast demo model
export const FORECAST_DEMO_MODEL: StatementsModelData = {
  name: 'Forecast Demo',
  periodRange: '2025Q1..Q4',
  actualsThrough: '2025Q1',
  values: [
    { nodeId: 'revenue', values: { '2025Q1': 1_000_000 } },
    { nodeId: 'expenses', values: { '2025Q1': 800_000 } },
  ],
  forecasts: [
    { nodeId: 'revenue', type: 'growth', value: 0.05 },
    { nodeId: 'expenses', type: 'curve', value: [0.02, 0.03, 0.04] },
  ],
  formulas: [
    { nodeId: 'net_income', formula: 'revenue - expenses' },
  ],
};

// Complete example model
export const COMPLETE_EXAMPLE_MODEL: StatementsModelData = {
  name: 'Complete Model',
  periodRange: '2024Q1..2025Q4',
  actualsThrough: '2024Q4',
  values: [
    {
      nodeId: 'revenue',
      values: {
        '2024Q1': 900_000,
        '2024Q2': 950_000,
        '2024Q3': 975_000,
        '2024Q4': 1_000_000,
      },
    },
    {
      nodeId: 'opex',
      values: {
        '2024Q1': 200_000,
        '2024Q2': 210_000,
        '2024Q3': 220_000,
        '2024Q4': 230_000,
      },
    },
  ],
  forecasts: [
    { nodeId: 'revenue', type: 'growth', value: 0.08 },
    { nodeId: 'opex', type: 'growth', value: 0.04 },
  ],
  formulas: [
    { nodeId: 'cogs', formula: 'revenue * 0.60' },
    { nodeId: 'gross_profit', formula: 'revenue - cogs' },
    { nodeId: 'ebitda', formula: 'gross_profit - opex' },
    { nodeId: 'gross_margin', formula: 'gross_profit / revenue' },
    { nodeId: 'ebitda_margin', formula: 'ebitda / revenue' },
  ],
};

// Default models
export const DEFAULT_STATEMENTS_MODELS: StatementsModelData[] = [
  BASIC_PNL_MODEL,
  FORECAST_DEMO_MODEL,
  COMPLETE_EXAMPLE_MODEL,
];

// Complete props bundle
export const DEFAULT_STATEMENTS_PROPS: StatementsModeingProps = {
  models: DEFAULT_STATEMENTS_MODELS,
};
