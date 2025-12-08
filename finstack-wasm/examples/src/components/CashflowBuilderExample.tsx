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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';

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

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});

export const CashflowBuilderExample: React.FC<CashflowBuilderProps> = (props) => {
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

          if (example.fixedCoupon) {
            const schedule = buildScheduleParams(example.fixedCoupon.schedule);
            const couponType = buildCouponType(example.fixedCoupon.couponType);
            const fixedSpec = new FixedCouponSpec(example.fixedCoupon.rate, schedule, couponType);
            builder = builder.fixedCf(fixedSpec);
          }

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

          if (example.amortization) {
            const finalNotional = Money.fromCode(
              example.amortization.finalNotional.amount,
              example.amortization.finalNotional.currency
            );
            const amortSpec = AmortizationSpec.linearTo(finalNotional);
            builder = builder.amortization(amortSpec);
          }

          if (example.stepUpProgram) {
            const schedule = ScheduleParams.semiannual30360();
            builder = builder.fixedStepup(example.stepUpProgram, schedule, CouponType.Cash());
          }

          if (example.paymentSplitProgram) {
            builder = builder.paymentSplitProgram(example.paymentSplitProgram);
          }

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
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (schedules.length === 0) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        <span className="ml-3 text-muted-foreground">Building cashflow examples…</span>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Cashflow Builder</CardTitle>
        <CardDescription>
          The{' '}
          <code className="rounded bg-muted px-1 py-0.5 font-mono text-sm">CashflowBuilder</code>{' '}
          provides a composable interface for creating complex coupon structures with fixed/floating
          rates, Cash/PIK/split payment types, amortization, step-up programs, and payment split
          transitions.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-8">
        {schedules.map((example, idx) => (
          <div key={idx} className="space-y-4">
            <div>
              <h3 className="text-lg font-semibold">{example.title}</h3>
              <p className="text-sm text-muted-foreground">{example.description}</p>
            </div>

            <div className="flex flex-wrap gap-2">
              <Badge variant="secondary" className="px-3 py-1">
                <span className="text-muted-foreground">Flows:</span>
                <span className="ml-1.5 font-mono">{example.flowCount}</span>
              </Badge>
              <Badge variant="secondary" className="px-3 py-1">
                <span className="text-muted-foreground">Notional:</span>
                <span className="ml-1.5 font-mono">
                  {currencyFormatter.format(example.notional)}
                </span>
              </Badge>
              <Badge variant="secondary" className="px-3 py-1">
                <span className="text-muted-foreground">Day Count:</span>
                <span className="ml-1.5 font-mono">{example.dayCount}</span>
              </Badge>
            </div>

            <div className="rounded-lg border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Date</TableHead>
                    <TableHead>Kind</TableHead>
                    <TableHead className="text-right">Amount</TableHead>
                    <TableHead className="text-right">Accrual Factor</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {example.flows.map((flow, flowIdx) => (
                    <TableRow key={flowIdx}>
                      <TableCell className="font-mono text-sm">{flow.date}</TableCell>
                      <TableCell>
                        <Badge variant={flow.kind === 'Fixed' ? 'default' : 'secondary'}>
                          {flow.kind}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right font-mono">
                        {currencyFormatter.format(flow.amount)}
                      </TableCell>
                      <TableCell className="text-right font-mono text-muted-foreground">
                        {flow.accrualFactor.toFixed(6)}
                      </TableCell>
                    </TableRow>
                  ))}
                  {example.flowCount > example.flows.length && (
                    <TableRow>
                      <TableCell colSpan={4} className="text-center text-muted-foreground italic">
                        … and {example.flowCount - example.flows.length} more flows
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </div>
          </div>
        ))}

        {/* Key Features */}
        <div className="rounded-lg border bg-muted/50 p-6 space-y-4">
          <h3 className="text-lg font-semibold">Key Features</h3>
          <ul className="space-y-2 text-sm">
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 rounded-full bg-primary flex-shrink-0" />
              <span>
                <strong>Fixed and Floating Coupons:</strong> Support for both fixed rates and
                floating indices (e.g., SOFR + margin)
              </span>
            </li>
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 rounded-full bg-primary flex-shrink-0" />
              <span>
                <strong>Forward Rate Incorporation:</strong> Use{' '}
                <code className="rounded bg-background px-1 py-0.5 font-mono text-xs">
                  buildWithCurves(market)
                </code>{' '}
                to include forward rates in floating cashflows
              </span>
            </li>
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 rounded-full bg-primary flex-shrink-0" />
              <span>
                <strong>Payment Types:</strong> Cash, PIK (payment-in-kind), or split percentages
                between cash and PIK
              </span>
            </li>
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 rounded-full bg-primary flex-shrink-0" />
              <span>
                <strong>Amortization:</strong> Linear amortization, step schedules, or custom
                principal repayment
              </span>
            </li>
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 rounded-full bg-primary flex-shrink-0" />
              <span>
                <strong>Step-Up Programs:</strong> Coupon rates that change over time based on date
                boundaries
              </span>
            </li>
          </ul>

          <div className="rounded-md border-l-4 border-primary bg-background p-4 mt-4">
            <h4 className="font-semibold mb-2">Floating Rate Calculation</h4>
            <div className="space-y-1 text-sm text-muted-foreground font-mono">
              <p>
                <span className="text-foreground font-medium">build():</span> coupon = outstanding ×
                (margin_bp × 0.0001 × gearing) × year_fraction
              </p>
              <p>
                <span className="text-foreground font-medium">buildWithCurves():</span> coupon =
                outstanding × (forward_rate × gearing + margin_bp × 0.0001) × year_fraction
              </p>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
