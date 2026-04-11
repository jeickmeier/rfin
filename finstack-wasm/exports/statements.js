import * as wasm from '../pkg/finstack_wasm.js';

export const statements = {
  validateFinancialModelJson: wasm.validateFinancialModelJson,
  modelNodeIds: wasm.modelNodeIds,
};
