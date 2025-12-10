import React from "react";
import { z } from "zod";

import { ComponentMode } from "../schemas/dashboard";

export interface RegisteredComponent<TProps> {
  type: string;
  Component: React.ComponentType<TProps>;
  propsSchema: z.ZodType<TProps>;
  description: string;
  exampleProps: TProps;
  allowedModes: ComponentMode[];
}

export class ComponentRegistry {
  private readonly components = new Map<string, RegisteredComponent<unknown>>();

  register<TProps>(entry: RegisteredComponent<TProps>): void {
    const normalized: RegisteredComponent<TProps> = {
      ...entry,
      allowedModes:
        entry.allowedModes ?? (["viewer", "editor", "llm-assisted"] as const),
    };
    this.components.set(entry.type, normalized);
  }

  get<TProps>(type: string): RegisteredComponent<TProps> | undefined {
    return this.components.get(type) as RegisteredComponent<TProps> | undefined;
  }

  list(): RegisteredComponent<unknown>[] {
    return Array.from(this.components.values());
  }
}

const CurveChartPropsSchema = z.object({
  title: z.string().default("Curve Chart"),
  curveId: z.string().min(1),
});

const BondPanelPropsSchema = z.object({
  bondId: z.string().min(1),
  showCashflows: z.boolean().default(true),
});

const SwapPanelPropsSchema = z.object({
  swapId: z.string().min(1),
  showParLegs: z.boolean().default(true),
});

type CurveChartProps = z.infer<typeof CurveChartPropsSchema>;
type BondPanelProps = z.infer<typeof BondPanelPropsSchema>;
type SwapPanelProps = z.infer<typeof SwapPanelPropsSchema>;

const CurveChart: React.FC<CurveChartProps> = ({ title, curveId }) => (
  <div data-testid="curve-chart">
    <strong>{title}</strong>
    <div>Curve: {curveId}</div>
  </div>
);

const BondPanel: React.FC<BondPanelProps> = ({ bondId, showCashflows }) => (
  <div data-testid="bond-panel">
    <strong>Bond</strong>
    <div>ID: {bondId}</div>
    <div>Cashflows: {showCashflows ? "on" : "off"}</div>
  </div>
);

const SwapPanel: React.FC<SwapPanelProps> = ({ swapId, showParLegs }) => (
  <div data-testid="swap-panel">
    <strong>Swap</strong>
    <div>ID: {swapId}</div>
    <div>Par legs: {showParLegs ? "on" : "off"}</div>
  </div>
);

export const createDefaultRegistry = (): ComponentRegistry => {
  const registry = new ComponentRegistry();

  registry.register<CurveChartProps>({
    type: "CurveChart",
    Component: CurveChart,
    propsSchema: CurveChartPropsSchema,
    description: "Line chart for discount or forward curves",
    exampleProps: { title: "USD OIS", curveId: "USD-OIS" },
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  registry.register<BondPanelProps>({
    type: "BondPanel",
    Component: BondPanel,
    propsSchema: BondPanelPropsSchema,
    description: "Displays bond summary and cashflows",
    exampleProps: { bondId: "US912828XG33", showCashflows: true },
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  registry.register<SwapPanelProps>({
    type: "SwapPanel",
    Component: SwapPanel,
    propsSchema: SwapPanelPropsSchema,
    description: "Interest rate swap inputs and PV",
    exampleProps: { swapId: "USD-IRS-5Y", showParLegs: true },
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  return registry;
};

export const defaultRegistry = createDefaultRegistry();
