import type { ReactNode } from "react";
import { GenericInstrumentPanel } from "../../../components/genui";
import { CashflowWaterfall } from "../views/CashflowWaterfall";
import {
  BondSpecSchema,
  type BondSpecInput,
  type CashflowWire,
} from "../../../schemas/valuations";
import { useValuation } from "../../../hooks/useValuation";

const defaultBond: BondSpecInput = {
  id: "BOND-001",
  currency: "USD",
  notional: 1_000_000,
  coupon_rate: 0.05,
  issue: "2024-01-01",
  maturity: "2029-01-01",
  discount_curve_id: "USD-OIS",
  credit_curve_id: null,
};

export interface BondPanelProps {
  preset?: Partial<BondSpecInput>;
  title?: string;
}

export function BondPanel({ preset, title = "Bond" }: BondPanelProps) {
  const { priceInstrument, result, status, error } = useValuation();

  const cashflows: CashflowWire[] = (
    Array.isArray((result?.raw as { cashflows?: unknown })?.cashflows)
      ? ((result?.raw as { cashflows?: CashflowWire[] }).cashflows ?? [])
      : []
  ).map((row) => ({
    ...row,
    rate: String((row as CashflowWire).rate ?? ""),
    notional: String((row as CashflowWire).notional ?? ""),
    discount_factor: String((row as CashflowWire).discount_factor ?? ""),
    present_value: String((row as CashflowWire).present_value ?? ""),
  }));

  const metrics: ReactNode = (
    <div className="grid grid-cols-2 gap-2 text-sm">
      <div className="font-medium">Present Value</div>
      <div data-testid="bond-pv">
        {result?.presentValue ?? result?.error?.message ?? "—"}
      </div>
      <div className="font-medium">Status</div>
      <div>{status}</div>
    </div>
  );

  return (
    <div data-testid="bond-panel">
      <GenericInstrumentPanel<BondSpecInput>
        title={title}
        description="Price a fixed coupon bond with discount and credit curves."
        schema={BondSpecSchema}
        defaultValues={{ ...defaultBond, ...(preset ?? {}) }}
        sections={[
          {
            title: "Instrument",
            fields: [
              { name: "id", label: "Instrument ID", kind: "text" },
              { name: "currency", label: "Currency", kind: "currency" },
              {
                name: "notional",
                label: "Notional",
                kind: "money",
                placeholder: "1000000",
              },
              {
                name: "coupon_rate",
                label: "Coupon Rate",
                kind: "rate",
                placeholder: "0.05",
              },
              { name: "issue", label: "Issue Date", kind: "date" },
              { name: "maturity", label: "Maturity Date", kind: "date" },
            ],
          },
          {
            title: "Curves",
            fields: [
              {
                name: "discount_curve_id",
                label: "Discount Curve",
                kind: "text",
              },
              {
                name: "credit_curve_id",
                label: "Credit Curve (optional)",
                kind: "text",
                placeholder: "CREDIT-USD",
              },
            ],
          },
        ]}
        onSubmit={async (values) => {
          console.log("[BondPanel] onSubmit values:", values);
          await priceInstrument({
            ...values,
            type: "Bond",
          });
        }}
        metrics={metrics}
        cashflows={
          cashflows.length ? <CashflowWaterfall cashflows={cashflows} /> : null
        }
        actions={
          error ? (
            <span className="text-xs text-red-500" data-testid="bond-error">
              {error.message}
            </span>
          ) : null
        }
      />
    </div>
  );
}
