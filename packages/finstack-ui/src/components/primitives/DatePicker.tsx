import type { ChangeEvent } from "react";
import { cn } from "../../utils/cn";
import { Input } from "../ui/input";

export interface DatePickerProps {
  value: string;
  onChange: (dateIso: string) => void;
  min?: string;
  max?: string;
  disabled?: boolean;
  className?: string;
  placeholder?: string;
}

/**
 * Business-day aware shell; calendars can be injected later.
 */
export function DatePicker({
  value,
  onChange,
  min,
  max,
  disabled,
  className,
  placeholder = "YYYY-MM-DD",
}: DatePickerProps) {
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    onChange(event.target.value);
  };

  return (
    <Input
      type="date"
      aria-label="date-picker"
      className={cn("w-full", className)}
      value={value}
      onChange={handleChange}
      min={min}
      max={max}
      disabled={disabled}
      placeholder={placeholder}
    />
  );
}
