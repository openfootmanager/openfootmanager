import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ProgressBar } from "./ProgressBar";

describe("ProgressBar", () => {
  it("renders without crashing", () => {
    const { container } = render(<ProgressBar value={50} />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it("clamps value to 0-100 range (upper)", () => {
    render(<ProgressBar value={150} showLabel />);
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("clamps value to 0-100 range (lower)", () => {
    render(<ProgressBar value={-20} showLabel />);
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  it("shows label when showLabel is true", () => {
    render(<ProgressBar value={75} showLabel />);
    expect(screen.getByText("75%")).toBeInTheDocument();
  });

  it("does not show label by default", () => {
    render(<ProgressBar value={75} />);
    expect(screen.queryByText("75%")).not.toBeInTheDocument();
  });

  it("uses auto variant: positive for >= 70", () => {
    const { container } = render(<ProgressBar value={80} />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("bg-positive-solid");
  });

  it("uses auto variant: warning for 40-69", () => {
    const { container } = render(<ProgressBar value={55} />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("bg-warning-solid");
  });

  it("uses auto variant: danger for < 40", () => {
    const { container } = render(<ProgressBar value={20} />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("bg-danger-solid");
  });

  it("uses explicit variant when specified", () => {
    const { container } = render(<ProgressBar value={80} variant="primary" />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("bg-primary-500");
  });

  it("applies sm height by default", () => {
    const { container } = render(<ProgressBar value={50} />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("h-1.5");
  });

  it("applies md height", () => {
    const { container } = render(<ProgressBar value={50} size="md" />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("h-2.5");
  });

  it("applies lg height", () => {
    const { container } = render(<ProgressBar value={50} size="lg" />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("h-4");
  });

  it("sets width style to match value", () => {
    const { container } = render(<ProgressBar value={65} />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.style.width).toBe("65%");
  });

  /**
   * Given a value of 95 that the backend rates bad
   * When the bar is drawn with the backend's tone
   * Then it fills in that tone, not in a colour of its own picked from the number.
   */
  it("fills with the backend's tone, not a colour from the value", () => {
    const { container } = render(<ProgressBar value={95} tone="bad" />);
    const bar = container.querySelector("[style]") as HTMLElement;
    expect(bar.className).toContain("bg-tone-bad-solid");
    expect(bar.className).not.toContain("bg-positive-solid");
  });

  it("merges custom className", () => {
    const { container } = render(<ProgressBar value={50} className="my-bar" />);
    const el = container.firstChild as HTMLElement;
    expect(el.className).toContain("my-bar");
  });
});
