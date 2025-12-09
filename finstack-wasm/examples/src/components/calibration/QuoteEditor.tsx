import React from 'react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Plus, Trash2, RefreshCw } from 'lucide-react';
import {
  type FrequencyType,
  FREQUENCY_OPTIONS,
  getSwapConventions,
  getRateBounds,
  isValidRate,
} from './CurrencyConventions';

/** Base quote data that can be edited */
export interface DepositQuoteData {
  type: 'deposit';
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  dayCount: string;
}

export interface SwapQuoteData {
  type: 'swap';
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  /** Fixed leg payment frequency */
  fixedFrequency: FrequencyType;
  /** Float leg payment frequency */
  floatFrequency: FrequencyType;
  fixedDayCount: string;
  floatDayCount: string;
  index: string;
}

export interface FraQuoteData {
  type: 'fra';
  startYear: number;
  startMonth: number;
  startDay: number;
  endYear: number;
  endMonth: number;
  endDay: number;
  rate: number;
  dayCount: string;
}

export type DiscountQuoteData = DepositQuoteData | SwapQuoteData;
/** Forward curve quotes can include FRAs, deposits, and swaps (for multi-curve calibration) */
export type ForwardQuoteData = FraQuoteData | DepositQuoteData | SwapQuoteData;

/** Credit CDS quote data */
export interface CdsQuoteData {
  entity: string;
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  spreadBps: number;
  recoveryRate: number;
  currency: string;
}

/** Inflation swap quote data */
export interface InflationSwapQuoteData {
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  rate: number;
  indexName: string;
}

/** Vol quote data */
export interface VolQuoteData {
  underlying: string;
  expiryYear: number;
  expiryMonth: number;
  expiryDay: number;
  strike: number;
  vol: number;
  optionType: 'Call' | 'Put';
}

/**
 * Generate default discount quotes relative to a base date.
 * Uses tenors (1M, 3M, 1Y, 3Y) instead of hardcoded dates.
 */
export function generateDefaultDiscountQuotes(
  baseYear: number,
  baseMonth: number,
  baseDay: number,
  currency: string = 'USD'
): DiscountQuoteData[] {
  const conventions = getSwapConventions(currency);
  // Generate maturity dates as offsets from base
  const addMonths = (y: number, m: number, d: number, months: number) => {
    const newMonth = m + months;
    const yearOffset = Math.floor((newMonth - 1) / 12);
    const finalMonth = ((newMonth - 1) % 12) + 1;
    return { y: y + yearOffset, m: finalMonth, d: Math.min(d, 28) }; // Safe day
  };

  const m1 = addMonths(baseYear, baseMonth, baseDay, 1);
  const m3 = addMonths(baseYear, baseMonth, baseDay, 3);
  const y1 = addMonths(baseYear, baseMonth, baseDay, 12);
  const y3 = addMonths(baseYear, baseMonth, baseDay, 36);

  return [
    {
      type: 'deposit',
      maturityYear: m1.y,
      maturityMonth: m1.m,
      maturityDay: m1.d,
      rate: 0.045,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'deposit',
      maturityYear: m3.y,
      maturityMonth: m3.m,
      maturityDay: m3.d,
      rate: 0.0465,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'swap',
      maturityYear: y1.y,
      maturityMonth: y1.m,
      maturityDay: y1.d,
      rate: 0.0475,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
    {
      type: 'swap',
      maturityYear: y3.y,
      maturityMonth: y3.m,
      maturityDay: y3.d,
      rate: 0.0485,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
  ];
}


/**
 * Generate default forward quotes relative to a base date.
 */
export function generateDefaultForwardQuotes(
  baseYear: number,
  baseMonth: number,
  baseDay: number,
  currency: string = 'USD'
): ForwardQuoteData[] {
  const conventions = getSwapConventions(currency);
  const addMonths = (y: number, m: number, d: number, months: number) => {
    const newMonth = m + months;
    const yearOffset = Math.floor((newMonth - 1) / 12);
    const finalMonth = ((newMonth - 1) % 12) + 1;
    return { y: y + yearOffset, m: finalMonth, d: Math.min(d, 28) };
  };

  const m1 = addMonths(baseYear, baseMonth, baseDay, 1);
  const m3 = addMonths(baseYear, baseMonth, baseDay, 3);
  const m6 = addMonths(baseYear, baseMonth, baseDay, 6);
  const m9 = addMonths(baseYear, baseMonth, baseDay, 9);
  const y2 = addMonths(baseYear, baseMonth, baseDay, 24);
  const y5 = addMonths(baseYear, baseMonth, baseDay, 60);

  return [
    // Short-end deposits
    {
      type: 'deposit',
      maturityYear: m1.y,
      maturityMonth: m1.m,
      maturityDay: m1.d,
      rate: 0.0535,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'deposit',
      maturityYear: m3.y,
      maturityMonth: m3.m,
      maturityDay: m3.d,
      rate: 0.054,
      dayCount: conventions.floatDayCount,
    },
    // FRAs for the near term
    {
      type: 'fra',
      startYear: m3.y,
      startMonth: m3.m,
      startDay: m3.d,
      endYear: m6.y,
      endMonth: m6.m,
      endDay: m6.d,
      rate: 0.052,
      dayCount: conventions.floatDayCount,
    },
    {
      type: 'fra',
      startYear: m6.y,
      startMonth: m6.m,
      startDay: m6.d,
      endYear: m9.y,
      endMonth: m9.m,
      endDay: m9.d,
      rate: 0.05,
      dayCount: conventions.floatDayCount,
    },
    // SOFR swaps for longer tenors
    {
      type: 'swap',
      maturityYear: y2.y,
      maturityMonth: y2.m,
      maturityDay: y2.d,
      rate: 0.0475,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
    {
      type: 'swap',
      maturityYear: y5.y,
      maturityMonth: y5.m,
      maturityDay: y5.d,
      rate: 0.045,
      fixedFrequency: conventions.fixedFrequency,
      floatFrequency: conventions.floatFrequency,
      fixedDayCount: conventions.fixedDayCount,
      floatDayCount: conventions.floatDayCount,
      index: conventions.defaultIndex,
    },
  ];
}

export const DEFAULT_CREDIT_QUOTES: CdsQuoteData[] = [
  {
    entity: 'ACME',
    maturityYear: 2027,
    maturityMonth: 1,
    maturityDay: 2,
    spreadBps: 120,
    recoveryRate: 0.4,
    currency: 'USD',
  },
  {
    entity: 'ACME',
    maturityYear: 2029,
    maturityMonth: 1,
    maturityDay: 2,
    spreadBps: 135,
    recoveryRate: 0.4,
    currency: 'USD',
  },
];

export const DEFAULT_INFLATION_QUOTES: InflationSwapQuoteData[] = [
  { maturityYear: 2026, maturityMonth: 1, maturityDay: 2, rate: 0.021, indexName: 'US-CPI-U' },
  { maturityYear: 2029, maturityMonth: 1, maturityDay: 2, rate: 0.023, indexName: 'US-CPI-U' },
];

export const DEFAULT_VOL_QUOTES: VolQuoteData[] = [
  {
    underlying: 'AAPL',
    expiryYear: 2024,
    expiryMonth: 7,
    expiryDay: 1,
    strike: 90,
    vol: 0.24,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2024,
    expiryMonth: 7,
    expiryDay: 1,
    strike: 100,
    vol: 0.22,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2024,
    expiryMonth: 7,
    expiryDay: 1,
    strike: 110,
    vol: 0.23,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2025,
    expiryMonth: 1,
    expiryDay: 2,
    strike: 90,
    vol: 0.26,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2025,
    expiryMonth: 1,
    expiryDay: 2,
    strike: 100,
    vol: 0.24,
    optionType: 'Call',
  },
  {
    underlying: 'AAPL',
    expiryYear: 2025,
    expiryMonth: 1,
    expiryDay: 2,
    strike: 110,
    vol: 0.25,
    optionType: 'Call',
  },
];

/** CDS Tranche quote data for base correlation calibration */
export interface TrancheQuoteData {
  index: string;
  attachment: number;
  detachment: number;
  maturityYear: number;
  maturityMonth: number;
  maturityDay: number;
  upfrontPct: number;
  runningSpreadBp: number;
}

/** CDS Vol quote data for CDS option pricing */
export interface CdsVolQuoteData {
  expiryMonths: number;
  strikeBps: number;
  vol: number;
  optionType: 'payer' | 'receiver';
}

/**
 * Default tranche quotes for CDX.NA.IG (equity sub-tranches for base correlation calibration).
 * Base correlation calibration requires equity sub-tranches [0, D] for each detachment point D.
 * Upfront values are synthetic placeholders for demonstration purposes.
 */
export const DEFAULT_TRANCHE_QUOTES: TrancheQuoteData[] = [
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 3.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 25.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 7.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 15.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 10.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 10.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 15.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 6.0,
    runningSpreadBp: 500.0,
  },
  {
    index: 'CDX.NA.IG.42',
    attachment: 0.0,
    detachment: 30.0,
    maturityYear: 2029,
    maturityMonth: 6,
    maturityDay: 20,
    upfrontPct: 2.5,
    runningSpreadBp: 500.0,
  },
];

/** Editable row for a deposit quote with validation */
const DepositQuoteRow: React.FC<{
  quote: DepositQuoteData;
  onChange: (quote: DepositQuoteData) => void;
  onRemove: () => void;
  currency?: string;
}> = ({ quote, onChange, onRemove, currency = 'USD' }) => {
  const rateValid = isValidRate(quote.rate, currency);
  const bounds = getRateBounds(currency);

  return (
    <tr className="border-b border-border/50">
      <td className="p-2 text-xs text-muted-foreground">Deposit</td>
      <td className="p-2">
        <Input
          type="date"
          className="h-7 text-xs w-32"
          value={`${quote.maturityYear}-${String(quote.maturityMonth).padStart(2, '0')}-${String(quote.maturityDay).padStart(2, '0')}`}
          onChange={(e) => {
            const [y, m, d] = e.target.value.split('-').map(Number);
            onChange({ ...quote, maturityYear: y, maturityMonth: m, maturityDay: d });
          }}
        />
      </td>
      <td className="p-2">
        <Input
          type="number"
          step="0.0001"
          min={bounds.minRate}
          max={bounds.maxRate}
          className={`h-7 text-xs w-24 font-mono ${!rateValid ? 'border-destructive bg-destructive/10' : ''}`}
          value={quote.rate}
          onChange={(e) => onChange({ ...quote, rate: parseFloat(e.target.value) || 0 })}
          title={`Rate must be between ${(bounds.minRate * 100).toFixed(1)}% and ${(bounds.maxRate * 100).toFixed(0)}%`}
        />
      </td>
      <td className="p-2 text-xs text-muted-foreground">—</td>
      <td className="p-2 text-xs text-muted-foreground">{quote.dayCount}</td>
      <td className="p-2">
        <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
          <Trash2 className="h-3 w-3 text-destructive" />
        </Button>
      </td>
    </tr>
  );
};

/** Editable row for a swap quote with frequency and validation */
const SwapQuoteRow: React.FC<{
  quote: SwapQuoteData;
  onChange: (quote: SwapQuoteData) => void;
  onRemove: () => void;
  currency?: string;
}> = ({ quote, onChange, onRemove, currency = 'USD' }) => {
  const rateValid = isValidRate(quote.rate, currency);
  const bounds = getRateBounds(currency);

  return (
    <tr className="border-b border-border/50">
      <td className="p-2 text-xs text-muted-foreground">Swap</td>
      <td className="p-2">
        <Input
          type="date"
          className="h-7 text-xs w-32"
          value={`${quote.maturityYear}-${String(quote.maturityMonth).padStart(2, '0')}-${String(quote.maturityDay).padStart(2, '0')}`}
          onChange={(e) => {
            const [y, m, d] = e.target.value.split('-').map(Number);
            onChange({ ...quote, maturityYear: y, maturityMonth: m, maturityDay: d });
          }}
        />
      </td>
      <td className="p-2">
        <Input
          type="number"
          step="0.0001"
          min={bounds.minRate}
          max={bounds.maxRate}
          className={`h-7 text-xs w-24 font-mono ${!rateValid ? 'border-destructive bg-destructive/10' : ''}`}
          value={quote.rate}
          onChange={(e) => onChange({ ...quote, rate: parseFloat(e.target.value) || 0 })}
          title={`Rate must be between ${(bounds.minRate * 100).toFixed(1)}% and ${(bounds.maxRate * 100).toFixed(0)}%`}
        />
      </td>
      <td className="p-2">
        <div className="flex gap-1">
          <select
            className="h-7 text-xs rounded border border-input bg-background px-1 w-16"
            value={quote.fixedFrequency}
            onChange={(e) => onChange({ ...quote, fixedFrequency: e.target.value as FrequencyType })}
            title="Fixed leg frequency"
          >
            {FREQUENCY_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label.substring(0, 4)}
              </option>
            ))}
          </select>
          <select
            className="h-7 text-xs rounded border border-input bg-background px-1 w-16"
            value={quote.floatFrequency}
            onChange={(e) => onChange({ ...quote, floatFrequency: e.target.value as FrequencyType })}
            title="Float leg frequency"
          >
            {FREQUENCY_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label.substring(0, 4)}
              </option>
            ))}
          </select>
        </div>
      </td>
      <td className="p-2 text-xs text-muted-foreground">{quote.index}</td>
      <td className="p-2">
        <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
          <Trash2 className="h-3 w-3 text-destructive" />
        </Button>
      </td>
    </tr>
  );
};

/** Editable row for a FRA quote */
const FraQuoteRow: React.FC<{
  quote: FraQuoteData;
  onChange: (quote: FraQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2 text-xs text-muted-foreground">FRA</td>
    <td className="p-2">
      <Input
        type="date"
        className="h-7 text-xs w-28"
        value={`${quote.startYear}-${String(quote.startMonth).padStart(2, '0')}-${String(quote.startDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, startYear: y, startMonth: m, startDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="date"
        className="h-7 text-xs w-28"
        value={`${quote.endYear}-${String(quote.endMonth).padStart(2, '0')}-${String(quote.endDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, endYear: y, endMonth: m, endDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.0001"
        className="h-7 text-xs w-24 font-mono"
        value={quote.rate}
        onChange={(e) => onChange({ ...quote, rate: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2 text-xs text-muted-foreground">{quote.dayCount}</td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** Discount quote editor with table of editable quotes */
export const DiscountQuoteEditor: React.FC<{
  quotes: DiscountQuoteData[];
  onChange: (quotes: DiscountQuoteData[]) => void;
  onCalibrate: () => void;
  disabled?: boolean;
  /** Currency for convention defaults and validation (default: USD) */
  currency?: string;
}> = ({ quotes, onChange, onCalibrate, disabled, currency = 'USD' }) => {
  const conventions = getSwapConventions(currency);

  const updateQuote = (index: number, quote: DiscountQuoteData) => {
    const newQuotes = [...quotes];
    newQuotes[index] = quote;
    onChange(newQuotes);
  };

  const removeQuote = (index: number) => {
    onChange(quotes.filter((_, i) => i !== index));
  };

  const addDeposit = () => {
    // Use date 6 months from now
    const futureDate = new Date();
    futureDate.setMonth(futureDate.getMonth() + 6);
    onChange([
      ...quotes,
      {
        type: 'deposit',
        maturityYear: futureDate.getFullYear(),
        maturityMonth: futureDate.getMonth() + 1,
        maturityDay: Math.min(futureDate.getDate(), 28),
        rate: 0.045,
        dayCount: conventions.floatDayCount,
      },
    ]);
  };

  const addSwap = () => {
    // Use date 2 years from now
    const futureDate = new Date();
    futureDate.setFullYear(futureDate.getFullYear() + 2);
    onChange([
      ...quotes,
      {
        type: 'swap',
        maturityYear: futureDate.getFullYear(),
        maturityMonth: futureDate.getMonth() + 1,
        maturityDay: Math.min(futureDate.getDate(), 28),
        rate: 0.048,
        fixedFrequency: conventions.fixedFrequency,
        floatFrequency: conventions.floatFrequency,
        fixedDayCount: conventions.fixedDayCount,
        floatDayCount: conventions.floatDayCount,
        index: conventions.defaultIndex,
      },
    ]);
  };

  // Check if all quotes are valid
  const allValid = quotes.every((q) =>
    q.type === 'deposit' ? isValidRate(q.rate, currency) : isValidRate(q.rate, currency)
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Market Quotes</span>
        <div className="flex gap-1">
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addDeposit}>
            <Plus className="h-3 w-3 mr-1" /> Deposit
          </Button>
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addSwap}>
            <Plus className="h-3 w-3 mr-1" /> Swap
          </Button>
        </div>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Type</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Maturity</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Rate</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Freq</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Index</th>
              <th className="p-2 w-8"></th>
            </tr>
          </thead>
          <tbody>
            {quotes.map((quote, idx) =>
              quote.type === 'deposit' ? (
                <DepositQuoteRow
                  key={`quote-${idx}`}
                  quote={quote}
                  onChange={(q) => updateQuote(idx, q)}
                  onRemove={() => removeQuote(idx)}
                  currency={currency}
                />
              ) : (
                <SwapQuoteRow
                  key={`quote-${idx}`}
                  quote={quote}
                  onChange={(q) => updateQuote(idx, q)}
                  onRemove={() => removeQuote(idx)}
                  currency={currency}
                />
              )
            )}
          </tbody>
        </table>
      </div>

      {!allValid && (
        <div className="text-xs text-destructive">
          Some rates are outside valid bounds for {currency}. Please check highlighted fields.
        </div>
      )}

      <Button
        onClick={onCalibrate}
        disabled={disabled || quotes.length < 2 || !allValid}
        className="w-full"
        size="sm"
      >
        <RefreshCw className="h-3 w-3 mr-2" />
        Calibrate Curve ({quotes.length} quotes)
      </Button>
    </div>
  );
};

/** Editable row for a forward deposit quote - shows maturity in start column, rate info */
const ForwardDepositQuoteRow: React.FC<{
  quote: DepositQuoteData;
  onChange: (quote: DepositQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2 text-xs text-muted-foreground">Deposit</td>
    <td className="p-2" colSpan={2}>
      <Input
        type="date"
        className="h-7 text-xs w-32"
        value={`${quote.maturityYear}-${String(quote.maturityMonth).padStart(2, '0')}-${String(quote.maturityDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, maturityYear: y, maturityMonth: m, maturityDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.0001"
        className="h-7 text-xs w-24 font-mono"
        value={quote.rate}
        onChange={(e) => onChange({ ...quote, rate: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2 text-xs text-muted-foreground">{quote.dayCount}</td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** Editable row for a forward swap quote */
const ForwardSwapQuoteRow: React.FC<{
  quote: SwapQuoteData;
  onChange: (quote: SwapQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2 text-xs text-muted-foreground">Swap</td>
    <td className="p-2" colSpan={2}>
      <Input
        type="date"
        className="h-7 text-xs w-32"
        value={`${quote.maturityYear}-${String(quote.maturityMonth).padStart(2, '0')}-${String(quote.maturityDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, maturityYear: y, maturityMonth: m, maturityDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.0001"
        className="h-7 text-xs w-24 font-mono"
        value={quote.rate}
        onChange={(e) => onChange({ ...quote, rate: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2 text-xs text-muted-foreground">{quote.index}</td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** Forward quote editor supporting FRAs, deposits, and SOFR swaps */
export const ForwardQuoteEditor: React.FC<{
  quotes: ForwardQuoteData[];
  onChange: (quotes: ForwardQuoteData[]) => void;
  onCalibrate: () => void;
  disabled?: boolean;
  /** Currency for convention defaults and validation (default: USD) */
  currency?: string;
}> = ({ quotes, onChange, onCalibrate, disabled, currency = 'USD' }) => {
  const conventions = getSwapConventions(currency);

  const updateQuote = (index: number, quote: ForwardQuoteData) => {
    const newQuotes = [...quotes];
    newQuotes[index] = quote;
    onChange(newQuotes);
  };

  const removeQuote = (index: number) => {
    onChange(quotes.filter((_, i) => i !== index));
  };

  const addDeposit = () => {
    // Use date 6 months from now
    const futureDate = new Date();
    futureDate.setMonth(futureDate.getMonth() + 6);
    onChange([
      ...quotes,
      {
        type: 'deposit',
        maturityYear: futureDate.getFullYear(),
        maturityMonth: futureDate.getMonth() + 1,
        maturityDay: Math.min(futureDate.getDate(), 28),
        rate: 0.053,
        dayCount: conventions.floatDayCount,
      },
    ]);
  };

  const addFra = () => {
    // Use FRA from 9M to 12M from now
    const startDate = new Date();
    startDate.setMonth(startDate.getMonth() + 9);
    const endDate = new Date();
    endDate.setMonth(endDate.getMonth() + 12);
    onChange([
      ...quotes,
      {
        type: 'fra',
        startYear: startDate.getFullYear(),
        startMonth: startDate.getMonth() + 1,
        startDay: Math.min(startDate.getDate(), 28),
        endYear: endDate.getFullYear(),
        endMonth: endDate.getMonth() + 1,
        endDay: Math.min(endDate.getDate(), 28),
        rate: 0.05,
        dayCount: conventions.floatDayCount,
      },
    ]);
  };

  const addSwap = () => {
    // Use date 3 years from now
    const futureDate = new Date();
    futureDate.setFullYear(futureDate.getFullYear() + 3);
    onChange([
      ...quotes,
      {
        type: 'swap',
        maturityYear: futureDate.getFullYear(),
        maturityMonth: futureDate.getMonth() + 1,
        maturityDay: Math.min(futureDate.getDate(), 28),
        rate: 0.046,
        fixedFrequency: conventions.fixedFrequency,
        floatFrequency: conventions.floatFrequency,
        fixedDayCount: conventions.fixedDayCount,
        floatDayCount: conventions.floatDayCount,
        index: conventions.defaultIndex,
      },
    ]);
  };

  // Check if all quotes are valid
  const allValid = quotes.every((q) => isValidRate(q.rate, currency));

  // Count quote types for display
  const deposits = quotes.filter((q) => q.type === 'deposit').length;
  const fras = quotes.filter((q) => q.type === 'fra').length;
  const swaps = quotes.filter((q) => q.type === 'swap').length;

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Market Quotes</span>
        <div className="flex gap-1">
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addDeposit}>
            <Plus className="h-3 w-3 mr-1" /> Deposit
          </Button>
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addFra}>
            <Plus className="h-3 w-3 mr-1" /> FRA
          </Button>
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addSwap}>
            <Plus className="h-3 w-3 mr-1" /> Swap
          </Button>
        </div>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Type</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Start/Maturity</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">End</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Rate</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Details</th>
              <th className="p-2 w-8"></th>
            </tr>
          </thead>
          <tbody>
            {quotes.map((quote, idx) => {
              if (quote.type === 'deposit') {
                return (
                  <ForwardDepositQuoteRow
                    key={`quote-${idx}`}
                    quote={quote}
                    onChange={(q) => updateQuote(idx, q)}
                    onRemove={() => removeQuote(idx)}
                  />
                );
              } else if (quote.type === 'swap') {
                return (
                  <ForwardSwapQuoteRow
                    key={`quote-${idx}`}
                    quote={quote}
                    onChange={(q) => updateQuote(idx, q)}
                    onRemove={() => removeQuote(idx)}
                  />
                );
              } else {
                return (
                  <FraQuoteRow
                    key={`quote-${idx}`}
                    quote={quote}
                    onChange={(q) => updateQuote(idx, q)}
                    onRemove={() => removeQuote(idx)}
                  />
                );
              }
            })}
          </tbody>
        </table>
      </div>

      {!allValid && (
        <div className="text-xs text-destructive">
          Some rates are outside valid bounds for {currency}. Please check highlighted fields.
        </div>
      )}

      <Button
        onClick={onCalibrate}
        disabled={disabled || quotes.length < 1 || !allValid}
        className="w-full"
        size="sm"
      >
        <RefreshCw className="h-3 w-3 mr-2" />
        Calibrate ({deposits} deposits, {fras} FRAs, {swaps} swaps)
      </Button>
    </div>
  );
};

/** Editable row for a CDS quote */
const CdsQuoteRow: React.FC<{
  quote: CdsQuoteData;
  onChange: (quote: CdsQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2">
      <Input
        type="text"
        className="h-7 text-xs w-20"
        value={quote.entity}
        onChange={(e) => onChange({ ...quote, entity: e.target.value })}
      />
    </td>
    <td className="p-2">
      <Input
        type="date"
        className="h-7 text-xs w-32"
        value={`${quote.maturityYear}-${String(quote.maturityMonth).padStart(2, '0')}-${String(quote.maturityDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, maturityYear: y, maturityMonth: m, maturityDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="1"
        className="h-7 text-xs w-20 font-mono"
        value={quote.spreadBps}
        onChange={(e) => onChange({ ...quote, spreadBps: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.05"
        className="h-7 text-xs w-16 font-mono"
        value={quote.recoveryRate}
        onChange={(e) => onChange({ ...quote, recoveryRate: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** Credit quote editor with table of editable CDS quotes */
export const CreditQuoteEditor: React.FC<{
  quotes: CdsQuoteData[];
  onChange: (quotes: CdsQuoteData[]) => void;
  onCalibrate: () => void;
  disabled?: boolean;
  entity: string;
}> = ({ quotes, onChange, onCalibrate, disabled, entity }) => {
  const updateQuote = (index: number, quote: CdsQuoteData) => {
    const newQuotes = [...quotes];
    newQuotes[index] = quote;
    onChange(newQuotes);
  };

  const removeQuote = (index: number) => {
    onChange(quotes.filter((_, i) => i !== index));
  };

  const addCds = () => {
    onChange([
      ...quotes,
      {
        entity,
        maturityYear: 2030,
        maturityMonth: 1,
        maturityDay: 2,
        spreadBps: 150,
        recoveryRate: 0.4,
        currency: 'USD',
      },
    ]);
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">CDS Quotes</span>
        <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addCds}>
          <Plus className="h-3 w-3 mr-1" /> Add CDS
        </Button>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Entity</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Maturity</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">
                Spread (bps)
              </th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Recovery</th>
              <th className="p-2 w-8"></th>
            </tr>
          </thead>
          <tbody>
            {quotes.map((quote, idx) => (
              <CdsQuoteRow
                key={`quote-${idx}`}
                quote={quote}
                onChange={(q) => updateQuote(idx, q)}
                onRemove={() => removeQuote(idx)}
              />
            ))}
          </tbody>
        </table>
      </div>

      <Button
        onClick={onCalibrate}
        disabled={disabled || quotes.length < 1}
        className="w-full"
        size="sm"
      >
        <RefreshCw className="h-3 w-3 mr-2" />
        Calibrate Curve ({quotes.length} quotes)
      </Button>
    </div>
  );
};

/** Editable row for an inflation swap quote */
const InflationSwapQuoteRow: React.FC<{
  quote: InflationSwapQuoteData;
  onChange: (quote: InflationSwapQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2">
      <Input
        type="date"
        className="h-7 text-xs w-32"
        value={`${quote.maturityYear}-${String(quote.maturityMonth).padStart(2, '0')}-${String(quote.maturityDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, maturityYear: y, maturityMonth: m, maturityDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.001"
        className="h-7 text-xs w-24 font-mono"
        value={quote.rate}
        onChange={(e) => onChange({ ...quote, rate: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2 text-xs text-muted-foreground">{quote.indexName}</td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** Inflation quote editor with table of editable inflation swap quotes */
export const InflationQuoteEditor: React.FC<{
  quotes: InflationSwapQuoteData[];
  onChange: (quotes: InflationSwapQuoteData[]) => void;
  onCalibrate: () => void;
  disabled?: boolean;
  indexName: string;
}> = ({ quotes, onChange, onCalibrate, disabled, indexName }) => {
  const updateQuote = (index: number, quote: InflationSwapQuoteData) => {
    const newQuotes = [...quotes];
    newQuotes[index] = quote;
    onChange(newQuotes);
  };

  const removeQuote = (index: number) => {
    onChange(quotes.filter((_, i) => i !== index));
  };

  const addSwap = () => {
    onChange([
      ...quotes,
      { maturityYear: 2030, maturityMonth: 1, maturityDay: 2, rate: 0.025, indexName },
    ]);
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Inflation Swap Quotes</span>
        <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addSwap}>
          <Plus className="h-3 w-3 mr-1" /> Add Swap
        </Button>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Maturity</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Rate</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Index</th>
              <th className="p-2 w-8"></th>
            </tr>
          </thead>
          <tbody>
            {quotes.map((quote, idx) => (
              <InflationSwapQuoteRow
                key={`quote-${idx}`}
                quote={quote}
                onChange={(q) => updateQuote(idx, q)}
                onRemove={() => removeQuote(idx)}
              />
            ))}
          </tbody>
        </table>
      </div>

      <Button
        onClick={onCalibrate}
        disabled={disabled || quotes.length < 1}
        className="w-full"
        size="sm"
      >
        <RefreshCw className="h-3 w-3 mr-2" />
        Calibrate Curve ({quotes.length} quotes)
      </Button>
    </div>
  );
};

/** Editable row for a vol quote */
const VolQuoteRow: React.FC<{
  quote: VolQuoteData;
  onChange: (quote: VolQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2">
      <Input
        type="date"
        className="h-7 text-xs w-32"
        value={`${quote.expiryYear}-${String(quote.expiryMonth).padStart(2, '0')}-${String(quote.expiryDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, expiryYear: y, expiryMonth: m, expiryDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="1"
        className="h-7 text-xs w-20 font-mono"
        value={quote.strike}
        onChange={(e) => onChange({ ...quote, strike: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.01"
        className="h-7 text-xs w-20 font-mono"
        value={quote.vol}
        onChange={(e) => onChange({ ...quote, vol: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <select
        className="h-7 text-xs rounded border border-input bg-background px-2"
        value={quote.optionType}
        onChange={(e) => onChange({ ...quote, optionType: e.target.value as 'Call' | 'Put' })}
      >
        <option value="Call">Call</option>
        <option value="Put">Put</option>
      </select>
    </td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** Vol quote editor with table of editable option vol quotes */
export const VolQuoteEditor: React.FC<{
  quotes: VolQuoteData[];
  onChange: (quotes: VolQuoteData[]) => void;
  onCalibrate: () => void;
  disabled?: boolean;
  underlying: string;
}> = ({ quotes, onChange, onCalibrate, disabled, underlying }) => {
  const updateQuote = (index: number, quote: VolQuoteData) => {
    const newQuotes = [...quotes];
    newQuotes[index] = quote;
    onChange(newQuotes);
  };

  const removeQuote = (index: number) => {
    onChange(quotes.filter((_, i) => i !== index));
  };

  const addVol = () => {
    onChange([
      ...quotes,
      {
        underlying,
        expiryYear: 2025,
        expiryMonth: 6,
        expiryDay: 1,
        strike: 100,
        vol: 0.25,
        optionType: 'Call',
      },
    ]);
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Option Vol Quotes</span>
        <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addVol}>
          <Plus className="h-3 w-3 mr-1" /> Add Quote
        </Button>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Expiry</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Strike</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Vol</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Type</th>
              <th className="p-2 w-8"></th>
            </tr>
          </thead>
          <tbody>
            {quotes.map((quote, idx) => (
              <VolQuoteRow
                key={`quote-${idx}`}
                quote={quote}
                onChange={(q) => updateQuote(idx, q)}
                onRemove={() => removeQuote(idx)}
              />
            ))}
          </tbody>
        </table>
      </div>

      <Button
        onClick={onCalibrate}
        disabled={disabled || quotes.length < 3}
        className="w-full"
        size="sm"
      >
        <RefreshCw className="h-3 w-3 mr-2" />
        Calibrate Surface ({quotes.length} quotes)
      </Button>
    </div>
  );
};

/** Editable row for a CDS tranche quote */
const TrancheQuoteRow: React.FC<{
  quote: TrancheQuoteData;
  onChange: (quote: TrancheQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2 text-xs text-muted-foreground font-mono">
      {quote.attachment}–{quote.detachment}%
    </td>
    <td className="p-2">
      <Input
        type="date"
        className="h-7 text-xs w-32"
        value={`${quote.maturityYear}-${String(quote.maturityMonth).padStart(2, '0')}-${String(quote.maturityDay).padStart(2, '0')}`}
        onChange={(e) => {
          const [y, m, d] = e.target.value.split('-').map(Number);
          onChange({ ...quote, maturityYear: y, maturityMonth: m, maturityDay: d });
        }}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.1"
        className="h-7 text-xs w-20 font-mono"
        value={quote.upfrontPct}
        onChange={(e) => onChange({ ...quote, upfrontPct: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="1"
        className="h-7 text-xs w-20 font-mono"
        value={quote.runningSpreadBp}
        onChange={(e) => onChange({ ...quote, runningSpreadBp: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** Tranche quote editor for base correlation calibration */
export const TrancheQuoteEditor: React.FC<{
  quotes: TrancheQuoteData[];
  onChange: (quotes: TrancheQuoteData[]) => void;
  onCalibrate: () => void;
  disabled?: boolean;
  indexId: string;
}> = ({ quotes, onChange, onCalibrate, disabled, indexId }) => {
  const updateQuote = (index: number, quote: TrancheQuoteData) => {
    const newQuotes = [...quotes];
    newQuotes[index] = quote;
    onChange(newQuotes);
  };

  const removeQuote = (index: number) => {
    onChange(quotes.filter((_, i) => i !== index));
  };

  const addTranche = () => {
    const existing = quotes.map((q) => q.detachment).sort((a, b) => a - b);
    const lastDetach = existing.length > 0 ? existing[existing.length - 1] : 0;
    onChange([
      ...quotes,
      {
        index: indexId,
        attachment: lastDetach,
        detachment: Math.min(lastDetach + 5, 100),
        maturityYear: 2029,
        maturityMonth: 6,
        maturityDay: 20,
        upfrontPct: 0.5,
        runningSpreadBp: 500.0,
      },
    ]);
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Tranche Quotes</span>
        <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addTranche}>
          <Plus className="h-3 w-3 mr-1" /> Add Tranche
        </Button>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Tranche</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Maturity</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Upfront %</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">
                Spread (bp)
              </th>
              <th className="p-2 w-8"></th>
            </tr>
          </thead>
          <tbody>
            {quotes.map((quote, idx) => (
              <TrancheQuoteRow
                key={`tranche-${idx}`}
                quote={quote}
                onChange={(q) => updateQuote(idx, q)}
                onRemove={() => removeQuote(idx)}
              />
            ))}
          </tbody>
        </table>
      </div>

      <Button
        onClick={onCalibrate}
        disabled={disabled || quotes.length < 2}
        className="w-full"
        size="sm"
      >
        <RefreshCw className="h-3 w-3 mr-2" />
        Calibrate Base Correlation ({quotes.length} tranches)
      </Button>
    </div>
  );
};

/** Editable row for a CDS vol quote */
const CdsVolQuoteRow: React.FC<{
  quote: CdsVolQuoteData;
  onChange: (quote: CdsVolQuoteData) => void;
  onRemove: () => void;
}> = ({ quote, onChange, onRemove }) => (
  <tr className="border-b border-border/50">
    <td className="p-2">
      <Input
        type="number"
        step="1"
        className="h-7 text-xs w-16 font-mono"
        value={quote.expiryMonths}
        onChange={(e) => onChange({ ...quote, expiryMonths: parseInt(e.target.value) || 6 })}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="5"
        className="h-7 text-xs w-20 font-mono"
        value={quote.strikeBps}
        onChange={(e) => onChange({ ...quote, strikeBps: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <Input
        type="number"
        step="0.01"
        className="h-7 text-xs w-20 font-mono"
        value={quote.vol}
        onChange={(e) => onChange({ ...quote, vol: parseFloat(e.target.value) || 0 })}
      />
    </td>
    <td className="p-2">
      <select
        className="h-7 text-xs rounded border border-input bg-background px-2"
        value={quote.optionType}
        onChange={(e) => onChange({ ...quote, optionType: e.target.value as 'payer' | 'receiver' })}
      >
        <option value="payer">Payer</option>
        <option value="receiver">Receiver</option>
      </select>
    </td>
    <td className="p-2">
      <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onRemove}>
        <Trash2 className="h-3 w-3 text-destructive" />
      </Button>
    </td>
  </tr>
);

/** CDS vol quote editor for CDS option volatility surface */
export const CdsVolQuoteEditor: React.FC<{
  quotes: CdsVolQuoteData[];
  onChange: (quotes: CdsVolQuoteData[]) => void;
  onCalibrate: () => void;
  disabled?: boolean;
}> = ({ quotes, onChange, onCalibrate, disabled }) => {
  const updateQuote = (index: number, quote: CdsVolQuoteData) => {
    const newQuotes = [...quotes];
    newQuotes[index] = quote;
    onChange(newQuotes);
  };

  const removeQuote = (index: number) => {
    onChange(quotes.filter((_, i) => i !== index));
  };

  const addVolQuote = () => {
    onChange([
      ...quotes,
      {
        expiryMonths: 12,
        strikeBps: 100,
        vol: 0.40,
        optionType: 'payer',
      },
    ]);
  };

  // Count unique expiries and strikes
  const uniqueExpiries = new Set(quotes.map((q) => q.expiryMonths)).size;
  const uniqueStrikes = new Set(quotes.map((q) => q.strikeBps)).size;

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">CDS Vol Quotes</span>
        <Button variant="outline" size="sm" className="h-7 text-xs" onClick={addVolQuote}>
          <Plus className="h-3 w-3 mr-1" /> Add Quote
        </Button>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">
                Expiry (M)
              </th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">
                Strike (bps)
              </th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Vol</th>
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">Type</th>
              <th className="p-2 w-8"></th>
            </tr>
          </thead>
          <tbody>
            {quotes.map((quote, idx) => (
              <CdsVolQuoteRow
                key={`cdsvol-${idx}`}
                quote={quote}
                onChange={(q) => updateQuote(idx, q)}
                onRemove={() => removeQuote(idx)}
              />
            ))}
          </tbody>
        </table>
      </div>

      <div className="text-xs text-muted-foreground">
        Surface: {uniqueExpiries} expiries × {uniqueStrikes} strikes
      </div>

      <Button
        onClick={onCalibrate}
        disabled={disabled || quotes.length < 3}
        className="w-full"
        size="sm"
      >
        <RefreshCw className="h-3 w-3 mr-2" />
        Build Vol Surface ({quotes.length} quotes)
      </Button>
    </div>
  );
};
