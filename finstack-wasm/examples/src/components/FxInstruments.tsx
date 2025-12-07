import React, { useEffect, useState } from 'react';
import {
  Currency,
  FsDate,
  DiscountCurve,
  FxMatrix,
  FxOption,
  FxSpot,
  FxSwap,
  MarketContext,
  Money,
  PricingRequest,
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
  pair: string;
  presentValue: number;
  keyMetric?: { name: string; value: number };
};

export const FxInstrumentsExample: React.FC = () => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(2024, 1, 2);
        const usd = new Currency('USD');
        const eur = new Currency('EUR');

        // Build market
        const usdDisc = new DiscountCurve(
          'USD-OIS',
          asOf,
          new Float64Array([0.0, 0.5, 1.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.9975, 0.9945, 0.972, 0.945]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const eurDisc = new DiscountCurve(
          'EUR-OIS',
          asOf,
          new Float64Array([0.0, 0.5, 1.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.998, 0.996, 0.98, 0.955]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const fx = new FxMatrix();
        fx.setQuote(eur, usd, 1.085);

        // Add FX volatility surface for options (flattened grid: row-major order)
        const fxVol = new VolSurface(
          'FX-VOL',
          new Float64Array([0.25, 0.5, 1.0, 2.0]),
          new Float64Array([1.05, 1.1, 1.15]),
          new Float64Array([0.14, 0.13, 0.12, 0.13, 0.12, 0.11, 0.12, 0.11, 0.1, 0.11, 0.1, 0.095])
        );

        const market = new MarketContext();
        market.insertDiscount(usdDisc);
        market.insertDiscount(eurDisc);
        market.insertFx(fx);
        market.insertSurface(fxVol);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // FX Spot
        const spot = new FxSpot(
          'eurusd_spot',
          eur,
          usd,
          new FsDate(2024, 1, 4), // T+2 settlement
          1.086,
          Money.fromCode(1_000_000, 'EUR')
        );
        const spotResult = registry.priceFxSpot(spot, 'discounting', market, asOf);
        results.push({
          name: 'EUR/USD Spot',
          type: 'FxSpot',
          pair: 'EURUSD',
          presentValue: spotResult.presentValue.amount,
        });

        // FX Option - Call
        const call = FxOption.europeanCall(
          'eurusd_call',
          eur,
          usd,
          1.1,
          new FsDate(2025, 1, 2),
          Money.fromCode(2_000_000, 'EUR'),
          'USD-OIS',
          'EUR-OIS',
          'FX-VOL'
        );
        const callOpts = new PricingRequest().withMetrics(['delta']);
        const callResult = registry.priceFxOption(call, 'discounting', market, asOf, callOpts);
        results.push({
          name: '1Y Call @ 1.10',
          type: 'FxOption',
          pair: 'EURUSD',
          presentValue: callResult.presentValue.amount,
          keyMetric: {
            name: 'Delta',
            // Normalize delta to per-unit if it comes back as notional-adjusted
            value:
              Math.abs(callResult.metric('delta') ?? 0) > 100
                ? (callResult.metric('delta') ?? 0) / 2_000_000 // Normalize by notional
                : (callResult.metric('delta') ?? 0),
          },
        });

        // FX Option - Put
        const put = FxOption.europeanPut(
          'eurusd_put',
          eur,
          usd,
          1.06,
          new FsDate(2024, 7, 2),
          Money.fromCode(1_500_000, 'EUR'),
          'USD-OIS',
          'EUR-OIS',
          'FX-VOL'
        );
        const putResult = registry.priceFxOption(put, 'discounting', market, asOf, null);
        results.push({
          name: '6M Put @ 1.06',
          type: 'FxOption',
          pair: 'EURUSD',
          presentValue: putResult.presentValue.amount,
        });

        // FX Swap
        const fxSwap = new FxSwap(
          'eurusd_swap',
          eur,
          usd,
          Money.fromCode(5_000_000, 'EUR'),
          new FsDate(2024, 1, 4),
          new FsDate(2024, 7, 4),
          'USD-OIS',
          'EUR-OIS',
          1.0865,
          1.092
        );
        const swapResult = registry.priceFxSwap(fxSwap, 'discounting', market, asOf);
        results.push({
          name: '6M FX Swap',
          type: 'FxSwap',
          pair: 'EURUSD',
          presentValue: swapResult.presentValue.amount,
        });

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('FX instruments error:', err);
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
    return <p>Building FX instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>FX Instruments</h2>
      <p>
        Foreign exchange instruments including spot transactions, European options (calls/puts), and
        FX swaps with near and far legs.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Pair</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, pair, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{pair}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(4)}` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
