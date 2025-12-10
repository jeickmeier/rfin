import { cn } from "../../utils/cn";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";

const DEFAULT_CURRENCIES = ["USD", "EUR", "GBP", "JPY", "CHF", "AUD", "CAD"];

export interface CurrencySelectProps {
  value: string;
  onChange: (currency: string) => void;
  currencies?: string[];
  disabled?: boolean;
  className?: string;
}

export function CurrencySelect({
  value,
  onChange,
  currencies = DEFAULT_CURRENCIES,
  disabled,
  className,
}: CurrencySelectProps) {
  return (
    <Select value={value} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger
        className={cn("w-full aria-[disabled=true]:opacity-60", className)}
        aria-label="currency-select"
      >
        <SelectValue placeholder="Select currency" />
      </SelectTrigger>
      <SelectContent>
        {currencies.map((code) => (
          <SelectItem key={code} value={code}>
            {code}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
