import React, { useEffect, useState } from 'react';
import {
  BaseCorrelationCurve,
  CDSIndex,
  CdsOption,
  CdsTranche,
  CreditDefaultSwap,
  CreditIndexData,
  Date as FsDate,
  DiscountCurve,
  HazardCurve,
  MarketContext,
  Money,
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
          new Float64Array([1.0, 0.9980, 0.9960, 0.9850, 0.9600]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const acmeHazard = new HazardCurve(
          'ACME-HZD',
          asOf,
          new Float64Array([0.0, 3.0, 5.0]),
          new Float64Array([0.0120, 0.0180, 0.0220]),
          0.40,
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
          new Float64Array([0.0100, 0.0160, 0.0190, 0.0210]),
          0.40,
          'act_365f',
          null,
          null,
          null,
          null,
          null
        );

        const baseCorr = new BaseCorrelationCurve(
          'CDX-IG-BC',
          new Float64Array([0.03, 0.06, 0.10, 0.30, 0.70, 1.00]),
          new Float64Array([0.10, 0.12, 0.15, 0.20, 0.23, 0.25])
        );

        const indexData = new CreditIndexData(125, 0.40, indexHazard, baseCorr, null, null);

        // Add CDS volatility surface for options (flattened grid: row-major order)
        const cdsVol = new VolSurface(
          'CDS-VOL',
          [0.5, 1.0, 3.0, 5.0],
          [0.0100, 0.0200, 0.0400],
          [0.45, 0.40, 0.35, 0.42, 0.38, 0.33, 0.38, 0.35, 0.30, 0.35, 0.32, 0.28]
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
        const cdsResult = registry.priceCreditDefaultSwapWithMetrics(
          cds,
          'discounting',
          market,
          ['par_spread', 'pv01']
        );
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
        const indexResult = registry.priceCDSIndexWithMetrics(
          index,
          'discounting',
          market,
          ['par_spread']
        );
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
        Credit instruments including single-name CDS, CDS indices, tranches, and options on CDS.
        Uses hazard curves for survival probabilities and base correlation for tranche pricing.
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
              <td>
                {keyMetric
                  ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)} bps`
                  : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};

