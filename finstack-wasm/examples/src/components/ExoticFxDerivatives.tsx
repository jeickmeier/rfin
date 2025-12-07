import React, { useEffect, useState } from 'react';
import {
  Currency,
  FsDate,
  DiscountCurve,
  FxBarrierOption,
  QuantoOption,
  FxMatrix,
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
  pair: string;
  presentValue: number;
  details?: string;
};

export const ExoticFxDerivativesExample: React.FC = () => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(2024, 1, 2);
        const usd = new Currency('USD');
        const eur = new Currency('EUR');
        const gbp = new Currency('GBP');

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

        const gbpDisc = new DiscountCurve(
          'GBP-OIS',
          asOf,
          new Float64Array([0.0, 0.5, 1.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.9978, 0.9955, 0.978, 0.952]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const fx = new FxMatrix();
        fx.setQuote(eur, usd, 1.085);
        fx.setQuote(gbp, usd, 1.265);

        // Add FX volatility surface for options (flattened grid: row-major order)
        const fxVol = new VolSurface(
          'FX-VOL',
          new Float64Array([0.25, 0.5, 1.0, 2.0]),
          new Float64Array([1.05, 1.1, 1.15]),
          new Float64Array([0.14, 0.13, 0.12, 0.13, 0.12, 0.11, 0.12, 0.11, 0.1, 0.11, 0.1, 0.095])
        );

        // Equity vol for quanto options
        const equityVol = new VolSurface(
          'EQUITY-VOL',
          new Float64Array([0.25, 0.5, 1.0, 2.0]),
          new Float64Array([100.0, 120.0, 140.0, 160.0]),
          new Float64Array([
            0.28, 0.26, 0.25, 0.24, 0.27, 0.25, 0.24, 0.23, 0.26, 0.24, 0.23, 0.22, 0.25, 0.23,
            0.22, 0.21,
          ])
        );

        const market = new MarketContext();
        market.insertDiscount(usdDisc);
        market.insertDiscount(eurDisc);
        market.insertDiscount(gbpDisc);
        market.insertFx(fx);
        market.insertSurface(fxVol);
        market.insertSurface(equityVol);
        // Add FX spot prices for barrier options
        market.insertPrice('EURUSD-SPOT', MarketScalar.price(Money.fromCode(1.085, 'USD')));
        market.insertPrice('GBPUSD-SPOT', MarketScalar.price(Money.fromCode(1.265, 'USD')));
        // Add equity spot prices for quanto options
        market.insertPrice('EUR-EQUITY-SPOT', MarketScalar.price(Money.fromCode(150.0, 'EUR')));
        market.insertPrice('GBP-EQUITY-SPOT', MarketScalar.price(Money.fromCode(140.0, 'GBP')));

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // FX Barrier Option - Up-and-Out Call on EUR/USD
        const fxBarrierJson = JSON.stringify({
          id: 'fx_barrier_up_out',
          domestic_currency: 'USD',
          foreign_currency: 'EUR',
          strike: { amount: 1.1, currency: 'USD' },
          barrier: { amount: 1.15, currency: 'USD' },
          option_type: 'call',
          barrier_type: 'UpAndOut',
          expiry: '2024-12-31',
          notional: { amount: 1.0, currency: 'USD' },
          day_count: 'Act365F',
          use_gobet_miri: false,
          domestic_discount_curve_id: 'USD-OIS',
          foreign_discount_curve_id: 'EUR-OIS',
          fx_spot_id: 'EURUSD-SPOT',
          fx_vol_id: 'FX-VOL',
          pricing_overrides: {
            adaptive_bumps: false,
          },
          attributes: { tags: [], meta: {} },
        });
        const fxBarrierOption = FxBarrierOption.fromJson(fxBarrierJson);
        const fxBarrierResult = registry.priceFxBarrierOption(
          fxBarrierOption,
          'monte_carlo_gbm',
          market,
          asOf,
          null
        );
        results.push({
          name: 'FX Barrier Up-and-Out Call',
          type: 'FxBarrierOption',
          pair: 'EUR/USD',
          presentValue: fxBarrierResult.presentValue.amount,
          details: 'Strike: 1.10, Barrier: 1.15',
        });

        // FX Barrier Option - Down-and-In Put on GBP/USD
        const fxBarrierPutJson = JSON.stringify({
          id: 'fx_barrier_down_in',
          domestic_currency: 'USD',
          foreign_currency: 'GBP',
          strike: { amount: 1.25, currency: 'USD' },
          barrier: { amount: 1.2, currency: 'USD' },
          option_type: 'put',
          barrier_type: 'DownAndIn',
          expiry: '2024-12-31',
          notional: { amount: 1.0, currency: 'USD' },
          day_count: 'Act365F',
          use_gobet_miri: false,
          domestic_discount_curve_id: 'USD-OIS',
          foreign_discount_curve_id: 'GBP-OIS',
          fx_spot_id: 'GBPUSD-SPOT',
          fx_vol_id: 'FX-VOL',
          pricing_overrides: {
            adaptive_bumps: false,
          },
          attributes: { tags: [], meta: {} },
        });
        const fxBarrierPut = FxBarrierOption.fromJson(fxBarrierPutJson);
        const fxBarrierPutResult = registry.priceFxBarrierOption(
          fxBarrierPut,
          'monte_carlo_gbm',
          market,
          asOf,
          null
        );
        results.push({
          name: 'FX Barrier Down-and-In Put',
          type: 'FxBarrierOption',
          pair: 'GBP/USD',
          presentValue: fxBarrierPutResult.presentValue.amount,
          details: 'Strike: 1.25, Barrier: 1.20',
        });

        // Quanto Option - Call on EUR equity, paid in USD
        const quantoJson = JSON.stringify({
          id: 'quanto_call_eur_usd',
          underlying_ticker: 'EUR-EQUITY',
          equity_strike: { amount: 150.0, currency: 'EUR' },
          expiry: '2024-12-31',
          option_type: 'call',
          notional: { amount: 1.0, currency: 'USD' },
          domestic_currency: 'USD',
          foreign_currency: 'EUR',
          correlation: 0.3,
          day_count: 'Act365F',
          discount_curve_id: 'USD-OIS',
          foreign_discount_curve_id: 'EUR-OIS',
          spot_id: 'EUR-EQUITY-SPOT',
          vol_surface_id: 'EQUITY-VOL',
          div_yield_id: null,
          fx_rate_id: null,
          fx_vol_id: 'FX-VOL',
          pricing_overrides: {
            adaptive_bumps: false,
          },
          attributes: { tags: [], meta: {} },
        });
        const quantoOption = QuantoOption.fromJson(quantoJson);
        // Quanto options are priced using priceQuantoOption
        const quantoResult = registry.priceQuantoOption(quantoOption, 'monte_carlo_gbm', market, asOf, null);
        results.push({
          name: 'Quanto Call (EUR equity, USD payment)',
          type: 'QuantoOption',
          pair: 'EUR/USD',
          presentValue: quantoResult.presentValue.amount,
          details: 'Strike: 150.0, Cross-currency',
        });

        // Quanto Option - Put on GBP equity, paid in USD
        const quantoPutJson = JSON.stringify({
          id: 'quanto_put_gbp_usd',
          underlying_ticker: 'GBP-EQUITY',
          equity_strike: { amount: 140.0, currency: 'GBP' },
          expiry: '2024-12-31',
          option_type: 'put',
          notional: { amount: 1.0, currency: 'USD' },
          domestic_currency: 'USD',
          foreign_currency: 'GBP',
          correlation: 0.3,
          day_count: 'Act365F',
          discount_curve_id: 'USD-OIS',
          foreign_discount_curve_id: 'GBP-OIS',
          spot_id: 'GBP-EQUITY-SPOT',
          vol_surface_id: 'EQUITY-VOL',
          div_yield_id: null,
          fx_rate_id: null,
          fx_vol_id: 'FX-VOL',
          pricing_overrides: {
            adaptive_bumps: false,
          },
          attributes: { tags: [], meta: {} },
        });
        const quantoPut = QuantoOption.fromJson(quantoPutJson);
        const quantoPutResult = registry.priceQuantoOption(quantoPut, 'monte_carlo_gbm', market, asOf, null);
        results.push({
          name: 'Quanto Put (GBP equity, USD payment)',
          type: 'QuantoOption',
          pair: 'GBP/USD',
          presentValue: quantoPutResult.presentValue.amount,
          details: 'Strike: 140.0, Cross-currency',
        });

        if (!cancelled) {
          setRows(results);
        }

        // Cleanup
        usdDisc.free();
        eurDisc.free();
        gbpDisc.free();
        fx.free();
        fxVol.free();
        equityVol.free();
        market.free();
      } catch (err) {
        if (!cancelled) {
          console.error('Exotic FX derivatives error:', err);
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
    return <p>Building exotic FX derivatives…</p>;
  }

  return (
    <section className="example-section">
      <h2>Exotic FX Derivatives</h2>
      <p>
        Exotic FX derivatives including FX barrier options (up-and-out, down-and-in) and quanto
        options (cross-currency equity options). These instruments are priced using Monte Carlo
        simulation with GBM processes.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Pair</th>
            <th>Present Value</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, pair, presentValue, details }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{pair}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{details ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
