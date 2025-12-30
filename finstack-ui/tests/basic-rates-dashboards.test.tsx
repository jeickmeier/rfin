import { readFileSync } from "fs";
import path from "path";
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";

import { DashboardDefinitionSchema } from "../src/schemas/dashboard";
import { DynamicRenderer, defaultRegistry } from "../src/engine";
import { FinstackContext } from "../src/hooks/useFinstack";

const fixtureDir = path.resolve(
  __dirname,
  "../fixtures/dashboards/basic-rates",
);

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

function loadFixture(file: string) {
  const contents = readFileSync(path.join(fixtureDir, file), "utf-8");
  return JSON.parse(contents);
}

describe("basic rates dashboard fixtures", () => {
  it("validate JSON fixtures against schema", () => {
    const files = [
      "bond-and-curve.json",
      "swap-with-cashflows.json",
      "calibration-dashboard.json",
    ];
    for (const file of files) {
      const parsed = loadFixture(file);
      const validated = DashboardDefinitionSchema.parse(parsed);
      expect(validated.schemaVersion).toBe("1");
    }
  });

  it("renders fixtures through DynamicRenderer", () => {
    const dashboards = [
      loadFixture("bond-and-curve.json"),
      loadFixture("swap-with-cashflows.json"),
      loadFixture("calibration-dashboard.json"),
    ];

    dashboards.forEach((dashboard) => {
      render(
        <FinstackContext.Provider value={mockContextValue}>
          <DynamicRenderer dashboard={dashboard} registry={defaultRegistry} />
        </FinstackContext.Provider>,
      );
    });

    expect(screen.getAllByTestId("bond-panel").length).toBeGreaterThan(0);
    expect(screen.getAllByTestId("curve-chart").length).toBeGreaterThan(0);
    expect(screen.getAllByTestId("swap-panel").length).toBeGreaterThan(0);
    expect(screen.getAllByTestId("cashflow-waterfall").length).toBeGreaterThan(
      0,
    );
    expect(
      screen.getAllByTestId("discount-curve-calibration").length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByTestId("forward-curve-calibration").length,
    ).toBeGreaterThan(0);
  });
});
