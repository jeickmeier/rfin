import * as wasm from '../pkg/finstack_wasm.js';

export const valuations = {
  validateValuationResultJson: wasm.validateValuationResultJson,
  validateInstrumentJson: wasm.validateInstrumentJson,
  priceInstrument: wasm.priceInstrument,
  priceInstrumentWithMetrics: wasm.priceInstrumentWithMetrics,
  listStandardMetrics: wasm.listStandardMetrics,
  attributePnl: wasm.attributePnl,
  attributePnlFromSpec: wasm.attributePnlFromSpec,
  validateAttributionJson: wasm.validateAttributionJson,
  defaultWaterfallOrder: wasm.defaultWaterfallOrder,
  defaultAttributionMetrics: wasm.defaultAttributionMetrics,
  computeFactorSensitivities: wasm.computeFactorSensitivities,
  computePnlProfiles: wasm.computePnlProfiles,
  decomposeFactorRisk: wasm.decomposeFactorRisk,
};
