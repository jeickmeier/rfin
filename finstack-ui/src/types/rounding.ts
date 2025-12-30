export interface RoundingContextInfo {
  label?: string;
  scale?: number;
  mode?: "nearest" | "up" | "down";
}

export interface MoneyDisplay {
  amount: string;
  currency?: string;
}
