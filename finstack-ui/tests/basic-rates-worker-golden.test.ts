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
  const priceSwapFn = vi.fn(() => ({
    presentValue: {
      amount: 456.78,
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
    InterestRateSwap: class {
      constructor(...args: unknown[]) {
        this.args = args;
      }
    },
    Money: {
      fromCode: (amt: number, currency: string) => ({ amount: amt, currency }),
    },
    createStandardRegistry: vi.fn(() => ({
      priceBond: priceBondFn,
      priceInterestRateSwap: priceSwapFn,
    })),
    PricerRegistry: class {
      priceBond = priceBondFn;
      priceInterestRateSwap = priceSwapFn;
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
  (globalThis as unknown as { self: unknown }).self = globalThis;
  (
    globalThis as unknown as { __finstackWasmInit?: Promise<void> }
  ).__finstackWasmInit = undefined;
});

describe("basic rates golden flows", () => {
  it("prices bond with golden PV and rounding", async () => {
    const { mock, priceBondFn } = createWasmMock();
    const testApi = await loadWorker(mock);

    await testApi.api.initialize(
      JSON.stringify({ outputScale: 2, roundingModeLabel: "nearest" }),
      JSON.stringify({ market: true }),
    );

    const result = await testApi.api.priceInstrument(
      JSON.stringify({
        instrumentId: "bond-golden",
        type: "Bond",
        issue: "2024-01-01",
        maturity: "2029-01-01",
      }),
    );

    expect(priceBondFn).toHaveBeenCalled();
    expect(result.presentValue).toBe("123.45");
    expect(result.meta?.rounding?.label).toBe("nearest");
  });

  it("prices swap using stubbed path", async () => {
    const { mock, initFn } = createWasmMock();
    const testApi = await loadWorker(mock);
    await testApi.api.initialize(null, null);
    const result = await testApi.api.priceInstrument(
      JSON.stringify({
        id: "swap-1",
        type: "InterestRateSwap",
        notional: 1000000,
        legs: [
          {
            id: "fixed",
            legType: "fixed",
            side: "pay",
            currency: "USD",
            notional: 1000000,
            rate: 0.0325,
            tenor: "6M",
            discount_curve_id: "USD-OIS",
          },
          {
            id: "float",
            legType: "float",
            side: "receive",
            currency: "USD",
            notional: 1000000,
            spread: 0.0005,
            tenor: "6M",
            discount_curve_id: "USD-OIS",
            forward_curve_id: "USD-LIBOR",
          },
        ],
        effective_date: "2024-01-01",
        maturity: "2029-01-01",
      }),
    );
    expect(result.presentValue).toBe("456.78");
    expect(result.diagnostics).toBeUndefined();
    expect(initFn).toHaveBeenCalled();
  });

  it("calibrates discount and forward curves", async () => {
    const testApi = await loadWorker(createWasmMock().mock);
    await testApi.api.initialize(null, null);
    const payload = JSON.stringify({
      config: { curve_id: "USD-OIS" },
      quotes: [{ tenor: "1M", rate: "0.05" }],
    });
    const discount = await testApi.api.calibrateDiscountCurve(payload);
    const forward = await testApi.api.calibrateForwardCurve(
      JSON.stringify({
        config: { curve_id: "USD-LIBOR" },
        quotes: [{ tenor: "1Y", rate: "0.055" }],
      }),
    );
    expect(discount.curveId).toBe("USD-OIS");
    expect(discount.points[0]?.rate).toBe("0.05");
    expect(forward.curveId).toBe("USD-LIBOR");
    expect(discount.simulated).toBe(true);
    expect(forward.simulated).toBe(true);
    expect(discount.diagnostics?.length).toBeGreaterThan(0);
  });
});
