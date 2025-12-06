import React, { useEffect, useState } from 'react';
import {
  Autocallable,
  Basket,
  FsDate,
  DiscountCurve,
  MarketContext,
  MarketScalar,
  Money,
  PrivateMarketsFund,
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
          new Float64Array([1.0, 0.995, 0.98, 0.96]),
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

        // Add volatility surface for autocallables
        const equityVol = new VolSurface(
          'EQUITY-VOL',
          new Float64Array([0.25, 0.5, 1.0, 2.0]),
          new Float64Array([120.0, 140.0, 160.0, 180.0]),
          new Float64Array([
            0.28, 0.26, 0.25, 0.24, 0.27, 0.25, 0.24, 0.23, 0.26, 0.24, 0.23, 0.22, 0.25, 0.23,
            0.22, 0.21,
          ])
        );
        market.insertSurface(equityVol);

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
            tranches: ['return_of_capital', { preferred_irr: { irr: 0.08 } }],
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

        // Autocallable - Simple autocall with barrier
        const autocallableJson = JSON.stringify({
          id: 'autocallable_simple',
          underlying_ticker: 'AAPL',
          notional: { amount: 1_000_000.0, currency: 'USD' },
          observation_dates: ['2025-01-02', '2026-01-02'],
          autocall_barriers: [1.2, 1.2],
          coupons: [0.08, 0.1],
          final_barrier: 0.75,
          final_payoff_type: {
            CapitalProtection: { floor: 0.9 },
          },
          participation_rate: 1.0,
          cap_level: 2.0,
          day_count: 'act_365f',
          disc_id: 'USD-OIS',
          spot_id: 'AAPL-SPOT',
          vol_id: 'EQUITY-VOL',
          div_yield_id: null,
          pricing_overrides: {
            quoted_clean_price: null,
            implied_volatility: null,
            quoted_spread_bp: null,
            upfront_payment: null,
            ytm_bump_bp: null,
            theta_period: null,
            mc_seed_scenario: null,
            adaptive_bumps: false,
            spot_bump_pct: null,
            vol_bump_pct: null,
            rate_bump_bp: null,
          },
          attributes: { tags: [], meta: {} },
        });
        const autocallable = Autocallable.fromJson(autocallableJson);
        const autocallableResult = registry.priceAutocallable(
          autocallable,
          'monte_carlo_gbm',
          market
        );
        results.push({
          name: 'Autocallable Note',
          type: 'Autocallable',
          presentValue: autocallableResult.presentValue.amount,
          complexity: '8% coupon, 120% barrier',
        });

        if (!cancelled) {
          setRows(results);
        }

        // Cleanup
        equityVol.free();
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
        Complex structured instruments including baskets, ABS, CLO, autocallables, and private
        markets funds. These instruments use JSON-based definitions for flexible modeling of complex
        structures.
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

      <div
        style={{
          marginTop: '2rem',
          padding: '1rem',
          backgroundColor: 'rgba(100, 108, 255, 0.05)',
          borderRadius: '6px',
        }}
      >
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
