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
  diagnostics?: string[];
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
  simulated?: boolean;
  error?: ReturnType<typeof normalizeError>;
}

type WasmBindings = typeof import("finstack-wasm") & {
  FsDate: new (year: number, month: number, day: number) => unknown;
  // Date utilities - calendar-aware arithmetic
  addMonths: (date: unknown, months: number) => unknown;
  DiscountCurve: new (
    id: string,
    baseDate: unknown,
    times: Float64Array,
    dfs: Float64Array,
    dayCount: string,
    interpolation: string,
    extrapolation: string,
    requireMonotonic: boolean,
  ) => unknown;
  Bond: {
    fixedSemiannual: (
      id: string,
      notional: unknown,
      couponRate: number,
      issue: unknown,
      maturity: unknown,
      discountCurve: string,
      quoted_clean_price?: unknown,
    ) => unknown;
  };
  Money: { fromCode: (amount: number, currency: string) => unknown };
  InterestRateSwap: new (
    instrumentId: string,
    notional: unknown,
    fixedRate: number,
    start: unknown,
    end: unknown,
    discountCurve: string,
    forwardCurve: string,
    side: string,
    fixedFrequency?: unknown,
    fixedDayCount?: unknown,
    floatFrequency?: unknown,
    floatDayCount?: unknown,
    businessDayConvention?: unknown,
    calendarId?: unknown,
    stubKind?: unknown,
    resetLagDays?: unknown,
  ) => unknown;
  DiscountCurveCalibrator: new (
    curveId: string,
    baseDate: unknown,
    currency: string,
  ) => {
    withConfig: (config: unknown) => unknown;
    calibrate: (quotes: unknown[], market: MarketContextLike | null) => unknown;
  };
  ForwardCurveCalibrator: new (
    curveId: string,
    tenorYears: number,
    baseDate: unknown,
    currency: string,
    discountCurveId: string,
  ) => {
    withConfig: (config: unknown) => unknown;
    calibrate: (quotes: unknown[], market: MarketContextLike) => unknown;
  };
  RatesQuote: {
    deposit: (maturity: unknown, rate: number, dayCount: string) => unknown;
    swap: (
      maturity: unknown,
      rate: number,
      fixedFreq: unknown,
      floatFreq: unknown,
      fixedDayCount: string,
      floatDayCount: string,
      index: string,
    ) => unknown;
  };
  Frequency: {
    annual: () => unknown;
    quarterly: () => unknown;
  };
  CalibrationConfig: new () => {
    withTolerance: (tolerance: number) => unknown;
    withMaxIterations: (maxIterations: number) => unknown;
  };
  MarketContext: new () => unknown;
};

type HydratableMarket = {
  insertDiscount?: (curve: unknown) => void;
  isEmpty?: () => boolean;
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
  } catch (e) {
    console.warn("[Worker] failed to extract rounding context", e);
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

function parseIsoDate(
  iso: string,
  label = "date",
  FsDateCtor?: new (year: number, month: number, day: number) => unknown,
) {
  const isoPattern = /^\d{4}-\d{2}-\d{2}$/;
  if (!isoPattern.test(iso)) {
    throw new Error(`Invalid ${label}: ${iso}`);
  }
  const [year, month, day] = iso.split("-").map(Number);
  if (
    !Number.isFinite(year) ||
    !Number.isFinite(month) ||
    !Number.isFinite(day)
  ) {
    throw new Error(`Invalid ${label}: ${iso}`);
  }
  if (!FsDateCtor) {
    throw new Error("Date constructor unavailable");
  }
  return new FsDateCtor(year, month, day);
}

function parseTenorToMonths(tenorInput: string | undefined): number {
  const tenor = (tenorInput ?? "").trim().toUpperCase();
  if (!tenor) return 12;
  const match = /^(\d+)([DWMY])$/.exec(tenor);
  if (!match) return 12;
  const value = Number.parseInt(match[1], 10);
  const unit = match[2];
  switch (unit) {
    case "D":
      return Math.ceil(value / 30);
    case "W":
      return Math.ceil((value * 7) / 30);
    case "M":
      return value;
    case "Y":
      return value * 12;
    default:
      return 12;
  }
}

function parseTenorToYears(tenorInput: string | undefined): number {
  const tenor = (tenorInput ?? "").trim().toUpperCase();
  if (!tenor) {
    return 1;
  }
  const match = /^(\d+)([DWMY])$/.exec(tenor);
  if (!match) {
    throw new Error(`Invalid tenor: ${tenorInput}`);
  }
  const value = Number.parseInt(match[1], 10);
  const unit = match[2];
  switch (unit) {
    case "D":
      return value / 365;
    case "W":
      return (value * 7) / 365;
    case "M":
      return value / 12;
    case "Y":
      return value;
    default:
      throw new Error(`Unsupported tenor unit: ${unit}`);
  }
}

function buildMarketContext(
  wasm: typeof import("finstack-wasm"),
  diagnostics: string[],
): MarketContextLike | null {
  let market: MarketContextLike | null = null;
  const wasmAny = wasm as unknown as WasmBindings;

  if (storedMarketJson) {
    const marketCtor = wasm.MarketContext as unknown as MarketContextStatic;
    let hydratableMarket: HydratableMarket | null = market as HydratableMarket;
    try {
      if (typeof marketCtor.fromJson === "function") {
        market = marketCtor.fromJson(storedMarketJson);
      } else if (typeof marketCtor.fromJSON === "function") {
        market = marketCtor.fromJSON(storedMarketJson);
      }
    } catch (e) {
      diagnostics.push(
        "MarketContext fromJson failed; attempting manual hydration",
      );
      console.warn("[Worker] MarketContext fromJson failed", e);
    }

    hydratableMarket = market as HydratableMarket;
    if (!market || hydratableMarket?.isEmpty?.()) {
      try {
        market = new wasm.MarketContext();
        hydratableMarket = market as HydratableMarket;
        const parsedMarket = parseJsonSafe<{
          discount_curves?: Array<{
            id: string;
            base_date: string;
            points: Array<{
              tenor_years?: number;
              time?: number;
              discount_factor?: number;
              df?: number;
            }>;
          }>;
        }>(storedMarketJson);

        if (parsedMarket?.discount_curves) {
          for (const curveData of parsedMarket.discount_curves) {
            try {
              const id = curveData.id;
              const baseDate = parseIsoDate(
                curveData.base_date,
                "base_date",
                wasmAny.FsDate,
              );
              const times = curveData.points.map(
                (p) => p.tenor_years ?? p.time ?? 0,
              );
              const dfs = curveData.points.map(
                (p) => p.discount_factor ?? p.df ?? 0,
              );

              const curve = new wasmAny.DiscountCurve(
                id,
                baseDate,
                new Float64Array(times),
                new Float64Array(dfs),
                "act_365f",
                "log_linear",
                "flat_forward",
                false,
              );
              hydratableMarket?.insertDiscount?.(curve);
            } catch (e) {
              console.warn(
                `Failed to hydrate discount curve ${curveData.id}`,
                e,
              );
              diagnostics.push(
                `Failed to hydrate discount curve ${curveData.id}`,
              );
            }
          }
        }
      } catch (e) {
        console.error("Manual market hydration failed", e);
        diagnostics.push("Manual market hydration failed");
        market = null;
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

  return market;
}

const api = {
  async initialize(
    configJson?: string | null,
    marketJson?: string | null,
  ): Promise<InitializeResult> {
    console.warn("[Worker] initialize called");
    try {
      await ensureWorkerWasmInit();
      console.warn("[Worker] WASM init complete");
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
    console.warn("[Worker] loadMarket called");
    await ensureWorkerWasmInit();
    storedMarketJson = marketJson;
    return marketHandle;
  },

  async priceInstrument(
    instrumentJson: string,
  ): Promise<WorkerValuationResult> {
    console.warn("[Worker] priceInstrument called");
    const diagnostics: string[] = [];
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
          } catch (e) {
            diagnostics.push("Failed to apply output scale");
            console.warn("[Worker] output scale application failed", e);
          }
        }
        const label =
          (cfgParsed?.roundingModeLabel as string | undefined) ??
          (cfgParsed?.rounding?.label as string | undefined);
        if (label) {
          try {
            config.setRoundingModeLabel(label);
          } catch (e) {
            diagnostics.push("Failed to apply rounding mode label");
            console.warn("[Worker] rounding mode label failed", e);
          }
        }
      }

      const wasmAny = wasm as unknown as WasmBindings;
      const market = buildMarketContext(wasm, diagnostics);

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

          const notional = wasmAny.Money.fromCode(notionalAmount, currencyCode);
          const issue = parseIsoDate(issueStr, "issue", wasmAny.FsDate);
          const maturity = parseIsoDate(
            maturityStr,
            "maturity",
            wasmAny.FsDate,
          );

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
          diagnostics.push("Failed to construct Bond from payload");
          // Fallthrough to null instrument
        }
      } else if (instrumentType.toLowerCase().includes("swap")) {
        try {
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
          if (!fixedLeg || !floatLeg) {
            throw new Error("Swap requires fixed and floating legs");
          }

          const currencyCode = String(
            parsed?.currency ?? fixedLeg.currency ?? floatLeg.currency ?? "USD",
          );
          const notionalAmount = Number(
            fixedLeg.notional ?? floatLeg.notional ?? parsed?.notional,
          );
          if (!Number.isFinite(notionalAmount)) {
            throw new Error("Swap notional is required");
          }
          const fixedRate = Number(fixedLeg.rate ?? 0);
          const side =
            String(fixedLeg.side ?? "pay").toLowerCase() === "receive"
              ? "receive_fixed"
              : "pay_fixed";
          const start = parseIsoDate(
            String(
              parsed?.effective_date ??
                parsed?.start ??
                fixedLeg.start ??
                parsed?.trade_date ??
                parsed?.issue ??
                parsed?.issue_date ??
                "",
            ),
            "effective_date",
            wasmAny.FsDate,
          );
          const end = parseIsoDate(
            String(parsed?.maturity ?? floatLeg.maturity ?? ""),
            "maturity",
            wasmAny.FsDate,
          );
          const discountCurve = String(
            fixedLeg.discount_curve_id ??
              floatLeg.discount_curve_id ??
              parsed?.discount_curve_id ??
              parsed?.discounting_curve_id ??
              parsed?.discount_curve ??
              "USD-OIS",
          );
          const forwardCurve = String(
            floatLeg.forward_curve_id ??
              floatLeg.index ??
              parsed?.forward_curve_id ??
              parsed?.forward_curve ??
              parsed?.index ??
              discountCurve,
          );

          const notional = wasmAny.Money.fromCode(notionalAmount, currencyCode);
          instrument = new wasmAny.InterestRateSwap(
            instrumentId,
            notional,
            fixedRate,
            start,
            end,
            discountCurve,
            forwardCurve,
            side,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
          );
        } catch (e) {
          diagnostics.push("Failed to construct InterestRateSwap from payload");
          console.error("Failed to construct swap from payload", e);
          // Fallthrough to error return below
        }
      } else if (instrumentType) {
        // Try generic fromJson if available (e.g. for other types in future)
        const ctor = (wasm as unknown as { Bond?: BondFactory }).Bond;
        if (ctor?.fromJson) {
          try {
            instrument = ctor.fromJson(instrumentJson);
          } catch (e) {
            diagnostics.push("fromJson construction failed");
            console.warn("[Worker] fromJson construction failed", e);
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
      const result =
        instrumentType.toLowerCase().includes("swap") &&
        "priceInterestRateSwap" in registry
          ? registry.priceInterestRateSwap(
              instrument,
              "discounting",
              market as MarketContextLike,
              asOf,
              null,
            )
          : registry.priceBond(
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
        diagnostics: diagnostics.length ? diagnostics : undefined,
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
        diagnostics: diagnostics.length ? diagnostics : undefined,
        error: normalizeError(err),
      };
    }
  },

  async calibrateDiscountCurve(
    payloadJson: string,
  ): Promise<WorkerCalibrationResult> {
    const diagnostics: string[] = [];
    try {
      await ensureWorkerWasmInit();
      const wasm = (await import("finstack-wasm")) as unknown as WasmBindings;

      const payload = parseJsonSafe<{
        quotes?: Array<{
          id?: string;
          instrument?: string;
          tenor?: string;
          rate?: string;
          curve_id?: string;
        }>;
        config?: {
          curve_id?: string;
          interpolation?: string;
          solver?: { tolerance?: number; max_iter?: number };
        };
      }>(payloadJson);

      const curveId = payload?.config?.curve_id ?? "USD-OIS";
      const currency = curveId.split("-")[0] ?? "USD";

      // Create base date (today)
      const today = new Date();
      const baseDate = new wasm.FsDate(
        today.getFullYear(),
        today.getMonth() + 1,
        today.getDate(),
      );

      // Create calibrator
      const calibrator = new wasm.DiscountCurveCalibrator(
        curveId,
        baseDate,
        currency,
      );

      // Apply config if provided
      let configuredCalibrator = calibrator;
      if (payload?.config?.solver) {
        const calibConfig = new wasm.CalibrationConfig();
        if (payload.config.solver.tolerance) {
          configuredCalibrator = calibrator.withConfig(
            calibConfig.withTolerance(payload.config.solver.tolerance),
          ) as typeof calibrator;
        }
        if (payload.config.solver.max_iter) {
          configuredCalibrator = calibrator.withConfig(
            calibConfig.withMaxIterations(payload.config.solver.max_iter),
          ) as typeof calibrator;
        }
      }

      // Convert quotes to WASM RatesQuote objects
      // Use WASM's addMonths for proper calendar-aware date arithmetic
      const wasmQuotes: unknown[] = [];
      for (const quote of payload?.quotes ?? []) {
        const rateValue = parseFloat(quote.rate ?? "0");
        const tenorStr = quote.tenor ?? "1M";
        const tenorMonths = parseTenorToMonths(tenorStr);

        // Use WASM's addMonths for calendar-correct maturity date calculation
        const maturityDate = wasm.addMonths(baseDate, tenorMonths);

        // Create deposit or swap quote based on instrument type
        const instrument = quote.instrument?.toUpperCase() ?? "OIS";
        try {
          if (
            instrument === "DEPOSIT" ||
            instrument === "OIS" ||
            tenorMonths <= 12
          ) {
            wasmQuotes.push(
              wasm.RatesQuote.deposit(maturityDate, rateValue, "act_360"),
            );
            diagnostics.push(
              `Created deposit quote: ${tenorStr} @ ${rateValue}`,
            );
          } else {
            // For longer tenors, use swap quotes
            wasmQuotes.push(
              wasm.RatesQuote.swap(
                maturityDate,
                rateValue,
                wasm.Frequency.annual(),
                wasm.Frequency.quarterly(),
                "30_360",
                "act_360",
                curveId,
              ),
            );
            diagnostics.push(`Created swap quote: ${tenorStr} @ ${rateValue}`);
          }
        } catch (e) {
          diagnostics.push(
            `Failed to create quote for tenor ${tenorStr}: ${e}`,
          );
        }
      }

      diagnostics.push(`Total quotes created: ${wasmQuotes.length}`);

      if (wasmQuotes.length === 0) {
        diagnostics.push("No valid quotes provided for calibration");
        return {
          curveId,
          points: [],
          diagnostics,
          simulated: false,
          error: normalizeError(new Error("No valid quotes")),
        };
      }

      // Calibrate
      const market = new wasm.MarketContext();
      const result = configuredCalibrator.calibrate(
        wasmQuotes,
        market,
      ) as unknown[];

      // Result is [curve, report]
      const calibratedCurve = result[0] as {
        id: string;
        df: (t: number) => number;
        zero: (t: number) => number;
      };
      const report = result[1] as {
        success: boolean;
        iterations: number;
        convergenceReason: string;
        rmse: number;
      };

      diagnostics.push(
        `Calibration ${report.success ? "succeeded" : "failed"}`,
      );
      diagnostics.push(`Iterations: ${report.iterations}`);
      diagnostics.push(`Convergence: ${report.convergenceReason}`);
      if (report.rmse !== undefined) {
        diagnostics.push(`RMSE: ${report.rmse.toFixed(8)}`);
      }

      // Determine the max tenor from input quotes
      const quoteTenorYears = (payload?.quotes ?? []).map((q) =>
        parseTenorToYears(q.tenor),
      );
      const maxQuoteTenor = Math.max(...quoteTenorYears, 0.5);

      // Show rates at actual quote tenors for verification
      for (const quote of payload?.quotes ?? []) {
        const years = parseTenorToYears(quote.tenor);
        const calibratedRate = calibratedCurve.zero(years);
        diagnostics.push(
          `Quote ${quote.tenor}: input=${quote.rate}, calibrated=${(calibratedRate * 100).toFixed(4)}%`,
        );
      }

      // Extract curve points at standard tenors up to max quote tenor + buffer
      const allTenors = ["1M", "3M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y"];
      const standardTenors = allTenors.filter(
        (t) => parseTenorToYears(t) <= maxQuoteTenor * 1.5, // Allow 50% buffer beyond max quote
      );

      const points = standardTenors.map((tenor) => {
        const years = parseTenorToYears(tenor);
        const zeroRate = calibratedCurve.zero(years);
        return {
          tenor,
          rate: (zeroRate * 100).toFixed(4), // percentage as number string (e.g., "5.2000")
        };
      });

      return {
        curveId: calibratedCurve.id ?? curveId,
        points,
        diagnostics,
        simulated: false,
      };
    } catch (err) {
      console.error("[Worker] calibrateDiscountCurve failed", err);
      diagnostics.push(`Calibration error: ${String(err)}`);
      // Fall back to simulation on error
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
      return {
        curveId,
        points,
        diagnostics: [
          ...diagnostics,
          "Falling back to simulated calibration due to WASM error",
        ],
        simulated: true,
        error: normalizeError(err),
      };
    }
  },

  async calibrateForwardCurve(
    payloadJson: string,
  ): Promise<WorkerCalibrationResult> {
    const diagnostics: string[] = [];
    try {
      await ensureWorkerWasmInit();
      const wasm = (await import("finstack-wasm")) as unknown as WasmBindings;

      const payload = parseJsonSafe<{
        quotes?: Array<{
          id?: string;
          instrument?: string;
          tenor?: string;
          rate?: string;
          curve_id?: string;
        }>;
        config?: {
          curve_id?: string;
          discount_curve_id?: string;
          tenor_years?: number;
          interpolation?: string;
          solver?: { tolerance?: number; max_iter?: number };
        };
      }>(payloadJson);

      const curveId = payload?.config?.curve_id ?? "USD-LIBOR";
      const discountCurveId = payload?.config?.discount_curve_id ?? "USD-OIS";
      const tenorYears = payload?.config?.tenor_years ?? 0.25; // 3M default
      const currency = curveId.split("-")[0] ?? "USD";

      // Create base date (today)
      const today = new Date();
      const baseDate = new wasm.FsDate(
        today.getFullYear(),
        today.getMonth() + 1,
        today.getDate(),
      );

      // Create calibrator
      const calibrator = new wasm.ForwardCurveCalibrator(
        curveId,
        tenorYears,
        baseDate,
        currency,
        discountCurveId,
      );

      // Apply config if provided
      let configuredCalibrator = calibrator;
      if (payload?.config?.solver) {
        const calibConfig = new wasm.CalibrationConfig();
        if (payload.config.solver.tolerance) {
          configuredCalibrator = calibrator.withConfig(
            calibConfig.withTolerance(payload.config.solver.tolerance),
          ) as typeof calibrator;
        }
        if (payload.config.solver.max_iter) {
          configuredCalibrator = calibrator.withConfig(
            calibConfig.withMaxIterations(payload.config.solver.max_iter),
          ) as typeof calibrator;
        }
      }

      // Convert quotes to WASM RatesQuote objects (IRS quotes)
      // Use WASM's addMonths for proper calendar-aware date arithmetic
      const wasmQuotes: unknown[] = [];
      for (const quote of payload?.quotes ?? []) {
        const rateValue = parseFloat(quote.rate ?? "0");
        const tenorStr = quote.tenor ?? "2Y";
        const tenorMonths = parseTenorToMonths(tenorStr);

        // Use WASM's addMonths for calendar-correct maturity date calculation
        const maturityDate = wasm.addMonths(baseDate, tenorMonths);

        // For forward curve, use swap quotes
        try {
          wasmQuotes.push(
            wasm.RatesQuote.swap(
              maturityDate,
              rateValue,
              wasm.Frequency.annual(),
              wasm.Frequency.quarterly(),
              "30_360",
              "act_360",
              curveId,
            ),
          );
          diagnostics.push(`Created swap quote: ${tenorStr} @ ${rateValue}`);
        } catch (e) {
          diagnostics.push(
            `Failed to create quote for tenor ${tenorStr}: ${e}`,
          );
        }
      }

      diagnostics.push(`Total quotes created: ${wasmQuotes.length}`);

      if (wasmQuotes.length === 0) {
        diagnostics.push("No valid quotes provided for calibration");
        return {
          curveId,
          points: [],
          diagnostics,
          simulated: false,
          error: normalizeError(new Error("No valid quotes")),
        };
      }

      // Build a market context with the discount curve for forward calibration
      // Forward curve calibration requires a discount curve in the market
      const market = buildMarketContext(wasm, diagnostics);
      if (!market) {
        diagnostics.push(
          `Forward curve calibration requires discount curve ${discountCurveId} in market data`,
        );
        return {
          curveId,
          points: [],
          diagnostics,
          simulated: false,
          error: normalizeError(
            new Error(
              `Discount curve ${discountCurveId} not found. Please calibrate or load a discount curve first.`,
            ),
          ),
        };
      }

      // Calibrate
      const result = configuredCalibrator.calibrate(
        wasmQuotes,
        market,
      ) as unknown[];

      // Result is [curve, report]
      const calibratedCurve = result[0] as {
        id: string;
        rate: (t: number) => number;
      };
      const report = result[1] as {
        success: boolean;
        iterations: number;
        convergenceReason: string;
        rmse: number;
      };

      diagnostics.push(
        `Calibration ${report.success ? "succeeded" : "failed"}`,
      );
      diagnostics.push(`Iterations: ${report.iterations}`);
      diagnostics.push(`Convergence: ${report.convergenceReason}`);
      if (report.rmse !== undefined) {
        diagnostics.push(`RMSE: ${report.rmse.toFixed(8)}`);
      }

      // Determine the max tenor from input quotes to avoid wild extrapolation
      const quoteTenorYears = (payload?.quotes ?? []).map((q) =>
        parseTenorToYears(q.tenor),
      );
      const maxQuoteTenor = Math.max(...quoteTenorYears, 1);

      // Show rates at actual quote tenors for verification
      for (const quote of payload?.quotes ?? []) {
        const years = parseTenorToYears(quote.tenor);
        const calibratedRate = calibratedCurve.rate(years);
        diagnostics.push(
          `Quote ${quote.tenor}: input=${quote.rate}, calibrated=${(calibratedRate * 100).toFixed(4)}%`,
        );
      }

      // Extract curve points at standard tenors up to max quote tenor + buffer
      const allTenors = [
        "3M",
        "6M",
        "1Y",
        "2Y",
        "3Y",
        "5Y",
        "7Y",
        "10Y",
        "15Y",
        "20Y",
        "30Y",
      ];
      const standardTenors = allTenors.filter(
        (t) => parseTenorToYears(t) <= maxQuoteTenor * 1.2, // Allow 20% buffer beyond max quote
      );

      const points = standardTenors.map((tenor) => {
        const years = parseTenorToYears(tenor);
        const fwdRate = calibratedCurve.rate(years);
        return {
          tenor,
          rate: (fwdRate * 100).toFixed(4), // percentage as number string (e.g., "5.5000")
        };
      });

      return {
        curveId: calibratedCurve.id ?? curveId,
        points,
        diagnostics,
        simulated: false,
      };
    } catch (err) {
      console.error("[Worker] calibrateForwardCurve failed", err);
      diagnostics.push(`Calibration error: ${String(err)}`);
      // Fall back to simulation on error
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
      return {
        curveId,
        points,
        diagnostics: [
          ...diagnostics,
          "Falling back to simulated calibration due to WASM error",
        ],
        simulated: true,
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

expose(api, self);
