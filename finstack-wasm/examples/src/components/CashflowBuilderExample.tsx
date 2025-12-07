import React, { useEffect, useState } from 'react';
import {
  CashflowBuilder,
  CouponType,
  FixedCouponSpec,
  FloatCouponParams,
  FloatingCouponSpec,
  ScheduleParams,
  Money,
  FsDate,
  AmortizationSpec,
  MarketContext,
  DiscountCurve,
  ForwardCurve,
} from 'finstack-wasm';
import { CashflowBuilderProps, DEFAULT_CASHFLOW_BUILDER_PROPS } from './data/cashflow-builder';

type RequiredCashflowBuilderProps = Required<CashflowBuilderProps>;

interface ExampleSchedule {
  title: string;
  description: string;
  flowCount: number;
  notional: number;
  dayCount: string;
  flows: Array<{
    date: string;
    kind: string;
    amount: number;
    accrualFactor: number;
  }>;
}

// Helper to build schedule params from data
function buildScheduleParams(data: { type: string }): ScheduleParams {
  switch (data.type) {
    case 'quarterlyAct360':
      return ScheduleParams.quarterlyAct360();
    case 'semiannual30360':
      return ScheduleParams.semiannual30360();
    case 'annualActAct':
      return ScheduleParams.annualActAct();
    default:
      return ScheduleParams.quarterlyAct360();
  }
}

// Helper to build coupon type from data
function buildCouponType(data: { type: string; cashPct?: number; pikPct?: number }): CouponType {
  switch (data.type) {
    case 'cash':
      return CouponType.Cash();
    case 'pik':
      return CouponType.PIK();
    case 'split':
      return CouponType.split(data.cashPct ?? 0.5, data.pikPct ?? 0.5);
    default:
      return CouponType.Cash();
  }
}

export const CashflowBuilderExample: React.FC<CashflowBuilderProps> = (props) => {
  // Merge with defaults - DEFAULT_CASHFLOW_BUILDER_PROPS always has these values defined
  const defaults = DEFAULT_CASHFLOW_BUILDER_PROPS as RequiredCashflowBuilderProps;
  const { examples = defaults.examples } = props;

  const [schedules, setSchedules] = useState<ExampleSchedule[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const results: ExampleSchedule[] = [];

        for (const example of examples) {
          const notional = Money.fromCode(example.notional.amount, example.notional.currency);
          const issue = new FsDate(
            example.issueDate.year,
            example.issueDate.month,
            example.issueDate.day
          );
          const maturity = new FsDate(
            example.maturityDate.year,
            example.maturityDate.month,
            example.maturityDate.day
          );

          let builder = new CashflowBuilder().principal(notional, issue, maturity);

          // Handle fixed coupon
          if (example.fixedCoupon) {
            const schedule = buildScheduleParams(example.fixedCoupon.schedule);
            const couponType = buildCouponType(example.fixedCoupon.couponType);
            const fixedSpec = new FixedCouponSpec(example.fixedCoupon.rate, schedule, couponType);
            builder = builder.fixedCf(fixedSpec);
          }

          // Handle floating coupon
          if (example.floatingCoupon) {
            const schedule = buildScheduleParams(example.floatingCoupon.schedule);
            const couponType = buildCouponType(example.floatingCoupon.couponType);
            const floatParams = new FloatCouponParams(
              example.floatingCoupon.indexId,
              example.floatingCoupon.marginBps,
              example.floatingCoupon.gearing,
              example.floatingCoupon.resetLagDays
            );
            const floatSpec = new FloatingCouponSpec(floatParams, schedule, couponType);
            builder = builder.floatingCf(floatSpec);
          }

          // Handle amortization
          if (example.amortization) {
            const finalNotional = Money.fromCode(
              example.amortization.finalNotional.amount,
              example.amortization.finalNotional.currency
            );
            const amortSpec = AmortizationSpec.linearTo(finalNotional);
            builder = builder.amortization(amortSpec);
          }

          // Handle step-up program
          if (example.stepUpProgram) {
            const schedule = ScheduleParams.semiannual30360();
            builder = builder.fixedStepup(example.stepUpProgram, schedule, CouponType.Cash());
          }

          // Handle payment split program
          if (example.paymentSplitProgram) {
            builder = builder.paymentSplitProgram(example.paymentSplitProgram);
          }

          // Build with or without market curves
          let cfSchedule;
          if (example.useMarketCurves && example.marketData) {
            const baseDate = new FsDate(
              example.marketData.discountCurve.baseDate.year,
              example.marketData.discountCurve.baseDate.month,
              example.marketData.discountCurve.baseDate.day
            );
            const market = new MarketContext();

            const discountCurve = new DiscountCurve(
              example.marketData.discountCurve.id,
              baseDate,
              new Float64Array(example.marketData.discountCurve.tenors),
              new Float64Array(example.marketData.discountCurve.discountFactors),
              example.marketData.discountCurve.dayCount,
              example.marketData.discountCurve.interpolation,
              example.marketData.discountCurve.extrapolation,
              example.marketData.discountCurve.continuous
            );
            market.insertDiscount(discountCurve);

            const fwdBaseDate = new FsDate(
              example.marketData.forwardCurve.baseDate.year,
              example.marketData.forwardCurve.baseDate.month,
              example.marketData.forwardCurve.baseDate.day
            );
            const forwardCurve = new ForwardCurve(
              example.marketData.forwardCurve.id,
              fwdBaseDate,
              example.marketData.forwardCurve.tenor,
              new Float64Array(example.marketData.forwardCurve.tenors),
              new Float64Array(example.marketData.forwardCurve.rates),
              example.marketData.forwardCurve.dayCount,
              example.marketData.forwardCurve.compounding,
              example.marketData.forwardCurve.interpolation
            );
            market.insertForward(forwardCurve);

            cfSchedule = builder.buildWithCurves(market);
          } else {
            cfSchedule = builder.build();
          }

          // Extract flows
          const flows = cfSchedule.flows();
          const maxFlows = example.floatingCoupon ? 6 : example.amortization ? 12 : 8;
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, maxFlows); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          results.push({
            title: example.title,
            description: example.description,
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        if (!cancelled) {
          setSchedules(results);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('CashflowBuilder error:', err);
          setError((err as Error).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [examples]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (schedules.length === 0) {
    return <p>Building cashflow examples…</p>;
  }

  const currencyFormatter = new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 2,
  });

  return (
    <section className="example-section">
      <h2>Cashflow Builder</h2>
      <p>
        The <code>CashflowBuilder</code> provides a composable interface for creating complex coupon
        structures with fixed/floating rates, Cash/PIK/split payment types, amortization, step-up
        programs, and payment split transitions. This mirrors the Python bindings for full feature
        parity.
      </p>

      {schedules.map((example, idx) => (
        <div key={idx} style={{ marginTop: idx > 0 ? '3rem' : '1rem' }}>
          <h3 style={{ fontSize: '1.3rem', marginBottom: '0.5rem' }}>{example.title}</h3>
          <p style={{ color: '#999', marginBottom: '1rem' }}>{example.description}</p>

          <div className="inline-cards">
            <div className="card">
              <strong>Total Flows</strong>
              <span>{example.flowCount}</span>
            </div>
            <div className="card">
              <strong>Notional</strong>
              <span>{currencyFormatter.format(example.notional)}</span>
            </div>
            <div className="card">
              <strong>Day Count</strong>
              <span>{example.dayCount}</span>
            </div>
          </div>

          <table style={{ marginTop: '1rem' }}>
            <thead>
              <tr>
                <th>Date</th>
                <th>Kind</th>
                <th style={{ textAlign: 'right' }}>Amount</th>
                <th style={{ textAlign: 'right' }}>Accrual Factor</th>
              </tr>
            </thead>
            <tbody>
              {example.flows.map((flow, flowIdx) => (
                <tr key={flowIdx}>
                  <td>{flow.date}</td>
                  <td>{flow.kind}</td>
                  <td style={{ textAlign: 'right' }}>{currencyFormatter.format(flow.amount)}</td>
                  <td style={{ textAlign: 'right' }}>{flow.accrualFactor.toFixed(6)}</td>
                </tr>
              ))}
              {example.flowCount > example.flows.length && (
                <tr>
                  <td
                    colSpan={4}
                    style={{ textAlign: 'center', color: '#888', fontStyle: 'italic' }}
                  >
                    … and {example.flowCount - example.flows.length} more flows
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      ))}

      <div
        style={{
          marginTop: '3rem',
          padding: '1.5rem',
          backgroundColor: 'rgba(100, 108, 255, 0.05)',
          borderRadius: '8px',
        }}
      >
        <h3 style={{ fontSize: '1.2rem', marginBottom: '1rem' }}>Key Features</h3>
        <ul style={{ paddingLeft: '1.5rem', lineHeight: '1.8' }}>
          <li>
            <strong>Fixed and Floating Coupons:</strong> Support for both fixed rates and floating
            indices (e.g., SOFR + margin)
          </li>
          <li>
            <strong>Forward Rate Incorporation:</strong> Use <code>buildWithCurves(market)</code> to
            include forward rates in floating cashflows, or <code>build()</code> for margin-only
            calculation
          </li>
          <li>
            <strong>Payment Types:</strong> Cash, PIK (payment-in-kind), or split percentages
            between cash and PIK
          </li>
          <li>
            <strong>Amortization:</strong> Linear amortization, step schedules, or custom principal
            repayment
          </li>
          <li>
            <strong>Step-Up Programs:</strong> Coupon rates that change over time based on date
            boundaries
          </li>
          <li>
            <strong>Payment Split Programs:</strong> Transition between cash and PIK payment types
            (e.g., cash → 50/50 → PIK)
          </li>
          <li>
            <strong>Builder Pattern:</strong> Fluent chainable API matching Python bindings
          </li>
        </ul>

        <div
          style={{
            marginTop: '1.5rem',
            padding: '1rem',
            backgroundColor: 'rgba(255, 255, 255, 0.03)',
            borderRadius: '6px',
            borderLeft: '3px solid #646cff',
          }}
        >
          <h4 style={{ fontSize: '1rem', marginBottom: '0.5rem' }}>Floating Rate Calculation</h4>
          <p style={{ margin: '0.5rem 0', color: '#aaa', fontSize: '0.95rem' }}>
            <strong>build():</strong> coupon = outstanding × (margin_bp × 0.0001 × gearing) ×
            year_fraction
          </p>
          <p style={{ margin: '0.5rem 0', color: '#aaa', fontSize: '0.95rem' }}>
            <strong>buildWithCurves():</strong> coupon = outstanding × (forward_rate × gearing +
            margin_bp × 0.0001) × year_fraction
          </p>
          <p style={{ margin: '0.5rem 0', color: '#bbb', fontSize: '0.9rem', fontStyle: 'italic' }}>
            Example 3 shows margin-only vs. Example 4 shows forward rate + margin
          </p>
        </div>
      </div>
    </section>
  );
};
