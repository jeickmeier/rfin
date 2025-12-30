import type { ChangeEvent } from "react";
import { cn } from "../../utils/cn";
import { Input } from "../ui/input";

export interface RateInputProps {
  value: number;
  onChange: (rate: number) => void;
  placeholder?: string;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  className?: string;
}

export function RateInput({
  value,
  onChange,
  placeholder = "0.05",
  min,
  max,
  step = 0.0001,
  disabled,
  className,
}: RateInputProps) {
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const next = Number(event.target.value);
    if (Number.isNaN(next)) {
      return;
    }
    if (typeof min === "number" && next < min) return;
    if (typeof max === "number" && next > max) return;
    onChange(next);
  };

  return (
    <Input
      type="number"
      step={step}
      min={min}
      max={max}
      className={cn("w-full", className)}
      value={value}
      onChange={handleChange}
      placeholder={placeholder}
      disabled={disabled}
      aria-label="rate-input"
    />
  );
}
