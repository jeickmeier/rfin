import React, { useEffect, useState } from 'react';
import {
  BaseCorrelationCurve,
  CDSIndex,
  CdsOption,
  CdsTranche,
  CreditDefaultSwap,
  CreditIndexData,
  FsDate,
  DiscountCurve,
  HazardCurve,
  MarketContext,
  Money,
  PricingRequest,
  RevolvingCredit,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

type InstrumentRow = {
  name: string;
  type: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
};

export const CreditInstrumentsExample: React.FC = () => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(2024, 1, 2);

        // Build market
        const discountCurve = new DiscountCurve(
          'USD-OIS',
          asOf,
          new Float64Array([0.0, 0.5, 1.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.998, 0.996, 0.985, 0.96]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const acmeHazard = new HazardCurve(
          'ACME-HZD',
          asOf,
          new Float64Array([0.0, 3.0, 5.0]),
          new Float64Array([0.012, 0.018, 0.022]),
          0.4,
          'act_365f',
          null,
          null,
          null,
          null,
          null
        );

        const indexHazard = new HazardCurve(
          'CDX-IG-HZD',
          asOf,
          new Float64Array([0.0, 3.0, 5.0, 7.0]),
          new Float64Array([0.01, 0.016, 0.019, 0.021]),
          0.4,
          'act_365f',
          null,
          null,
          null,
          null,
          null
        );

        const baseCorr = new BaseCorrelationCurve(
          'CDX-IG-BC',
          new Float64Array([0.03, 0.06, 0.1, 0.3, 0.7, 1.0]),
          new Float64Array([0.1, 0.12, 0.15, 0.2, 0.23, 0.25])
        );

        const indexData = new CreditIndexData(125, 0.4, indexHazard, baseCorr, null, null);

        // Add CDS volatility surface for options (flattened grid: row-major order)
        const cdsVol = new VolSurface(
          'CDS-VOL',
          new Float64Array([0.5, 1.0, 3.0, 5.0]),
          new Float64Array([0.01, 0.02, 0.04]),
          new Float64Array([0.45, 0.4, 0.35, 0.42, 0.38, 0.33, 0.38, 0.35, 0.3, 0.35, 0.32, 0.28])
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        market.insertHazard(acmeHazard);
        market.insertHazard(indexHazard);
        market.insertCreditIndex('CDX.NA.IG', indexData);
        market.insertSurface(cdsVol);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Credit Default Swap
        const cds = CreditDefaultSwap.buyProtection(
          'acme_cds',
          Money.fromCode(10_000_000, 'USD'),
          120.0,
          new FsDate(2024, 1, 3),
          new FsDate(2029, 1, 2),
          'USD-OIS',
          'ACME-HZD',
          null
        );
        const cdsOpts = new PricingRequest().withMetrics(['par_spread', 'pv01']);
        const cdsResult = registry.priceCreditDefaultSwap(cds, 'discounting', market, cdsOpts);
        results.push({
          name: 'ACME 5Y CDS',
          type: 'CreditDefaultSwap',
          presentValue: cdsResult.presentValue.amount,
          keyMetric: {
            name: 'Par Spread',
            // Check if already in bps or decimal - if > 10, assume already in bps
            value: (() => {
              const raw = cdsResult.metric('par_spread') ?? 0;
              return Math.abs(raw) > 10 ? raw : raw * 10000;
            })(),
          },
        });

        // CDS Index
        const index = new CDSIndex(
          'cdx_trad',
          'CDX.NA.IG',
          42,
          1,
          Money.fromCode(25_000_000, 'USD'),
          100.0,
          new FsDate(2024, 1, 3),
          new FsDate(2029, 1, 2),
          'USD-OIS',
          'CDX-IG-HZD',
          'pay_protection',
          0.4,
          null
        );
        const indexOpts = new PricingRequest().withMetrics(['par_spread']);
        const indexResult = registry.priceCDSIndex(index, 'discounting', market, indexOpts);
        results.push({
          name: 'CDX.NA.IG S42 V1',
          type: 'CDSIndex',
          presentValue: indexResult.presentValue.amount,
          keyMetric: {
            name: 'Par Spread',
            // Check if already in bps or decimal - if > 10, assume already in bps
            value: (() => {
              const raw = indexResult.metric('par_spread') ?? 0;
              return Math.abs(raw) > 10 ? raw : raw * 10000;
            })(),
          },
        });

        // CDS Tranche
        const tranche = new CdsTranche(
          'cdx_mez_tranche',
          'CDX.NA.IG',
          42,
          3.0,
          7.0,
          Money.fromCode(10_000_000, 'USD'),
          new FsDate(2029, 1, 2),
          500.0,
          'USD-OIS',
          'CDX-IG-HZD',
          'buy_protection',
          4,
          null
        );
        const trancheResult = registry.priceCdsTranche(tranche, 'discounting', market);
        results.push({
          name: 'CDX Mezzanine (3-7%)',
          type: 'CdsTranche',
          presentValue: trancheResult.presentValue.amount,
        });

        // CDS Option
        const option = new CdsOption(
          'acme_cdsopt',
          Money.fromCode(5_000_000, 'USD'),
          150.0,
          new FsDate(2025, 1, 2),
          new FsDate(2029, 1, 2),
          'USD-OIS',
          'ACME-HZD',
          'CDS-VOL',
          'call',
          0.4,
          false,
          null
        );
        const optionResult = registry.priceCdsOption(option, 'discounting', market);
        results.push({
          name: 'CDS Option @ 150bp',
          type: 'CdsOption',
          presentValue: optionResult.presentValue.amount,
        });

        // Revolving Credit Facility - Deterministic
        const revolvingCreditDetJson = JSON.stringify({
          id: 'rc_facility_det',
          commitment_amount: { amount: 10_000_000.0, currency: 'USD' },
          drawn_amount: { amount: 5_000_000.0, currency: 'USD' },
          commitment_date: '2024-01-02',
          maturity_date: '2026-01-02',
          base_rate_spec: {
            Fixed: { rate: 0.05 },
          },
          day_count: 'act360',
          payment_frequency: { Months: 3 },
          fees: {
            upfront_fee: { amount: 50_000.0, currency: 'USD' },
            commitment_fee_bp: 25.0,
            usage_fee_bp: 10.0,
            facility_fee_bp: 5.0,
          },
          draw_repay_spec: {
            Deterministic: [
              {
                date: '2024-07-01',
                amount: { amount: 2_000_000.0, currency: 'USD' },
                is_draw: true,
              },
              {
                date: '2025-01-01',
                amount: { amount: 1_000_000.0, currency: 'USD' },
                is_draw: false,
              },
            ],
          },
          disc_id: 'USD-OIS',
          attributes: { tags: [], meta: {} },
        });
        const revolvingCreditDet = RevolvingCredit.fromJson(revolvingCreditDetJson);
        const rcDetResult = registry.priceRevolvingCredit(
          revolvingCreditDet,
          'discounting',
          market
        );
        results.push({
          name: 'Revolving Credit (Deterministic)',
          type: 'RevolvingCredit',
          presentValue: rcDetResult.presentValue.amount,
          keyMetric: {
            name: 'Utilization',
            value: (5_000_000 / 10_000_000) * 100, // Initial utilization %
          },
        });

        // Revolving Credit Facility - Stochastic (Monte Carlo)
        const revolvingCreditStochJson = JSON.stringify({
          id: 'rc_facility_stoch',
          commitment_amount: { amount: 10_000_000.0, currency: 'USD' },
          drawn_amount: { amount: 4_000_000.0, currency: 'USD' },
          commitment_date: '2024-01-02',
          maturity_date: '2026-01-02',
          base_rate_spec: {
            Fixed: { rate: 0.055 },
          },
          day_count: 'act360',
          payment_frequency: { Months: 3 },
          fees: {
            upfront_fee: null,
            commitment_fee_bp: 30.0,
            usage_fee_bp: 15.0,
            facility_fee_bp: 10.0,
          },
          draw_repay_spec: {
            Stochastic: {
              utilization_process: {
                MeanReverting: {
                  target_rate: 0.6, // Target 60% utilization
                  speed: 2.0, // Mean reversion speed
                  volatility: 0.15, // 15% volatility
                },
              },
              num_paths: 1000,
              seed: 42,
            },
          },
          disc_id: 'USD-OIS',
          attributes: { tags: [], meta: {} },
        });
        const revolvingCreditStoch = RevolvingCredit.fromJson(revolvingCreditStochJson);
        const rcStochResult = registry.priceRevolvingCredit(
          revolvingCreditStoch,
          'monte_carlo_gbm',
          market
        );
        results.push({
          name: 'Revolving Credit (Stochastic MC)',
          type: 'RevolvingCredit',
          presentValue: rcStochResult.presentValue.amount,
          keyMetric: {
            name: 'Initial Util',
            value: (4_000_000 / 10_000_000) * 100, // Initial utilization %
          },
        });

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Credit instruments error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building credit instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>Credit Derivatives</h2>
      <p>
        Credit instruments including single-name CDS, CDS indices, tranches, options on CDS, and
        revolving credit facilities. Uses hazard curves for survival probabilities, base correlation
        for tranche pricing, and supports both deterministic and stochastic utilization for
        revolving credit.
      </p>

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
    </section>
  );
};
