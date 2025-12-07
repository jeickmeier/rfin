import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DiscountCurve,
  BarrierOption,
  AsianOption,
  LookbackOption,
  CliquetOption,
  MarketContext,
  MarketScalar,
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
  presentValue: number;
  keyMetric?: { name: string; value: number };
  details?: string;
};

export const ExoticEquityOptionsExample: React.FC = () => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(2024, 1, 2);

        const discountCurve = new DiscountCurve(
          'USD-OIS',
          asOf,
          new Float64Array([0.0, 0.5, 1.0, 3.0, 5.0]),
          new Float64Array([1.0, 0.997, 0.994, 0.9725, 0.948]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        // Add equity market data (flattened grid: row-major order)
        const equityVol = new VolSurface(
          'EQUITY-VOL',
          new Float64Array([0.25, 0.5, 1.0, 2.0]),
          new Float64Array([120.0, 140.0, 160.0, 180.0]),
          new Float64Array([
            0.28, 0.26, 0.25, 0.24, 0.27, 0.25, 0.24, 0.23, 0.26, 0.24, 0.23, 0.22, 0.25, 0.23,
            0.22, 0.21,
          ])
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

        // Barrier Options - Up-and-Out Call
        try {
          const barrierOption1 = new BarrierOption(
            'barrier_up_out_call',
            'AAPL',
            150.0, // strike
            180.0, // barrier
            'call',
            'up_and_out',
            new FsDate(2024, 12, 31),
            Money.fromCode(150.0, 'USD'),
            'USD-OIS',
            'AAPL-SPOT',
            'EQUITY-VOL',
            'AAPL-DIVYIELD', // dividend_yield_id
            false // use_gobet_miri
          );
          const barrierOpts1 = new PricingRequest().withMetrics(['delta', 'gamma']);
          const barrierResult1 = registry.priceBarrierOption(
            barrierOption1,
            'monte_carlo_gbm',
            market,
            asOf,
            barrierOpts1
          );
          results.push({
            name: 'Barrier Up-and-Out Call',
            type: 'BarrierOption',
            presentValue: barrierResult1.presentValue.amount,
            keyMetric: { name: 'Delta', value: barrierResult1.metric('delta') ?? 0 },
            details: 'Strike: $150, Barrier: $180',
          });
        } catch (barrierErr) {
          console.error('Barrier option 1 error:', barrierErr);
          // Skip this option if construction fails
        }

        // Barrier Options - Down-and-In Put
        try {
          const barrierOption2 = new BarrierOption(
            'barrier_down_in_put',
            'AAPL',
            140.0, // strike
            130.0, // barrier
            'put',
            'down_and_in',
            new FsDate(2024, 12, 31),
            Money.fromCode(140.0, 'USD'),
            'USD-OIS',
            'AAPL-SPOT',
            'EQUITY-VOL',
            'AAPL-DIVYIELD',
            false
          );
          const barrierResult2 = registry.priceBarrierOption(
            barrierOption2,
            'monte_carlo_gbm',
            market,
            asOf,
            null
          );
          results.push({
            name: 'Barrier Down-and-In Put',
            type: 'BarrierOption',
            presentValue: barrierResult2.presentValue.amount,
            details: 'Strike: $140, Barrier: $130',
          });
        } catch (barrierErr) {
          console.error('Barrier option 2 error:', barrierErr);
          // Skip this option if construction fails
        }

        // Asian Option - Arithmetic Average Call
        const fixingDates: string[] = [];
        for (let i = 1; i <= 12; i++) {
          const date = new FsDate(2024, i, 15);
          fixingDates.push(
            `${date.year}-${String(date.month).padStart(2, '0')}-${String(date.day).padStart(2, '0')}`
          );
        }
        try {
          const asianOption1 = new AsianOption(
            'asian_arithmetic_call',
            'AAPL',
            150.0, // strike
            new FsDate(2024, 12, 31),
            fixingDates,
            Money.fromCode(150.0, 'USD'),
            'USD-OIS',
            'AAPL-SPOT',
            'EQUITY-VOL',
            'arithmetic', // averaging_method
            'call', // option_type
            'AAPL-DIVYIELD'
          );
          const asianResult1 = registry.priceAsianOption(
            asianOption1,
            'monte_carlo_gbm',
            market,
            asOf,
            null
          );
          results.push({
            name: 'Asian Arithmetic Call',
            type: 'AsianOption',
            presentValue: asianResult1.presentValue.amount,
            details: 'Strike: $150, 12 monthly fixings',
          });
        } catch (asianErr) {
          console.error('Asian option 1 error:', asianErr);
          // Skip this option if construction fails
        }

        // Asian Option - Geometric Average Put
        try {
          const asianOption2 = new AsianOption(
            'asian_geometric_put',
            'AAPL',
            145.0,
            new FsDate(2024, 12, 31),
            fixingDates,
            Money.fromCode(145.0, 'USD'),
            'USD-OIS',
            'AAPL-SPOT',
            'EQUITY-VOL',
            'geometric',
            'put',
            'AAPL-DIVYIELD'
          );
          const asianResult2 = registry.priceAsianOption(
            asianOption2,
            'monte_carlo_gbm',
            market,
            asOf,
            null
          );
          results.push({
            name: 'Asian Geometric Put',
            type: 'AsianOption',
            presentValue: asianResult2.presentValue.amount,
            details: 'Strike: $145, 12 monthly fixings',
          });
        } catch (asianErr) {
          console.error('Asian option 2 error:', asianErr);
          // Skip this option if construction fails
        }

        // Lookback Option - Fixed Strike (uses JSON construction)
        try {
          const lookbackJson = JSON.stringify({
            id: 'lookback_fixed_strike',
            underlying_ticker: 'AAPL',
            strike: { amount: 150.0, currency: 'USD' },
            expiry: '2024-12-31',
            lookback_type: 'FixedStrike',
            option_type: 'call',
            notional: { amount: 1.0, currency: 'USD' },
            day_count: 'act_365f',
            discount_curve_id: 'USD-OIS',
            spot_id: 'AAPL-SPOT',
            vol_id: 'EQUITY-VOL',
            div_yield_id: 'AAPL-DIVYIELD',
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
          const lookbackOption = LookbackOption.fromJson(lookbackJson);
          const lookbackResult = registry.priceLookbackOption(
            lookbackOption,
            'monte_carlo_gbm',
            market,
            asOf,
            null
          );
          results.push({
            name: 'Lookback Fixed Strike Call',
            type: 'LookbackOption',
            presentValue: lookbackResult.presentValue.amount,
            details: 'Strike: $150',
          });
        } catch (lookbackErr) {
          console.error('Lookback option error:', lookbackErr);
          // Skip this option if construction fails
        }

        // Cliquet Option (uses JSON construction)
        try {
          const cliquetJson = JSON.stringify({
            id: 'cliquet_local_floor',
            underlying_ticker: 'AAPL',
            reset_dates: ['2024-04-01', '2024-07-01', '2024-10-01', '2024-12-31'],
            local_cap: 0.15,
            global_cap: 0.3,
            notional: { amount: 1_000_000.0, currency: 'USD' },
            day_count: 'act_365f',
            discount_curve_id: 'USD-OIS',
            spot_id: 'AAPL-SPOT',
            vol_id: 'EQUITY-VOL',
            div_yield_id: 'AAPL-DIVYIELD',
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
          const cliquetOption = CliquetOption.fromJson(cliquetJson);
          const cliquetResult = registry.priceCliquetOption(
            cliquetOption,
            'monte_carlo_gbm',
            market,
            asOf,
            null
          );
          results.push({
            name: 'Cliquet Option',
            type: 'CliquetOption',
            presentValue: cliquetResult.presentValue.amount,
            details: 'Local cap: 15%, Global cap: 30%',
          });
        } catch (cliquetErr) {
          console.error('Cliquet option error:', cliquetErr);
          // Skip this option if construction fails
        }

        if (!cancelled) {
          setRows(results);
        }

        // Cleanup
        discountCurve.free();
        equityVol.free();
        market.free();
      } catch (err) {
        if (!cancelled) {
          console.error('Exotic equity options error:', err);
          const errorMessage = err instanceof Error ? err.message : String(err);
          setError(errorMessage);
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
    return <p>Building exotic equity options…</p>;
  }

  return (
    <section className="example-section">
      <h2>Exotic Equity Options</h2>
      <p>
        Exotic equity options including barrier options (up-and-out, down-and-in), Asian options
        (arithmetic and geometric averaging), lookback options, and cliquet options. These
        instruments are priced using Monte Carlo simulation and other advanced models.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Present Value</th>
            <th>Key Metric</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, presentValue, keyMetric, details }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(4)}` : '—'}</td>
              <td>{details ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
