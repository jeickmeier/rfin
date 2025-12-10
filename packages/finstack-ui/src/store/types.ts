import { z } from "zod";

import {
  MarketContextWireSchema,
  ValuationResultWireSchema,
} from "../schemas/engine";
import { DashboardDefinitionSchema } from "../schemas/dashboard";

export const EngineStateSchema = z.object({
  schemaVersion: z.literal("1"),
  market: MarketContextWireSchema.optional(),
  valuations: z.array(ValuationResultWireSchema).default([]),
  dashboards: z.array(DashboardDefinitionSchema).default([]),
});

export type EngineState = z.infer<typeof EngineStateSchema>;

export const UIStateSchema = z.object({
  activeView: z.string().optional(),
  panelState: z.record(z.unknown()).default({}),
  selections: z.record(z.unknown()).default({}),
  filters: z.record(z.unknown()).default({}),
});

export type UIState = z.infer<typeof UIStateSchema>;

export const RootStateSchema = z.object({
  engine: EngineStateSchema,
  ui: UIStateSchema,
});

export type RootState = z.infer<typeof RootStateSchema>;

export const createEmptyState = (): RootState => ({
  engine: {
    schemaVersion: "1",
    market: undefined,
    valuations: [],
    dashboards: [],
  },
  ui: {
    activeView: undefined,
    panelState: {},
    selections: {},
    filters: {},
  },
});
