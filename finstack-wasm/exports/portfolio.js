import * as wasm from '../pkg/finstack_wasm.js';

export const portfolio = {
  parsePortfolioSpec: wasm.parsePortfolioSpec,
  buildPortfolioFromSpec: wasm.buildPortfolioFromSpec,
  portfolioResultTotalValue: wasm.portfolioResultTotalValue,
  portfolioResultGetMetric: wasm.portfolioResultGetMetric,
  aggregateMetrics: wasm.aggregateMetrics,
  valuePortfolio: wasm.valuePortfolio,
  aggregateFullCashflows: wasm.aggregateFullCashflows,
  aggregateCashflows: wasm.aggregateCashflows,
  applyScenarioAndRevalue: wasm.applyScenarioAndRevalue,
  optimizePortfolio: wasm.optimizePortfolio,
};
