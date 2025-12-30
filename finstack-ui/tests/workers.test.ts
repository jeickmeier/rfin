import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("comlink", () => ({
  expose: vi.fn(),
}));

type WasmMock = Record<string, unknown>;

function createWasmMock() {
  const initFn = vi.fn(async () => {});
  const priceBondFn = vi.fn(() => ({
    presentValue: {
      amount: 123.45,
      currency: { code: "USD" },
    },
  }));

  const mock: WasmMock = {
    default: initFn,
    Currency: class {
      constructor(public code: string) {}
    },
    FinstackConfig: class {
      roundingMode = "nearest";
      setOutputScale() {}
      setRoundingModeLabel() {}
    },
    MarketContext: class {
      static fromJson(json: string) {
        return { fromJson: json };
      }
      static fromJSON(json: string) {
        return { fromJSON: json };
      }
    },
    Bond: {
      fromJson: vi.fn((json: string) => ({ bondJson: json })),
      fixedSemiannual: vi.fn(
        (
          id: string,
          notional: unknown,
          couponRate: number,
          issue: unknown,
          maturity: unknown,
          discountCurve: string,
        ) => ({
          id,
          notional,
          couponRate,
          issue,
          maturity,
          discountCurve,
        }),
      ),
    },
    Money: {
      fromCode: (amount: number, currency: string) => ({ amount, currency }),
    },
    createStandardRegistry: vi.fn(() => ({
      priceBond: priceBondFn,
    })),
    PricerRegistry: class {
      priceBond = priceBondFn;
    },
    FsDate: class {
      constructor(
        public year: number,
        public month: number,
        public day: number,
      ) {}
    },
  };

  return { mock, initFn, priceBondFn };
}

async function loadWorker(mock: WasmMock) {
  vi.resetModules();
  vi.doMock("finstack-wasm", () => mock);
  const mod = await import("../src/workers/finstackEngine");
  return mod.__test__;
}

beforeEach(() => {
  // Make worker globals available in the jsdom test environment.
  (globalThis as unknown as { self: unknown }).self = globalThis;
  (
    globalThis as unknown as { __finstackWasmInit?: Promise<void> }
  ).__finstackWasmInit = undefined;
});

describe("finstackEngine worker", () => {
  it("initializes and prices a bond instrument with rounding context", async () => {
    const { mock, initFn, priceBondFn } = createWasmMock();
    const testApi = await loadWorker(mock);

    const initResult = await testApi.api.initialize(
      JSON.stringify({ outputScale: 2, roundingModeLabel: "nearest" }),
      JSON.stringify({ market: true }),
    );
    expect(initResult).toEqual({ configApplied: true, marketApplied: true });

    const result = await testApi.api.priceInstrument(
      JSON.stringify({
        instrumentId: "bond-1",
        type: "Bond",
        issue: "2024-01-01",
        maturity: "2029-01-01",
      }),
    );

    expect(initFn).toHaveBeenCalledTimes(1);
    expect(priceBondFn).toHaveBeenCalled();
    expect(result.presentValue).toBe("123.45");
    expect(result.marketHandle).toBe("market-main");
    expect(result.meta?.rounding?.label).toBe("nearest");
    expect(result.meta?.rounding?.scale).toBe(2);
  });

  it("returns normalized error for unsupported instruments", async () => {
    const { mock } = createWasmMock();
    const testApi = await loadWorker(mock);

    await testApi.api.initialize(null, null);
    const result = await testApi.api.priceInstrument("{}");

    expect(result.presentValue).toBe("0");
    expect(result.error?.message).toContain("Unsupported instrument");
  });

  it("extracts rounding from various config shapes", async () => {
    const testApi = await loadWorker(createWasmMock().mock);
    const config = JSON.stringify({
      rounding: { scale: 3, label: "bankers" },
    });
    const nested = testApi.extractRounding(config);
    expect(nested?.label).toBe("bankers");
    expect(nested?.scale).toBe(3);

    const legacy = testApi.extractRounding(
      JSON.stringify({ roundingModeLabel: "nearest", scale: 1 }),
    );
    expect(legacy?.label).toBe("nearest");
    expect(legacy?.scale).toBe(1);
  });

  it("caches WASM init promises", async () => {
    const { mock, initFn } = createWasmMock();
    const testApi = await loadWorker(mock);

    await testApi.ensureWorkerWasmInit();
    await testApi.ensureWorkerWasmInit();

    expect(initFn).toHaveBeenCalledTimes(1);
  });

  it("parses JSON safely", async () => {
    const testApi = await loadWorker(createWasmMock().mock);
    expect(testApi.parseJsonSafe<{ ok: boolean }>('{ "ok": true }')?.ok).toBe(
      true,
    );
    expect(testApi.parseJsonSafe("not-json")).toBeNull();
  });
});
