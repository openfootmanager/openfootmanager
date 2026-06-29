import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Button } from "./Button";

describe("Button", () => {
  it("renders children text", () => {
    render(<Button>Click Me</Button>);
    expect(screen.getByText("Click Me")).toBeInTheDocument();
  });

  it("renders as a button element", () => {
    render(<Button>Btn</Button>);
    expect(screen.getByRole("button")).toBeInTheDocument();
  });

  it("fires onClick when clicked", () => {
    const handler = vi.fn();
    render(<Button onClick={handler}>Go</Button>);
    fireEvent.click(screen.getByRole("button"));
    expect(handler).toHaveBeenCalledOnce();
  });

  it("is disabled when disabled prop is set", () => {
    render(<Button disabled>Nope</Button>);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("applies primary variant by default", () => {
    render(<Button>Primary</Button>);
    expect(screen.getByRole("button").className).toContain("bg-primary-700");
  });

  it("applies accent variant classes", () => {
    render(<Button variant="accent">Accent</Button>);
    expect(screen.getByRole("button").className).toContain("bg-accent-400");
  });

  it("applies ghost variant classes", () => {
    render(<Button variant="ghost">Ghost</Button>);
    expect(screen.getByRole("button").className).toContain("bg-transparent");
  });

  it("applies outline variant classes", () => {
    render(<Button variant="outline">Outline</Button>);
    expect(screen.getByRole("button").className).toContain("border-2");
  });

  it("applies md size by default", () => {
    render(<Button>Med</Button>);
    expect(screen.getByRole("button").className).toContain("text-sm");
  });

  it("applies sm size", () => {
    render(<Button size="sm">Small</Button>);
    expect(screen.getByRole("button").className).toContain("text-xs");
  });

  it("applies lg size", () => {
    render(<Button size="lg">Large</Button>);
    expect(screen.getByRole("button").className).toContain("text-base");
  });

  it("renders icon on the left when provided", () => {
    render(<Button icon={<span data-testid="left-icon">L</span>}>With Icon</Button>);
    expect(screen.getByTestId("left-icon")).toBeInTheDocument();
  });

  it("renders iconRight on the right when provided", () => {
    render(<Button iconRight={<span data-testid="right-icon">R</span>}>With Right</Button>);
    expect(screen.getByTestId("right-icon")).toBeInTheDocument();
  });

  it("merges custom className", () => {
    render(<Button className="extra-class">Custom</Button>);
    expect(screen.getByRole("button").className).toContain("extra-class");
  });
});
