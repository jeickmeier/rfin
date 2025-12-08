import React, { useEffect, useState } from 'react';
import { CashFlow, Money, FsDate } from 'finstack-wasm';
import { CashflowBasicsProps, DEFAULT_CASHFLOW_PROPS } from './data/cashflows';
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

type RequiredCashflowBasicsProps = Required<CashflowBasicsProps>;

interface CashflowRow {
  label: string;
  kind: string;
  date: string;
  amount: string;
  accrual: string;
  resetDate?: string | null;
}

interface CashflowTupleView {
  date: string;
  amount: string;
  kind: string;
  accrualFactor: string;
  resetDate: string;
}

interface ScheduleRow {
  date: string;
  kind: string;
  amount: string;
}

interface CashflowState {
  rows: CashflowRow[];
  tuple: CashflowTupleView;
  schedule: ScheduleRow[];
}

const toIso = (date: FsDate): string =>
  `${date.year}-${String(date.month).padStart(2, '0')}-${String(date.day).padStart(2, '0')}`;

const asDisplayMoney = (money: Money): string => money.format();

const asAccrual = (value: number): string => `${value.toFixed(4)}`;

export const CashflowBasicsExample: React.FC<CashflowBasicsProps> = (props) => {
  const defaults = DEFAULT_CASHFLOW_PROPS as RequiredCashflowBasicsProps;
  const { cashflows = defaults.cashflows } = props;

  const [state, setState] = useState<CashflowState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const computeState = () => {
      try {
        const flows: Array<{ label: string; flow: CashFlow }> = [];

        for (const cfData of cashflows) {
          const date = new FsDate(cfData.date.year, cfData.date.month, cfData.date.day);
          const money = Money.fromCode(cfData.amount.amount, cfData.amount.currency);

          let flow: CashFlow;
          switch (cfData.type) {
            case 'fixed':
              flow = CashFlow.fixed(date, money, cfData.accrualFactor ?? 0.25);
              break;
            case 'floating': {
              const resetDate = cfData.resetDate
                ? new FsDate(cfData.resetDate.year, cfData.resetDate.month, cfData.resetDate.day)
                : date;
              flow = CashFlow.floating(date, money, resetDate, cfData.accrualFactor ?? 0.25);
              break;
            }
            case 'fee':
              flow = CashFlow.fee(date, money);
              break;
            case 'principalExchange':
              flow = CashFlow.principalExchange(date, money);
              break;
            default:
              throw new Error(`Unknown cashflow type: ${cfData.type}`);
          }

          flows.push({ label: cfData.label, flow });
        }

        const rows: CashflowRow[] = flows.map(({ label, flow }) => ({
          label,
          kind: flow.kind.name,
          date: toIso(flow.date),
          amount: asDisplayMoney(flow.amount),
          accrual: asAccrual(flow.accrualFactor),
          resetDate: flow.resetDate ? toIso(flow.resetDate) : null,
        }));

        const firstFixed = flows.find((f) => f.flow.kind.name === 'Fixed');
        const tupleView = firstFixed ? firstFixed.flow.toTuple() : flows[0].flow.toTuple();
        const tuple: CashflowTupleView = {
          date: toIso(tupleView[0]),
          amount: asDisplayMoney(tupleView[1]),
          kind: tupleView[2].name,
          accrualFactor: asAccrual(tupleView[3]),
          resetDate: tupleView[4] ? toIso(tupleView[4]) : '(none)',
        };

        const schedule: ScheduleRow[] = flows
          .slice()
          .sort((lhs, rhs) => {
            const leftDate = lhs.flow.date;
            const rightDate = rhs.flow.date;
            if (leftDate.year !== rightDate.year) return leftDate.year - rightDate.year;
            if (leftDate.month !== rightDate.month) return leftDate.month - rightDate.month;
            return leftDate.day - rightDate.day;
          })
          .map(({ flow }) => ({
            date: toIso(flow.date),
            kind: flow.kind.name,
            amount: flow.amount.amount.toLocaleString(undefined, {
              minimumFractionDigits: 2,
              maximumFractionDigits: 2,
            }),
          }));

        return { rows, tuple, schedule };
      } catch (err) {
        setError((err as Error).message);
        return null;
      }
    };

    const computedState = computeState();
    if (computedState) {
      setTimeout(() => setState(computedState), 0);
    }
  }, [cashflows]);

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (!state) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        <span className="ml-3 text-muted-foreground">Preparing cashflow examples…</span>
      </div>
    );
  }

  const kindVariant = (kind: string) => {
    switch (kind) {
      case 'Fixed':
        return 'default';
      case 'Floating':
        return 'secondary';
      case 'Fee':
        return 'outline';
      default:
        return 'secondary';
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Cashflow Primitives</CardTitle>
        <CardDescription>
          Create fixed, floating, fee, and principal cashflows directly from the wasm bindings –
          mirroring the Python tutorial.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-8">
        {/* Constructed Cashflows */}
        <div className="space-y-3">
          <h3 className="text-lg font-semibold">Constructed Cashflows</h3>
          <div className="rounded-lg border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Label</TableHead>
                  <TableHead>Kind</TableHead>
                  <TableHead>Date</TableHead>
                  <TableHead className="text-right">Amount</TableHead>
                  <TableHead className="text-right">Accrual</TableHead>
                  <TableHead>Reset Date</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {state.rows.map((row) => (
                  <TableRow key={row.label}>
                    <TableCell className="font-medium">{row.label}</TableCell>
                    <TableCell>
                      <Badge variant={kindVariant(row.kind)}>{row.kind}</Badge>
                    </TableCell>
                    <TableCell className="font-mono text-sm">{row.date}</TableCell>
                    <TableCell className="text-right font-mono">{row.amount}</TableCell>
                    <TableCell className="text-right font-mono">{row.accrual}</TableCell>
                    <TableCell className="font-mono text-sm text-muted-foreground">
                      {row.resetDate ?? '—'}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>

        {/* Tuple Conversion */}
        <div className="space-y-3">
          <h3 className="text-lg font-semibold">Tuple Conversion</h3>
          <div className="grid gap-3 sm:grid-cols-5">
            {[
              { label: 'Date', value: state.tuple.date },
              { label: 'Amount', value: state.tuple.amount },
              { label: 'Kind', value: state.tuple.kind },
              { label: 'Accrual', value: state.tuple.accrualFactor },
              { label: 'Reset', value: state.tuple.resetDate },
            ].map((item) => (
              <div key={item.label} className="rounded-lg border bg-muted/50 p-3">
                <div className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  {item.label}
                </div>
                <div className="mt-1 font-mono text-sm">{item.value}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Sorted Schedule */}
        <div className="space-y-3">
          <h3 className="text-lg font-semibold">Sorted Schedule</h3>
          <div className="rounded-lg border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Kind</TableHead>
                  <TableHead className="text-right">Amount (USD)</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {state.schedule.map((row, idx) => (
                  <TableRow key={`${row.date}-${idx}`}>
                    <TableCell className="font-mono text-sm">{row.date}</TableCell>
                    <TableCell>
                      <Badge variant={kindVariant(row.kind)}>{row.kind}</Badge>
                    </TableCell>
                    <TableCell className="text-right font-mono">{row.amount}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
