import React from 'react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Plus, Trash2, RefreshCw } from 'lucide-react';
import type {
  CdsQuoteData,
  InflationSwapQuoteData,
  VolQuoteData,
} from './quoteTypes';

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
