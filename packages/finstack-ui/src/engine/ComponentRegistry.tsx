import React from "react";
import type { ColumnDef } from "@tanstack/react-table";
import { z } from "zod";

import { CurveChart } from "../components/charts";
import { VirtualDataTable } from "../components/tables";
import {
  BondPanel,
  CashflowWaterfall,
  DiscountCurveCalibration,
  InterestRateSwapPanel,
  ForwardCurveCalibration,
} from "../domains/valuations";
import { SwapFormSchema } from "../domains/valuations/instruments/InterestRateSwapPanel";
import { ComponentMode } from "../schemas/dashboard";
import { BondSpecSchema, CashflowWireSchema } from "../schemas/valuations";

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
  title: z.string().optional(),
  series: z.array(
    z.object({
      label: z.string(),
      points: z.array(
        z.object({
          tenor: z.string(),
          rate: z.string(),
        }),
      ),
      color: z.string().optional(),
    }),
  ),
  height: z.number().optional(),
  yLabel: z.string().optional(),
});

const BondPanelPropsSchema = z
  .object({
    title: z.string().optional(),
    preset: BondSpecSchema.partial().optional(),
  })
  .default({});

const SwapPanelPropsSchema = z
  .object({
    title: z.string().optional(),
    preset: SwapFormSchema.partial().optional(),
  })
  .default({});

const CalibrationPropsSchema = z.object({}).default({});

const CashflowWaterfallPropsSchema = z.object({
  cashflows: CashflowWireSchema.array(),
});

const VirtualDataTablePropsSchema = z.object({
  columns: z.array(
    z.object({
      header: z.string(),
      accessorKey: z.string(),
    }),
  ),
  data: z.array(z.record(z.string(), z.union([z.string(), z.number()]))),
  rowHeight: z.number().optional(),
  height: z.number().optional(),
});

type CurveChartProps = z.infer<typeof CurveChartPropsSchema>;
type BondPanelProps = z.infer<typeof BondPanelPropsSchema>;
type SwapPanelProps = z.infer<typeof SwapPanelPropsSchema>;
type CashflowWaterfallProps = z.infer<typeof CashflowWaterfallPropsSchema>;
type VirtualDataTableProps = z.infer<typeof VirtualDataTablePropsSchema>;

const CashflowWaterfallWrapper: React.FC<CashflowWaterfallProps> = ({
  cashflows,
}) => <CashflowWaterfall cashflows={cashflows} />;

const VirtualDataTableWrapper: React.FC<VirtualDataTableProps> = ({
  data,
  columns,
  rowHeight,
  height,
}) => {
  const columnDefs = columns.map((col) => ({
    header: col.header,
    accessorKey: col.accessorKey,
  })) as ColumnDef<Record<string, unknown>>[];

  return (
    <VirtualDataTable
      data={data}
      columns={columnDefs}
      rowHeight={rowHeight}
      height={height}
    />
  );
};

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
    description: "Displays bond inputs, PV, and cashflows",
    exampleProps: {
      title: "USD Bond",
      preset: BondSpecSchema.parse({
        id: "BOND-001",
        currency: "USD",
        notional: 1000000,
        coupon_rate: 0.05,
        issue: "2024-01-01",
        maturity: "2029-01-01",
        discount_curve_id: "USD-OIS",
        credit_curve_id: null,
      }),
    },
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  registry.register<SwapPanelProps>({
    type: "InterestRateSwapPanel",
    Component: InterestRateSwapPanel,
    propsSchema: SwapPanelPropsSchema,
    description: "Interest rate swap inputs and PV",
    exampleProps: {
      title: "USD IRS",
      preset: SwapFormSchema.parse({
        id: "SWAP-001",
        currency: "USD",
        notional: 1000000,
        pay_fixed_rate: 0.0325,
        receive_float_spread: 0.0005,
        effective_date: "2024-01-01",
        maturity: "2029-01-01",
        tenor: "6M",
        discount_curve_id: "USD-OIS",
        forward_curve_id: "USD-LIBOR",
      }),
    },
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  registry.register<z.infer<typeof CalibrationPropsSchema>>({
    type: "DiscountCurveCalibration",
    Component: DiscountCurveCalibration,
    propsSchema: CalibrationPropsSchema,
    description: "Quote grid and calibration UI for discount curves",
    exampleProps: {},
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  registry.register<z.infer<typeof CalibrationPropsSchema>>({
    type: "ForwardCurveCalibration",
    Component: ForwardCurveCalibration,
    propsSchema: CalibrationPropsSchema,
    description: "Quote grid and calibration UI for forward curves",
    exampleProps: {},
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  registry.register<CashflowWaterfallProps>({
    type: "CashflowWaterfall",
    Component: CashflowWaterfallWrapper,
    propsSchema: CashflowWaterfallPropsSchema,
    description: "Virtualized cashflow table",
    exampleProps: {
      cashflows: [
        {
          period: 1,
          leg: "fixed",
          rate: "0.05",
          notional: "1000000",
          discount_factor: "0.99",
          present_value: "5000",
        },
      ],
    },
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  registry.register<VirtualDataTableProps>({
    type: "VirtualDataTable",
    Component: VirtualDataTableWrapper,
    propsSchema: VirtualDataTablePropsSchema,
    description: "Virtualized table for large datasets",
    exampleProps: {
      columns: [
        { header: "Column A", accessorKey: "a" },
        { header: "Column B", accessorKey: "b" },
      ],
      data: [
        { a: "row-1-a", b: "row-1-b" },
        { a: "row-2-a", b: "row-2-b" },
      ],
      rowHeight: 32,
      height: 240,
    },
    allowedModes: ["viewer", "editor", "llm-assisted"],
  });

  return registry;
};

export const defaultRegistry = createDefaultRegistry();
