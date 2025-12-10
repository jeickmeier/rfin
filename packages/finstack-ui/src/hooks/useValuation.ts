import { useCallback, useState } from "react";
import type { WorkerValuationResult } from "../workers/finstackEngine";
import { useFinstackEngine } from "./useFinstackEngine";

export type ValuationStatus = "idle" | "loading" | "success" | "error";

export interface UseValuationResult {
  status: ValuationStatus;
  result: WorkerValuationResult | null;
  error: Error | null;
  priceInstrument: (
    instrument: unknown,
  ) => Promise<WorkerValuationResult | null>;
  isReady: boolean;
}

export function useValuation(): UseValuationResult {
  const { priceInstrument, isReady } = useFinstackEngine();
  const [status, setStatus] = useState<ValuationStatus>("idle");
  const [result, setResult] = useState<WorkerValuationResult | null>(null);
  const [error, setError] = useState<Error | null>(null);

  const price = useCallback(
    async (instrument: unknown) => {
      if (!isReady) {
        setError(new Error("Engine not ready"));
        setStatus("error");
        return null;
      }
      setStatus("loading");
      setError(null);
      try {
        const json = JSON.stringify(instrument);
        const response = await priceInstrument(json);
        if (response.error) {
          const err = new Error(response.error.message ?? "valuation failed");
          setError(err);
          setStatus("error");
          setResult(response);
          return response;
        }
        setResult(response);
        setStatus("success");
        return response;
      } catch (err) {
        const asError = err instanceof Error ? err : new Error(String(err));
        setError(asError);
        setStatus("error");
        return null;
      }
    },
    [isReady, priceInstrument],
  );

  return { status, result, error, priceInstrument: price, isReady };
}
