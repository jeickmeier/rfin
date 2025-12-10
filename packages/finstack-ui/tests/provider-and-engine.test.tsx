import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { FinstackProvider, useFinstack } from "../src/hooks/useFinstack";
import { useFinstackEngine } from "../src/hooks/useFinstackEngine";
import { useEffect, useState } from "react";

const initializeMock = vi.fn(async () => ({
  configApplied: true,
  marketApplied: true,
}));
const loadMarketMock = vi.fn(async () => "market-main");
const priceInstrumentMock = vi.fn(async () => ({
  instrumentId: "bond-1",
  presentValue: "101.25",
  marketHandle: "market-main",
}));

vi.mock("../src/lib/wasmSingleton", () => ({
  ensureWasmInit: vi.fn(async () => Promise.resolve()),
  canInitWasm: vi.fn(() => true),
  __esModule: true,
}));

vi.mock("../src/workers/pool", () => ({
  getEngineWorker: vi.fn(async () => ({
    initialize: initializeMock,
    loadMarket: loadMarketMock,
    priceInstrument: priceInstrumentMock,
  })),
}));

function ProviderState() {
  const ctx = useFinstack();
  return (
    <div data-testid="state">
      {ctx.isReady ? "ready" : ctx.isLoading ? "loading" : "idle"}|
      {ctx.marketHandle ?? "none"}
    </div>
  );
}

function EngineProbe() {
  const { isReady, priceInstrument } = useFinstackEngine();
  const [pv, setPv] = useState("pending");

  useEffect(() => {
    if (isReady) {
      priceInstrument(
        JSON.stringify({ instrumentId: "bond-1", presentValue: "99.10" }),
      ).then((result) => setPv(result.presentValue));
    }
  }, [isReady, priceInstrument]);

  return <div data-testid="engine-pv">{pv}</div>;
}

describe("FinstackProvider and useFinstackEngine", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("initializes WASM + worker and exposes ready state", async () => {
    render(
      <FinstackProvider configJson="{}" marketJson="{}">
        <ProviderState />
        <EngineProbe />
      </FinstackProvider>,
    );

    await waitFor(() =>
      expect(screen.getByTestId("state").textContent).toContain(
        "ready|market-main",
      ),
    );
    await waitFor(() =>
      expect(screen.getByTestId("engine-pv").textContent).toBe("101.25"),
    );

    expect(initializeMock).toHaveBeenCalled();
    expect(loadMarketMock).toHaveBeenCalledWith("{}");
    expect(priceInstrumentMock).toHaveBeenCalled();
  });
});
