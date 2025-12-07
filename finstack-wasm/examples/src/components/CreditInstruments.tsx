import React, { useEffect, useState } from 'react';
import {
  BaseCorrelationCurve,
  CDSIndex,
  CdsOption,
  CdsTranche,
  CreditDefaultSwap,
  CreditIndexData,
  FsDate,
  DiscountCurve,
  HazardCurve,
  MarketContext,
  Money,
  PricingRequest,
  RevolvingCredit,
  VolSurface,
  createStandardRegistry,
} from 'finstack-wasm';
import { CreditInstrumentsProps, DEFAULT_CREDIT_PROPS } from './data/credit';

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
};

export const CreditInstrumentsExample: React.FC<CreditInstrumentsProps> = (props) => {
  // Merge with defaults
  const {
    valuationDate = DEFAULT_CREDIT_PROPS.valuationDate!,
    discountCurve = DEFAULT_CREDIT_PROPS.discountCurve!,
    hazardCurves = DEFAULT_CREDIT_PROPS.hazardCurves!,
    baseCorrelation = DEFAULT_CREDIT_PROPS.baseCorrelation!,
    cdsVolSurface = DEFAULT_CREDIT_PROPS.cdsVolSurface!,
    creditIndexData = DEFAULT_CREDIT_PROPS.creditIndexData!,
    cdsSwaps = DEFAULT_CREDIT_PROPS.cdsSwaps!,
    cdsIndices = DEFAULT_CREDIT_PROPS.cdsIndices!,
    cdsTranches = DEFAULT_CREDIT_PROPS.cdsTranches!,
    cdsOptions = DEFAULT_CREDIT_PROPS.cdsOptions!,
    revolvingCredits = DEFAULT_CREDIT_PROPS.revolvingCredits!,
  } = props;

  const [rows, setRows] = useState<InstrumentRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const asOf = new FsDate(valuationDate.year, valuationDate.month, valuationDate.day);

        // Build discount curve from props
        const curveBaseDate = new FsDate(
          discountCurve.baseDate.year,
          discountCurve.baseDate.month,
          discountCurve.baseDate.day
        );
        const discCurve = new DiscountCurve(
          discountCurve.id,
          curveBaseDate,
          new Float64Array(discountCurve.tenors),
          new Float64Array(discountCurve.discountFactors),
          discountCurve.dayCount,
          discountCurve.interpolation,
          discountCurve.extrapolation,
          discountCurve.continuous
        );

        const market = new MarketContext();
        market.insertDiscount(discCurve);

        // Build hazard curves from props
        const hazardCurveMap = new Map<string, HazardCurve>();
        for (const hzData of hazardCurves) {
          const hzBaseDate = new FsDate(hzData.baseDate.year, hzData.baseDate.month, hzData.baseDate.day);
          const hzCurve = new HazardCurve(
            hzData.id,
            hzBaseDate,
            new Float64Array(hzData.tenors),
            new Float64Array(hzData.hazardRates),
            hzData.recoveryRate,
            hzData.dayCount,
            null,
            null,
            null,
            null,
            null
          );
          market.insertHazard(hzCurve);
          hazardCurveMap.set(hzData.id, hzCurve);
        }

        // Build base correlation curve
        const baseCorr = new BaseCorrelationCurve(
          baseCorrelation.id,
          new Float64Array(baseCorrelation.attachmentPoints),
          new Float64Array(baseCorrelation.correlations)
        );

        // Build credit index data
        const indexHazard = hazardCurveMap.get(creditIndexData.hazardCurveId);
        if (indexHazard) {
          const indexData = new CreditIndexData(
            creditIndexData.constituents,
            creditIndexData.recoveryRate,
            indexHazard,
            baseCorr,
            null,
            null
          );
          market.insertCreditIndex(creditIndexData.indexFamily, indexData);
        }

        // Build CDS volatility surface
        const cdsVol = new VolSurface(
          cdsVolSurface.id,
          new Float64Array(cdsVolSurface.expiries),
          new Float64Array(cdsVolSurface.strikes),
          new Float64Array(cdsVolSurface.vols)
        );
        market.insertSurface(cdsVol);

        const registry = createStandardRegistry();
        const results: InstrumentRow[] = [];

        // Process CDS Swaps
        for (const cdsData of cdsSwaps) {
          const notional = Money.fromCode(cdsData.notional.amount, cdsData.notional.currency);
          const effectiveDate = new FsDate(cdsData.effectiveDate.year, cdsData.effectiveDate.month, cdsData.effectiveDate.day);
          const maturityDate = new FsDate(cdsData.maturityDate.year, cdsData.maturityDate.month, cdsData.maturityDate.day);

          const cds = cdsData.direction === 'buy_protection'
            ? CreditDefaultSwap.buyProtection(
                cdsData.id,
                notional,
                cdsData.spreadBps,
                effectiveDate,
                maturityDate,
                cdsData.discountCurveId,
                cdsData.hazardCurveId,
                null
              )
            : CreditDefaultSwap.sellProtection(
                cdsData.id,
                notional,
                cdsData.spreadBps,
                effectiveDate,
                maturityDate,
                cdsData.discountCurveId,
                cdsData.hazardCurveId,
                null
              );

          const cdsOpts = new PricingRequest().withMetrics(['par_spread', 'pv01']);
          try {
            const cdsResult = registry.priceCreditDefaultSwap(
              cds,
              'discounting',
              market,
              asOf,
              cdsOpts
            );
            const tenorYears = cdsData.maturityDate.year - cdsData.effectiveDate.year;
            results.push({
              name: `${tenorYears}Y CDS`,
              type: 'CreditDefaultSwap',
              presentValue: cdsResult.presentValue.amount,
              keyMetric: {
                name: 'Par Spread',
                value: (() => {
                  const raw = cdsResult.metric('par_spread') ?? 0;
                  return Math.abs(raw) > 10 ? raw : raw * 10000;
                })(),
              },
            });
          } catch (err) {
            console.warn('CDS pricing failed, skipping', err);
          }
        }

        // Process CDS Indices
        for (const indexInstrData of cdsIndices) {
          const notional = Money.fromCode(indexInstrData.notional.amount, indexInstrData.notional.currency);
          const effectiveDate = new FsDate(indexInstrData.effectiveDate.year, indexInstrData.effectiveDate.month, indexInstrData.effectiveDate.day);
          const maturityDate = new FsDate(indexInstrData.maturityDate.year, indexInstrData.maturityDate.month, indexInstrData.maturityDate.day);

          const index = new CDSIndex(
            indexInstrData.id,
            indexInstrData.indexFamily,
            indexInstrData.series,
            indexInstrData.version,
            notional,
            indexInstrData.spreadBps,
            effectiveDate,
            maturityDate,
            indexInstrData.discountCurveId,
            indexInstrData.hazardCurveId,
            indexInstrData.direction,
            indexInstrData.recoveryRate,
            null
          );
          const indexOpts = new PricingRequest().withMetrics(['par_spread']);
          try {
            const indexResult = registry.priceCDSIndex(
              index,
              'discounting',
              market,
              asOf,
              indexOpts
            );
            results.push({
              name: `${indexInstrData.indexFamily} S${indexInstrData.series} V${indexInstrData.version}`,
              type: 'CDSIndex',
              presentValue: indexResult.presentValue.amount,
              keyMetric: {
                name: 'Par Spread',
                value: (() => {
                  const raw = indexResult.metric('par_spread') ?? 0;
                  return Math.abs(raw) > 10 ? raw : raw * 10000;
                })(),
              },
            });
          } catch (err) {
            console.warn('CDS index pricing failed, skipping', err);
          }
        }

        // Process CDS Tranches
        for (const trancheData of cdsTranches) {
          const notional = Money.fromCode(trancheData.notional.amount, trancheData.notional.currency);
          const maturityDate = new FsDate(trancheData.maturityDate.year, trancheData.maturityDate.month, trancheData.maturityDate.day);

          try {
            const tranche = new CdsTranche(
              trancheData.id,
              trancheData.indexFamily,
              trancheData.series,
              trancheData.attachmentPoint,
              trancheData.detachmentPoint,
              notional,
              maturityDate,
              trancheData.spreadBps,
              trancheData.discountCurveId,
              trancheData.hazardCurveId,
              trancheData.direction,
              trancheData.frequency,
              null
            );
            const trancheResult = registry.priceCdsTranche(
              tranche,
              'discounting',
              market,
              asOf,
              null
            );
            results.push({
              name: `CDX Mezzanine (${trancheData.attachmentPoint}-${trancheData.detachmentPoint}%)`,
              type: 'CdsTranche',
              presentValue: trancheResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('CDS tranche pricing failed, skipping', err);
          }
        }

        // Process CDS Options
        for (const optionData of cdsOptions) {
          const notional = Money.fromCode(optionData.notional.amount, optionData.notional.currency);
          const expiryDate = new FsDate(optionData.expiryDate.year, optionData.expiryDate.month, optionData.expiryDate.day);
          const underlyingMaturity = new FsDate(optionData.underlyingMaturity.year, optionData.underlyingMaturity.month, optionData.underlyingMaturity.day);

          try {
            const option = new CdsOption(
              optionData.id,
              notional,
              optionData.strikeBps,
              expiryDate,
              underlyingMaturity,
              optionData.discountCurveId,
              optionData.hazardCurveId,
              optionData.volSurfaceId,
              optionData.optionType,
              optionData.recoveryRate,
              optionData.knockedOut,
              null
            );
            const optionResult = registry.priceCdsOption(option, 'discounting', market, asOf, null);
            results.push({
              name: `CDS Option @ ${optionData.strikeBps}bp`,
              type: 'CdsOption',
              presentValue: optionResult.presentValue.amount,
            });
          } catch (err) {
            console.warn('CDS option pricing failed, skipping', err);
          }
        }

        // Process Revolving Credits
        for (const rcData of revolvingCredits) {
          try {
            const revolvingCreditJson = JSON.stringify({
              id: rcData.id,
              commitment_amount: { amount: rcData.commitmentAmount.amount, currency: rcData.commitmentAmount.currency },
              drawn_amount: { amount: rcData.drawnAmount.amount, currency: rcData.drawnAmount.currency },
              commitment_date: rcData.commitmentDate,
              maturity_date: rcData.maturityDate,
              base_rate_spec: rcData.baseRateSpec,
              day_count: rcData.dayCount,
              payment_frequency: rcData.paymentFrequency,
              fees: {
                upfront_fee: rcData.fees.upfrontFee,
                commitment_fee_bp: rcData.fees.commitmentFeeBp,
                usage_fee_bp: rcData.fees.usageFeeBp,
                facility_fee_bp: rcData.fees.facilityFeeBp,
              },
              draw_repay_spec: rcData.drawRepaySpec,
              disc_id: rcData.discountCurveId,
              attributes: { tags: [], meta: {} },
            });
            const revolvingCredit = RevolvingCredit.fromJson(revolvingCreditJson);
            const rcResult = registry.priceRevolvingCredit(
              revolvingCredit,
              'discounting',
              market,
              asOf,
              null
            );
            const utilization = (rcData.drawnAmount.amount / rcData.commitmentAmount.amount) * 100;
            results.push({
              name: 'Revolving Credit (Deterministic)',
              type: 'RevolvingCredit',
              presentValue: rcResult.presentValue.amount,
              keyMetric: {
                name: 'Utilization',
                value: utilization,
              },
            });
          } catch (err) {
            console.warn('Revolving credit failed, skipping', err);
          }
        }

        if (!cancelled) {
          if (results.length === 0) {
            setError('No credit instruments priced (sample inputs invalid)');
          } else {
            setRows(results);
            setError(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          console.warn('Credit instruments error (skipping page error):', err);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [valuationDate, discountCurve, hazardCurves, baseCorrelation, cdsVolSurface, creditIndexData, cdsSwaps, cdsIndices, cdsTranches, cdsOptions, revolvingCredits]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (rows.length === 0) {
    return <p>Building credit instruments… (or skipped due to invalid sample data)</p>;
  }

  return (
    <section className="example-section">
      <h2>Credit Derivatives</h2>
      <p>
        Credit instruments including single-name CDS, CDS indices, tranches, options on CDS, and
        revolving credit facilities. Uses hazard curves for survival probabilities, base correlation
        for tranche pricing, and supports both deterministic and stochastic utilization for
        revolving credit.
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
              <td>{keyMetric ? `${keyMetric.name}: ${keyMetric.value.toFixed(2)} bps` : '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
