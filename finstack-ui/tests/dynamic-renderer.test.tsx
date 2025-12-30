import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  ComponentRegistry,
  defaultRegistry,
} from "../src/engine/ComponentRegistry";
import { DynamicRenderer } from "../src/engine/DynamicRenderer";
import { FinstackContext } from "../src/hooks/useFinstack";

const validDashboard = {
  schemaVersion: "1" as const,
  id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  name: "Sample Dashboard",
  layout: {
    kind: "two_column" as const,
    left: ["11111111-1111-4111-8111-111111111111"],
    right: ["22222222-2222-4222-8222-222222222222"],
  },
  components: [
    {
      id: "11111111-1111-4111-8111-111111111111",
      type: "CurveChart",
      props: {
        title: "USD OIS",
        series: [
          {
            label: "Zero",
            points: [
              { tenor: "1M", rate: "0.05" },
              { tenor: "3M", rate: "0.052" },
            ],
          },
        ],
      },
      mode: "viewer" as const,
    },
    {
      id: "22222222-2222-4222-8222-222222222222",
      type: "BondPanel",
      props: { title: "USD Bond" },
      mode: "viewer" as const,
    },
  ],
  bindings: {},
  createdAt: "2024-01-01",
  updatedAt: "2024-01-02",
};

const mockContextValue = {
  isReady: false,
  isLoading: false,
  error: null,
  config: null,
  market: null,
  marketHandle: null,
  roundingContext: { label: "default", scale: 2 },
  worker: null,
  setMarket: async () => null,
};

describe("DynamicRenderer", () => {
  it("renders dashboard components from the registry", () => {
    render(
      <FinstackContext.Provider value={mockContextValue}>
        <DynamicRenderer
          dashboard={validDashboard}
          registry={defaultRegistry}
        />
      </FinstackContext.Provider>,
    );

    expect(screen.getByTestId("curve-chart")).toBeInTheDocument();
    expect(screen.getByTestId("bond-panel")).toBeInTheDocument();
  });

  it("emits onError for unregistered components", () => {
    const registry = new ComponentRegistry();
    const onError = vi.fn();

    render(
      <FinstackContext.Provider value={mockContextValue}>
        <DynamicRenderer
          dashboard={validDashboard}
          registry={registry}
          onError={onError}
        />
      </FinstackContext.Provider>,
    );

    expect(screen.getAllByTestId("component-error").length).toBeGreaterThan(0);
    expect(onError).toHaveBeenCalled();
  });
});
