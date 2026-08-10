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
 * Open the nation picker and choose the entry named `name`.
 *
 * The exact match is load-bearing: each option renders a flag beside its name,
 * and while that flag labelled itself an option read out as "EnglandEngland".
 * It is decoration next to the name it duplicates, so it is now hidden from the
 * accessible name and this matches what a screen reader would announce.
 */
async function pickNation(name: string) {
  const trigger = await screen.findByRole("button", { name: /worldEditor\.countryNation/ });
  fireEvent.mouseDown(trigger);
  fireEvent.mouseDown(await screen.findByRole("option", { name }));
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

  it("opens a real listbox, since that is what the trigger promises", async () => {
    // The trigger carries `aria-haspopup="listbox"`. The popup was a plain div
    // of buttons, so a screen reader was told to expect options and handed
    // controls with no position, no count and no selected state — a promise the
    // markup did not keep.
    renderForm({ id: "ENG", name: "England" });

    const trigger = await screen.findByRole("button", { name: /worldEditor\.countryNation/ });
    fireEvent.mouseDown(trigger);

    const listbox = await screen.findByRole("listbox");
    expect(trigger).toHaveAttribute("aria-controls", listbox.id);
    expect(await screen.findByRole("option", { name: "England" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("option", { name: "Brazil" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
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

  it("does not offer the picker for a new country until the catalog has loaded", async () => {
    // `CountryCombobox` loads its own copy of the name data, so it can render a
    // list of countries before `useNations` has resolved. Picking one then finds
    // no catalog entry and stores the *code* as the name — the exact thing the
    // canonical-name rule exists to prevent.
    let release!: (nations: typeof CATALOG) => void;
    invoke.mockImplementation((cmd: string) =>
      cmd === "get_nations"
        ? new Promise((resolve) => {
            release = resolve;
          })
        : Promise.reject(new Error(cmd)),
    );
    renderForm({});

    expect(
      screen.queryByRole("button", { name: /worldEditor\.countryNation/ }),
    ).not.toBeInTheDocument();

    release(CATALOG);
    expect(
      await screen.findByRole("button", { name: /worldEditor\.countryNation/ }),
    ).toBeInTheDocument();
  });

  it("authors a new country by hand when the catalog cannot be read", async () => {
    // With no catalog there is nothing to call built-in, and the picker would
    // fall back to offering every ISO country — a list this package format does
    // not accept. Worse, adopting one would store its code as its name.
    invoke.mockRejectedValue(new Error("offline"));
    renderForm({});

    expect(await screen.findByLabelText("worldEditor.countryId")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /worldEditor\.countryNation/ }),
    ).not.toBeInTheDocument();
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
