import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { DatePicker } from "./DatePicker";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
    i18n: { language: "en" },
  }),
}));

afterEach(() => {
  vi.useRealTimers();
});

// By accessible name rather than placeholder: the placeholder is "DD"/"YYYY"
// decoration, and querying it would pass even if the spoken name were missing.
const dayInput = () => screen.getByRole("textbox", { name: "date.dayLabel" });
const yearInput = () => screen.getByRole("textbox", { name: "date.yearLabel" });

describe("DatePicker", () => {
  it("renders the initial ISO date across the day, month, and year fields", () => {
    render(<DatePicker value="1999-02-03" onChange={vi.fn()} />);

    expect(dayInput()).toHaveValue("03");
    expect(yearInput()).toHaveValue("1999");
    expect(screen.getByRole("button", { name: "February" })).toBeInTheDocument();
  });

  it("emits a padded ISO date once all fields are complete", async () => {
    const onChange = vi.fn();
    render(<DatePicker value="" onChange={onChange} />);

    fireEvent.change(dayInput(), { target: { value: "7" } });
    fireEvent.blur(dayInput());

    fireEvent.click(screen.getAllByRole("button")[0]);
    fireEvent.click(screen.getByRole("button", { name: "March" }));

    fireEvent.change(yearInput(), { target: { value: "2024" } });

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith("2024-03-07");
    });
  });

  it("clamps the selected day when changing to a shorter month", async () => {
    const onChange = vi.fn();
    render(<DatePicker value="2024-01-31" onChange={onChange} />);

    onChange.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "January" }));
    fireEvent.click(screen.getByRole("button", { name: "February" }));

    await waitFor(() => {
      expect(dayInput()).toHaveValue("29");
      expect(onChange).toHaveBeenLastCalledWith("2024-02-29");
    });
  });

  it("revalidates leap-day selections when the year changes", async () => {
    const onChange = vi.fn();
    render(<DatePicker value="2024-02-29" onChange={onChange} />);

    onChange.mockClear();
    fireEvent.change(yearInput(), { target: { value: "2023" } });

    await waitFor(() => {
      expect(dayInput()).toHaveValue("28");
      expect(onChange).toHaveBeenLastCalledWith("2023-02-28");
    });
  });

  it("normalizes two-digit years on blur using the current century", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2025-01-15T12:00:00Z"));

    const onChange = vi.fn();
    render(<DatePicker value="" onChange={onChange} />);

    fireEvent.change(dayInput(), { target: { value: "1" } });
    fireEvent.blur(dayInput());

    fireEvent.click(screen.getAllByRole("button")[0]);
    fireEvent.click(screen.getByRole("button", { name: "January" }));

    const year = yearInput();
    fireEvent.change(year, { target: { value: "26" } });
    fireEvent.blur(year);

    expect(year).toHaveValue("1926");
    expect(onChange).toHaveBeenLastCalledWith("1926-01-01");
  });

  it("closes the month dropdown on outside clicks", () => {
    render(<DatePicker value="" onChange={vi.fn()} />);

    fireEvent.click(screen.getAllByRole("button")[0]);
    expect(screen.getByRole("button", { name: "January" })).toBeInTheDocument();

    fireEvent.mouseDown(document.body);

    expect(screen.queryByRole("button", { name: "January" })).not.toBeInTheDocument();
  });

  it("stays silent when it mounts with a date already set", () => {
    // The parts used to start blank while `value` already held a date, so the
    // first commit looked exactly like a user clearing the field. Feeding the
    // result back — which is what the editor forms do — then alternated
    // between clearing and restoring the date forever.
    const emitted: string[] = [];

    function ControlledHost() {
      const [value, setValue] = useState("1999-02-03");
      return (
        <DatePicker
          value={value}
          onChange={(v) => {
            emitted.push(v);
            // Stop feeding back once it is clearly not settling. The cycle is
            // synchronous inside `act()`, so an uncapped host would hang the
            // worker rather than fail — and a test that hangs reports nothing.
            if (emitted.length < 8) {
              setValue(v);
            }
          }}
        />
      );
    }

    render(<ControlledHost />);

    expect(emitted).toEqual([]);
  });

  it("still reports a date that needed normalising", () => {
    // The guard above must not swallow the picker's own corrections: an
    // unpadded value differs from its normalised form and is still news.
    const onChange = vi.fn();

    render(<DatePicker value="1999-2-3" onChange={onChange} />);

    expect(onChange).toHaveBeenCalledWith("1999-02-03");
  });

  it("stays silent when it mounts with no date", () => {
    // A pristine field starts with all three parts blank. Reporting that as a
    // change would mark an untouched form dirty the moment it opens.
    const onChange = vi.fn();

    render(<DatePicker value="" onChange={onChange} />);

    expect(onChange).not.toHaveBeenCalled();
  });

  it("stays silent while only part of the date has been cleared", () => {
    const onChange = vi.fn();
    render(<DatePicker value="2024-03-07" onChange={onChange} />);
    onChange.mockClear();

    fireEvent.change(dayInput(), { target: { value: "" } });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("reports an empty date once every field has been cleared", async () => {
    // Without this a date could be set but never removed, so an author who
    // typed the wrong birthday was stuck with one.
    const onChange = vi.fn();
    render(<DatePicker value="2024-03-07" onChange={onChange} />);
    onChange.mockClear();

    fireEvent.change(dayInput(), { target: { value: "" } });
    fireEvent.change(yearInput(), { target: { value: "" } });

    fireEvent.click(screen.getByRole("button", { name: "March" }));
    // The mock echoes an untranslated key back, so this is the placeholder row.
    fireEvent.click(screen.getByRole("button", { name: "date.noMonth" }));

    await waitFor(() => {
      expect(onChange).toHaveBeenLastCalledWith("");
    });
  });
});
