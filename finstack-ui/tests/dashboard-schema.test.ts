import { describe, expect, it } from "vitest";

import {
  DashboardDefinitionSchema,
  migrateDashboard,
} from "../src/schemas/dashboard";

const sampleDashboard = {
  schemaVersion: "1",
  id: "11111111-1111-4111-8111-111111111111",
  name: "Rates Overview",
  layout: {
    kind: "single" as const,
    components: ["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"],
  },
  components: [
    {
      id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      type: "CurveChart",
      props: { title: "USD OIS" },
      mode: "viewer" as const,
    },
  ],
  bindings: {
    curve: { source: "market", path: "discount_curves.USD-OIS" },
  },
  userIntent: "Display USD discount curve",
  createdAt: "2024-01-01",
  updatedAt: "2024-01-02",
};

describe("DashboardDefinitionSchema", () => {
  it("validates v1 dashboard definitions", () => {
    const parsed = DashboardDefinitionSchema.parse(sampleDashboard);
    expect(parsed.schemaVersion).toBe("1");
    expect(parsed.components).toHaveLength(1);
  });

  it("rejects unsupported migrations", () => {
    expect(() => migrateDashboard(sampleDashboard, "0", "1")).toThrow(
      /Unsupported dashboard migration/,
    );
  });
});
