import { z } from "zod";

import type {
  BondSpec,
  CurvePointWire,
  DiscountCurveWire,
  MarketContextWire,
  ValuationResultWire,
} from "./generated";

const isoDate = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}/, "ISO 8601 date string expected");

export const CurvePointWireSchema: z.ZodType<CurvePointWire> = z.object({
  tenor_years: z.number(),
  discount_factor: z.number(),
});

export const DiscountCurveWireSchema: z.ZodType<DiscountCurveWire> = z.object({
  id: z.string().min(1, "discount_curve_id required"),
  base_date: isoDate,
  points: z
    .array(CurvePointWireSchema)
    .min(1, "at least one curve point is required"),
});

export const MarketContextWireSchema: z.ZodType<MarketContextWire> = z.object({
  as_of: isoDate,
  discount_curves: z.array(DiscountCurveWireSchema),
});

export const BondSpecSchema: z.ZodType<BondSpec> = z.object({
  id: z.string().min(1, "bond id required"),
  currency: z.string().length(3, "ISO 4217 currency code required"),
  notional: z.number(),
  coupon_rate: z.number(),
  issue: isoDate,
  maturity: isoDate,
  discount_curve_id: z.string().min(1, "discount_curve_id required"),
  credit_curve_id: z.string().min(1).nullable().optional(),
});

export const ValuationResultWireSchema: z.ZodType<ValuationResultWire> = z
  .object({
    instrument_id: z.string().min(1, "instrument_id required"),
    present_value: z.number(),
    currency: z.string().length(3, "ISO 4217 currency code required"),
    as_of: isoDate,
    metrics: z.record(z.number()).default({}),
  })
  .passthrough();

export const EngineSchemas = {
  isoDate,
  CurvePointWireSchema,
  DiscountCurveWireSchema,
  MarketContextWireSchema,
  BondSpecSchema,
  ValuationResultWireSchema,
};

export const IsoDateString = isoDate;

export type {
  BondSpec,
  CurvePointWire,
  DiscountCurveWire,
  MarketContextWire,
  ValuationResultWire,
};
