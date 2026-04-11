import * as wasm from '../pkg/finstack_wasm.js';

export const valuations = {
  validateValuationResultJson: wasm.validateValuationResultJson,
  validateInstrumentJson: wasm.validateInstrumentJson,
  priceInstrument: wasm.priceInstrument,
  priceInstrumentWithMetrics: wasm.priceInstrumentWithMetrics,
  listStandardMetrics: wasm.listStandardMetrics,
};
