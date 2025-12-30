import { beforeEach, describe, expect, it, vi } from "vitest";

type WasmMock = Record<string, unknown>;

function createWasmMock(hasInit = true) {
  const initFn = vi.fn(async () => {});
  const mock: WasmMock = {
    default: hasInit ? initFn : undefined,
    init: hasInit ? undefined : undefined,
  };
  return { mock, initFn };
}

async function loadLib(mock: WasmMock) {
  vi.resetModules();
  vi.doMock("finstack-wasm", () => mock);
  return import("../src/lib/wasmSingleton");
}

beforeEach(() => {
  (globalThis as unknown as { window?: unknown }).window = {};
});

describe("wasmSingleton", () => {
  it("reports it can init in browser-like environments", async () => {
    const lib = await loadLib(createWasmMock().mock);
    expect(lib.canInitWasm()).toBe(true);
  });

  it("no-ops when window is unavailable", async () => {
    const { mock } = createWasmMock();
    const lib = await loadLib(mock);
    (globalThis as unknown as { window?: unknown }).window = undefined;
    await lib.ensureWasmInit();
    // Restore for other tests
    (globalThis as unknown as { window?: unknown }).window = {};
  });

  it("initializes only once even when called multiple times", async () => {
    const { mock, initFn } = createWasmMock();
    const lib = await loadLib(mock);
    lib.__resetWasmInitForTests();
    await lib.ensureWasmInit();
    await lib.ensureWasmInit();
    expect(initFn).toHaveBeenCalledTimes(1);
  });

  it("throws when no init function is available", async () => {
    const lib = await loadLib({ default: undefined, init: undefined });
    lib.__resetWasmInitForTests();
    await expect(lib.ensureWasmInit()).rejects.toThrow(
      "finstack-wasm init function not found",
    );
  });
});
