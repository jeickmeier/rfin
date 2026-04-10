import React from 'react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Plus, Trash2, RefreshCw } from 'lucide-react';
import type { TrancheQuoteData, CdsVolQuoteData } from './quoteTypes';

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
        vol: 0.4,
        optionType: 'payer',
      },
    ]);
  };

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
