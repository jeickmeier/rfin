/**
 * Credit Default Swap (CDS) instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import {
  CreditDefaultSwap,
  FsDate,
  MarketContext,
  Money,
  PricingRequest,
  createStandardRegistry,
} from 'finstack-wasm';
import type { CdsData } from '../data/credit';
import { currencyFormatter, type InstrumentRow } from './useCreditMarket';

export interface CDSInstrumentProps {
  cdsSwaps: CdsData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSFormState {
  notional: number;
  spreadBps: number;
  tenorYears: number;
  direction: 'buy_protection' | 'sell_protection';
  currency: string;
}

export const CDSInstrument: React.FC<CDSInstrumentProps> = ({ cdsSwaps, market, asOf }) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first CDS swap
  const initialCds = cdsSwaps[0];
  const [formState, setFormState] = useState<CDSFormState>({
    notional: initialCds?.notional.amount ?? 10_000_000,
    spreadBps: initialCds?.spreadBps ?? 100,
    tenorYears: initialCds ? initialCds.maturityDate.year - initialCds.effectiveDate.year : 5,
    direction: initialCds?.direction ?? 'buy_protection',
    currency: initialCds?.notional.currency ?? 'USD',
  });

  const calculateCDS = useCallback(() => {
    try {
      const registry = createStandardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);

      // Use asOf date as effective date
      const effectiveDate = asOf;
      const maturityDate = new FsDate(asOf.year + formState.tenorYears, asOf.month, asOf.day);

      const cds =
        formState.direction === 'buy_protection'
          ? new CreditDefaultSwap(
              'interactive_cds',
              notional,
              formState.spreadBps,
              effectiveDate,
              maturityDate,
              initialCds?.discountCurveId ?? 'USD-OIS',
              initialCds?.hazardCurveId ?? 'ACME-HZD',
              'buy_protection',
              null
            )
          : new CreditDefaultSwap(
              'interactive_cds',
              notional,
              formState.spreadBps,
              effectiveDate,
              maturityDate,
              initialCds?.discountCurveId ?? 'USD-OIS',
              initialCds?.hazardCurveId ?? 'ACME-HZD',
              'sell_protection',
              null
            );

      const cdsOpts = new PricingRequest().withMetrics(['par_spread', 'pv01']);
      const cdsResult = registry.priceInstrument(cds, 'discounting', market, asOf, cdsOpts);

      const result: InstrumentRow = {
        name: `${formState.tenorYears}Y CDS`,
        type: 'CreditDefaultSwap',
        presentValue: cdsResult.presentValue.amount,
        keyMetric: {
          name: 'Par Spread',
          value: (() => {
            const raw = cdsResult.metric('par_spread') ?? 0;
            return Math.abs(raw) > 10 ? raw : raw * 10000;
          })(),
        },
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialCds]);

  // Calculate on form state change when form is visible
  useEffect(() => {
    if (showForm) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      calculateCDS();
    }
  }, [showForm, calculateCDS]);

  // Initial calculation from props
  useEffect(() => {
    if (showForm) return; // Skip if using form

    let cancelled = false;

    (async () => {
      try {
        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const cdsData of cdsSwaps) {
          const notional = Money.fromCode(cdsData.notional.amount, cdsData.notional.currency);
          const effectiveDate = new FsDate(
            cdsData.effectiveDate.year,
            cdsData.effectiveDate.month,
            cdsData.effectiveDate.day
          );
          const maturityDate = new FsDate(
            cdsData.maturityDate.year,
            cdsData.maturityDate.month,
            cdsData.maturityDate.day
          );

          const cds =
            cdsData.direction === 'buy_protection'
              ? new CreditDefaultSwap(
                  cdsData.id,
                  notional,
                  cdsData.spreadBps,
                  effectiveDate,
                  maturityDate,
                  cdsData.discountCurveId,
                  cdsData.hazardCurveId,
                  'buy_protection',
                  null
                )
              : new CreditDefaultSwap(
                  cdsData.id,
                  notional,
                  cdsData.spreadBps,
                  effectiveDate,
                  maturityDate,
                  cdsData.discountCurveId,
                  cdsData.hazardCurveId,
                  'sell_protection',
                  null
                );

          const cdsOpts = new PricingRequest().withMetrics(['par_spread', 'pv01']);
          try {
            const cdsResult = registry.priceInstrument(cds, 'discounting', market, asOf, cdsOpts);
            const tenorYears = cdsData.maturityDate.year - cdsData.effectiveDate.year;
            results.push({
              name: `${tenorYears}Y CDS`,
              type: 'CreditDefaultSwap',
              presentValue: cdsResult.presentValue.amount,
              keyMetric: {
                name: 'Par Spread',
                value: (() => {
                  const raw = cdsResult.metric('par_spread') ?? 0;
                  return Math.abs(raw) > 10 ? raw : raw * 10000;
                })(),
              },
            });
          } catch (err) {
            console.warn('CDS pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS instruments priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsSwaps, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSFormState, value: string | number) => {
    setFormState((prev) => ({ ...prev, [field]: value }));
  };

  if (error && !showForm) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0 && !showForm) {
    return <p>Loading CDS instruments...</p>;
  }

  return (
    <div className="instrument-group">
      <h3>
        Credit Default Swaps{' '}
        <button
          className="toggle-form-btn"
          onClick={() => setShowForm(!showForm)}
          style={{ marginLeft: '1rem', fontSize: '0.8rem' }}
        >
          {showForm ? '✕ Hide Calculator' : '⚙ Interactive Calculator'}
        </button>
      </h3>
      <p>
        Single-name CDS contracts providing credit protection on reference entities. Uses hazard
        curves for survival probabilities and calculates par spread as the fair premium.
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
            <label htmlFor="cds-notional">Notional</label>
            <input
              id="cds-notional"
              type="number"
              value={formState.notional}
              onChange={(e) =>
                handleInputChange('notional', Number.parseFloat(e.target.value) || 0)
              }
              step={1000000}
            />
          </div>
          <div className="form-field">
            <label htmlFor="cds-spread">Spread (bps)</label>
            <input
              id="cds-spread"
              type="number"
              value={formState.spreadBps}
              onChange={(e) =>
                handleInputChange('spreadBps', Number.parseFloat(e.target.value) || 0)
              }
              step={5}
            />
          </div>
          <div className="form-field">
            <label htmlFor="cds-tenor">Tenor (years)</label>
            <input
              id="cds-tenor"
              type="number"
              value={formState.tenorYears}
              onChange={(e) =>
                handleInputChange('tenorYears', Number.parseInt(e.target.value, 10) || 1)
              }
              min={1}
              max={30}
            />
          </div>
          <div className="form-field">
            <label htmlFor="cds-direction">Direction</label>
            <select
              id="cds-direction"
              value={formState.direction}
              onChange={(e) => handleInputChange('direction', e.target.value)}
            >
              <option value="buy_protection">Buy Protection</option>
              <option value="sell_protection">Sell Protection</option>
            </select>
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
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)} bps` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
