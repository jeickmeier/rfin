import { z } from "zod";

import { IsoDateString } from "./engine";

export const ComponentModeSchema = z.enum(["viewer", "editor", "llm-assisted"]);
export type ComponentMode = z.infer<typeof ComponentModeSchema>;

export const BindingSourceSchema = z.enum([
  "market",
  "portfolio",
  "statements",
  "scenarios",
]);
export type BindingSource = z.infer<typeof BindingSourceSchema>;

export const BindingPathSchema = z.object({
  source: BindingSourceSchema,
  path: z
    .string()
    .regex(
      /^[a-zA-Z0-9_.[\]-]+$/,
      "binding path must follow documented DSL (dot and bracket access only)",
    ),
  required: z.boolean().default(true),
  description: z.string().optional(),
});
export type BindingPath = z.infer<typeof BindingPathSchema>;

export const DataBindingsSchema = z.record(z.string(), BindingPathSchema);
export type DataBindings = z.infer<typeof DataBindingsSchema>;

export const ComponentInstanceSchema = z.object({
  id: z.string().uuid(),
  type: z.string().min(1, "component type is required"),
  props: z.record(z.unknown()).default({}),
  mode: ComponentModeSchema.default("viewer"),
});
export type ComponentInstance = z.infer<typeof ComponentInstanceSchema>;

const SingleLayoutSchema = z.object({
  kind: z.literal("single"),
  components: z.array(z.string().uuid()),
});

const TwoColumnLayoutSchema = z.object({
  kind: z.literal("two_column"),
  left: z.array(z.string().uuid()),
  right: z.array(z.string().uuid()),
});

const GridLayoutSchema = z.object({
  kind: z.literal("grid"),
  columns: z.number().int().min(1),
  rows: z.number().int().min(1),
  order: z.array(z.string().uuid()),
});

const TabSetLayoutSchema = z.object({
  kind: z.literal("tab_set"),
  tabs: z
    .array(
      z.object({
        id: z.string().uuid(),
        title: z.string().min(1),
        components: z.array(z.string().uuid()),
      }),
    )
    .min(1, "at least one tab is required"),
});

const ReportLayoutSchema = z.object({
  kind: z.literal("report"),
  sections: z
    .array(
      z.object({
        id: z.string().uuid(),
        title: z.string().min(1),
        components: z.array(z.string().uuid()),
      }),
    )
    .min(1, "at least one report section is required"),
});

export const LayoutTemplateSchema = z.discriminatedUnion("kind", [
  SingleLayoutSchema,
  TwoColumnLayoutSchema,
  GridLayoutSchema,
  TabSetLayoutSchema,
  ReportLayoutSchema,
]);
export type LayoutTemplate = z.infer<typeof LayoutTemplateSchema>;

export const DashboardDefinitionSchema = z.object({
  schemaVersion: z.literal("1"),
  id: z.string().uuid(),
  name: z.string().min(1, "dashboard name required"),
  layout: LayoutTemplateSchema,
  components: z.array(ComponentInstanceSchema),
  bindings: DataBindingsSchema.default({}),
  userIntent: z.string().optional(),
  createdAt: IsoDateString,
  updatedAt: IsoDateString,
});

export type DashboardDefinition = z.infer<typeof DashboardDefinitionSchema>;

export type VersionedDashboardDefinition = DashboardDefinition;

export const BindingExamples = {
  discountCurve: {
    source: "market",
    path: "discount_curves.USD-OIS",
    description: "Use USD OIS curve for discounting",
  },
  portfolioPv: {
    source: "portfolio",
    path: "books.CoreBook.pv",
    description: "Portfolio PV for CoreBook in base currency",
  },
  statementKpi: {
    source: "statements",
    path: "metrics.ebitda",
    description: "Statement KPI: EBITDA",
  },
};

export function migrateDashboard(
  data: unknown,
  fromVersion: string,
  toVersion: string,
): VersionedDashboardDefinition {
  if (fromVersion !== "1" || toVersion !== "1") {
    throw new Error(
      `Unsupported dashboard migration path: ${fromVersion} -> ${toVersion}`,
    );
  }

  return DashboardDefinitionSchema.parse(data);
}
