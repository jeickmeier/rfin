import * as wasm from '../pkg/finstack_wasm.js';

export const cashflows = {
  buildCashflowSchedule: wasm.buildCashflowSchedule,
  validateCashflowSchedule: wasm.validateCashflowSchedule,
  datedFlows: wasm.datedFlows,
  accruedInterest: wasm.accruedInterest,
  bondFromCashflows: wasm.bondFromCashflows,
};
