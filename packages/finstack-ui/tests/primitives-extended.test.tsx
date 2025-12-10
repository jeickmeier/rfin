import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("../src/components/ui/select", () => {
  const React = require("react");
  const Select = ({
    value,
    onValueChange,
    disabled,
    children,
  }: {
    value: string;
    onValueChange: (val: string) => void;
    disabled?: boolean;
    children: React.ReactNode;
  }) => (
    <select
      aria-label="currency-select"
      value={value}
      onChange={(e) => onValueChange(e.target.value)}
      disabled={disabled}
    >
      {children}
    </select>
  );

  const SelectItem = ({
    value,
    children,
  }: {
    value: string;
    children: React.ReactNode;
  }) => (
    <option value={value} aria-label={`option-${value}`}>
      {children}
    </option>
  );

  // Unused in these tests but required exports.
  const SelectTrigger = ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  );
  const SelectValue = () => <span />;
  const SelectContent = ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  );

  return {
    Select,
    SelectItem,
    SelectTrigger,
    SelectValue,
    SelectContent,
  };
});

describe("CurrencySelect", () => {
  it("renders options and calls onChange", async () => {
    const handleChange = vi.fn();
    const { CurrencySelect } = await import(
      "../src/components/primitives/CurrencySelect"
    );

    render(<CurrencySelect value="USD" onChange={handleChange} />);

    const select = screen.getByLabelText("currency-select") as HTMLSelectElement;
    expect(select.value).toBe("USD");
    fireEvent.change(select, { target: { value: "EUR" } });
    expect(handleChange).toHaveBeenCalledWith("EUR");
  });

  it("respects custom currency list and disabled state", async () => {
    const { CurrencySelect } = await import(
      "../src/components/primitives/CurrencySelect"
    );
    render(
      <CurrencySelect
        value="JPY"
        currencies={["JPY", "AUD"]}
        onChange={() => {}}
        disabled
      />,
    );
    const select = screen.getByLabelText("currency-select") as HTMLSelectElement;
    expect(select.querySelectorAll("option")).toHaveLength(2);
    expect(select.disabled).toBe(true);
  });
});

describe("DatePicker", () => {
  it("renders date input and forwards changes", async () => {
    const { DatePicker } = await import(
      "../src/components/primitives/DatePicker"
    );
    const handleChange = vi.fn();
    render(<DatePicker value="2024-01-01" onChange={handleChange} />);
    const input = screen.getByLabelText("date-picker") as HTMLInputElement;
    expect(input.value).toBe("2024-01-01");
    fireEvent.change(input, { target: { value: "2024-02-15" } });
    expect(handleChange).toHaveBeenCalledWith("2024-02-15");
  });

  it("respects min/max and placeholder", async () => {
    const { DatePicker } = await import(
      "../src/components/primitives/DatePicker"
    );
    render(
      <DatePicker
        value=""
        onChange={() => {}}
        min="2024-01-01"
        max="2024-12-31"
        placeholder="YYYY-MM-DD"
      />,
    );
    const input = screen.getByLabelText("date-picker") as HTMLInputElement;
    expect(input.min).toBe("2024-01-01");
    expect(input.max).toBe("2024-12-31");
    expect(input.placeholder).toBe("YYYY-MM-DD");
  });
});
