import {
  type ReactNode,
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { Remote } from "comlink";
import { canInitWasm, ensureWasmInit } from "../lib/wasmSingleton";
import type { FinstackEngineWorkerApi } from "../workers/finstackEngine";
import { getEngineWorker } from "../workers/pool";
import type { RoundingContextInfo } from "../types/rounding";
import { normalizeError, type NormalizedError } from "../utils/errors";

function parseObjectRecordOrNull(
  json: string | null | undefined,
): Record<string, unknown> | null {
  if (!json) {
    return null;
  }
  try {
    const parsed = JSON.parse(json) as unknown;
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    // ignore parse errors; callers fall back to null
  }
  return null;
}

export interface FinstackContextValue {
  isReady: boolean;
  isLoading: boolean;
  error: NormalizedError | null;
  /**
   * Parsed configuration wire object (JSON-serializable), if provided.
   * Engine configuration lives in the worker; this is a UI-facing view.
   */
  config: Record<string, unknown> | null;
  /**
   * Parsed market wire object (JSON-serializable), if provided.
   * Engine state lives in the worker; this is a UI-facing view.
   */
  market: Record<string, unknown> | null;
  marketHandle: string | null;
  roundingContext: RoundingContextInfo;
  worker: Remote<FinstackEngineWorkerApi> | null;
  setMarket: (marketJson: string) => Promise<string | null>;
}

interface FinstackProviderProps {
  children: ReactNode;
  configJson?: string;
  marketJson?: string;
  autoInit?: boolean;
  /**
   * When true, the provider will throw a promise while WASM/worker
   * initialization is pending so that callers can use React.Suspense.
   * Defaults to false to avoid surprising behavior.
   */
  suspense?: boolean;
}

export const FinstackContext = createContext<FinstackContextValue | undefined>(
  undefined,
);

export function FinstackProvider({
  children,
  configJson,
  marketJson,
  autoInit = true,
  suspense = false,
}: FinstackProviderProps) {
  const workerRef = useRef<Remote<FinstackEngineWorkerApi> | null>(null);
  const initPromiseRef = useRef<Promise<void> | null>(null);
  const [isReady, setReady] = useState(false);
  const [isLoading, setLoading] = useState(Boolean(autoInit));
  const [error, setError] = useState<NormalizedError | null>(null);
  const [config, setConfig] = useState<Record<string, unknown> | null>(null);
  const [market, setMarketState] = useState<Record<string, unknown> | null>(
    null,
  );
  const [marketHandle, setMarketHandle] = useState<string | null>(null);

  useEffect(() => {
    if (!autoInit) {
      return;
    }

    let cancelled = false;
    async function bootstrap() {
      if (!canInitWasm()) {
        setReady(false);
        setLoading(false);
        return;
      }

      setLoading(true);
      setError(null);

      try {
        await ensureWasmInit();
        const worker = await getEngineWorker();
        workerRef.current = worker;
        await worker.initialize(configJson ?? null, marketJson ?? null);
        // Parse config and market JSON for UI-facing context
        setConfig(parseObjectRecordOrNull(configJson));
        setMarketState(parseObjectRecordOrNull(marketJson));
        const handle = marketJson ? await worker.loadMarket(marketJson) : null;
        if (!cancelled) {
          setMarketHandle(handle);
          setReady(true);
        }
      } catch (err) {
        if (!cancelled) {
          setError(normalizeError(err));
          setReady(false);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    const promise = bootstrap();
    initPromiseRef.current = promise;
    return () => {
      cancelled = true;
    };
  }, [autoInit, configJson, marketJson]);

  const setMarket = async (marketJsonInput: string) => {
    if (!workerRef.current) {
      return null;
    }
    const handle = await workerRef.current.loadMarket(marketJsonInput);
    // Keep UI-facing copy of the market wire object in sync
    setMarketState(parseObjectRecordOrNull(marketJsonInput));
    setMarketHandle(handle);
    return handle;
  };

  const roundingContext: RoundingContextInfo = useMemo(
    () => ({
      label: "default",
      scale: 2,
    }),
    [],
  );

  // Optional Suspense integration – throw the initialization promise while
  // loading so that callers can wrap the provider in <Suspense>.
  if (
    autoInit &&
    suspense &&
    !isReady &&
    !error &&
    initPromiseRef.current
  ) {
    throw initPromiseRef.current;
  }

  const value = useMemo<FinstackContextValue>(
    () => ({
      isReady,
      isLoading,
      error,
      config,
      market,
      marketHandle,
      roundingContext,
      worker: workerRef.current,
      setMarket,
    }),
    [config, error, isLoading, isReady, market, marketHandle, roundingContext],
  );

  return (
    <FinstackContext.Provider value={value}>
      {children}
    </FinstackContext.Provider>
  );
}

export function useFinstack(): FinstackContextValue {
  const ctx = useContext(FinstackContext);
  if (!ctx) {
    throw new Error("useFinstack must be used within a FinstackProvider");
  }
  return ctx;
}
