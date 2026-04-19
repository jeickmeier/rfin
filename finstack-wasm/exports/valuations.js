import * as wasm from '../pkg/finstack_wasm.js';
import { correlation } from './valuations/correlation.js';

export const valuations = {
  correlation,
  instruments: {
    validateInstrumentJson: wasm.validateInstrumentJson,
    priceInstrument: wasm.priceInstrument,
    priceInstrumentWithMetrics: wasm.priceInstrumentWithMetrics,
    listStandardMetrics: wasm.listStandardMetrics,
    listStandardMetricsGrouped: wasm.listStandardMetricsGrouped,
  },
  validateValuationResultJson: wasm.validateValuationResultJson,
  validateInstrumentJson: wasm.validateInstrumentJson,
  priceInstrument: wasm.priceInstrument,
  priceInstrumentWithMetrics: wasm.priceInstrumentWithMetrics,
  listStandardMetrics: wasm.listStandardMetrics,
  listStandardMetricsGrouped: wasm.listStandardMetricsGrouped,
  attributePnl: wasm.attributePnl,
  attributePnlFromSpec: wasm.attributePnlFromSpec,
  validateAttributionJson: wasm.validateAttributionJson,
  defaultWaterfallOrder: wasm.defaultWaterfallOrder,
  defaultAttributionMetrics: wasm.defaultAttributionMetrics,
  computeFactorSensitivities: wasm.computeFactorSensitivities,
  computePnlProfiles: wasm.computePnlProfiles,
  decomposeFactorRisk: wasm.decomposeFactorRisk,
};
