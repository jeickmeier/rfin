import type { ChangeEvent } from "react";
import { cn } from "../../utils/cn";
import { Input } from "../ui/input";

export interface TenorInputProps {
  value: string;
  onChange: (tenor: string) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
}

const TENOR_REGEX = /^[0-9]{0,3}[DWMY]?$/i;

export function TenorInput({
  value,
  onChange,
  placeholder = "6M",
  disabled,
  className,
}: TenorInputProps) {
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const next = event.target.value.toUpperCase();
    if (TENOR_REGEX.test(next)) {
      onChange(next);
    }
  };

  return (
    <Input
      aria-label="tenor-input"
      type="text"
      inputMode="text"
      maxLength={4}
      className={cn("w-full uppercase", className)}
      value={value}
      onChange={handleChange}
      placeholder={placeholder}
      disabled={disabled}
    />
  );
}
