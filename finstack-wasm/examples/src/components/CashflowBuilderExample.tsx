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

export const CashflowBuilderExample: React.FC = () => {
  const [schedules, setSchedules] = useState<ExampleSchedule[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const notional = Money.fromCode(1_000_000, 'USD');
        const issue = new FsDate(2025, 1, 15);
        const maturity = new FsDate(2030, 1, 15);
        const examples: ExampleSchedule[] = [];

        // Example 1: Simple Fixed Coupon Bond (5% quarterly)
        {
          const schedule = ScheduleParams.quarterlyAct360();
          const fixedSpec = new FixedCouponSpec(
            0.05, // 5% annual rate
            schedule,
            CouponType.Cash()
          );

          const cfSchedule = new CashflowBuilder()
            .principal(notional, issue, maturity)
            .fixedCf(fixedSpec)
            .build();

          const flows = cfSchedule.flows();
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, 8); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          examples.push({
            title: 'Simple Fixed Coupon (5% Quarterly)',
            description: 'Standard quarterly coupons paid in cash with Act/360 day count',
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        // Example 2: PIK Toggle Bond (70% Cash / 30% PIK)
        {
          const schedule = ScheduleParams.semiannual30360();
          const fixedSpec = new FixedCouponSpec(
            0.08, // 8% annual rate
            schedule,
            CouponType.split(0.7, 0.3) // 70% cash, 30% PIK
          );

          const cfSchedule = new CashflowBuilder()
            .principal(notional, issue, maturity)
            .fixedCf(fixedSpec)
            .build();

          const flows = cfSchedule.flows();
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, 8); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          examples.push({
            title: 'PIK Toggle Bond (70% Cash / 30% PIK)',
            description: 'Semi-annual coupons split between cash payment and capitalization',
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        // Example 3: Floating Rate Note - Without Curves (Margin Only)
        {
          const schedule = ScheduleParams.quarterlyAct360();
          const floatParams = new FloatCouponParams(
            'USD-SOFR-3M', // index
            150.0,         // margin in bps
            1.0,           // gearing
            2              // reset lag days
          );
          const floatSpec = new FloatingCouponSpec(
            floatParams,
            schedule,
            CouponType.Cash()
          );

          const cfSchedule = new CashflowBuilder()
            .principal(notional, issue, maturity)
            .floatingCf(floatSpec)
            .build();  // No market curves - uses margin only

          const flows = cfSchedule.flows();
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, 6); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          examples.push({
            title: 'Floating Rate Note - Margin Only (No Curves)',
            description: 'Uses only margin (150 bps): coupon = outstanding × 0.0150 × year_fraction',
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        // Example 3b: Floating Rate Note - With Market Curves (Forward Rate + Margin)
        {
          // Create market context with forward curve
          const baseDate = new FsDate(2025, 1, 2);
          const market = new MarketContext();

          const discountCurve = new DiscountCurve(
            'USD-OIS',
            baseDate,
            new Float64Array([0.0, 1.0, 2.0, 3.0]),
            new Float64Array([1.0, 0.9950, 0.9880, 0.9800]),
            'act_365f',
            'monotone_convex',
            'flat_forward',
            true
          );
          market.insertDiscount(discountCurve);

          // Forward curve: rates increase over time (3% → 3.5% → 4%)
          const forwardCurve = new ForwardCurve(
            'USD-SOFR-3M',
            baseDate,
            0.25,  // 3-month tenor
            new Float64Array([0.0, 0.5, 1.0, 2.0]),
            new Float64Array([0.0300, 0.0325, 0.0350, 0.0400]),
            'act_360',
            2,
            'linear'
          );
          market.insertForward(forwardCurve);

          const schedule = ScheduleParams.quarterlyAct360();
          const floatParams = new FloatCouponParams(
            'USD-SOFR-3M',
            150.0,
            1.0,
            2
          );
          const floatSpec = new FloatingCouponSpec(
            floatParams,
            schedule,
            CouponType.Cash()
          );

          const cfSchedule = new CashflowBuilder()
            .principal(notional, issue, maturity)
            .floatingCf(floatSpec)
            .buildWithCurves(market);  // WITH market curves - uses forward rates

          const flows = cfSchedule.flows();
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, 6); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          examples.push({
            title: 'Floating Rate Note - With Forward Rates (Market Curves)',
            description: 'Uses forward_rate × gearing + margin: coupon = outstanding × (fwd_rate + 0.0150) × yf',
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        // Example 4: Amortizing Loan (Linear Amortization)
        {
          const schedule = ScheduleParams.quarterlyAct360();
          const fixedSpec = new FixedCouponSpec(
            0.06, // 6% annual rate
            schedule,
            CouponType.Cash()
          );

          const finalNotional = Money.fromCode(200_000, 'USD');
          const amortSpec = AmortizationSpec.linearTo(finalNotional);

          const cfSchedule = new CashflowBuilder()
            .principal(notional, issue, maturity)
            .amortization(amortSpec)
            .fixedCf(fixedSpec)
            .build();

          const flows = cfSchedule.flows();
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, 12); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          examples.push({
            title: 'Amortizing Loan (Linear to $200K)',
            description: 'Quarterly coupons with linear amortization from $1M to $200K',
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        // Example 5: Step-Up Coupon Structure
        {
          const schedule = ScheduleParams.semiannual30360();
          const stepProgram = [
            ['2027-01-15', 0.04],  // 4% until 2027
            ['2029-01-15', 0.05],  // 5% until 2029
            ['2030-01-15', 0.06],  // 6% until maturity
          ];

          const cfSchedule = new CashflowBuilder()
            .principal(notional, issue, maturity)
            .fixedStepup(stepProgram, schedule, CouponType.Cash())
            .build();

          const flows = cfSchedule.flows();
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, 12); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          examples.push({
            title: 'Step-Up Coupon Structure (4% → 5% → 6%)',
            description: 'Semi-annual coupons with step-up rates: 4% for 2 years, then 5%, then 6%',
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        // Example 6: Payment Split Program (Cash → PIK Transition)
        {
          const schedule = ScheduleParams.quarterlyAct360();
          const fixedSpec = new FixedCouponSpec(
            0.07, // 7% annual rate
            schedule,
            CouponType.Cash() // Initial default
          );

          const splitProgram = [
            ['2027-01-15', 'cash'],           // 100% cash until 2027
            ['2028-01-15', 'split:0.5:0.5'],  // 50/50 split from 2027-2028
            ['2030-01-15', 'pik'],            // 100% PIK thereafter
          ];

          const cfSchedule = new CashflowBuilder()
            .principal(notional, issue, maturity)
            .fixedCf(fixedSpec)
            .paymentSplitProgram(splitProgram)
            .build();

          const flows = cfSchedule.flows();
          const flowData = [];
          for (let i = 0; i < Math.min(flows.length, 15); i++) {
            const cf = flows[i] as any;
            flowData.push({
              date: `${cf.date.year}-${String(cf.date.month).padStart(2, '0')}-${String(cf.date.day).padStart(2, '0')}`,
              kind: cf.kind.name,
              amount: cf.amount.amount,
              accrualFactor: cf.accrualFactor,
            });
          }

          examples.push({
            title: 'Payment Split Program (Cash → PIK Transition)',
            description: 'Quarterly coupons transitioning from 100% cash to 50/50 split to 100% PIK',
            flowCount: flows.length,
            notional: cfSchedule.notional.amount,
            dayCount: cfSchedule.dayCount.name,
            flows: flowData,
          });
        }

        if (!cancelled) {
          setSchedules(examples);
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
  }, []);

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
        The <code>CashflowBuilder</code> provides a composable interface for creating complex
        coupon structures with fixed/floating rates, Cash/PIK/split payment types, amortization,
        step-up programs, and payment split transitions. This mirrors the Python bindings for
        full feature parity.
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
                  <td colSpan={4} style={{ textAlign: 'center', color: '#888', fontStyle: 'italic' }}>
                    … and {example.flowCount - example.flows.length} more flows
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      ))}

      <div style={{ marginTop: '3rem', padding: '1.5rem', backgroundColor: 'rgba(100, 108, 255, 0.05)', borderRadius: '8px' }}>
        <h3 style={{ fontSize: '1.2rem', marginBottom: '1rem' }}>Key Features</h3>
        <ul style={{ paddingLeft: '1.5rem', lineHeight: '1.8' }}>
          <li><strong>Fixed and Floating Coupons:</strong> Support for both fixed rates and floating indices (e.g., SOFR + margin)</li>
          <li><strong>Forward Rate Incorporation:</strong> Use <code>buildWithCurves(market)</code> to include forward rates in floating cashflows, or <code>build()</code> for margin-only calculation</li>
          <li><strong>Payment Types:</strong> Cash, PIK (payment-in-kind), or split percentages between cash and PIK</li>
          <li><strong>Amortization:</strong> Linear amortization, step schedules, or custom principal repayment</li>
          <li><strong>Step-Up Programs:</strong> Coupon rates that change over time based on date boundaries</li>
          <li><strong>Payment Split Programs:</strong> Transition between cash and PIK payment types (e.g., cash → 50/50 → PIK)</li>
          <li><strong>Builder Pattern:</strong> Fluent chainable API matching Python bindings</li>
        </ul>

        <div style={{ marginTop: '1.5rem', padding: '1rem', backgroundColor: 'rgba(255, 255, 255, 0.03)', borderRadius: '6px', borderLeft: '3px solid #646cff' }}>
          <h4 style={{ fontSize: '1rem', marginBottom: '0.5rem' }}>Floating Rate Calculation</h4>
          <p style={{ margin: '0.5rem 0', color: '#aaa', fontSize: '0.95rem' }}>
            <strong>build():</strong> coupon = outstanding × (margin_bp × 0.0001 × gearing) × year_fraction
          </p>
          <p style={{ margin: '0.5rem 0', color: '#aaa', fontSize: '0.95rem' }}>
            <strong>buildWithCurves():</strong> coupon = outstanding × (forward_rate × gearing + margin_bp × 0.0001) × year_fraction
          </p>
          <p style={{ margin: '0.5rem 0', color: '#bbb', fontSize: '0.9rem', fontStyle: 'italic' }}>
            Example 3 shows margin-only vs. Example 3b shows forward rate + margin
          </p>
        </div>
      </div>
    </section>
  );
};

