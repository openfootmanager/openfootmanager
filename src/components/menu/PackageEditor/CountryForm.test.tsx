import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { CountryForm } from "./CountryForm";
import { emptyCountry } from "./helpers";
import { resetNationsCache } from "../../../services/nationsService";
import type { CountryDef } from "./types";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

// Set per test — the language a country is authored in must not change what
// gets written to the package.
let language = "en";
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language } }),
}));

/** A slice of the backend catalog, shaped exactly as `get_nations` returns it. */
const CATALOG = [
  { code: "BR", name: "Brazil", region: "south-america" },
  { code: "ENG", name: "England", region: "europe" },
  { code: "AR", name: "Argentina", region: "south-america" },
];

beforeEach(() => {
  language = "en";
  resetNationsCache();
  invoke.mockReset();
  invoke.mockImplementation((cmd: string) =>
    cmd === "get_nations" ? Promise.resolve(CATALOG) : Promise.reject(new Error(cmd)),
  );
});

function renderForm(country: Partial<CountryDef>, updateField = vi.fn()) {
  const editing: CountryDef = { ...emptyCountry(), ...country };
  render(
    <CountryForm
      editing={editing}
      editingIndex={country.id ? 0 : null}
      confederations={[]}
      isBusy={false}
      onBack={() => {}}
      onSave={() => {}}
      updateField={updateField}
    />,
  );
  return updateField;
}

/**
 * Open the nation picker and choose the entry labelled `name`.
 *
 * Each option renders its flag beside the name, and the flag carries a label of
 * its own, so an option's accessible name comes out as "EnglandEngland" — hence
 * the prefix match rather than an exact one.
 */
async function pickNation(name: string) {
  const trigger = await screen.findByRole("button", { name: /worldEditor\.countryNation/ });
  fireEvent.mouseDown(trigger);
  const option = await screen.findByRole("button", { name: new RegExp(`^${name}`) });
  fireEvent.mouseDown(option);
}

describe("CountryForm nation picker", () => {
  it("writes the catalog's canonical name, not the one the author sees", async () => {
    // The trap: `countryName()` is locale-aware, so filling `name` from the
    // rendered label would persist "Brasil" into a package authored in pt-BR
    // and "Brazil" into the same package authored in English. The stored name
    // has to be the catalog's, so the id and name agree across languages.
    language = "pt-BR";
    const updateField = renderForm({});

    await pickNation("Brasil");

    expect(updateField).toHaveBeenCalledWith("id", "BR");
    expect(updateField).toHaveBeenCalledWith("name", "Brazil");
  });

  it("fills both fields from one choice", async () => {
    const updateField = renderForm({});

    await pickNation("England");

    expect(updateField).toHaveBeenCalledWith("id", "ENG");
    expect(updateField).toHaveBeenCalledWith("name", "England");
  });

  it("offers no free-text id while a built-in nation is being picked", async () => {
    // Typing an arbitrary id is what made countries untranslatable: a package
    // saying `br`/`BRA`/`brasil` cannot be matched to a flag or a name.
    renderForm({});

    await screen.findByRole("button", { name: /worldEditor\.countryNation/ });
    expect(screen.queryByLabelText("worldEditor.countryId")).not.toBeInTheDocument();
  });

  it("opens a country the catalog does not know as authored, and rewrites nothing", async () => {
    // A historical or invented country is legitimate. Showing it in the picker
    // would either blank it or silently snap it to a neighbour; both lose data.
    const updateField = renderForm({ id: "YUG", name: "Yugoslavia", confederation: "europe" });

    expect(await screen.findByLabelText("worldEditor.countryId")).toHaveValue("YUG");
    expect(screen.getByLabelText("worldEditor.countryName")).toHaveValue("Yugoslavia");
    expect(updateField).not.toHaveBeenCalled();
  });

  it("opens a country the catalog does know in the picker", async () => {
    renderForm({ id: "BR", name: "Brazil", confederation: "south-america" });

    // The trigger renders the code until the name resources resolve.
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /worldEditor\.countryNation/ }),
      ).toHaveTextContent("Brazil"),
    );
    expect(screen.queryByLabelText("worldEditor.countryId")).not.toBeInTheDocument();
  });

  it("hands back the free-text fields through the custom-country escape hatch", async () => {
    renderForm({});

    fireEvent.click(await screen.findByRole("button", { name: "worldEditor.countryCustom" }));

    expect(screen.getByLabelText("worldEditor.countryId")).toBeInTheDocument();
    expect(screen.getByLabelText("worldEditor.countryName")).toBeInTheDocument();
  });

  it("falls back to authoring by hand when the catalog cannot be read", async () => {
    // Without the catalog nothing can be called built-in. Offering an empty
    // picker would strand the author, so the free-text pair stands in.
    invoke.mockRejectedValue(new Error("offline"));
    renderForm({ id: "BR", name: "Brazil", confederation: "south-america" });

    await waitFor(() =>
      expect(screen.getByLabelText("worldEditor.countryId")).toHaveValue("BR"),
    );
  });
});
