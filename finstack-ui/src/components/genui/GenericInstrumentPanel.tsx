import { zodResolver } from "@hookform/resolvers/zod";
import type { ReactNode } from "react";
import { Controller, useForm } from "react-hook-form";
import type { z } from "zod";

import {
  AmountInput,
  CurrencySelect,
  DatePicker,
  RateInput,
  TenorInput,
} from "../primitives";
import { Button } from "../ui/button";
import { Input } from "../ui/input";

type FieldKind =
  | "text"
  | "number"
  | "money"
  | "currency"
  | "rate"
  | "tenor"
  | "date"
  | "enum"
  | "curveId";

export interface FieldDescriptor<TValues> {
  name: keyof TValues & string;
  label: string;
  kind: FieldKind;
  placeholder?: string;
  options?: { label: string; value: string }[];
  helperText?: string;
}

export interface GenericInstrumentPanelProps<TValues> {
  title: string;
  description?: string;
  schema: z.ZodType<TValues>;
  defaultValues: Partial<TValues>;
  sections: Array<{
    title: string;
    fields: Array<FieldDescriptor<TValues>>;
  }>;
  onSubmit: (values: TValues) => Promise<void> | void;
  metrics?: ReactNode;
  marketData?: ReactNode;
  cashflows?: ReactNode;
  actions?: ReactNode;
  submitLabel?: string;
}

export function GenericInstrumentPanel<TValues>({
  title,
  description,
  schema,
  defaultValues,
  sections,
  onSubmit,
  metrics,
  marketData,
  cashflows,
  actions,
  submitLabel = "Price",
}: GenericInstrumentPanelProps<TValues>) {
  const form = useForm<TValues>({
    resolver: zodResolver(schema),
    mode: "onBlur",
    defaultValues,
  });

  const renderField = (
    field: FieldDescriptor<TValues>,
    value: unknown,
    onChange: (val: unknown) => void,
  ) => {
    switch (field.kind) {
      case "money":
        return (
          <AmountInput
            value={String(value ?? "")}
            onChange={(v) => onChange(Number(v))}
            placeholder={field.placeholder}
          />
        );
      case "currency":
        return (
          <CurrencySelect
            value={(value as string) ?? "USD"}
            onChange={(v) => onChange(v)}
          />
        );
      case "rate":
        return (
          <RateInput
            value={typeof value === "number" ? value : 0}
            onChange={(v) => onChange(v)}
            placeholder={field.placeholder}
          />
        );
      case "tenor":
        return (
          <TenorInput
            value={(value as string) ?? ""}
            onChange={(v) => onChange(v)}
            placeholder={field.placeholder}
          />
        );
      case "date":
        return (
          <DatePicker
            value={(value as string) ?? ""}
            onChange={(v) => onChange(v)}
            placeholder={field.placeholder}
          />
        );
      case "enum":
      case "curveId":
        return (
          <select
            className="w-full border rounded px-2 py-1 text-sm"
            value={(value as string) ?? ""}
            onChange={(e) => onChange(e.target.value)}
          >
            <option value="">Select...</option>
            {field.options?.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        );
      case "number":
        return (
          <Input
            type="number"
            value={value === undefined ? "" : Number(value)}
            onChange={(e) => onChange(Number(e.target.value))}
            placeholder={field.placeholder}
          />
        );
      case "text":
      default:
        return (
          <Input
            type="text"
            value={(value as string) ?? ""}
            onChange={(e) => onChange(e.target.value)}
            placeholder={field.placeholder}
          />
        );
    }
  };

  return (
    <div className="flex flex-col gap-4" data-testid="generic-instrument-panel">
      <header className="space-y-1">
        <h3 className="text-lg font-semibold">{title}</h3>
        {description ? (
          <p className="text-sm text-muted-foreground">{description}</p>
        ) : null}
      </header>

      <form
        onSubmit={form.handleSubmit(onSubmit)}
        className="space-y-4 rounded-md border p-4"
      >
        <div className="grid gap-4 md:grid-cols-2">
          {sections.map((section) => (
            <fieldset
              key={section.title}
              className="space-y-2 rounded-md border p-3"
            >
              <legend className="px-1 text-sm font-semibold">
                {section.title}
              </legend>
              {section.fields.map((field) => (
                <Controller
                  key={field.name}
                  name={field.name as keyof TValues}
                  control={form.control}
                  render={({ field: ctrl, fieldState }) => (
                    <div className="space-y-1">
                      <label className="text-sm font-medium">
                        {field.label}
                      </label>
                      {renderField(field, ctrl.value, ctrl.onChange)}
                      {field.helperText ? (
                        <p className="text-xs text-muted-foreground">
                          {field.helperText}
                        </p>
                      ) : null}
                      {fieldState.error ? (
                        <p className="text-xs text-red-500">
                          {fieldState.error.message}
                        </p>
                      ) : null}
                    </div>
                  )}
                />
              ))}
            </fieldset>
          ))}
        </div>

        <div className="flex items-center gap-2">
          <Button type="submit" disabled={form.formState.isSubmitting}>
            {submitLabel}
          </Button>
          {actions}
        </div>
      </form>

      {marketData ? (
        <section className="rounded-md border p-3" data-testid="market-data">
          <h4 className="text-sm font-semibold mb-2">Market Data</h4>
          {marketData}
        </section>
      ) : null}

      {metrics ? (
        <section className="rounded-md border p-3" data-testid="metrics">
          <h4 className="text-sm font-semibold mb-2">Metrics</h4>
          {metrics}
        </section>
      ) : null}

      {cashflows ? (
        <section className="rounded-md border p-3" data-testid="cashflows">
          <h4 className="text-sm font-semibold mb-2">Cashflows</h4>
          {cashflows}
        </section>
      ) : null}
    </div>
  );
}
