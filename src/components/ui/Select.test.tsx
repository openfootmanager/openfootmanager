import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Globe } from "lucide-react";
import { Select } from "./Select";

describe("Select", () => {
  it("renders the selected option label", () => {
    render(
      <Select value="en" aria-label="Language">
        <option value="en">English</option>
        <option value="pt">Português</option>
      </Select>,
    );

    expect(screen.getByRole("combobox", { name: "Language" })).toHaveTextContent(
      "English",
    );
  });

  it("opens the option list when clicked", () => {
    render(
      <Select defaultValue="en" aria-label="Language">
        <option value="en">English</option>
        <option value="pt">Português</option>
      </Select>,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "Language" }));

    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Português" })).toBeInTheDocument();
  });

  it("calls onChange and updates the displayed label when an option is chosen", () => {
    const onChange = vi.fn();

    render(
      <Select defaultValue="en" aria-label="Language" onChange={onChange}>
        <option value="en">English</option>
        <option value="pt">Português</option>
      </Select>,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "Language" }));
    fireEvent.click(screen.getByRole("option", { name: "Português" }));

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0].target.value).toBe("pt");
    expect(screen.getByRole("combobox", { name: "Language" })).toHaveTextContent(
      "Português",
    );
  });

  it("applies custom classes and renders the leading icon", () => {
    render(
      <Select
        defaultValue="en"
        aria-label="Language"
        icon={<Globe />}
        variant="subtle"
        className="my-select"
      >
        <option value="en">English</option>
      </Select>,
    );

    const combobox = screen.getByRole("combobox", { name: "Language" });
    expect(combobox.className).toContain("bg-gray-100");
    expect(combobox.className).toContain("my-select");
    expect(document.querySelector("svg")).toBeInTheDocument();
  });

  it("submits a hidden input value when a name is provided", () => {
    render(
      <Select defaultValue="pt" name="language" aria-label="Language">
        <option value="en">English</option>
        <option value="pt">Português</option>
      </Select>,
    );

    const hiddenInput = document.querySelector(
      'input[type="hidden"][name="language"]',
    ) as HTMLInputElement | null;

    expect(hiddenInput).not.toBeNull();
    expect(hiddenInput?.value).toBe("pt");
  });

  // Issue #282: the menu used to render inside the trigger's wrapper, so an
  // overflow-hidden ancestor (e.g. the tactics pitch) clipped it entirely for
  // slots near the container edge, and it was capped at the trigger's width.
  it("renders the open menu in a portal so overflow-hidden ancestors cannot clip it", () => {
    render(
      <div data-testid="clipping-ancestor" style={{ overflow: "hidden" }}>
        <Select defaultValue="en" aria-label="Language">
          <option value="en">English</option>
          <option value="pt">Português</option>
        </Select>
      </div>,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "Language" }));

    const listbox = screen.getByRole("listbox");
    expect(listbox).toBeInTheDocument();
    expect(screen.getByTestId("clipping-ancestor")).not.toContainElement(
      listbox,
    );
  });

  it("closes the portaled menu on an outside pointer press but not on a menu press", () => {
    render(
      <Select defaultValue="en" aria-label="Language">
        <option value="en">English</option>
        <option value="pt">Português</option>
      </Select>,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "Language" }));
    // A press inside the menu must not dismiss it before the click lands.
    fireEvent.mouseDown(screen.getByRole("option", { name: "Português" }));
    expect(screen.getByRole("listbox")).toBeInTheDocument();

    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });
});
