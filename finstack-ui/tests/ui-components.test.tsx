import { render, screen } from "@testing-library/react";
import React from "react";
import { describe, expect, it, vi } from "vitest";

vi.mock("@radix-ui/react-select", () => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const React = require("react");
  const make = (tag: string) => {
    const Component = React.forwardRef<
      HTMLDivElement,
      React.ComponentPropsWithoutRef<"div">
    >(({ children, ...props }, ref) => (
      <div data-testid={tag} ref={ref} {...props}>
        {children}
      </div>
    ));
    Component.displayName = `Mock(${tag})`;
    return Component;
  };

  return {
    Root: ({ children }: { children: React.ReactNode }) => (
      <div data-testid="select-root">{children}</div>
    ),
    Group: make("select-group"),
    Value: make("select-value"),
    Trigger: make("select-trigger"),
    Content: make("select-content"),
    ScrollUpButton: make("select-scroll-up"),
    ScrollDownButton: make("select-scroll-down"),
    Label: make("select-label"),
    Item: make("select-item"),
    Separator: make("select-separator"),
    ItemIndicator: ({ children }: { children: React.ReactNode }) => (
      <span data-testid="select-item-indicator">{children}</span>
    ),
    ItemText: ({ children }: { children: React.ReactNode }) => (
      <span data-testid="select-item-text">{children}</span>
    ),
    Viewport: ({ children }: { children: React.ReactNode }) => (
      <div data-testid="select-viewport">{children}</div>
    ),
    Icon: ({ children }: { children: React.ReactNode }) => (
      <span data-testid="select-icon">{children}</span>
    ),
    Portal: ({ children }: { children: React.ReactNode }) => (
      <div data-testid="select-portal">{children}</div>
    ),
  };
});

vi.mock("lucide-react", () => ({
  Check: () => <span data-testid="icon-check" />,
  ChevronDown: () => <span data-testid="icon-chevron-down" />,
  ChevronUp: () => <span data-testid="icon-chevron-up" />,
}));

describe("Button", () => {
  it("renders variants and sizes", async () => {
    const { Button } = await import("../src/components/ui/button");
    render(
      <Button variant="destructive" size="sm">
        Delete
      </Button>,
    );
    const button = screen.getByText("Delete");
    expect(button.className).toContain("destructive");
    expect(button.className).toContain("h-9");
  });

  it("supports asChild rendering", async () => {
    const { Button } = await import("../src/components/ui/button");
    const Link = ({ children }: { children: React.ReactNode }) => (
      <a data-testid="link">{children}</a>
    );
    render(
      <Button asChild>
        <Link>Go</Link>
      </Button>,
    );
    expect(screen.getByTestId("link")).toBeInTheDocument();
  });
});

describe("Select UI kit components", () => {
  it("renders trigger, content, and items", async () => {
    const {
      Select,
      SelectTrigger,
      SelectContent,
      SelectItem,
      SelectLabel,
      SelectSeparator,
      SelectScrollUpButton,
      SelectScrollDownButton,
    } = await import("../src/components/ui/select");

    render(
      <Select>
        <SelectTrigger>open</SelectTrigger>
        <SelectContent position="popper">
          <SelectScrollUpButton />
          <SelectLabel>Label</SelectLabel>
          <SelectItem value="one">One</SelectItem>
          <SelectSeparator />
          <SelectItem value="two">Two</SelectItem>
          <SelectScrollDownButton />
        </SelectContent>
      </Select>,
    );

    expect(screen.getByTestId("select-trigger")).toBeInTheDocument();
    expect(screen.getByTestId("select-content")).toBeInTheDocument();
    expect(screen.getAllByTestId("select-item")).toHaveLength(2);
    expect(screen.getByTestId("select-label")).toBeInTheDocument();
    expect(screen.getByTestId("select-separator")).toBeInTheDocument();
  });
});
