/**
 * CDS Option instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import { CdsOption, FsDate, MarketContext, Money, createStandardRegistry } from 'finstack-wasm';
import type { CdsOptionData } from '../data/credit';
import { currencyFormatter, type InstrumentRow } from './useCreditMarket';

export interface CDSOptionInstrumentProps {
  cdsOptions: CdsOptionData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSOptionFormState {
  notional: number;
  strikeBps: number;
  optionType: 'call' | 'put';
  recoveryRate: number;
  expiryMonths: number;
  underlyingTenorYears: number;
  knockedOut: boolean;
  currency: string;
}

export const CDSOptionInstrument: React.FC<CDSOptionInstrumentProps> = ({
  cdsOptions,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first option
  const initialOption = cdsOptions[0];
  const [formState, setFormState] = useState<CDSOptionFormState>({
    notional: initialOption?.notional.amount ?? 5_000_000,
    strikeBps: initialOption?.strikeBps ?? 150,
    optionType: initialOption?.optionType ?? 'call',
    recoveryRate: initialOption?.recoveryRate ?? 0.4,
    expiryMonths: initialOption
      ? (initialOption.expiryDate.year - 2024) * 12 + initialOption.expiryDate.month
      : 12,
    underlyingTenorYears: initialOption
      ? initialOption.underlyingMaturity.year - initialOption.expiryDate.year
      : 5,
    knockedOut: initialOption?.knockedOut ?? false,
    currency: initialOption?.notional.currency ?? 'USD',
  });

  const calculateOption = useCallback(() => {
    try {
      const registry = createStandardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);

      // Calculate expiry date from months
      const expiryYear = asOf.year + Math.floor(formState.expiryMonths / 12);
      const expiryMonth = ((asOf.month - 1 + formState.expiryMonths) % 12) + 1;
      const expiryDate = new FsDate(expiryYear, expiryMonth, asOf.day);

      const underlyingMaturity = new FsDate(
        expiryYear + formState.underlyingTenorYears,
        expiryMonth,
        asOf.day
      );

      const option = new CdsOption(
        'interactive_option',
        notional,
        formState.strikeBps,
        expiryDate,
        underlyingMaturity,
        initialOption?.discountCurveId ?? 'USD-OIS',
        initialOption?.hazardCurveId ?? 'ACME-HZD',
        initialOption?.volSurfaceId ?? 'CDS-VOL',
        formState.optionType,
        formState.recoveryRate,
        formState.knockedOut,
        null
      );

      const optionResult = registry.priceCdsOption(option, 'discounting', market, asOf, null);

      const result: InstrumentRow = {
        name: `CDS ${formState.optionType.charAt(0).toUpperCase() + formState.optionType.slice(1)} @ ${formState.strikeBps}bp`,
        type: 'CdsOption',
        presentValue: optionResult.presentValue.amount,
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS Option pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialOption]);

  // Calculate when form is visible and form state changes
  useEffect(() => {
    if (showForm) {
      // Use setTimeout to avoid React compiler warning about setState in effect
      const timer = setTimeout(() => {
        calculateOption();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [showForm, formState, calculateOption]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const optionData of cdsOptions) {
          const notional = Money.fromCode(optionData.notional.amount, optionData.notional.currency);
          const expiryDate = new FsDate(
            optionData.expiryDate.year,
            optionData.expiryDate.month,
            optionData.expiryDate.day
          );
          const underlyingMaturity = new FsDate(
            optionData.underlyingMaturity.year,
            optionData.underlyingMaturity.month,
            optionData.underlyingMaturity.day
          );

          try {
            const option = new CdsOption(
              optionData.id,
              notional,
              optionData.strikeBps,
              expiryDate,
              underlyingMaturity,
              optionData.discountCurveId,
              optionData.hazardCurveId,
              optionData.volSurfaceId,
              optionData.optionType,
              optionData.recoveryRate,
              optionData.knockedOut,
              null
            );
            const optionResult = registry.priceCdsOption(option, 'discounting', market, asOf, null);
            results.push({
              name: `CDS ${optionData.optionType.charAt(0).toUpperCase() + optionData.optionType.slice(1)} @ ${optionData.strikeBps}bp`,
              type: 'CdsOption',
              presentValue: optionResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('CDS option pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS options priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS Option pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsOptions, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSOptionFormState, value: string | number | boolean) => {
    setFormState((prev) => ({ ...prev, [field]: value }));
  };

  if (error && !showForm) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0 && !showForm) {
    return <p>Loading CDS options...</p>;
  }

  return (
    <div className="instrument-group">
      <h3>
        CDS Options{' '}
        <button
          className="toggle-form-btn"
          onClick={() => setShowForm(!showForm)}
          style={{ marginLeft: '1rem', fontSize: '0.8rem' }}
        >
          {showForm ? '✕ Hide Calculator' : '⚙ Interactive Calculator'}
        </button>
      </h3>
      <p>
        Options on credit default swaps (payer/receiver swaptions). Uses CDS volatility surfaces for
        Black-style pricing. Includes knockout feature support for credit events before expiry.
      </p>

      {showForm && (
        <div
          className="instrument-form"
          style={{
            background: 'var(--surface-2, #1a1a2e)',
            padding: '1rem',
            borderRadius: '8px',
            marginBottom: '1rem',
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
            gap: '1rem',
          }}
        >
          <div className="form-field">
            <label htmlFor="opt-notional">Notional</label>
            <input
              id="opt-notional"
              type="number"
              value={formState.notional}
              onChange={(e) =>
                handleInputChange('notional', Number.parseFloat(e.target.value) || 0)
              }
              step={1000000}
            />
          </div>
          <div className="form-field">
            <label htmlFor="opt-strike">Strike (bps)</label>
            <input
              id="opt-strike"
              type="number"
              value={formState.strikeBps}
              onChange={(e) =>
                handleInputChange('strikeBps', Number.parseFloat(e.target.value) || 0)
              }
              step={10}
            />
          </div>
          <div className="form-field">
            <label htmlFor="opt-expiry">Expiry (months)</label>
            <input
              id="opt-expiry"
              type="number"
              value={formState.expiryMonths}
              onChange={(e) =>
                handleInputChange('expiryMonths', Number.parseInt(e.target.value, 10) || 1)
              }
              min={1}
              max={60}
            />
          </div>
          <div className="form-field">
            <label htmlFor="opt-tenor">Underlying Tenor (years)</label>
            <input
              id="opt-tenor"
              type="number"
              value={formState.underlyingTenorYears}
              onChange={(e) =>
                handleInputChange('underlyingTenorYears', Number.parseInt(e.target.value, 10) || 1)
              }
              min={1}
              max={10}
            />
          </div>
          <div className="form-field">
            <label htmlFor="opt-type">Option Type</label>
            <select
              id="opt-type"
              value={formState.optionType}
              onChange={(e) => handleInputChange('optionType', e.target.value)}
            >
              <option value="call">Call (Payer)</option>
              <option value="put">Put (Receiver)</option>
            </select>
          </div>
          <div className="form-field">
            <label htmlFor="opt-recovery">Recovery Rate</label>
            <input
              id="opt-recovery"
              type="number"
              value={formState.recoveryRate}
              onChange={(e) =>
                handleInputChange('recoveryRate', Number.parseFloat(e.target.value) || 0)
              }
              step={0.05}
              min={0}
              max={1}
            />
          </div>
          <div className="form-field">
            <label
              htmlFor="opt-knockout"
              style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}
            >
              <input
                id="opt-knockout"
                type="checkbox"
                checked={formState.knockedOut}
                onChange={(e) => handleInputChange('knockedOut', e.target.checked)}
              />{' '}
              Knocked Out
            </label>
          </div>
        </div>
      )}

      {error && showForm && <p className="error">{error}</p>}

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)}` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
