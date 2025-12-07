import React, { useEffect, useState } from 'react';
import {
  FsDate,
  DiscountCurve,
  ForwardCurve,
  CmsOption,
  RangeAccrual,
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
  notional: number;
  presentValue: number;
  details?: string;
};

export const ExoticRatesDerivativesExample: React.FC = () => {
  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(2024, 1, 2);
        const notional = Money.fromCode(10_000_000, 'USD');

        // Build market
        const discountCurve = new DiscountCurve(
          'USD-OIS',
          asOf,
          new Float64Array([0.0, 0.5, 1.0, 2.0, 5.0, 10.0]),
          new Float64Array([1.0, 0.995, 0.99, 0.975, 0.94, 0.87]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const forwardCurve = new ForwardCurve(
          'USD-SOFR-3M',
          asOf,
          0.25,
          new Float64Array([0.0, 1.0, 2.0, 5.0, 10.0]),
          new Float64Array([0.03, 0.032, 0.034, 0.036, 0.038]),
          'act_360',
          2,
          'linear'
        );

        // Add volatility surfaces for options
        const swaptionVol = new VolSurface(
          'SWAPTION-VOL',
          new Float64Array([1.0, 2.0, 5.0, 10.0]),
          new Float64Array([0.02, 0.03, 0.04, 0.05]),
          new Float64Array([
            0.3, 0.29, 0.28, 0.27, 0.28, 0.27, 0.26, 0.25, 0.26, 0.25, 0.24, 0.23, 0.24, 0.23, 0.22,
            0.21,
          ])
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        market.insertForward(forwardCurve);
        market.insertSurface(swaptionVol);
        // Add spot price for range accrual
        market.insertPrice('USD-SOFR-3M-SPOT', MarketScalar.price(Money.fromCode(0.032, 'USD')));

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // CMS Option - Call on 10Y swap rate
        const cmsCallJson = JSON.stringify({
          id: 'cms_call_10y',
          cms_tenor: 10.0,
          strike_rate: 0.035,
          fixing_dates: ['2025-01-02'],
          payment_dates: ['2025-01-02'],
          accrual_fractions: [1.0],
          option_type: 'call',
          notional: { amount: 10_000_000.0, currency: 'USD' },
          day_count: 'Act365F',
          swap_fixed_freq: { Months: 6 },
          swap_float_freq: { Months: 3 },
          swap_day_count: 'Act360',
          discount_curve_id: 'USD-OIS',
          forward_curve_id: 'USD-SOFR-3M',
          vol_surface_id: 'SWAPTION-VOL',
          pricing_overrides: {
            adaptive_bumps: false,
          },
          attributes: { tags: [], meta: {} },
        });
        const cmsCall = CmsOption.fromJson(cmsCallJson);
        // CMS options use monte_carlo_hull_white_1f model
        const cmsCallResult = registry.priceCmsOption(cmsCall, 'monte_carlo_hull_white_1f', market, asOf, null);
        results.push({
          name: 'CMS Call (10Y Swap Rate)',
          type: 'CmsOption',
          notional: notional.amount,
          presentValue: cmsCallResult.presentValue.amount,
          details: 'Strike: 3.5%, 1Y expiry',
        });

        // CMS Option - Put on 5Y swap rate
        const cmsPutJson = JSON.stringify({
          id: 'cms_put_5y',
          cms_tenor: 5.0,
          strike_rate: 0.03,
          fixing_dates: ['2025-01-02'],
          payment_dates: ['2025-01-02'],
          accrual_fractions: [1.0],
          option_type: 'put',
          notional: { amount: 10_000_000.0, currency: 'USD' },
          day_count: 'Act365F',
          swap_fixed_freq: { Months: 6 },
          swap_float_freq: { Months: 3 },
          swap_day_count: 'Act360',
          discount_curve_id: 'USD-OIS',
          forward_curve_id: 'USD-SOFR-3M',
          vol_surface_id: 'SWAPTION-VOL',
          pricing_overrides: {
            adaptive_bumps: false,
          },
          attributes: { tags: [], meta: {} },
        });
        const cmsPut = CmsOption.fromJson(cmsPutJson);
        const cmsPutResult = registry.priceCmsOption(cmsPut, 'monte_carlo_hull_white_1f', market, asOf, null);
        results.push({
          name: 'CMS Put (5Y Swap Rate)',
          type: 'CmsOption',
          notional: notional.amount,
          presentValue: cmsPutResult.presentValue.amount,
          details: 'Strike: 3.0%, 1Y expiry',
        });

        // Range Accrual Note
        // Generate monthly observation dates from start to end
        const observationDates: string[] = [];
        const startDate = new Date(2024, 0, 2); // 2024-01-02
        const endDate = new Date(2025, 0, 2); // 2025-01-02
        const currentDate = new Date(startDate);
        while (currentDate <= endDate) {
          observationDates.push(
            `${currentDate.getFullYear()}-${String(currentDate.getMonth() + 1).padStart(2, '0')}-${String(currentDate.getDate()).padStart(2, '0')}`
          );
          currentDate.setMonth(currentDate.getMonth() + 1);
        }

        const rangeAccrualJson = JSON.stringify({
          id: 'range_accrual_1',
          underlying_ticker: 'USD-SOFR-3M',
          observation_dates: observationDates,
          lower_bound: 0.02,
          upper_bound: 0.05,
          coupon_rate: 0.06,
          notional: { amount: 10_000_000.0, currency: 'USD' },
          day_count: 'Act365F',
          discount_curve_id: 'USD-OIS',
          spot_id: 'USD-SOFR-3M-SPOT',
          vol_surface_id: 'SWAPTION-VOL',
          div_yield_id: null,
          pricing_overrides: {
            adaptive_bumps: false,
          },
          attributes: { tags: [], meta: {} },
        });
        const rangeAccrual = RangeAccrual.fromJson(rangeAccrualJson);
        // Range accruals use monte_carlo_gbm model
        const rangeAccrualResult = registry.priceRangeAccrual(
          rangeAccrual,
          'monte_carlo_gbm',
          market,
          asOf,
          null
        );
        results.push({
          name: 'Range Accrual Note',
          type: 'RangeAccrual',
          notional: notional.amount,
          presentValue: rangeAccrualResult.presentValue.amount,
          details: '6% coupon, range: 2%-5%',
        });

        if (!cancelled) {
          setRows(results);
        }

        // Cleanup
        discountCurve.free();
        forwardCurve.free();
        swaptionVol.free();
        market.free();
      } catch (err) {
        if (!cancelled) {
          console.error('Exotic rates derivatives error:', err);
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
    return <p>Building exotic rates derivatives…</p>;
  }

  return (
    <section className="example-section">
      <h2>Exotic Rates Derivatives</h2>
      <p>
        Exotic interest rate derivatives including CMS options (options on swap rates) and range
        accrual notes (accrual based on reference rate staying within a range). These instruments
        are priced using Monte Carlo simulation with Hull-White and GBM processes.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Notional</th>
            <th>Present Value</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, notional, presentValue, details }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(notional)}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>{details ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
