import type { ColumnDef } from "@tanstack/react-table";
import { VirtualDataTable } from "../../../components/tables";
import { type CashflowWire } from "../../../schemas/valuations";

export interface CashflowWaterfallProps {
  cashflows: CashflowWire[];
}

export function CashflowWaterfall({ cashflows }: CashflowWaterfallProps) {
  const columns: ColumnDef<CashflowWire>[] = [
    { header: "Period", accessorKey: "period" },
    { header: "Leg", accessorKey: "leg" },
    { header: "Rate", accessorKey: "rate" },
    { header: "Notional", accessorKey: "notional" },
    { header: "DF", accessorKey: "discount_factor" },
    { header: "PV", accessorKey: "present_value" },
  ];

  const totalPv = cashflows.reduce((acc, row) => {
    const val = Number(row.present_value);
    return acc + (Number.isFinite(val) ? val : 0);
  }, 0);

  return (
    <div className="space-y-2" data-testid="cashflow-waterfall">
      <VirtualDataTable data={cashflows} columns={columns} />
      <div className="flex justify-end text-sm font-semibold">
        Total PV: {totalPv.toFixed(2)}
      </div>
    </div>
  );
}
