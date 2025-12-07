/**
 * CDS Tranche instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import { CdsTranche, FsDate, MarketContext, Money, createStandardRegistry } from 'finstack-wasm';
import type { CdsTrancheData } from '../../data/credit';
import { currencyFormatter, type InstrumentRow } from './useCreditMarket';

export interface CDSTrancheInstrumentProps {
  cdsTranches: CdsTrancheData[];
  market: MarketContext;
  asOf: FsDate;
}

interface CDSTrancheFormState {
  notional: number;
  attachmentPoint: number; // As percentage (3 = 3%)
  detachmentPoint: number; // As percentage (7 = 7%)
  spreadBps: number;
  series: number;
  direction: 'buy_protection' | 'sell_protection';
  currency: string;
  indexFamily: string;
}

export const CDSTrancheInstrument: React.FC<CDSTrancheInstrumentProps> = ({
  cdsTranches,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first tranche
  const initialTranche = cdsTranches[0];
  const [formState, setFormState] = useState<CDSTrancheFormState>({
    notional: initialTranche?.notional.amount ?? 10_000_000,
    attachmentPoint: initialTranche?.attachmentPoint ?? 3,
    detachmentPoint: initialTranche?.detachmentPoint ?? 7,
    spreadBps: initialTranche?.spreadBps ?? 500,
    series: initialTranche?.series ?? 42,
    direction: initialTranche?.direction ?? 'buy_protection',
    currency: initialTranche?.notional.currency ?? 'USD',
    indexFamily: initialTranche?.indexFamily ?? 'CDX.NA.IG',
  });

  const calculateTranche = useCallback(() => {
    try {
      const registry = createStandardRegistry();
      const notional = Money.fromCode(formState.notional, formState.currency);
      const maturityDate = new FsDate(asOf.year + 5, asOf.month, asOf.day);

      const tranche = new CdsTranche(
        'interactive_tranche',
        formState.indexFamily,
        formState.series,
        formState.attachmentPoint,
        formState.detachmentPoint,
        notional,
        maturityDate,
        formState.spreadBps,
        initialTranche?.discountCurveId ?? 'USD-OIS',
        formState.indexFamily, // creditIndexId - must match the key in market.insertCreditIndex()
        formState.direction,
        initialTranche?.frequency ?? 4,
        null
      );

      const trancheResult = registry.priceCdsTranche(tranche, 'discounting', market, asOf, null);

      const result: InstrumentRow = {
        name: `${formState.indexFamily} Tranche (${formState.attachmentPoint}-${formState.detachmentPoint}%)`,
        type: 'CdsTranche',
        presentValue: trancheResult.presentValue.amount,
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`CDS Tranche pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialTranche]);

  // Calculate when form is visible and form state changes
  useEffect(() => {
    if (showForm) {
      // Use setTimeout to avoid React compiler warning about setState in effect
      const timer = setTimeout(() => {
        calculateTranche();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [showForm, formState, calculateTranche]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const trancheData of cdsTranches) {
          const notional = Money.fromCode(
            trancheData.notional.amount,
            trancheData.notional.currency
          );
          const maturityDate = new FsDate(
            trancheData.maturityDate.year,
            trancheData.maturityDate.month,
            trancheData.maturityDate.day
          );

          try {
            const tranche = new CdsTranche(
              trancheData.id,
              trancheData.indexFamily,
              trancheData.series,
              trancheData.attachmentPoint,
              trancheData.detachmentPoint,
              notional,
              maturityDate,
              trancheData.spreadBps,
              trancheData.discountCurveId,
              trancheData.creditIndexId,
              trancheData.direction,
              trancheData.frequency,
              null
            );
            const trancheResult = registry.priceCdsTranche(
              tranche,
              'discounting',
              market,
              asOf,
              null
            );

            results.push({
              name: `${trancheData.indexFamily} Tranche (${trancheData.attachmentPoint}-${trancheData.detachmentPoint}%)`,
              type: 'CdsTranche',
              presentValue: trancheResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('CDS tranche pricing failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No CDS tranches priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`CDS Tranche pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cdsTranches, market, asOf, showForm]);

  const handleInputChange = (field: keyof CDSTrancheFormState, value: string | number) => {
    setFormState((prev) => ({ ...prev, [field]: value }));
  };

  if (error && !showForm) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0 && !showForm) {
    return <p>Loading CDS tranches...</p>;
  }

  return (
    <div className="instrument-group">
      <h3>
        CDS Tranches{' '}
        <button
          className="toggle-form-btn"
          onClick={() => setShowForm(!showForm)}
          style={{ marginLeft: '1rem', fontSize: '0.8rem' }}
        >
          {showForm ? '✕ Hide Calculator' : '⚙ Interactive Calculator'}
        </button>
      </h3>
      <p>
        Synthetic CDO tranches with attachment and detachment points. Uses base correlation curves
        for correlated default modeling. Equity tranches absorb first losses, while senior tranches
        are protected by subordination.
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
            <label htmlFor="tranche-notional">Notional</label>
            <input
              id="tranche-notional"
              type="number"
              value={formState.notional}
              onChange={(e) =>
                handleInputChange('notional', Number.parseFloat(e.target.value) || 0)
              }
              step={1000000}
            />
          </div>
          <div className="form-field">
            <label htmlFor="tranche-attach">Attachment Point (%)</label>
            <input
              id="tranche-attach"
              type="number"
              value={formState.attachmentPoint}
              onChange={(e) =>
                handleInputChange('attachmentPoint', Number.parseFloat(e.target.value) || 0)
              }
              step={1}
              min={0}
              max={100}
            />
          </div>
          <div className="form-field">
            <label htmlFor="tranche-detach">Detachment Point (%)</label>
            <input
              id="tranche-detach"
              type="number"
              value={formState.detachmentPoint}
              onChange={(e) =>
                handleInputChange('detachmentPoint', Number.parseFloat(e.target.value) || 0)
              }
              step={1}
              min={0}
              max={100}
            />
          </div>
          <div className="form-field">
            <label htmlFor="tranche-spread">Spread (bps)</label>
            <input
              id="tranche-spread"
              type="number"
              value={formState.spreadBps}
              onChange={(e) =>
                handleInputChange('spreadBps', Number.parseFloat(e.target.value) || 0)
              }
              step={10}
            />
          </div>
          <div className="form-field">
            <label htmlFor="tranche-series">Series</label>
            <input
              id="tranche-series"
              type="number"
              value={formState.series}
              onChange={(e) =>
                handleInputChange('series', Number.parseInt(e.target.value, 10) || 1)
              }
              min={1}
            />
          </div>
          <div className="form-field">
            <label htmlFor="tranche-direction">Direction</label>
            <select
              id="tranche-direction"
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

      {rows.length > 0 && (
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
      )}
    </div>
  );
};
