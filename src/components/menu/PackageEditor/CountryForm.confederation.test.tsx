import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { CountryForm } from "./CountryForm";
import { emptyCountry } from "./helpers";
import { resetNationsCache } from "../../../services/nationsService";
import type { ConfederationDef, CountryDef } from "./types";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Region labels resolve through `teamSelect.regionLabels.<id>`; echo the id
    // back so a test can tell which region an option stands for.
    t: (key: string, opts?: { defaultValue?: string }) =>
      key.startsWith("teamSelect.regionLabels.")
        ? key.replace("teamSelect.regionLabels.", "")
        : (opts?.defaultValue ?? key),
    i18n: { language: "en" },
  }),
}));

/** Shaped as `get_nations` returns it: every nation carries its region. */
const CATALOG = [
  { code: "BR", name: "Brazil", region: "south-america" },
  { code: "ENG", name: "England", region: "europe" },
  { code: "NG", name: "Nigeria", region: "africa" },
  { code: "CR", name: "Costa Rica", region: "central-america" },
];

beforeEach(() => {
  resetNationsCache();
  invoke.mockReset();
  invoke.mockImplementation((cmd: string) =>
    cmd === "get_nations" ? Promise.resolve(CATALOG) : Promise.reject(new Error(cmd)),
  );
});

function renderForm(country: Partial<CountryDef>, confederations: ConfederationDef[] = []) {
  const updateField = vi.fn();
  render(
    <CountryForm
      editing={{ ...emptyCountry(), ...country }}
      editingIndex={0}
      confederations={confederations}
      isBusy={false}
      onBack={() => {}}
      onSave={() => {}}
      updateField={updateField}
    />,
  );
  return updateField;
}

async function openConfederations() {
  const control = await screen.findByRole("combobox", {
    name: /worldEditor\.countryConfederation/,
  });
  fireEvent.click(control);
}

describe("choosing a country's confederation", () => {
  it("offers the built-in regions, marked as built in", async () => {
    // Reported from real use: "it's not clear in the world editor what
    // confederation values are defaults, if we have any." They were not offered
    // at all — the field was free text unless the package defined its own.
    renderForm({ id: "BR", name: "Brazil" });

    await openConfederations();

    expect(
      screen.getByRole("group", { name: "worldEditor.confederationBuiltIn" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "south-america" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "africa" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "central-america" })).toBeInTheDocument();
  });

  it("keeps a package's own confederations separate from the built-in ones", async () => {
    // Which of these the author controls is the distinction that was missing:
    // editing a built-in is not the same act as editing your own.
    renderForm({ id: "BR", name: "Brazil" }, [{ id: "fictland", name: "Fictland Union" }]);

    await openConfederations();

    expect(
      screen.getByRole("group", { name: "worldEditor.confederationInPackage" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Fictland Union" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "europe" })).toBeInTheDocument();
  });

  it("shows a value the catalog does not know without discarding it", async () => {
    // The reported package used `uefa`, which is not a built-in id. Dropping it
    // from the list would silently blank the field on the next save — authored
    // data must be shown as written, never rewritten.
    const updateField = renderForm({ id: "ES", name: "Spain", confederation: "uefa" });

    await openConfederations();

    expect(screen.getByRole("option", { name: /uefa/ })).toBeInTheDocument();
    expect(updateField).not.toHaveBeenCalled();
  });

  it("reports the id, not the label, when a built-in region is chosen", async () => {
    const updateField = renderForm({ id: "BR", name: "Brazil" });

    await openConfederations();
    fireEvent.click(screen.getByRole("option", { name: "africa" }));

    expect(updateField).toHaveBeenCalledWith("confederation", "africa");
  });
});
