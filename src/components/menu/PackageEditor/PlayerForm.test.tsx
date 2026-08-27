import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PlayerForm } from "./PlayerForm";
import { emptyPlayer, emptyAttributes } from "./helpers";
import type { PlayerDef } from "./types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Echo the key back so a test can name the string it expects without
    // depending on the English wording, which translators may change.
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: "en" },
  }),
}));

function renderForm(player: Partial<PlayerDef> = {}) {
  const updateField = vi.fn();
  render(
    <PlayerForm
      editing={{ ...emptyPlayer(), ...player }}
      editingIndex={0}
      isBusy={false}
      teams={[]}
      onBack={() => {}}
      onSave={() => {}}
      updateField={updateField}
      onAssetError={() => {}}
    />,
  );
  return { updateField };
}

function potentialInput() {
  return screen.getByRole("spinbutton", { name: "worldEditor.playerPotential" });
}

describe("PlayerForm potential", () => {
  it("shows an authored ceiling", () => {
    renderForm({ overall: 55, potential: 92 });

    expect(potentialInput()).toHaveValue(92);
  });

  it("stays available when ability is given as attributes", () => {
    // A ceiling is orthogonal to how current ability was expressed, so gating it
    // on the overall/attributes toggle would hide it from exactly the authors
    // exercising the most precise control.
    renderForm({ overall: null, potential: 88, attributes: emptyAttributes() });

    expect(potentialInput()).toHaveValue(88);
  });

  it("hands a blank field back to the engine's roll", () => {
    const { updateField } = renderForm({ overall: 55, potential: 92 });

    fireEvent.change(potentialInput(), { target: { value: "" } });

    expect(updateField).toHaveBeenCalledWith("potential", null);
  });

  it("keeps a typed ceiling as written rather than raising it to ability", () => {
    // Below-ability is a real authoring mistake, but the package validator is
    // what reports it. Repairing it here would hide the error the author needs.
    const { updateField } = renderForm({ overall: 70 });

    fireEvent.change(potentialInput(), { target: { value: "60" } });

    expect(updateField).toHaveBeenCalledWith("potential", 60);
  });

  it("clamps to the legal range", () => {
    const { updateField } = renderForm({ overall: 70 });

    fireEvent.change(potentialInput(), { target: { value: "140" } });

    expect(updateField).toHaveBeenCalledWith("potential", 99);
  });

  it("clamps scientific notation rather than truncating it", () => {
    // `type="number"` accepts `1e2` as a valid value, but parseInt stops at the
    // `e` and yields 1 — so a user asking for 100 silently got the opposite end
    // of the range.
    const { updateField } = renderForm({ overall: 70 });

    fireEvent.change(potentialInput(), { target: { value: "1e2" } });

    expect(updateField).toHaveBeenCalledWith("potential", 99);
  });

  it("ignores a value that is not a number at all", () => {
    const { updateField } = renderForm({ overall: 70, potential: 80 });

    fireEvent.change(potentialInput(), { target: { value: "abc" } });

    // Null, not 1: an unparseable entry is an absent ceiling, not the worst
    // possible one.
    expect(updateField).toHaveBeenCalledWith("potential", null);
  });
});
