import type { ReactNode } from "react";
import { DynamicRenderer, defaultRegistry } from "../engine";
import type { DashboardDefinition } from "../schemas/dashboard";
import { FinstackProvider } from "../hooks/useFinstack";
import { defaultConfigJson, defaultMarketJson } from "./basicRatesDefaults";

const defaultDashboard: DashboardDefinition = {
  schemaVersion: "1",
  id: "55555555-5555-4555-8555-555555555555",
  name: "Interest Rates Starter",
  layout: {
    kind: "two_column",
    left: ["aaaa5555-aaaa-4aaa-8aaa-aaaa55555555"],
    right: [
      "bbbb5555-bbbb-4bbb-8bbb-bbbb55555555",
      "cccc5555-cccc-4ccc-8ccc-cccc55555555",
    ],
  },
  components: [
    {
      id: "aaaa5555-aaaa-4aaa-8aaa-aaaa55555555",
      type: "InterestRateSwapPanel",
      props: { title: "USD Interest Rate Swap" },
      mode: "viewer",
    },
    {
      id: "bbbb5555-bbbb-4bbb-8bbb-bbbb55555555",
      type: "DiscountCurveCalibration",
      props: {},
      mode: "editor",
    },
    {
      id: "cccc5555-cccc-4ccc-8ccc-cccc55555555",
      type: "ForwardCurveCalibration",
      props: {},
      mode: "editor",
    },
  ],
  bindings: {},
  userIntent:
    "Initialize WASM and render swap + discount/forward curve calibration",
  createdAt: "2024-01-04",
  updatedAt: "2024-01-04",
};

export interface InterestRateAppProps {
  dashboard?: DashboardDefinition;
  configJson?: string;
  marketJson?: string;
  autoInit?: boolean;
  children?: ReactNode;
}

export function InterestRateApp({
  dashboard = defaultDashboard,
  configJson = defaultConfigJson,
  marketJson = defaultMarketJson,
  autoInit = true,
  children,
}: InterestRateAppProps) {
  return (
    <FinstackProvider
      configJson={configJson}
      marketJson={marketJson}
      autoInit={autoInit}
    >
      <DynamicRenderer dashboard={dashboard} registry={defaultRegistry} />
      {children}
    </FinstackProvider>
  );
}
