/**
 * Revolving Credit instrument component with interactive form.
 */
import React, { useEffect, useState, useCallback } from 'react';
import { FsDate, MarketContext, RevolvingCredit, createStandardRegistry } from 'finstack-wasm';
import type { RevolvingCreditData } from '../data/credit';
import { currencyFormatter, type InstrumentRow } from './useCreditMarket';

export interface RevolvingCreditInstrumentProps {
  revolvingCredits: RevolvingCreditData[];
  market: MarketContext;
  asOf: FsDate;
}

interface RevolvingCreditFormState {
  commitmentAmount: number;
  drawnAmount: number;
  fixedRate: number;
  commitmentFeeBp: number;
  usageFeeBp: number;
  facilityFeeBp: number;
  tenorYears: number;
  currency: string;
}

export const RevolvingCreditInstrument: React.FC<RevolvingCreditInstrumentProps> = ({
  revolvingCredits,
  market,
  asOf,
}) => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // Initialize form state from first revolving credit
  const initialRc = revolvingCredits[0];
  const [formState, setFormState] = useState<RevolvingCreditFormState>({
    commitmentAmount: initialRc?.commitmentAmount.amount ?? 10_000_000,
    drawnAmount: initialRc?.drawnAmount.amount ?? 5_000_000,
    fixedRate: (initialRc?.baseRateSpec as { Fixed?: { rate: number } })?.Fixed?.rate ?? 0.05,
    commitmentFeeBp: initialRc?.fees.commitmentFeeBp ?? 25,
    usageFeeBp: initialRc?.fees.usageFeeBp ?? 10,
    facilityFeeBp: initialRc?.fees.facilityFeeBp ?? 5,
    tenorYears: 2,
    currency: initialRc?.commitmentAmount.currency ?? 'USD',
  });

  const calculateRevolvingCredit = useCallback(() => {
    try {
      const registry = createStandardRegistry();

      const commitmentDate = `${asOf.year}-${String(asOf.month).padStart(2, '0')}-${String(asOf.day).padStart(2, '0')}`;
      const maturityDate = `${asOf.year + formState.tenorYears}-${String(asOf.month).padStart(2, '0')}-${String(asOf.day).padStart(2, '0')}`;

      const revolvingCreditJson = JSON.stringify({
        id: 'interactive_rc',
        commitment_amount: { amount: formState.commitmentAmount, currency: formState.currency },
        drawn_amount: { amount: formState.drawnAmount, currency: formState.currency },
        commitment_date: commitmentDate,
        maturity_date: maturityDate,
        base_rate_spec: { Fixed: { rate: formState.fixedRate } },
        day_count: 'act360',
        payment_frequency: { count: 3, unit: 'months' },
        fees: {
          upfront_fee: null,
          commitment_fee_bp: formState.commitmentFeeBp,
          usage_fee_bp: formState.usageFeeBp,
          facility_fee_bp: formState.facilityFeeBp,
        },
        draw_repay_spec: { Deterministic: [] },
        discount_curve_id: initialRc?.discountCurveId ?? 'USD-OIS',
        attributes: { tags: [], meta: {} },
      });

      const revolvingCredit = RevolvingCredit.fromJson(revolvingCreditJson);
      const rcResult = registry.priceInstrument(revolvingCredit, 'discounting', market, asOf, null);
      const utilization = (formState.drawnAmount / formState.commitmentAmount) * 100;

      const result: InstrumentRow = {
        name: 'Revolving Credit (Interactive)',
        type: 'RevolvingCredit',
        presentValue: rcResult.presentValue.amount,
        keyMetric: {
          name: 'Utilization',
          value: utilization,
        },
      };

      setRows([result]);
      setError(null);
    } catch (err) {
      setError(`Revolving Credit pricing error: ${err}`);
    }
  }, [formState, market, asOf, initialRc]);

  useEffect(() => {
    if (showForm) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      calculateRevolvingCredit();
    }
  }, [showForm, calculateRevolvingCredit]);

  useEffect(() => {
    if (showForm) return;

    let cancelled = false;

    (async () => {
      try {
        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        for (const rcData of revolvingCredits) {
          try {
            const revolvingCreditJson = JSON.stringify({
              id: rcData.id,
              commitment_amount: {
                amount: rcData.commitmentAmount.amount,
                currency: rcData.commitmentAmount.currency,
              },
              drawn_amount: {
                amount: rcData.drawnAmount.amount,
                currency: rcData.drawnAmount.currency,
              },
              commitment_date: rcData.commitmentDate,
              maturity_date: rcData.maturityDate,
              base_rate_spec: rcData.baseRateSpec,
              day_count: rcData.dayCount,
              payment_frequency: rcData.paymentFrequency,
              fees: {
                upfront_fee: rcData.fees.upfrontFee,
                commitment_fee_bp: rcData.fees.commitmentFeeBp,
                usage_fee_bp: rcData.fees.usageFeeBp,
                facility_fee_bp: rcData.fees.facilityFeeBp,
              },
              draw_repay_spec: rcData.drawRepaySpec,
              discount_curve_id: rcData.discountCurveId,
              attributes: { tags: [], meta: {} },
            });
            const revolvingCredit = RevolvingCredit.fromJson(revolvingCreditJson);
            const rcResult = registry.priceInstrument(
              revolvingCredit,
              'discounting',
              market,
              asOf,
              null
            );
            const utilization = (rcData.drawnAmount.amount / rcData.commitmentAmount.amount) * 100;

            const mode = 'Deterministic' in rcData.drawRepaySpec ? 'Deterministic' : 'Stochastic';

            results.push({
              name: `Revolving Credit (${mode})`,
              type: 'RevolvingCredit',
              presentValue: rcResult.presentValue.amount,
              keyMetric: {
                name: 'Utilization',
                value: utilization,
              },
            });
          } catch (err) {
            console.warn('Revolving credit failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No revolving credits priced');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(`Revolving Credit pricing error: ${err}`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [revolvingCredits, market, asOf, showForm]);

  const handleInputChange = (field: keyof RevolvingCreditFormState, value: number) => {
    setFormState((prev) => ({ ...prev, [field]: value }));
  };

  if (error && !showForm) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0 && !showForm) {
    return <p>Loading revolving credits...</p>;
  }

  return (
    <div className="instrument-group">
      <h3>
        Revolving Credit Facilities{' '}
        <button
          className="toggle-form-btn"
          onClick={() => setShowForm(!showForm)}
          style={{ marginLeft: '1rem', fontSize: '0.8rem' }}
        >
          {showForm ? '✕ Hide Calculator' : '⚙ Interactive Calculator'}
        </button>
      </h3>
      <p>
        Bank credit facilities with flexible draw/repay schedules. Supports both deterministic
        utilization scenarios and stochastic mean-reverting utilization processes for Monte Carlo
        valuation. Includes commitment fees, usage fees, and facility fees.
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
            <label htmlFor="rc-commitment">Commitment Amount</label>
            <input
              id="rc-commitment"
              type="number"
              value={formState.commitmentAmount}
              onChange={(e) =>
                handleInputChange('commitmentAmount', Number.parseFloat(e.target.value) || 0)
              }
              step={1000000}
            />
          </div>
          <div className="form-field">
            <label htmlFor="rc-drawn">Drawn Amount</label>
            <input
              id="rc-drawn"
              type="number"
              value={formState.drawnAmount}
              onChange={(e) =>
                handleInputChange('drawnAmount', Number.parseFloat(e.target.value) || 0)
              }
              step={500000}
              max={formState.commitmentAmount}
            />
          </div>
          <div className="form-field">
            <label htmlFor="rc-tenor">Tenor (years)</label>
            <input
              id="rc-tenor"
              type="number"
              value={formState.tenorYears}
              onChange={(e) =>
                handleInputChange('tenorYears', Number.parseInt(e.target.value, 10) || 1)
              }
              min={1}
              max={10}
            />
          </div>
          <div className="form-field">
            <label htmlFor="rc-rate">Fixed Rate (%)</label>
            <input
              id="rc-rate"
              type="number"
              value={(formState.fixedRate * 100).toFixed(2)}
              onChange={(e) =>
                handleInputChange('fixedRate', (Number.parseFloat(e.target.value) || 0) / 100)
              }
              step={0.25}
              min={0}
            />
          </div>
          <div className="form-field">
            <label htmlFor="rc-commit-fee">Commitment Fee (bps)</label>
            <input
              id="rc-commit-fee"
              type="number"
              value={formState.commitmentFeeBp}
              onChange={(e) =>
                handleInputChange('commitmentFeeBp', Number.parseFloat(e.target.value) || 0)
              }
              step={5}
              min={0}
            />
          </div>
          <div className="form-field">
            <label htmlFor="rc-usage-fee">Usage Fee (bps)</label>
            <input
              id="rc-usage-fee"
              type="number"
              value={formState.usageFeeBp}
              onChange={(e) =>
                handleInputChange('usageFeeBp', Number.parseFloat(e.target.value) || 0)
              }
              step={5}
              min={0}
            />
          </div>
          <div className="form-field">
            <label htmlFor="rc-facility-fee">Facility Fee (bps)</label>
            <input
              id="rc-facility-fee"
              type="number"
              value={formState.facilityFeeBp}
              onChange={(e) =>
                handleInputChange('facilityFeeBp', Number.parseFloat(e.target.value) || 0)
              }
              step={5}
              min={0}
            />
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
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(1)}%` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
