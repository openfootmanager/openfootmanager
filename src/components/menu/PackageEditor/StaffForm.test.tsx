import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StaffForm } from "./StaffForm";
import { emptyStaff } from "./helpers";
import type { StaffDef } from "./types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Echo the key back so a test can name the string it expects without
    // depending on the English wording, which translators may change. Both
    // default forms i18next accepts are honoured, since the shared controls
    // this form renders use the bare-string one.
    t: (key: string, opts?: string | { defaultValue?: string }) =>
      typeof opts === "string" ? opts : opts?.defaultValue ?? key,
    i18n: { language: "en" },
  }),
}));

function renderForm(staff: Partial<StaffDef> = {}) {
  render(
    <StaffForm
      editing={{ ...emptyStaff(), ...staff }}
      editingIndex={0}
      isBusy={false}
      teams={[]}
      onBack={() => {}}
      onSave={() => {}}
      updateField={vi.fn()}
    />,
  );
}

describe("StaffForm", () => {
  it("explains that a club's manager comes from its assistant manager", () => {
    // There is no `manager` schema. A club's manager is built at world start
    // from its `AssistantManager` staff member (ofm_core `ai_hiring.rs`), and
    // nothing in the editor said so — authors reasonably read "Asst. Manager"
    // as the wrong slot and concluded managers could not be authored at all.
    renderForm({ role: "AssistantManager" });

    fireEvent.click(screen.getByRole("button", { name: "worldEditor.helpLabel" }));

    expect(screen.getByText("worldEditor.staffRoleHelp")).toBeInTheDocument();
  });

  it("captions the nationality picker once", () => {
    // CountryCombobox renders its own caption, so the form's extra <label>
    // printed "Nationality" twice above the same control.
    renderForm();

    expect(screen.getAllByText("worldEditor.staffNationality")).toHaveLength(1);
  });

  it("takes a date of birth through the same picker the player form uses", () => {
    // Staff were the last field in the app on a native <input type="date">,
    // which renders in the browser's locale, not the game's.
    renderForm({ dateOfBirth: "1978-05-27" });

    // By accessible name, not placeholder: the placeholder is "DD"/"YYYY"
    // decoration and would pass even with the spoken name missing.
    expect(screen.getByRole("textbox", { name: "date.dayLabel" })).toHaveValue("27");
    expect(screen.getByRole("textbox", { name: "date.yearLabel" })).toHaveValue("1978");
  });

  it("keeps the date of birth caption attached to the three controls under it", () => {
    // The native input it replaced was named by its <label htmlFor>. Three
    // controls cannot be, so the caption has to name the group instead —
    // otherwise a screen reader announces a day, a month and a year that
    // belong to nothing.
    renderForm({ dateOfBirth: "1978-05-27" });

    const field = screen.getByRole("group", { name: "worldEditor.staffDateOfBirth" });

    expect(field).toContainElement(screen.getByRole("textbox", { name: "date.dayLabel" }));
    expect(field).toContainElement(screen.getByRole("textbox", { name: "date.yearLabel" }));
  });
});
