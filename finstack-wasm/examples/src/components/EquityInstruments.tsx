import React, { useEffect, useState } from 'react';
import {
  Currency,
  Date as FsDate,
  DiscountCurve,
  Equity,
  EquityOption,
  MarketContext,
  MarketScalar,
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
  ticker: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
};

export const EquityInstrumentsExample: React.FC = () => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(2024, 1, 2);
        const usd = new Currency('USD');

        // Build market
        const discountCurve = new DiscountCurve(
          'USD-OIS',
          asOf,
          new Float64Array([0.0, 0.5, 1.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.9970, 0.9940, 0.9725, 0.9480]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        // Add equity market data (flattened grid: row-major order)
        const equityVol = new VolSurface(
          'EQUITY-VOL',
          [0.25, 0.5, 1.0, 2.0],
          [120.0, 140.0, 160.0, 180.0],
          [0.28, 0.26, 0.25, 0.24, 0.27, 0.25, 0.24, 0.23, 0.26, 0.24, 0.23, 0.22, 0.25, 0.23, 0.22, 0.21]
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        market.insertPrice('AAPL', MarketScalar.price(Money.fromCode(150.0, 'USD')));
        market.insertPrice('AAPL-SPOT', MarketScalar.price(Money.fromCode(150.0, 'USD')));
        market.insertPrice('EQUITY-SPOT', MarketScalar.price(Money.fromCode(150.0, 'USD')));
        market.insertPrice('AAPL-DIVYIELD', MarketScalar.unitless(0.015));
        market.insertPrice('EQUITY-DIVYIELD', MarketScalar.unitless(0.015));
        market.insertSurface(equityVol);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Equity Spot
        const equity = new Equity('aapl_position', 'AAPL', usd, 1000.0, null);
        const equityResult = registry.priceEquity(equity, 'discounting', market);
        results.push({
          name: 'AAPL Stock (1000 shares)',
          type: 'Equity',
          ticker: 'AAPL',
          presentValue: equityResult.presentValue.amount,
        });

        // Equity Call Option
        const call = EquityOption.europeanCall(
          'aapl_call_150',
          'AAPL',
          150.0,
          new FsDate(2024, 12, 31),
          Money.fromCode(150.0, 'USD'),
          100.0
        );
        const callResult = registry.priceEquityOptionWithMetrics(
          call,
          'discounting',
          market,
          ['delta', 'gamma']
        );
        results.push({
          name: 'AAPL Call @ $150 (1Y)',
          type: 'EquityOption',
          ticker: 'AAPL',
          presentValue: callResult.presentValue.amount,
          keyMetric: { name: 'Delta', value: callResult.metric('delta') ?? 0 },
        });

        // Equity Put Option
        const put = EquityOption.europeanPut(
          'aapl_put_140',
          'AAPL',
          140.0,
          new FsDate(2024, 9, 30),
          Money.fromCode(140.0, 'USD'),
          100.0
        );
        const putResult = registry.priceEquityOption(put, 'discounting', market);
        results.push({
          name: 'AAPL Put @ $140 (9M)',
          type: 'EquityOption',
          ticker: 'AAPL',
          presentValue: putResult.presentValue.amount,
        });

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Equity instruments error:', err);
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
    return <p>Building equity instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>Equity Instruments</h2>
      <p>
        Equity spot positions and European-style equity options (calls and puts). Options
        are priced using market data for spot prices and dividend yields.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Ticker</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, ticker, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{ticker}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>
                {keyMetric
                  ? `${keyMetric.name}: ${keyMetric.value.toFixed(4)}`
                  : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};

