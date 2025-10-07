import React, { useEffect, useState } from 'react';
import {
  Basket,
  FsDate,
  DiscountCurve,
  MarketContext,
  MarketScalar,
  Money,
  PrivateMarketsFund,
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
  complexity: string;
};

export const StructuredProductsExample: React.FC = () => {
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
          new Float64Array([0.0, 1.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.9950, 0.9800, 0.9600]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        
        // Add spot prices for basket constituents
        market.insertPrice('AAPL-SPOT', MarketScalar.price(Money.fromCode(150.0, 'USD')));
        market.insertPrice('MSFT-SPOT', MarketScalar.price(Money.fromCode(380.0, 'USD')));

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Basket - Simple multi-asset basket
        const basketJson = JSON.stringify({
          id: 'multi_asset_basket',
          currency: 'USD',
          discount_curve_id: 'USD-OIS',
          expense_ratio: 0.0025,
          constituents: [
            {
              id: 'AAPL',
              reference: { price_id: 'AAPL-SPOT', asset_type: 'equity' },
              weight: 0.5,
              ticker: 'AAPL',
            },
            {
              id: 'MSFT',
              reference: { price_id: 'MSFT-SPOT', asset_type: 'equity' },
              weight: 0.5,
              ticker: 'MSFT',
            },
          ],
          pricing_config: {
            days_in_year: 365.25,
            fx_policy: 'cashflow_date',
          },
        });
        const basket = Basket.fromJson(basketJson);
        const basketResult = registry.priceBasket(basket, 'discounting', market);
        results.push({
          name: 'Tech Stock Basket',
          type: 'Basket',
          presentValue: basketResult.presentValue.amount,
          complexity: '2 constituents',
        });

        // Private Markets Fund - Simplified
        const fundJson = JSON.stringify({
          id: 'pe_fund_1',
          currency: 'USD',
          disc_id: 'USD-OIS',
          spec: {
            style: 'european',
            catchup_mode: 'full',
            irr_basis: 'act_365f',
            tranches: [
              'return_of_capital',
              { preferred_irr: { irr: 0.08 } },
            ],
          },
          events: [
            {
              date: '2024-01-02',
              amount: { amount: 2_000_000.0, currency: 'USD' },
              kind: 'contribution',
            },
            {
              date: '2028-12-31',
              amount: { amount: 3_000_000.0, currency: 'USD' },
              kind: 'distribution',
            },
          ],
        });
        const fund = PrivateMarketsFund.fromJson(fundJson);
        const fundResult = registry.pricePrivateMarketsFund(fund, 'discounting', market);
        results.push({
          name: 'PE Fund (8% Pref)',
          type: 'PrivateMarketsFund',
          presentValue: fundResult.presentValue.amount,
          complexity: '2 events, waterfall',
        });

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Structured products error:', err);
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
    return <p>Building structured products…</p>;
  }

  return (
    <section className="example-section">
      <h2>Structured Products</h2>
      <p>
        Complex structured instruments including baskets, ABS, CLO, and private markets funds.
        These instruments use JSON-based definitions for flexible modeling of complex structures.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Present Value</th>
            <th>Complexity</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, presentValue, complexity }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{complexity}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <div style={{ marginTop: '2rem', padding: '1rem', backgroundColor: 'rgba(100, 108, 255, 0.05)', borderRadius: '6px' }}>
        <h3 style={{ fontSize: '1.1rem', marginBottom: '0.5rem' }}>JSON-Based Instruments</h3>
        <p style={{ color: '#aaa', margin: 0 }}>
          Structured products (ABS, CLO, CMBS, RMBS, Basket, PrivateMarketsFund) use JSON
          definitions for maximum flexibility. Create instruments using <code>fromJson()</code> and
          serialize with <code>toJson()</code> for storage or transmission.
        </p>
      </div>
    </section>
  );
};

