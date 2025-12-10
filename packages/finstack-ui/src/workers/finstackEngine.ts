import { expose } from "comlink";
import { normalizeError } from "../utils/errors";
import type { RoundingContextInfo } from "../types/rounding";

interface InitializeResult {
  configApplied: boolean;
  marketApplied: boolean;
}

export interface WorkerValuationResult {
  instrumentId: string;
  presentValue: string;
  marketHandle: string | null;
  meta?: {
    rounding?: RoundingContextInfo;
  };
  raw?: unknown;
  error?: ReturnType<typeof normalizeError>;
}

type MarketContextLike = unknown;

type MarketContextStatic = {
  fromJson?: (json: string) => MarketContextLike;
  fromJSON?: (json: string) => MarketContextLike;
};

type BondFactory = {
  fromJson?: (json: string) => unknown;
};

type RegistryLike = {
  priceBond(
    instrument: unknown,
    method: string,
    market: MarketContextLike,
    asOf: unknown,
    currency: unknown | null,
  ): unknown;
};

async function ensureWorkerWasmInit() {
  if (
    !(self as unknown as { __finstackWasmInit?: Promise<void> })
      .__finstackWasmInit
  ) {
    (
      self as unknown as { __finstackWasmInit?: Promise<void> }
    ).__finstackWasmInit = (async () => {
      const mod = await import("finstack-wasm");
      const initFn =
        (mod as unknown as { default?: () => Promise<unknown> }).default ??
        (mod as unknown as { init?: () => Promise<unknown> }).init;
      if (typeof initFn === "function") {
        await initFn();
      }
    })();
  }

  await (self as unknown as { __finstackWasmInit?: Promise<void> })
    .__finstackWasmInit;
}

let marketHandle = "market-main";
let storedMarketJson: string | null = null;
let storedConfigJson: string | null = null;
let storedRounding: RoundingContextInfo | undefined;

function extractRounding(
  configJson?: string | null,
): RoundingContextInfo | undefined {
  if (!configJson) return undefined;
  try {
    const parsed = JSON.parse(configJson) as Record<string, unknown>;
    if (typeof parsed !== "object" || !parsed) return undefined;
    const label =
      (parsed.roundingModeLabel as string | undefined) ??
      (parsed.rounding?.label as string | undefined);
    const scale =
      (parsed.outputScale as number | undefined) ??
      (parsed.rounding?.scale as number | undefined) ??
      (parsed.scale as number | undefined);
    if (label || Number.isFinite(scale)) {
      return { label, scale: typeof scale === "number" ? scale : undefined };
    }
  } catch {
    /* ignore */
  }
  return undefined;
}

function parseJsonSafe<T = unknown>(payload: string): T | null {
  try {
    return JSON.parse(payload) as T;
  } catch {
    return null;
  }
}

const api = {
  async initialize(
    configJson?: string | null,
    marketJson?: string | null,
  ): Promise<InitializeResult> {
    await ensureWorkerWasmInit();
    storedConfigJson = configJson ?? null;
    storedRounding = extractRounding(configJson);
    if (marketJson) {
      storedMarketJson = marketJson;
    }
    return {
      configApplied: Boolean(configJson),
      marketApplied: Boolean(marketJson),
    };
  },

  async loadMarket(marketJson: string): Promise<string> {
    await ensureWorkerWasmInit();
    storedMarketJson = marketJson;
    return marketHandle;
  },

  async priceInstrument(
    instrumentJson: string,
  ): Promise<WorkerValuationResult> {
    try {
      await ensureWorkerWasmInit();
      const parsed = parseJsonSafe<Record<string, unknown>>(instrumentJson);
      const instrumentId =
        (parsed?.instrumentId as string | undefined) ??
        (parsed?.id as string | undefined) ??
        "instrument";

      const wasm = await import("finstack-wasm");
      const config = new wasm.FinstackConfig();
      if (storedConfigJson) {
        const cfgParsed =
          parseJsonSafe<Record<string, unknown>>(storedConfigJson);
        const outputScale = (cfgParsed?.outputScale ??
          cfgParsed?.rounding?.scale) as number | undefined;
        if (Number.isFinite(outputScale)) {
          try {
            config.setOutputScale(
              new wasm.Currency("USD"),
              outputScale as number,
            );
          } catch {
            /* best-effort */
          }
        }
        const label =
          (cfgParsed?.roundingModeLabel as string | undefined) ??
          (cfgParsed?.rounding?.label as string | undefined);
        if (label) {
          try {
            config.setRoundingModeLabel(label);
          } catch {
            /* ignore */
          }
        }
      }

      // Build market if possible
      let market: MarketContextLike | null = null;
      if (storedMarketJson) {
        const marketCtor = wasm.MarketContext as unknown as MarketContextStatic;
        try {
          if (typeof marketCtor.fromJson === "function") {
            market = marketCtor.fromJson(storedMarketJson);
          } else if (typeof marketCtor.fromJSON === "function") {
            market = marketCtor.fromJSON(storedMarketJson);
          }
        } catch {
          market = null;
        }
      }
      if (!market) {
        try {
          market = new wasm.MarketContext();
        } catch {
          market = null;
        }
      }

      // Try to hydrate instrument using known wasm constructors (Bond only for now)
      const instrumentCandidates: Array<() => unknown> = [];
      if (
        parsed?.type === "Bond" ||
        parsed?.instrumentType === "Bond" ||
        parsed?.kind === "bond"
      ) {
        const ctor = (wasm as unknown as { Bond?: BondFactory }).Bond;
        if (ctor?.fromJson) {
          instrumentCandidates.push(() => ctor.fromJson(instrumentJson));
        }
      }

      let instrument: unknown = null;
      for (const build of instrumentCandidates) {
        try {
          instrument = build();
          if (instrument) break;
        } catch {
          instrument = null;
        }
      }

      if (!market) {
        throw new Error("Market context unavailable for pricing");
      }

      if (!instrument) {
        throw new Error(
          "Unsupported instrument payload for finstack-wasm pricing",
        );
      }

      const wasmWithRegistry = wasm as {
        createStandardRegistry?: () => RegistryLike;
        PricerRegistry: new () => RegistryLike;
      };
      const registry: RegistryLike =
        typeof wasmWithRegistry.createStandardRegistry === "function"
          ? wasmWithRegistry.createStandardRegistry()
          : new wasmWithRegistry.PricerRegistry();
      const today = new Date();
      const FsDateCtor = (
        wasm as unknown as {
          FsDate: new (year: number, month: number, day: number) => unknown;
        }
      ).FsDate;
      const asOf = new FsDateCtor(
        today.getFullYear(),
        today.getMonth() + 1,
        today.getDate(),
      );
      const result = registry.priceBond(
        instrument,
        "discounting",
        market as MarketContextLike,
        asOf,
        null,
      );
      const pvMoney =
        (
          result as {
            presentValue?: {
              amount?: number;
              currency?: { code?: string };
            };
          }
        ).presentValue ?? {};
      const amount =
        typeof pvMoney.amount === "number" ? pvMoney.amount.toString() : "0";

      return {
        instrumentId,
        presentValue: amount,
        marketHandle: storedMarketJson ? marketHandle : null,
        meta: {
          rounding: storedRounding ?? {
            label: config.roundingMode?.toString(),
            scale: undefined,
          },
        },
        raw: parsed,
        error: undefined,
      };
    } catch (err) {
      return {
        instrumentId: "instrument",
        presentValue: "0",
        marketHandle: storedMarketJson ? marketHandle : null,
        error: normalizeError(err),
      };
    }
  },
};

export type FinstackEngineWorkerApi = typeof api;

// Expose internals for unit testing without altering worker surface.
export const __test__ = {
  ensureWorkerWasmInit,
  extractRounding,
  parseJsonSafe,
  api,
};

expose(api);
