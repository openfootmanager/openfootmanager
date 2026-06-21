import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Checkbox } from "./Checkbox";

describe("Checkbox", () => {
  it("renders unchecked with no checkmark icon", () => {
    render(
      <Checkbox checked={false} onChange={vi.fn()} aria-label="Select item" />,
    );

    expect(screen.getByRole("checkbox", { name: "Select item" })).not.toBeChecked();
    expect(document.querySelector("svg")).not.toBeInTheDocument();
  });

  it("renders checked with a checkmark icon", () => {
    render(
      <Checkbox checked={true} onChange={vi.fn()} aria-label="Select item" />,
    );

    expect(screen.getByRole("checkbox", { name: "Select item" })).toBeChecked();
    expect(document.querySelector("svg")).toBeInTheDocument();
  });

  it("calls onChange when clicked", () => {
    const onChange = vi.fn();

    render(
      <Checkbox checked={false} onChange={onChange} aria-label="Select item" />,
    );

    fireEvent.click(screen.getByRole("checkbox", { name: "Select item" }));

    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("is disabled when the disabled prop is set", () => {
    render(
      <Checkbox
        checked={false}
        onChange={vi.fn()}
        disabled
        aria-label="Select item"
      />,
    );

    expect(screen.getByRole("checkbox", { name: "Select item" })).toBeDisabled();
  });

  it("forwards data-testid to the hidden input", () => {
    render(
      <Checkbox
        checked={false}
        onChange={vi.fn()}
        aria-label="Select item"
        data-testid="my-checkbox"
      />,
    );

    expect(screen.getByTestId("my-checkbox")).toBeInTheDocument();
  });
});
