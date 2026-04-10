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
import type {
  DepositQuoteData,
  SwapQuoteData,
  FraQuoteData,
  DiscountQuoteData,
  ForwardQuoteData,
} from './quoteTypes';

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
            onChange={(e) =>
              onChange({ ...quote, fixedFrequency: e.target.value as FrequencyType })
            }
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
            onChange={(e) =>
              onChange({ ...quote, floatFrequency: e.target.value as FrequencyType })
            }
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

  const allValid = quotes.every((q) => isValidRate(q.rate, currency));

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
              <th className="p-2 text-left text-xs font-medium text-muted-foreground">
                Start/Maturity
              </th>
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
