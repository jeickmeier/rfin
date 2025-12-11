import { z } from "zod";

const IsoDate = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}$/, "must be an ISO date (YYYY-MM-DD)");

export const BondSpecSchema = z
  .object({
    id: z.string().min(1, "id is required"),
    currency: z.string().min(3, "currency code required"),
    notional: z.number().finite(),
    coupon_rate: z.number().finite(),
    issue: IsoDate,
    maturity: IsoDate,
    discount_curve_id: z.string().min(1, "discount curve id required"),
    credit_curve_id: z.string().nullable().optional(),
  })
  .strict();

export type BondSpecInput = z.infer<typeof BondSpecSchema>;

export const SwapLegSchema = z
  .object({
    id: z.string().min(1),
    side: z.enum(["pay", "receive"]),
    legType: z.enum(["fixed", "float"]),
    currency: z.string().min(3),
    notional: z.number().finite(),
    rate: z.number().finite().optional(),
    spread: z.number().finite().optional(),
    index: z.string().optional(),
    tenor: z.string().min(1),
    day_count: z.string().optional(),
    payment_frequency: z.string().optional(),
    discount_curve_id: z.string().min(1),
    forward_curve_id: z.string().nullable().optional(),
    start: IsoDate.optional(),
    maturity: IsoDate.optional(),
  })
  .strict();

export type SwapLegInput = z.infer<typeof SwapLegSchema>;

export const SwapSpecSchema = z
  .object({
    id: z.string().min(1),
    legs: z.array(SwapLegSchema).min(2, "at least two legs"),
    effective_date: IsoDate,
    maturity: IsoDate,
    currency: z.string().min(3).optional(),
    discounting_curve_id: z.string().optional(),
  })
  .strict();

export type SwapSpecInput = z.infer<typeof SwapSpecSchema>;

export const CashflowWireSchema = z
  .object({
    period: z.number().int(),
    leg: z.string(),
    rate: z.string(),
    notional: z.string(),
    discount_factor: z.string(),
    present_value: z.string(),
  })
  .strict();

export type CashflowWire = z.infer<typeof CashflowWireSchema>;

export const CalibrationQuoteSchema = z
  .object({
    id: z.string().min(1),
    instrument: z.string().min(1),
    tenor: z.string().optional(),
    rate: z.string().min(1),
    curve_id: z.string().min(1),
  })
  .strict();

export type CalibrationQuote = z.infer<typeof CalibrationQuoteSchema>;

export const CalibrationConfigSchema = z
  .object({
    curve_id: z.string().min(1),
    interpolation: z
      .enum(["linear", "cubic", "log_linear"])
      .default("linear")
      .optional(),
    solver: z
      .object({
        tolerance: z.number().positive().max(1).optional(),
        max_iter: z.number().int().positive().max(10000).optional(),
      })
      .default({})
      .optional(),
  })
  .strict();

export type CalibrationConfig = z.infer<typeof CalibrationConfigSchema>;
