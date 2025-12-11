import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("../src/hooks/useValuation", () => {
  return {
    useValuation: () => ({
      priceInstrument: vi.fn(),
      result: {
        presentValue: "0.00",
        diagnostics: ["Swap pricing unavailable (mock)"],
        error: { message: "Swap pricing not implemented" },
        raw: {},
        marketHandle: null,
        instrumentId: "swap-mock",
      },
      status: "error",
      error: new Error("Swap pricing not implemented"),
      isReady: true,
    }),
  };
});

import { InterestRateSwapPanel } from "../src/domains/valuations/instruments/InterestRateSwapPanel";

describe("InterestRateSwapPanel", () => {
  it("shows friendly diagnostics when swap pricing is unavailable", () => {
    render(<InterestRateSwapPanel />);

    expect(screen.getByTestId("swap-panel")).toBeInTheDocument();
    expect(screen.getByTestId("swap-error").textContent).toContain(
      "Swap pricing not implemented",
    );
    expect(
      screen.getByText("Swap pricing unavailable (mock)"),
    ).toBeInTheDocument();
  });
});
