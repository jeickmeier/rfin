import React, { useEffect, useState } from 'react';
import { CashFlow, Money, FsDate } from 'finstack-wasm';
import { CashflowBasicsProps, DEFAULT_CASHFLOW_PROPS } from './data/cashflows';

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
  // Merge with defaults
  const { cashflows = DEFAULT_CASHFLOW_PROPS.cashflows! } = props;

  const [state, setState] = useState<CashflowState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const computeState = () => {
      try {
        // Build cashflows from props
        const flows: Array<{ label: string; flow: CashFlow }> = [];

        for (const cfData of cashflows) {
          const date = new FsDate(cfData.date.year, cfData.date.month, cfData.date.day);
          const money = Money.fromCode(cfData.amount.amount, cfData.amount.currency);

          let flow: CashFlow;
          switch (cfData.type) {
            case 'fixed':
              flow = CashFlow.fixed(date, money, cfData.accrualFactor ?? 0.25);
              break;
            case 'floating':
              const resetDate = cfData.resetDate
                ? new FsDate(cfData.resetDate.year, cfData.resetDate.month, cfData.resetDate.day)
                : date;
              flow = CashFlow.floating(date, money, resetDate, cfData.accrualFactor ?? 0.25);
              break;
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

        // Get tuple view from first fixed cashflow
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
            if (leftDate.year !== rightDate.year) {
              return leftDate.year - rightDate.year;
            }
            if (leftDate.month !== rightDate.month) {
              return leftDate.month - rightDate.month;
            }
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
      // Use setTimeout to avoid synchronous setState in effect
      setTimeout(() => setState(computedState), 0);
    }
  }, [cashflows]);

  if (error) {
    return <p className="error">{error}</p>;
  }

  if (!state) {
    return <p>Preparing cashflow examples…</p>;
  }

  return (
    <section className="example-section">
      <h2>Cashflow Primitives</h2>
      <p>
        Create fixed, floating, fee, and principal cashflows directly from the wasm bindings –
        mirroring the Python tutorial.
      </p>

      <h3>Constructed Cashflows</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Label</th>
            <th>Kind</th>
            <th>Date</th>
            <th>Amount</th>
            <th>Accrual</th>
            <th>Reset Date</th>
          </tr>
        </thead>
        <tbody>
          {state.rows.map((row) => (
            <tr key={row.label}>
              <td>{row.label}</td>
              <td>{row.kind}</td>
              <td>{row.date}</td>
              <td>{row.amount}</td>
              <td>{row.accrual}</td>
              <td>{row.resetDate ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>Tuple Conversion</h3>
      <div className="inline-cards">
        <div className="card">
          <strong>Date</strong>
          <span>{state.tuple.date}</span>
        </div>
        <div className="card">
          <strong>Amount</strong>
          <span>{state.tuple.amount}</span>
        </div>
        <div className="card">
          <strong>Kind</strong>
          <span>{state.tuple.kind}</span>
        </div>
        <div className="card">
          <strong>Accrual</strong>
          <span>{state.tuple.accrualFactor}</span>
        </div>
        <div className="card">
          <strong>Reset</strong>
          <span>{state.tuple.resetDate}</span>
        </div>
      </div>

      <h3>Sorted Schedule</h3>
      <table className="data-table compact">
        <thead>
          <tr>
            <th>Date</th>
            <th>Kind</th>
            <th>Amount (USD)</th>
          </tr>
        </thead>
        <tbody>
          {state.schedule.map((row, idx) => (
            <tr key={`${row.date}-${idx}`}>
              <td>{row.date}</td>
              <td>{row.kind}</td>
              <td>{row.amount}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
};
