/**
 * CDS Index instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import {
  CDSIndex,
  FsDate,
  MarketContext,
  Money,
  PricingRequest,
  standardRegistry,
} from 'finstack-wasm';
import type { CdsIndexInstrumentData } from '../data/credit';
import { currencyFormatter, type InstrumentRow } from './useCreditMarket';

export interface CDSIndexInstrumentProps {
  cdsIndices: CdsIndexInstrumentData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSIndexFormState {
  notional: number;
  spreadBps: number;
  series: number;
  version: number;
  recoveryRate: number;
  direction: 'pay_protection' | 'receive_protection';
  currency: string;
  indexFamily: string;
}

export const CDSIndexInstrument: React.FC<CDSIndexInstrumentProps> = ({
  cdsIndices,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first index
  const initialIndex = cdsIndices[0];
  const [formState, setFormState] = useState<CDSIndexFormState>({
    notional: initialIndex?.notional.amount ?? 25_000_000,
    spreadBps: initialIndex?.spreadBps ?? 100,
    series: initialIndex?.series ?? 42,
    version: initialIndex?.version ?? 1,
    recoveryRate: initialIndex?.recoveryRate ?? 0.4,
    direction: initialIndex?.direction ?? 'pay_protection',
    currency: initialIndex?.notional.currency ?? 'USD',
    indexFamily: initialIndex?.indexFamily ?? 'CDX.NA.IG',
  });

  const calculateIndex = useCallback(() => {
    try {
      const registry = standardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);

      const effectiveDate = asOf;
      const maturityDate = new FsDate(asOf.year + 5, asOf.month, asOf.day);

      const index = new CDSIndex(
        'interactive_index',
        formState.indexFamily,
        formState.series,
        formState.version,
        notional,
        formState.spreadBps,
        effectiveDate,
        maturityDate,
        initialIndex?.discountCurveId ?? 'USD-OIS',
        initialIndex?.hazardCurveId ?? 'CDX-IG-HZD',
        formState.direction,
        formState.recoveryRate,
        null
      );

      const indexOpts = new PricingRequest().withMetrics(['par_spread']);
      const indexResult = registry.priceInstrument(index, 'discounting', market, asOf, indexOpts);

      const result: InstrumentRow = {
        name: `${formState.indexFamily} S${formState.series} V${formState.version}`,
        type: 'CDSIndex',
        presentValue: indexResult.presentValue.amount,
        keyMetric: {
          name: 'Par Spread',
          value: (() => {
            const raw = indexResult.metric('par_spread') ?? 0;
            return Math.abs(raw) > 10 ? raw : raw * 10000;
          })(),
        },
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS Index pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialIndex]);

  useEffect(() => {
    if (showForm) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      calculateIndex();
    }
  }, [showForm, calculateIndex]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = standardRegistry();
        const results: InstrumentRow[] = [];

        for (const indexInstrData of cdsIndices) {
          const notional = Money.fromCode(
            indexInstrData.notional.amount,
            indexInstrData.notional.currency
          );
          const effectiveDate = new FsDate(
            indexInstrData.effectiveDate.year,
            indexInstrData.effectiveDate.month,
            indexInstrData.effectiveDate.day
          );
          const maturityDate = new FsDate(
            indexInstrData.maturityDate.year,
            indexInstrData.maturityDate.month,
            indexInstrData.maturityDate.day
          );

          const index = new CDSIndex(
            indexInstrData.id,
            indexInstrData.indexFamily,
            indexInstrData.series,
            indexInstrData.version,
            notional,
            indexInstrData.spreadBps,
            effectiveDate,
            maturityDate,
            indexInstrData.discountCurveId,
            indexInstrData.hazardCurveId,
            indexInstrData.direction,
            indexInstrData.recoveryRate,
            null
          );

          const indexOpts = new PricingRequest().withMetrics(['par_spread']);
          try {
            const indexResult = registry.priceInstrument(
              index,
              'discounting',
              market,
              asOf,
              indexOpts
            );
            results.push({
              name: `${indexInstrData.indexFamily} S${indexInstrData.series} V${indexInstrData.version}`,
              type: 'CDSIndex',
              presentValue: indexResult.presentValue.amount,
              keyMetric: {
                name: 'Par Spread',
                value: (() => {
                  const raw = indexResult.metric('par_spread') ?? 0;
                  return Math.abs(raw) > 10 ? raw : raw * 10000;
                })(),
              },
            });
          } catch (err) {
            console.warn('CDS index pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS indices priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS Index pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsIndices, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSIndexFormState, value: string | number) => {
    setFormState((prev) => ({ ...prev, [field]: value }));
  };

  if (error && !showForm) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0 && !showForm) {
    return <p>Loading CDS indices...</p>;
  }

  return (
    <div className="instrument-group">
      <h3>
        CDS Indices{' '}
        <button
          className="toggle-form-btn"
          onClick={() => setShowForm(!showForm)}
          style={{ marginLeft: '1rem', fontSize: '0.8rem' }}
        >
          {showForm ? '✕ Hide Calculator' : '⚙ Interactive Calculator'}
        </button>
      </h3>
      <p>
        Standardized credit index contracts (e.g., CDX, iTraxx) representing a basket of credit
        default swaps. Provides diversified credit exposure with standardized terms.
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
            <label htmlFor="idx-notional">Notional</label>
            <input
              id="idx-notional"
              type="number"
              value={formState.notional}
              onChange={(e) =>
                handleInputChange('notional', Number.parseFloat(e.target.value) || 0)
              }
              step={1000000}
            />
          </div>
          <div className="form-field">
            <label htmlFor="idx-spread">Spread (bps)</label>
            <input
              id="idx-spread"
              type="number"
              value={formState.spreadBps}
              onChange={(e) =>
                handleInputChange('spreadBps', Number.parseFloat(e.target.value) || 0)
              }
              step={5}
            />
          </div>
          <div className="form-field">
            <label htmlFor="idx-series">Series</label>
            <input
              id="idx-series"
              type="number"
              value={formState.series}
              onChange={(e) =>
                handleInputChange('series', Number.parseInt(e.target.value, 10) || 1)
              }
              min={1}
            />
          </div>
          <div className="form-field">
            <label htmlFor="idx-version">Version</label>
            <input
              id="idx-version"
              type="number"
              value={formState.version}
              onChange={(e) =>
                handleInputChange('version', Number.parseInt(e.target.value, 10) || 1)
              }
              min={1}
            />
          </div>
          <div className="form-field">
            <label htmlFor="idx-recovery">Recovery Rate</label>
            <input
              id="idx-recovery"
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
            <label htmlFor="idx-direction">Direction</label>
            <select
              id="idx-direction"
              value={formState.direction}
              onChange={(e) => handleInputChange('direction', e.target.value)}
            >
              <option value="pay_protection">Pay Protection</option>
              <option value="receive_protection">Receive Protection</option>
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
