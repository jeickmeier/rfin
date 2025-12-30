import { useCallback } from "react";
import type { Remote } from "comlink";
import { useFinstack } from "./useFinstack";
import type {
  FinstackEngineWorkerApi,
  WorkerCalibrationResult,
  WorkerValuationResult,
} from "../workers/finstackEngine";

interface UseFinstackEngineResult {
  worker: Remote<FinstackEngineWorkerApi> | null;
  isReady: boolean;
  isLoading: boolean;
  error: ReturnType<typeof useFinstack>["error"];
  priceInstrument: (instrumentJson: string) => Promise<WorkerValuationResult>;
  calibrateDiscountCurve: (
    payloadJson: string,
  ) => Promise<WorkerCalibrationResult>;
  calibrateForwardCurve: (
    payloadJson: string,
  ) => Promise<WorkerCalibrationResult>;
}

export function useFinstackEngine(): UseFinstackEngineResult {
  const { worker, isReady, isLoading, error } = useFinstack();

  const priceInstrument = useCallback(
    async (instrumentJson: string) => {
      if (!worker) {
        throw new Error("Finstack engine worker is not ready");
      }
      return worker.priceInstrument(instrumentJson);
    },
    [worker],
  );

  const calibrateDiscountCurve = useCallback(
    async (payloadJson: string) => {
      if (!worker) {
        throw new Error("Finstack engine worker is not ready");
      }
      return worker.calibrateDiscountCurve(payloadJson);
    },
    [worker],
  );

  const calibrateForwardCurve = useCallback(
    async (payloadJson: string) => {
      if (!worker) {
        throw new Error("Finstack engine worker is not ready");
      }
      return worker.calibrateForwardCurve(payloadJson);
    },
    [worker],
  );

  return {
    worker,
    isReady: isReady && Boolean(worker),
    isLoading,
    error,
    priceInstrument,
    calibrateDiscountCurve,
    calibrateForwardCurve,
  };
}
