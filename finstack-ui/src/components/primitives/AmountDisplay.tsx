import { useContext, useMemo } from "react";
import type { RoundingContextInfo } from "../../types/rounding";
import { formatAmount } from "../../utils/amount";
import { cn } from "../../utils/cn";
import { FinstackContext } from "../../hooks/useFinstack";

export interface AmountDisplayProps {
  value: string;
  currency?: string;
  roundingContext?: RoundingContextInfo;
  className?: string;
}

const FALLBACK_ROUNDING: RoundingContextInfo = { scale: 2, label: "default" };

export function AmountDisplay({
  value,
  currency,
  roundingContext,
  className,
}: AmountDisplayProps) {
  const finstackCtx = useContext(FinstackContext);
  const resolvedRounding =
    roundingContext ?? finstackCtx?.roundingContext ?? FALLBACK_ROUNDING;

  const display = useMemo(
    () => formatAmount(value, { currency, roundingContext: resolvedRounding }),
    [currency, resolvedRounding, value],
  );

  return (
    <span className={cn("tabular-nums", className)} aria-label="amount-display">
      {display}
    </span>
  );
}
