import { zodResolver } from "@hookform/resolvers/zod";
import type { ReactNode } from "react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

import {
  CurrencySelect,
  RateInput,
  TenorInput,
} from "../../../components/primitives";
import { Button } from "../../../components/ui/button";
import { Input } from "../../../components/ui/input";
import {
  CashflowWaterfall,
  normalizeCashflows,
} from "../views/CashflowWaterfall";
import { SwapSpecSchema, type CashflowWire } from "../../../schemas/valuations";
import { useValuation } from "../../../hooks/useValuation";

const IsoDate = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}$/, "must be an ISO date (YYYY-MM-DD)");

export const SwapFormSchema = z.object({
  id: z.string().min(1),
  currency: z.string().min(3),
  notional: z.number().positive(),
  pay_fixed_rate: z.number().finite(),
  receive_float_spread: z.number().finite(),
  effective_date: IsoDate,
  maturity: IsoDate,
  tenor: z.string().min(1),
  discount_curve_id: z.string().min(1),
  forward_curve_id: z.string().min(1),
});

type SwapFormValues = z.infer<typeof SwapFormSchema>;

const defaultSwap: SwapFormValues = {
  id: "SWAP-001",
  currency: "USD",
  notional: 1_000_000,
  pay_fixed_rate: 0.0325,
  receive_float_spread: 0.0005,
  effective_date: "2024-01-01",
  maturity: "2029-01-01",
  tenor: "6M",
  discount_curve_id: "USD-OIS",
  forward_curve_id: "USD-LIBOR",
};

export interface InterestRateSwapPanelProps {
  preset?: Partial<SwapFormValues>;
  title?: string;
}

export function InterestRateSwapPanel({
  preset,
  title = "Interest Rate Swap",
}: InterestRateSwapPanelProps) {
  const { priceInstrument, result, status, error } = useValuation();
  const cashflows: CashflowWire[] = normalizeCashflows(
    (result?.raw as { cashflows?: unknown })?.cashflows,
  );

  const form = useForm<SwapFormValues>({
    resolver: zodResolver(SwapFormSchema),
    defaultValues: { ...defaultSwap, ...(preset ?? {}) },
    mode: "onBlur",
  });

  const metrics: ReactNode = (
    <div className="grid grid-cols-2 gap-2 text-sm">
      <div className="font-medium">Present Value</div>
      <div data-testid="swap-pv">
        {result?.presentValue ??
          result?.error?.message ??
          error?.message ??
          "—"}
      </div>
      <div className="font-medium">Status</div>
      <div>{status}</div>
    </div>
  );

  const onSubmit = async (values: SwapFormValues) => {
    const payload = {
      id: values.id,
      effective_date: values.effective_date,
      maturity: values.maturity,
      legs: [
        {
          id: `${values.id}-fixed`,
          side: "pay",
          legType: "fixed",
          currency: values.currency,
          notional: values.notional,
          rate: values.pay_fixed_rate,
          tenor: values.tenor,
          discount_curve_id: values.discount_curve_id,
          maturity: values.maturity,
        },
        {
          id: `${values.id}-float`,
          side: "receive",
          legType: "float",
          currency: values.currency,
          notional: values.notional,
          spread: values.receive_float_spread,
          index: values.forward_curve_id,
          tenor: values.tenor,
          discount_curve_id: values.discount_curve_id,
          forward_curve_id: values.forward_curve_id,
          maturity: values.maturity,
        },
      ],
      discounting_curve_id: values.discount_curve_id,
    };

    const parsed = SwapSpecSchema.safeParse(payload);
    if (!parsed.success) {
      throw parsed.error;
    }

    await priceInstrument({
      ...parsed.data,
      type: "InterestRateSwap",
    });
  };

  return (
    <div className="space-y-4" data-testid="swap-panel">
      <header className="space-y-1">
        <h3 className="text-lg font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">
          Configure fixed and floating legs, then request valuation.
        </p>
      </header>

      <form
        className="space-y-3 rounded-md border p-4"
        onSubmit={form.handleSubmit(onSubmit)}
      >
        <div className="grid gap-3 md:grid-cols-2">
          <Controller
            control={form.control}
            name="id"
            render={({ field, fieldState }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Swap ID</label>
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
            name="currency"
            render={({ field, fieldState }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Currency</label>
                <CurrencySelect
                  value={field.value}
                  onValueChange={field.onChange}
                />
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
            name="notional"
            render={({ field, fieldState }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Notional</label>
                <Input
                  type="number"
                  value={field.value}
                  onChange={(e) => field.onChange(Number(e.target.value))}
                />
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
            name="tenor"
            render={({ field, fieldState }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Tenor</label>
                <TenorInput
                  value={field.value}
                  onChange={field.onChange}
                  placeholder="6M"
                />
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
            name="effective_date"
            render={({ field, fieldState }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Effective Date</label>
                <Input
                  type="date"
                  value={field.value}
                  onChange={field.onChange}
                  aria-label="swap-effective-date"
                />
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
            name="maturity"
            render={({ field, fieldState }) => (
              <div className="space-y-1">
                <label className="text-sm font-medium">Maturity</label>
                <Input
                  type="date"
                  value={field.value}
                  onChange={field.onChange}
                  aria-label="swap-maturity"
                />
                {fieldState.error ? (
                  <p className="text-xs text-red-500">
                    {fieldState.error.message}
                  </p>
                ) : null}
              </div>
            )}
          />
        </div>

        <div className="grid gap-3 md:grid-cols-2 rounded-md border p-3">
          <div className="space-y-2">
            <h4 className="text-sm font-semibold">Fixed Leg (Pay)</h4>
            <Controller
              control={form.control}
              name="pay_fixed_rate"
              render={({ field, fieldState }) => (
                <div className="space-y-1">
                  <label className="text-sm font-medium">Fixed Rate</label>
                  <RateInput
                    value={field.value}
                    onChange={field.onChange}
                    placeholder="0.03"
                  />
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
              name="discount_curve_id"
              render={({ field, fieldState }) => (
                <div className="space-y-1">
                  <label className="text-sm font-medium">Discount Curve</label>
                  <Input {...field} />
                  {fieldState.error ? (
                    <p className="text-xs text-red-500">
                      {fieldState.error.message}
                    </p>
                  ) : null}
                </div>
              )}
            />
          </div>

          <div className="space-y-2">
            <h4 className="text-sm font-semibold">Floating Leg (Receive)</h4>
            <Controller
              control={form.control}
              name="receive_float_spread"
              render={({ field, fieldState }) => (
                <div className="space-y-1">
                  <label className="text-sm font-medium">Spread</label>
                  <RateInput
                    value={field.value}
                    onChange={field.onChange}
                    placeholder="0.0005"
                  />
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
              name="forward_curve_id"
              render={({ field, fieldState }) => (
                <div className="space-y-1">
                  <label className="text-sm font-medium">Forward Curve</label>
                  <Input {...field} />
                  {fieldState.error ? (
                    <p className="text-xs text-red-500">
                      {fieldState.error.message}
                    </p>
                  ) : null}
                </div>
              )}
            />
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button type="submit" disabled={form.formState.isSubmitting}>
            Price Swap
          </Button>
          {error ? (
            <span className="text-xs text-red-500" data-testid="swap-error">
              {error.message}
            </span>
          ) : result?.error ? (
            <span className="text-xs text-red-500" data-testid="swap-error">
              {result.error.message ?? "Swap pricing unavailable"}
            </span>
          ) : null}
        </div>
      </form>

      <section className="rounded-md border p-3" data-testid="swap-metrics">
        <h4 className="text-sm font-semibold mb-2">Metrics</h4>
        {metrics}
        {result?.diagnostics?.length ? (
          <div className="mt-2 rounded border p-2 bg-muted/40">
            <p className="text-xs font-semibold">Diagnostics</p>
            <ul className="list-disc pl-5 text-xs text-muted-foreground">
              {result.diagnostics.map((d, idx) => (
                <li key={idx}>{d}</li>
              ))}
            </ul>
          </div>
        ) : null}
      </section>

      {cashflows.length ? (
        <section className="rounded-md border p-3" data-testid="swap-cashflows">
          <h4 className="text-sm font-semibold mb-2">Cashflows</h4>
          <CashflowWaterfall cashflows={cashflows} />
        </section>
      ) : null}
    </div>
  );
}
