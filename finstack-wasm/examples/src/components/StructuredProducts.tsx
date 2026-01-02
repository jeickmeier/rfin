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
import {
  StructuredProductsProps,
  DEFAULT_STRUCTURED_PRODUCTS_PROPS,
} from './data/structured-products';

type RequiredStructuredProductsProps = Required<StructuredProductsProps>;

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

export const StructuredProductsExample: React.FC<StructuredProductsProps> = (props) => {
  const defaults = DEFAULT_STRUCTURED_PRODUCTS_PROPS as RequiredStructuredProductsProps;
  const {
    valuationDate = defaults.valuationDate,
    market = defaults.market,
    baskets = defaults.baskets,
    privateMarketsFunds = defaults.privateMarketsFunds,
    autocallables = defaults.autocallables,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

        // Build discount curve
        const discountCurve = new DiscountCurve(
          market.discountCurve.id,
          asOf,
          new Float64Array(market.discountCurve.tenors),
          new Float64Array(market.discountCurve.discountFactors),
          market.discountCurve.dayCount,
          market.discountCurve.interpolation,
          market.discountCurve.extrapolation,
          market.discountCurve.continuous
        );

        // Build market context
        const marketCtx = new MarketContext();
        marketCtx.insertDiscount(discountCurve);

        // Add spot prices
        for (const spot of market.spotPrices) {
          marketCtx.insertPrice(
            spot.id,
            MarketScalar.price(Money.fromCode(spot.price.amount, spot.price.currency))
          );
        }

        // Build vol surface
        const equityVol = new VolSurface(
          market.volSurface.id,
          new Float64Array(market.volSurface.expiries),
          new Float64Array(market.volSurface.strikes),
          new Float64Array(market.volSurface.vols)
        );
        marketCtx.insertSurface(equityVol);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process baskets
        for (const basketData of baskets) {
          try {
            const basketJson = JSON.stringify({
              id: basketData.id,
              currency: basketData.currency,
              discount_curve_id: basketData.discount_curve_id,
              expense_ratio: basketData.expense_ratio,
              constituents: basketData.constituents.map((c) => ({
                id: c.id,
                reference: { price_id: c.priceId, asset_type: c.assetType },
                weight: c.weight,
                ticker: c.ticker,
              })),
              pricing_config: {
                days_in_year: 365.25,
                fx_policy: 'cashflow_date',
              },
            });
            const basket = Basket.fromJson(basketJson);
            const result = registry.priceInstrument(basket, 'discounting', marketCtx, asOf, null);

            results.push({
              name: 'Tech Stock Basket',
              type: 'Basket',
              presentValue: result.presentValue.amount,
              complexity: `${basketData.constituents.length} constituents`,
            });
          } catch (err) {
            console.error(`Basket ${basketData.id} error:`, err);
          }
        }

        // Process private markets funds
        for (const fundData of privateMarketsFunds) {
          try {
            const fundJson = JSON.stringify({
              id: fundData.id,
              currency: fundData.currency,
              discount_curve_id: fundData.discount_curve_id,
              spec: fundData.spec,
              events: fundData.events.map((e) => ({
                date: e.date,
                amount: { amount: e.amount.amount, currency: e.amount.currency },
                kind: e.kind,
              })),
            });
            const fund = PrivateMarketsFund.fromJson(fundJson);
            const result = registry.priceInstrument(fund, 'discounting', marketCtx, asOf, null);

            // Find preferred IRR from spec
            const prefIrr = fundData.spec.tranches.find(
              (t): t is { preferred_irr: { irr: number } } =>
                typeof t === 'object' && 'preferred_irr' in t
            );
            const irrStr = prefIrr ? `${(prefIrr.preferred_irr.irr * 100).toFixed(0)}% Pref` : '';

            results.push({
              name: `PE Fund (${irrStr})`,
              type: 'PrivateMarketsFund',
              presentValue: result.presentValue.amount,
              complexity: `${fundData.events.length} events, waterfall`,
            });
          } catch (err) {
            console.error(`Private markets fund ${fundData.id} error:`, err);
          }
        }

        // Process autocallables
        for (const autoData of autocallables) {
          try {
            const autocallableJson = JSON.stringify({
              id: autoData.id,
              underlying_ticker: autoData.underlying_ticker,
              notional: { amount: autoData.notional.amount, currency: autoData.notional.currency },
              observation_dates: autoData.observation_dates,
              autocall_barriers: autoData.autocall_barriers,
              coupons: autoData.coupons,
              final_barrier: autoData.final_barrier,
              final_payoff_type: autoData.final_payoff_type,
              participation_rate: autoData.participation_rate,
              cap_level: autoData.cap_level,
              day_count: autoData.day_count,
              discount_curve_id: autoData.discount_curve_id,
              spot_id: autoData.spot_id,
              vol_surface_id: autoData.vol_surface_id,
              div_yield_id: null,
              pricing_overrides: {
                quoted_clean_price: null,
                implied_volatility: null,
                quoted_spread_bp: null,
                upfront_payment: null,
                ytm_bump_decimal: null,
                theta_period: null,
                mc_seed_scenario: null,
                adaptive_bumps: false,
                spot_bump_pct: null,
                vol_bump_pct: null,
                rate_bump_bp: null,
                rho_bump_decimal: null,
                vega_bump_decimal: null,
              },
              attributes: { tags: [], meta: {} },
            });
            const autocallable = Autocallable.fromJson(autocallableJson);
            const result = registry.priceInstrument(
              autocallable,
              'monte_carlo_gbm',
              marketCtx,
              asOf,
              null
            );

            const couponPct = autoData.coupons[0] * 100;
            const barrierPct = autoData.autocall_barriers[0] * 100;

            results.push({
              name: 'Autocallable Note',
              type: 'Autocallable',
              presentValue: result.presentValue.amount,
              complexity: `${couponPct.toFixed(0)}% coupon, ${barrierPct.toFixed(0)}% barrier`,
            });
          } catch (err) {
            console.error(`Autocallable ${autoData.id} error:`, err);
          }
        }

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
  }, [valuationDate, market, baskets, privateMarketsFunds, autocallables]);

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
