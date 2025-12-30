import { describe, expect, it } from "vitest";
import {
  formatAmount,
  normalizeAmountInput,
  roundAmountString,
} from "../src/utils/amount";
import { normalizeError } from "../src/utils/errors";

describe("amount utils", () => {
  it("returns original value when scale is invalid", () => {
    expect(roundAmountString("123.45", -1)).toBe("123.45");
    expect(roundAmountString("123.45", Number.NaN)).toBe("123.45");
  });

  it("returns zero for empty or non-numeric inputs", () => {
    expect(roundAmountString("", 2)).toBe("0");
    expect(roundAmountString("   ", 2)).toBe("0");
  });

  it("rounds and formats with grouping", () => {
    expect(roundAmountString("1234.567", 2)).toBe("1,234.57");
    expect(
      formatAmount("1234.567", {
        currency: "USD",
        roundingContext: { scale: 2 },
      }),
    ).toBe("USD 1,234.57");
  });

  it("normalizes free-form numeric input safely", () => {
    expect(normalizeAmountInput("  -1,234.50.0 ")).toBe("-1234.50");
    expect(normalizeAmountInput("abc")).toBe("0");
  });
});

describe("error utils", () => {
  it("normalizes Error instances", () => {
    const err = new Error("boom");
    const normalized = normalizeError(err);
    expect(normalized.message).toBe("boom");
    expect(normalized.stack).toBeDefined();
  });

  it("normalizes string errors", () => {
    expect(normalizeError("oops").message).toBe("oops");
  });

  it("normalizes unknown errors", () => {
    expect(normalizeError({ detail: "bad" }).message).toBe("Unknown error");
  });
});
