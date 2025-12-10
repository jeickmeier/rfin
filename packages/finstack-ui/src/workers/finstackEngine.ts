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
  cashflows?: Array<{
    period: number;
    leg: string;
    rate: string;
    notional: string;
    discount_factor: string;
    present_value: string;
  }>;
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

export interface WorkerCalibrationResult {
  curveId: string;
  points: { tenor: string; rate: string }[];
  diagnostics?: string[];
  error?: ReturnType<typeof normalizeError>;
}

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
    console.log("[Worker] initialize called");
    try {
      await ensureWorkerWasmInit();
      console.log("[Worker] WASM init complete");
      storedConfigJson = configJson ?? null;
      storedRounding = extractRounding(configJson);
      if (marketJson) {
        storedMarketJson = marketJson;
      }
      return {
        configApplied: Boolean(configJson),
        marketApplied: Boolean(marketJson),
      };
    } catch (e) {
      console.error("[Worker] initialize failed", e);
      throw e;
    }
  },

  async loadMarket(marketJson: string): Promise<string> {
    console.log("[Worker] loadMarket called");
    await ensureWorkerWasmInit();
    storedMarketJson = marketJson;
    return marketHandle;
  },

  async priceInstrument(
    instrumentJson: string,
  ): Promise<WorkerValuationResult> {
    console.log("[Worker] priceInstrument called");
    try {
      await ensureWorkerWasmInit();
      const parsed = parseJsonSafe<Record<string, unknown>>(instrumentJson);
      const instrumentId =
        (parsed?.instrumentId as string | undefined) ??
        (parsed?.id as string | undefined) ??
        "instrument";
      const instrumentType = String(
        (parsed?.type as string | undefined) ??
          (parsed?.instrumentType as string | undefined) ??
          (parsed?.kind as string | undefined) ??
          "",
      );

      // Lightweight stub for swaps until WASM integration lands.
      if (instrumentType.toLowerCase().includes("swap")) {
        const legs = Array.isArray(parsed?.legs)
          ? (parsed.legs as Array<Record<string, unknown>>)
          : [];
        const fixedLeg = legs.find(
          (l) =>
            String(l.legType ?? l.type ?? l.kind).toLowerCase() === "fixed",
        );
        const floatLeg = legs.find(
          (l) =>
            String(l.legType ?? l.type ?? l.kind).toLowerCase() === "float",
        );
        const notional =
          Number(
            fixedLeg?.notional ?? floatLeg?.notional ?? parsed?.notional,
          ) || 1_000_000;
        const fixedRate = Number(fixedLeg?.rate ?? 0.03);
        const floatSpread = Number(floatLeg?.spread ?? 0.0005);
        const pv = (notional * (floatSpread - fixedRate) * 0.1).toFixed(2);
        const cashflows = [
          {
            period: 1,
            leg: "fixed",
            rate: String(fixedRate),
            notional: String(notional),
            discount_factor: "0.99",
            present_value: (notional * 0.0005).toFixed(2),
          },
          {
            period: 1,
            leg: "float",
            rate: String(floatSpread),
            notional: String(notional),
            discount_factor: "0.99",
            present_value: (notional * 0.0004).toFixed(2),
          },
        ];
        return {
          instrumentId,
          presentValue: pv,
          marketHandle: storedMarketJson ? marketHandle : null,
          meta: { rounding: storedRounding },
          cashflows,
          raw: { ...parsed, cashflows },
        };
      }

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
      const wasmAny = wasm as any;

      if (storedMarketJson) {
        // Try automatic deserialization first
        const marketCtor = wasm.MarketContext as unknown as MarketContextStatic;
        try {
          if (typeof marketCtor.fromJson === "function") {
            market = marketCtor.fromJson(storedMarketJson);
          } else if (typeof marketCtor.fromJSON === "function") {
            market = marketCtor.fromJSON(storedMarketJson);
          }
        } catch {
          // ignore
        }

        // Manual hydration if automatic failed or returned empty
        if (!market || (market as any).isEmpty?.()) {
          try {
            market = new wasm.MarketContext();
            const parsedMarket = parseJsonSafe<any>(storedMarketJson);

            if (parsedMarket?.discount_curves) {
              for (const curveData of parsedMarket.discount_curves) {
                try {
                  const id = curveData.id;
                  const baseDateParts = curveData.base_date
                    .split("-")
                    .map(Number);
                  const baseDate = new wasmAny.FsDate(
                    baseDateParts[0],
                    baseDateParts[1],
                    baseDateParts[2],
                  );
                  const times = curveData.points.map(
                    (p: any) => p.tenor_years || p.time,
                  );
                  const dfs = curveData.points.map(
                    (p: any) => p.discount_factor || p.df,
                  );

                  // Default params for DiscountCurve constructor
                  const curve = new wasmAny.DiscountCurve(
                    id,
                    baseDate,
                    new Float64Array(times),
                    new Float64Array(dfs),
                    "act_365f", // day_count
                    "log_linear", // interp
                    "flat_forward", // extrapolation
                    false, // require_monotonic
                  );
                  (market as any).insertDiscount(curve);
                } catch (e) {
                  console.warn(
                    `Failed to hydrate discount curve ${curveData.id}`,
                    e,
                  );
                }
              }
            }
          } catch (e) {
            console.error("Manual market hydration failed", e);
            market = null; // reset to force empty creation below if needed
          }
        }
      }

      if (!market) {
        try {
          market = new wasm.MarketContext();
        } catch {
          market = null;
        }
      }

      // Hydrate instrument using specific WASM constructors
      let instrument: unknown = null;

      if (
        parsed?.type === "Bond" ||
        parsed?.instrumentType === "Bond" ||
        parsed?.kind === "bond"
      ) {
        try {
          const id = String(parsed.instrumentId || parsed.id || "BOND");
          const notionalAmount = Number(parsed.notional ?? 0);
          const currencyCode = String(parsed.currency ?? "USD");
          const couponRate = Number(parsed.coupon_rate ?? parsed.rate ?? 0);
          const issueStr = String(parsed.issue ?? parsed.issue_date ?? "");
          const maturityStr = String(
            parsed.maturity ?? parsed.maturity_date ?? "",
          );
          const discountCurve = String(
            parsed.discount_curve_id ?? parsed.discount_curve ?? "USD-OIS",
          );

          const parseDate = (iso: string) => {
            const parts = iso.split("-").map(Number);
            if (parts.length !== 3) throw new Error(`Invalid date: ${iso}`);
            return new wasmAny.FsDate(parts[0], parts[1], parts[2]);
          };

          const notional = wasmAny.Money.fromCode(notionalAmount, currencyCode);
          const issue = parseDate(issueStr);
          const maturity = parseDate(maturityStr);

          console.log("[Worker] Bond.fixedSemiannual args:", {
            id,
            notionalAmount,
            couponRate,
            issueStr,
            maturityStr,
            discountCurve,
          });

          // Use fixedSemiannual for the demo
          instrument = wasmAny.Bond.fixedSemiannual(
            id,
            notional,
            couponRate,
            issue,
            maturity,
            discountCurve,
            undefined, // quoted_clean_price
          );
        } catch (e) {
          console.error("Failed to construct Bond from payload", e);
          // Fallthrough to null instrument
        }
      } else {
        // Try generic fromJson if available (e.g. for other types in future)
        const ctor = (wasm as unknown as { Bond?: BondFactory }).Bond;
        if (ctor?.fromJson) {
          try {
            instrument = ctor.fromJson(instrumentJson);
          } catch {
            /* ignore */
          }
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
        cashflows: Array.isArray((parsed as { cashflows?: unknown }).cashflows)
          ? ((parsed as { cashflows: WorkerValuationResult["cashflows"] })
              .cashflows ?? [])
          : undefined,
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

  async calibrateDiscountCurve(
    payloadJson: string,
  ): Promise<WorkerCalibrationResult> {
    try {
      await ensureWorkerWasmInit();
      const payload = parseJsonSafe<{
        quotes?: Array<{ tenor?: string; rate?: string }>;
        config?: { curve_id?: string };
      }>(payloadJson);
      const curveId = payload?.config?.curve_id ?? "curve";
      const points =
        payload?.quotes?.map((q, idx) => ({
          tenor: q.tenor ?? `${idx + 1}M`,
          rate: q.rate ?? "0.0",
        })) ?? [];
      return { curveId, points, diagnostics: ["calibration simulated"] };
    } catch (err) {
      return {
        curveId: "curve",
        points: [],
        error: normalizeError(err),
      };
    }
  },

  async calibrateForwardCurve(
    payloadJson: string,
  ): Promise<WorkerCalibrationResult> {
    try {
      await ensureWorkerWasmInit();
      const payload = parseJsonSafe<{
        quotes?: Array<{ tenor?: string; rate?: string }>;
        config?: { curve_id?: string };
      }>(payloadJson);
      const curveId = payload?.config?.curve_id ?? "curve-fwd";
      const points =
        payload?.quotes?.map((q, idx) => ({
          tenor: q.tenor ?? `${idx + 1}Y`,
          rate: q.rate ?? "0.0",
        })) ?? [];
      return { curveId, points, diagnostics: ["calibration simulated"] };
    } catch (err) {
      return {
        curveId: "curve-fwd",
        points: [],
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

console.log("[Worker] exposing api");
expose(api, self);
