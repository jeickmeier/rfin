import { useCallback, useEffect, useState } from "react";
import type { Remote } from "comlink";
import { useFinstack } from "./useFinstack";
import type {
  FinstackEngineWorkerApi,
  WorkerValuationResult,
} from "../workers/finstackEngine";

interface UseFinstackEngineResult {
  worker: Remote<FinstackEngineWorkerApi> | null;
  isReady: boolean;
  isLoading: boolean;
  error: ReturnType<typeof useFinstack>["error"];
  priceInstrument: (instrumentJson: string) => Promise<WorkerValuationResult>;
}

export function useFinstackEngine(): UseFinstackEngineResult {
  const { worker, isReady, isLoading, error } = useFinstack();
  const [engine, setEngine] = useState<Remote<FinstackEngineWorkerApi> | null>(
    worker,
  );

  useEffect(() => {
    setEngine(worker);
  }, [worker]);

  const priceInstrument = useCallback(
    async (instrumentJson: string) => {
      if (!engine) {
        throw new Error("Finstack engine worker is not ready");
      }
      return engine.priceInstrument(instrumentJson);
    },
    [engine],
  );

  return {
    worker: engine,
    isReady: isReady && Boolean(engine),
    isLoading,
    error,
    priceInstrument,
  };
}
