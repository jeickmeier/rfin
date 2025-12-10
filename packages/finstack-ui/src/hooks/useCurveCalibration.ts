import { useCallback, useState } from "react";
import { useFinstackEngine } from "./useFinstackEngine";
import type {
  CalibrationConfig,
  CalibrationQuote,
} from "../schemas/valuations";

export type CalibrationStatus = "idle" | "loading" | "success" | "error";

export interface CalibrationResult {
  curveId: string;
  points: { tenor: string; rate: string }[];
  diagnostics?: string[];
  error?: string;
}

export function useCurveCalibration(kind: "discount" | "forward") {
  const { calibrateDiscountCurve, calibrateForwardCurve, isReady } =
    useFinstackEngine();
  const [status, setStatus] = useState<CalibrationStatus>("idle");
  const [result, setResult] = useState<CalibrationResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const calibrate = useCallback(
    async (quotes: CalibrationQuote[], config: CalibrationConfig) => {
      if (!isReady) {
        setError("Engine not ready");
        setStatus("error");
        return null;
      }
      setStatus("loading");
      setError(null);
      try {
        const payload = {
          quotes,
          config,
        };
        const fn =
          kind === "discount" ? calibrateDiscountCurve : calibrateForwardCurve;
        const res = await fn(JSON.stringify(payload));
        const normalized: CalibrationResult = {
          curveId: res.curveId ?? config.curve_id,
          points: res.points ?? [],
          diagnostics: res.diagnostics ?? [],
          error: res.error?.message,
        };
        setResult(normalized);
        setStatus(res.error ? "error" : "success");
        if (res.error) {
          setError(res.error.message);
        }
        return normalized;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setStatus("error");
        return null;
      }
    },
    [calibrateDiscountCurve, calibrateForwardCurve, isReady, kind],
  );

  return { status, result, error, calibrate };
}
