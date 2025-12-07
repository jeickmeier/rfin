/**
 * Cashflow primitives fixture data.
 */

export interface CashflowData {
  label: string;
  type: 'fixed' | 'floating' | 'fee' | 'principalExchange';
  date: { year: number; month: number; day: number };
  amount: { amount: number; currency: string };
  accrualFactor?: number;
  resetDate?: { year: number; month: number; day: number };
}

export interface CashflowBasicsProps {
  cashflows?: CashflowData[];
}

// Default cashflows
export const DEFAULT_CASHFLOWS: CashflowData[] = [
  {
    label: 'Fixed coupon',
    type: 'fixed',
    date: { year: 2025, month: 3, day: 15 },
    amount: { amount: 12_500.0, currency: 'USD' },
    accrualFactor: 0.25,
  },
  {
    label: 'Floating coupon',
    type: 'floating',
    date: { year: 2025, month: 6, day: 15 },
    amount: { amount: 13_750.0, currency: 'USD' },
    resetDate: { year: 2025, month: 3, day: 15 },
    accrualFactor: 0.25,
  },
  {
    label: 'Up-front fee',
    type: 'fee',
    date: { year: 2025, month: 1, day: 15 },
    amount: { amount: 150_000.0, currency: 'USD' },
  },
  {
    label: 'Principal exchange',
    type: 'principalExchange',
    date: { year: 2030, month: 3, day: 15 },
    amount: { amount: -5_000_000.0, currency: 'USD' },
  },
];

// Complete props bundle for default cashflows example
export const DEFAULT_CASHFLOW_PROPS: CashflowBasicsProps = {
  cashflows: DEFAULT_CASHFLOWS,
};

