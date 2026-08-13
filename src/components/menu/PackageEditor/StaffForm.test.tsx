import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StaffForm } from "./StaffForm";
import { emptyStaff } from "./helpers";
import type { StaffDef } from "./types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Echo the key back so a test can name the string it expects without
    // depending on the English wording, which translators may change.
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: "en" },
  }),
}));

function renderForm(staff: Partial<StaffDef> = {}) {
  const updateField = vi.fn();
  render(
    <StaffForm
      editing={{ ...emptyStaff(), ...staff }}
      editingIndex={0}
      isBusy={false}
      teams={[]}
      onBack={() => {}}
      onSave={() => {}}
      updateField={updateField}
    />,
  );
  return updateField;
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
});
