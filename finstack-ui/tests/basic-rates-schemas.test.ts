import { describe, expect, it } from "vitest";

import {
  BondSpecSchema,
  CalibrationConfigSchema,
  CalibrationQuoteSchema,
  SwapSpecSchema,
} from "../src/schemas/valuations";
import { SwapFormSchema } from "../src/domains/valuations/instruments/InterestRateSwapPanel";

describe("basic rates schemas", () => {
  it("validates BondSpec", () => {
    const parsed = BondSpecSchema.parse({
      id: "BOND-001",
      currency: "USD",
      notional: 1000000,
      coupon_rate: 0.05,
      issue: "2024-01-01",
      maturity: "2029-01-01",
      discount_curve_id: "USD-OIS",
      credit_curve_id: null,
    });
    expect(parsed.currency).toBe("USD");
  });

  it("validates SwapSpec", () => {
    const parsed = SwapSpecSchema.parse({
      id: "SWAP-001",
      effective_date: "2024-01-01",
      maturity: "2029-01-01",
      legs: [
        {
          id: "leg-fixed",
          side: "pay",
          legType: "fixed",
          currency: "USD",
          notional: 1000000,
          rate: 0.03,
          tenor: "6M",
          discount_curve_id: "USD-OIS",
        },
        {
          id: "leg-float",
          side: "receive",
          legType: "float",
          currency: "USD",
          notional: 1000000,
          spread: 0.0005,
          tenor: "6M",
          discount_curve_id: "USD-OIS",
          forward_curve_id: "USD-LIBOR",
        },
      ],
    });
    expect(parsed.legs).toHaveLength(2);
  });

  it("validates swap form preset schema", () => {
    const parsed = SwapFormSchema.parse({
      id: "SWAP-002",
      currency: "USD",
      notional: 500000,
      pay_fixed_rate: 0.031,
      receive_float_spread: 0.0004,
      effective_date: "2024-02-01",
      maturity: "2027-02-01",
      tenor: "3M",
      discount_curve_id: "USD-OIS",
      forward_curve_id: "USD-SOFFR",
    });
    expect(parsed.tenor).toBe("3M");
  });

  it("validates calibration quote and config", () => {
    const quote = CalibrationQuoteSchema.parse({
      id: "q1",
      instrument: "OIS",
      tenor: "1M",
      rate: "0.05",
      curve_id: "USD-OIS",
    });
    expect(quote.instrument).toBe("OIS");

    const config = CalibrationConfigSchema.parse({
      curve_id: "USD-OIS",
      interpolation: "linear",
      solver: { tolerance: 1e-8, max_iter: 100 },
    });
    expect(config.solver?.max_iter).toBe(100);
  });
});
