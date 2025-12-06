import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DiscountCurve,
  InflationCurve,
  InflationLinkedBond,
  InflationSwap,
  MarketContext,
  Money,
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
  keyMetric?: { name: string; value: string };
};

export const InflationInstrumentsExample: React.FC = () => {
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
          new Float64Array([1.0, 0.998, 0.996, 0.982, 0.96]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const inflationCurve = new InflationCurve(
          'US-CPI',
          300.0,
          new Float64Array([0.0, 1.0, 2.0, 5.0, 10.0]),
          new Float64Array([300.0, 303.0, 306.5, 320.0, 345.0]),
          'log_linear'
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        market.insertInflation(inflationCurve);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Inflation-Linked Bond (TIPS)
        const ilb = new InflationLinkedBond(
          'tips_2033',
          Money.fromCode(1_000_000, 'USD'),
          0.0125,
          asOf,
          new FsDate(2034, 1, 15),
          300.0,
          'USD-OIS',
          'US-CPI',
          'tips',
          'semi_annual',
          null,
          null
        );
        const ilbResult = registry.priceInflationLinkedBond(ilb, 'discounting', market);
        results.push({
          name: 'US TIPS 2034',
          type: 'InflationLinkedBond',
          presentValue: ilbResult.presentValue.amount,
          keyMetric: { name: 'Real Coupon', value: '1.25%' },
        });

        // Inflation Swap
        const infSwap = new InflationSwap(
          'zc_inflation_swap',
          Money.fromCode(5_000_000, 'USD'),
          0.025,
          asOf,
          new FsDate(2030, 1, 2),
          'USD-OIS',
          'US-CPI',
          'pay_fixed',
          'act_act'
        );
        const swapResult = registry.priceInflationSwap(infSwap, 'discounting', market);
        results.push({
          name: 'ZC Inflation Swap (6Y)',
          type: 'InflationSwap',
          presentValue: swapResult.presentValue.amount,
          keyMetric: { name: 'Fixed Rate', value: '2.50%' },
        });

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Inflation instruments error:', err);
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
    return <p>Building inflation instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>Inflation Instruments</h2>
      <p>
        Inflation-linked bonds (TIPS-style) and zero-coupon inflation swaps. These instruments use
        inflation curves to project CPI levels and adjust cashflows accordingly.
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
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value}` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
