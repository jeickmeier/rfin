import type { ReactNode } from "react";
import { DynamicRenderer, defaultRegistry } from "../engine";
import type { DashboardDefinition } from "../schemas/dashboard";
import { FinstackProvider } from "../hooks/useFinstack";
import { defaultConfigJson, defaultMarketJson } from "./basicRatesDefaults";

const defaultDashboard: DashboardDefinition = {
  schemaVersion: "1",
  id: "44444444-4444-4444-8444-444444444444",
  name: "Basic Rates Starter",
  layout: {
    kind: "two_column",
    left: ["aaaa4444-aaaa-4aaa-8aaa-aaaa44444444"],
    right: ["bbbb4444-bbbb-4bbb-8bbb-bbbb44444444"],
  },
  components: [
    {
      id: "aaaa4444-aaaa-4aaa-8aaa-aaaa44444444",
      type: "BondPanel",
      props: { title: "USD Bond" },
      mode: "viewer",
    },
    {
      id: "bbbb4444-bbbb-4bbb-8bbb-bbbb44444444",
      type: "DiscountCurveCalibration",
      props: {},
      mode: "editor",
    },
  ],
  bindings: {},
  userIntent: "Initialize WASM and render bond + discount calibration",
  createdAt: "2024-01-04",
  updatedAt: "2024-01-04",
};

export interface BasicRatesAppProps {
  dashboard?: DashboardDefinition;
  configJson?: string;
  marketJson?: string;
  autoInit?: boolean;
  children?: ReactNode;
}

export function BasicRatesApp({
  dashboard = defaultDashboard,
  configJson = defaultConfigJson,
  marketJson = defaultMarketJson,
  autoInit = true,
  children,
}: BasicRatesAppProps) {
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
