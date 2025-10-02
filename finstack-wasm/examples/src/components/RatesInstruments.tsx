import React, { useEffect, useState } from 'react';
import {
  Date as FsDate,
  DayCount,
  DiscountCurve,
  ForwardCurve,
  ForwardRateAgreement,
  InterestRateFuture,
  InterestRateOption,
  InterestRateSwap,
  MarketContext,
  Money,
  PricingRequest,
  Swaption,
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
  keyMetric?: { name: string; value: number };
};

export const RatesInstrumentsExample: React.FC = () => {
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
          new Float64Array([0.0, 0.5, 1.0, 2.0, 5.0]),
          new Float64Array([1.0, 0.9950, 0.9900, 0.9750, 0.9400]),
          'act_365f',
          'monotone_convex',
          'flat_forward',
          true
        );

        const forwardCurve = new ForwardCurve(
          'USD-SOFR-3M',
          asOf,
          0.25,
          new Float64Array([0.0, 1.0, 2.0, 5.0]),
          new Float64Array([0.0300, 0.0320, 0.0340, 0.0360]),
          'act_360',
          2,
          'linear'
        );

        // Add volatility surfaces for options (flattened grid: row-major order)
        const swaptionVol = new VolSurface(
          'SWAPTION-VOL',
          [1.0, 2.0, 5.0],
          [0.02, 0.03, 0.04],
          [0.30, 0.29, 0.28, 0.28, 0.27, 0.26, 0.26, 0.25, 0.24]
        );

        const capVol = new VolSurface(
          'IR-CAP-VOL',
          [0.5, 1.0, 2.0, 5.0],
          [0.01, 0.02, 0.03, 0.04],
          [0.38, 0.36, 0.34, 0.32, 0.35, 0.33, 0.31, 0.30, 0.32, 0.31, 0.29, 0.28, 0.28, 0.27, 0.26, 0.25]
        );

        const market = new MarketContext();
        market.insertDiscount(discountCurve);
        market.insertForward(forwardCurve);
        market.insertSurface(swaptionVol);
        market.insertSurface(capVol);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Interest Rate Swap
        const swap = InterestRateSwap.usdReceiveFixed(
          'irs_receive_fixed',
          notional,
          0.0325,
          asOf,
          new FsDate(2029, 1, 2)
        );
        const swapOpts = new PricingRequest().withMetrics(['dv01', 'annuity', 'par_rate']);
        const swapResult = registry.priceInterestRateSwap(swap, 'discounting', market, swapOpts);
        results.push({
          name: '5Y IRS (Receive Fixed)',
          type: 'InterestRateSwap',
          notional: notional.amount,
          presentValue: swapResult.presentValue.amount,
          keyMetric: { name: 'DV01', value: swapResult.metric('dv01') ?? 0 },
        });

        // Forward Rate Agreement
        const fra = new ForwardRateAgreement(
          'fra_3x6',
          notional,
          0.0360,
          new FsDate(2024, 4, 2),
          new FsDate(2024, 4, 4),
          new FsDate(2024, 7, 4),
          'USD-OIS',
          'USD-SOFR-3M',
          DayCount.act360(),
          2,
          true
        );
        const fraOpts = new PricingRequest().withMetrics(['par_rate']);
        const fraResult = registry.priceForwardRateAgreement(fra, 'discounting', market, fraOpts);
        results.push({
          name: '3x6 FRA',
          type: 'ForwardRateAgreement',
          notional: notional.amount,
          presentValue: fraResult.presentValue.amount,
          keyMetric: {
            name: 'Par Rate (bps)',
            // Par rate comes in decimal form (e.g., 0.0307), multiply by 10000 for bps
            value: Math.abs((fraResult.metric('par_rate') ?? 0) * 10000),
          },
        });

        // Swaption
        const swaption = Swaption.payer(
          'swaption_1y5y',
          notional,
          0.0325,
          new FsDate(2025, 1, 2),
          new FsDate(2025, 1, 2),
          new FsDate(2030, 1, 2),
          'USD-OIS',
          'USD-SOFR-3M',
          null,
          null,
          null
        );
        const swaptionResult = registry.priceSwaption(swaption, 'discounting', market);
        results.push({
          name: '1Yx5Y Payer Swaption',
          type: 'Swaption',
          notional: notional.amount,
          presentValue: swaptionResult.presentValue.amount,
        });

        // Basis Swap - Note: Need separate forward curves for each tenor
        // Skipping basis swap example due to missing 6M forward curve
        // In production, you would have both USD-SOFR-3M and USD-SOFR-6M curves

        // Interest Rate Cap
        const cap = InterestRateOption.cap(
          'cap_5y',
          notional,
          0.04,
          asOf,
          new FsDate(2029, 1, 2),
          'USD-OIS',
          'USD-SOFR-3M',
          null,
          4,
          DayCount.act360()
        );
        const capResult = registry.priceInterestRateOption(cap, 'discounting', market);
        results.push({
          name: '5Y Cap @ 4%',
          type: 'InterestRateOption',
          notional: notional.amount,
          presentValue: capResult.presentValue.amount,
        });

        // Interest Rate Future
        const future = new InterestRateFuture(
          'sofr_fut_mar24',
          Money.fromCode(1_000_000, 'USD'),
          97.25,
          new FsDate(2024, 3, 16),
          new FsDate(2024, 3, 18),
          new FsDate(2024, 3, 18),
          new FsDate(2024, 6, 18),
          'USD-OIS',
          'USD-SOFR-3M',
          'long',
          DayCount.act360()
        );
        const futureResult = registry.priceInterestRateFuture(future, 'discounting', market);
        results.push({
          name: 'SOFR Future (Mar 24)',
          type: 'InterestRateFuture',
          notional: 1_000_000,
          presentValue: futureResult.presentValue.amount,
        });

        if (!cancelled) {
          setRows(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Rates instruments error:', err);
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
    return <p>Building rates instruments…</p>;
  }

  return (
    <section className="example-section">
      <h2>Interest Rate Instruments</h2>
      <p>
        Comprehensive suite of interest rate derivatives including swaps, FRAs, swaptions,
        basis swaps, caps/floors, and futures. All instruments are priced using the standard
        registry with market curves.
      </p>

      <table>
        <thead>
          <tr>
            <th>Instrument</th>
            <th>Type</th>
            <th>Notional</th>
            <th>Present Value</th>
            <th>Key Metric</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ name, type, notional, presentValue, keyMetric }) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{type}</td>
              <td>{currencyFormatter.format(notional)}</td>
              <td>{currencyFormatter.format(presentValue)}</td>
              <td>
                {keyMetric
                  ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)}`
                  : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};

