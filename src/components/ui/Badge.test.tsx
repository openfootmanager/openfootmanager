import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Badge } from "./Badge";

describe("Badge", () => {
  it("renders children text", () => {
    render(<Badge>GK</Badge>);
    expect(screen.getByText("GK")).toBeInTheDocument();
  });

  it("applies neutral variant classes by default", () => {
    render(<Badge>DEF</Badge>);
    const el = screen.getByText("DEF");
    expect(el.className).toContain("bg-sunken");
  });

  it("applies the correct variant classes", () => {
    render(<Badge variant="danger">RED</Badge>);
    const el = screen.getByText("RED");
    expect(el.className).toContain("bg-danger-tint");
  });

  it("applies sm size by default", () => {
    render(<Badge>SM</Badge>);
    const el = screen.getByText("SM");
    expect(el.className).toContain("text-caption");
  });

  it("applies md size when specified", () => {
    render(<Badge size="md">MD</Badge>);
    const el = screen.getByText("MD");
    expect(el.className).toContain("text-body");
  });

  it("merges custom className", () => {
    render(<Badge className="my-custom">TEST</Badge>);
    const el = screen.getByText("TEST");
    expect(el.className).toContain("my-custom");
  });

  it("renders as a span element", () => {
    render(<Badge>TAG</Badge>);
    const el = screen.getByText("TAG");
    expect(el.tagName).toBe("SPAN");
  });
});
