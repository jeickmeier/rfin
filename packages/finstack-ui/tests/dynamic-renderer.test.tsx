import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  ComponentRegistry,
  defaultRegistry,
} from "../src/engine/ComponentRegistry";
import { DynamicRenderer } from "../src/engine/DynamicRenderer";

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
      props: { title: "USD OIS", curveId: "USD-OIS" },
      mode: "viewer" as const,
    },
    {
      id: "22222222-2222-4222-8222-222222222222",
      type: "BondPanel",
      props: { bondId: "US912828XG33", showCashflows: true },
      mode: "viewer" as const,
    },
  ],
  bindings: {},
  createdAt: "2024-01-01",
  updatedAt: "2024-01-02",
};

describe("DynamicRenderer", () => {
  it("renders dashboard components from the registry", () => {
    render(
      <DynamicRenderer dashboard={validDashboard} registry={defaultRegistry} />,
    );

    expect(screen.getByTestId("curve-chart")).toBeInTheDocument();
    expect(screen.getByTestId("bond-panel")).toBeInTheDocument();
  });

  it("emits onError for unregistered components", () => {
    const registry = new ComponentRegistry();
    const onError = vi.fn();

    render(
      <DynamicRenderer
        dashboard={validDashboard}
        registry={registry}
        onError={onError}
      />,
    );

    expect(screen.getAllByTestId("component-error").length).toBeGreaterThan(0);
    expect(onError).toHaveBeenCalled();
  });
});
