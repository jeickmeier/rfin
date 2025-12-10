import { zodResolver } from "@hookform/resolvers/zod";
import type { ColumnDef } from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { Controller, useForm } from "react-hook-form";

import { CurveChart } from "../../../components/charts";
import { Input } from "../../../components/ui/input";
import { Button } from "../../../components/ui/button";
import { VirtualDataTable } from "../../../components/tables";
import {
  CalibrationConfigSchema,
  CalibrationQuoteSchema,
  type CalibrationConfig,
  type CalibrationQuote,
} from "../../../schemas/valuations";
import { useCurveCalibration } from "../../../hooks/useCurveCalibration";

const defaultQuotes: CalibrationQuote[] = [
  {
    id: "d1",
    instrument: "OIS",
    tenor: "1M",
    rate: "0.052",
    curve_id: "USD-OIS",
  },
  {
    id: "d2",
    instrument: "OIS",
    tenor: "3M",
    rate: "0.053",
    curve_id: "USD-OIS",
  },
  {
    id: "d3",
    instrument: "OIS",
    tenor: "6M",
    rate: "0.054",
    curve_id: "USD-OIS",
  },
];

export function DiscountCurveCalibration() {
  const [quotes, setQuotes] = useState<CalibrationQuote[]>(defaultQuotes);
  const { calibrate, result, status, error } = useCurveCalibration("discount");

  const form = useForm<CalibrationConfig>({
    resolver: zodResolver(CalibrationConfigSchema),
    defaultValues: { curve_id: "USD-OIS", interpolation: "linear" },
  });

  const updateQuote = (index: number, patch: Partial<CalibrationQuote>) => {
    setQuotes((prev) =>
      prev.map((q, i) => (i === index ? { ...q, ...patch } : q)),
    );
  };

  const columns = useMemo<ColumnDef<CalibrationQuote>[]>(
    () => [
      {
        header: "Instrument",
        accessorKey: "instrument",
        cell: ({ row }) => (
          <Input
            value={row.original.instrument}
            onChange={(e) =>
              updateQuote(row.index, { instrument: e.target.value })
            }
          />
        ),
      },
      {
        header: "Tenor",
        accessorKey: "tenor",
        cell: ({ row }) => (
          <Input
            value={row.original.tenor}
            onChange={(e) => updateQuote(row.index, { tenor: e.target.value })}
          />
        ),
      },
      {
        header: "Rate",
        accessorKey: "rate",
        cell: ({ row }) => (
          <Input
            type="number"
            value={row.original.rate}
            onChange={(e) => updateQuote(row.index, { rate: e.target.value })}
          />
        ),
      },
      {
        header: "Curve",
        accessorKey: "curve_id",
        cell: ({ row }) => (
          <Input
            value={row.original.curve_id}
            onChange={(e) =>
              updateQuote(row.index, { curve_id: e.target.value })
            }
          />
        ),
      },
    ],
    [],
  );

  const addQuote = () => {
    setQuotes((prev) => [
      ...prev,
      {
        id: `q-${prev.length + 1}`,
        instrument: "OIS",
        tenor: "1Y",
        rate: "0.055",
        curve_id: form.getValues("curve_id") ?? "USD-OIS",
      },
    ]);
  };

  const onCalibrate = async (config: CalibrationConfig) => {
    const parsedQuotes = CalibrationQuoteSchema.array().safeParse(quotes);
    if (!parsedQuotes.success) {
      throw parsedQuotes.error;
    }
    await calibrate(parsedQuotes.data, config);
  };

  return (
    <div className="space-y-4" data-testid="discount-curve-calibration">
      <header className="space-y-1">
        <h3 className="text-lg font-semibold">Discount Curve Calibration</h3>
        <p className="text-sm text-muted-foreground">
          Provide quotes and configuration to build a discount curve.
        </p>
      </header>

      <section className="space-y-2 rounded-md border p-3">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-semibold">Quotes</h4>
          <Button size="sm" onClick={addQuote}>
            Add Quote
          </Button>
        </div>
        <VirtualDataTable data={quotes} columns={columns} />
      </section>

      <form
        className="space-y-3 rounded-md border p-3"
        onSubmit={form.handleSubmit(onCalibrate)}
      >
        <div className="grid gap-3 md:grid-cols-2">
          <Controller
            control={form.control}
            name="curve_id"
            render={({ field, fieldState }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Curve ID</label>
                <Input {...field} />
                {fieldState.error ? (
                  <p className="text-xs text-red-500">
                    {fieldState.error.message}
                  </p>
                ) : null}
              </div>
            )}
          />
          <Controller
            control={form.control}
            name="interpolation"
            render={({ field }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Interpolation</label>
                <select
                  className="w-full border rounded px-2 py-1 text-sm"
                  value={field.value}
                  onChange={field.onChange}
                >
                  <option value="linear">Linear</option>
                  <option value="cubic">Cubic</option>
                  <option value="log_linear">Log Linear</option>
                </select>
              </div>
            )}
          />
        </div>

        <Button type="submit" disabled={form.formState.isSubmitting}>
          Calibrate
        </Button>
        {error ? <span className="text-xs text-red-500">{error}</span> : null}
        <p className="text-xs text-muted-foreground">Status: {status}</p>
      </form>

      {result ? (
        <section className="space-y-2 rounded-md border p-3">
          <h4 className="text-sm font-semibold">Calibrated Curve</h4>
          <CurveChart
            title={`Curve ${result.curveId}`}
            series={[{ label: "Zero", points: result.points }]}
          />
          {result.diagnostics?.length ? (
            <ul className="list-disc pl-5 text-sm text-muted-foreground">
              {result.diagnostics.map((d, idx) => (
                <li key={idx}>{d}</li>
              ))}
            </ul>
          ) : null}
        </section>
      ) : null}
    </div>
  );
}
