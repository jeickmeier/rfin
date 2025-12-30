import { RoundingContextInfo } from "../types/rounding";

const DECIMAL_SANITIZER = /[^0-9.-]/g;

export interface FormatAmountOptions {
  currency?: string;
  roundingContext?: RoundingContextInfo;
}

/**
 * Round a decimal string without converting to JS number math.
 * Uses string/BigInt operations to avoid precision loss.
 */
export function roundAmountString(value: string, scale: number): string {
  if (!Number.isFinite(scale) || scale < 0) {
    return value;
  }

  const trimmed = value.trim();
  const isNegative = trimmed.startsWith("-");
  const sanitized = trimmed.replace(DECIMAL_SANITIZER, "");
  const withoutSign = isNegative ? sanitized.slice(1) : sanitized;

  if (!withoutSign) {
    return "0";
  }

  const [intPartRaw, fracPartRaw = ""] = withoutSign.split(".");
  const intPart = intPartRaw || "0";
  const fracPart = fracPartRaw;

  const combined = `${intPart}${fracPart.padEnd(scale + 1, "0")}`;
  const targetLength = intPart.length + scale;
  const roundingDigit = combined[targetLength] ?? "0";

  let rounded = BigInt(combined.slice(0, targetLength) || "0");
  if (roundingDigit >= "5") {
    rounded += BigInt(1);
  }

  const roundedStr = rounded.toString().padStart(targetLength || 1, "0");
  const integerPortion =
    scale === 0 ? roundedStr : roundedStr.slice(0, roundedStr.length - scale);
  const fractionalPortion =
    scale === 0 ? "" : roundedStr.slice(-scale).padEnd(scale, "0");

  const groupedInt = integerPortion.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const result = fractionalPortion
    ? `${groupedInt}.${fractionalPortion}`
    : groupedInt;
  return isNegative ? `-${result}` : result;
}

export function formatAmount(
  value: string,
  options: FormatAmountOptions = {},
): string {
  const scale = options.roundingContext?.scale ?? 2;
  const rounded = roundAmountString(value, scale);
  return options.currency ? `${options.currency} ${rounded}` : rounded;
}

export function normalizeAmountInput(raw: string): string {
  const trimmed = raw.trim();
  const negative = trimmed.startsWith("-");
  const body = negative ? trimmed.slice(1) : trimmed;
  const cleaned = body.replace(/[^0-9.]/g, "");

  const [intPart, fracPart = ""] = cleaned.split(".");
  const safeInt = intPart || "0";
  const safeFrac = fracPart.replace(/\./g, "");
  const rebuilt = safeFrac ? `${safeInt}.${safeFrac}` : safeInt;

  return negative ? `-${rebuilt}` : rebuilt;
}
