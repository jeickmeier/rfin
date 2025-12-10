import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AmountDisplay } from "../src/components/primitives/AmountDisplay";
import { AmountInput } from "../src/components/primitives/AmountInput";
import { TenorInput } from "../src/components/primitives/TenorInput";

describe("primitives", () => {
  it("formats amount strings without JS number math", () => {
    render(
      <AmountDisplay
        value="1234.567"
        currency="USD"
        roundingContext={{ scale: 2 }}
      />,
    );
    expect(screen.getByLabelText("amount-display")).toHaveTextContent(
      "USD 1,234.57",
    );
  });

  it("emits normalized string values from AmountInput", () => {
    const handleChange = vi.fn();
    render(<AmountInput value="0" onChange={handleChange} currency="USD" />);
    fireEvent.change(screen.getByLabelText("amount-input"), {
      target: { value: "1,234.5" },
    });
    expect(handleChange).toHaveBeenCalledWith("1234.5");
  });

  it("allows tenor-like patterns", () => {
    const handleChange = vi.fn();
    render(<TenorInput value="6M" onChange={handleChange} />);
    fireEvent.change(screen.getByLabelText("tenor-input"), {
      target: { value: "12y" },
    });
    expect(handleChange).toHaveBeenCalledWith("12Y");
  });
});
