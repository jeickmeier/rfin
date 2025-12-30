import { describe, expect, it } from "vitest";

import {
  DashboardActionSchema,
  applyDashboardAction,
} from "../src/engine/dashboardActions";
import { DashboardDefinition } from "../src/schemas/dashboard";

const baseDashboard: DashboardDefinition = {
  schemaVersion: "1",
  id: "00000000-0000-4000-8000-000000000000",
  name: "Edit Dashboard",
  layout: {
    kind: "two_column",
    left: ["11111111-1111-4111-8111-111111111111"],
    right: ["22222222-2222-4222-8222-222222222222"],
  },
  components: [
    {
      id: "11111111-1111-4111-8111-111111111111",
      type: "CurveChart",
      props: { title: "USD OIS", curveId: "USD-OIS" },
      mode: "viewer",
    },
    {
      id: "22222222-2222-4222-8222-222222222222",
      type: "BondPanel",
      props: { bondId: "US912828XG33", showCashflows: true },
      mode: "viewer",
    },
  ],
  bindings: {},
  createdAt: "2024-01-01",
  updatedAt: "2024-01-02",
};

describe("dashboardActions", () => {
  it("adds a component into the requested layout column", () => {
    const action = DashboardActionSchema.parse({
      kind: "add_component",
      component: {
        id: "33333333-3333-4333-8333-333333333333",
        type: "SwapPanel",
        props: { swapId: "USD-IRS-5Y", showParLegs: false },
        mode: "viewer",
      },
      placement: { column: "right" },
    });

    const updated = applyDashboardAction(baseDashboard, action);
    expect(
      updated.components.some(
        (c) => c.id === "33333333-3333-4333-8333-333333333333",
      ),
    ).toBe(true);
    expect(updated.layout.kind).toBe("two_column");
    expect(updated.layout.right).toContain(
      "33333333-3333-4333-8333-333333333333",
    );
  });

  it("updates component props immutably", () => {
    const action = DashboardActionSchema.parse({
      kind: "update_component",
      id: "11111111-1111-4111-8111-111111111111",
      props: { title: "Updated" },
      mode: "editor",
    });

    const updated = applyDashboardAction(baseDashboard, action);
    const updatedComponent = updated.components.find(
      (c) => c.id === "11111111-1111-4111-8111-111111111111",
    );
    expect(updatedComponent?.props).toMatchObject({ title: "Updated" });
    expect(updatedComponent?.mode).toBe("editor");
    expect(
      baseDashboard.components.find(
        (c) => c.id === "11111111-1111-4111-8111-111111111111",
      )?.props,
    ).toMatchObject({
      title: "USD OIS",
    });
  });

  it("removes components and cleans layout references", () => {
    const action = DashboardActionSchema.parse({
      kind: "remove_component",
      id: "22222222-2222-4222-8222-222222222222",
    });

    const updated = applyDashboardAction(baseDashboard, action);
    expect(
      updated.components.find(
        (c) => c.id === "22222222-2222-4222-8222-222222222222",
      ),
    ).toBeUndefined();
    expect(updated.layout.kind).toBe("two_column");
    expect(updated.layout.right).not.toContain(
      "22222222-2222-4222-8222-222222222222",
    );
  });

  it("reorders a layout container", () => {
    const action = DashboardActionSchema.parse({
      kind: "reorder_components",
      container: { column: "left" },
      order: ["11111111-1111-4111-8111-111111111111"],
    });

    const updated = applyDashboardAction(baseDashboard, action);
    expect(updated.layout.kind).toBe("two_column");
    expect(updated.layout.left).toEqual([
      "11111111-1111-4111-8111-111111111111",
    ]);
  });
});
