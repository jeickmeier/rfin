import type { ChangeEvent } from "react";
import { normalizeAmountInput } from "../../utils/amount";
import { cn } from "../../utils/cn";
import { Input } from "../ui/input";

export interface AmountInputProps {
  value: string;
  onChange: (value: string) => void;
  currency?: string;
  name?: string;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
}

export function AmountInput({
  value,
  onChange,
  currency,
  name,
  placeholder,
  disabled,
  className,
}: AmountInputProps) {
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const next = normalizeAmountInput(event.target.value);
    onChange(next);
  };

  return (
    <div className={cn("flex items-center gap-2", className)}>
      {currency ? (
        <span
          className="text-sm text-muted-foreground"
          aria-label="amount-input-currency"
        >
          {currency}
        </span>
      ) : null}
      <Input
        type="text"
        inputMode="decimal"
        className="flex-1"
        value={value}
        onChange={handleChange}
        name={name}
        placeholder={placeholder ?? "0.00"}
        disabled={disabled}
        aria-label="amount-input"
      />
    </div>
  );
}
