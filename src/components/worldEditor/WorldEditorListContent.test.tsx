import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WorldEditorListContent } from "./WorldEditorListContent";
import { emptyPlayer } from "../menu/PackageEditor/helpers";
import type { PlayerDef } from "../menu/PackageEditor/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../hooks/useAssetDataUrl", () => ({
  useAssetDataUrl: () => null,
  evictAssetDataUrl: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: string | Record<string, string | number>) => {
      if (opts && typeof opts === "object") {
        const parts = Object.entries(opts)
          .filter(([name]) => name !== "defaultValue")
          .map(([name, value]) => `${name}=${value}`);
        return parts.length > 0 ? `${key} ${parts.join(" ")}` : key;
      }
      return typeof opts === "string" ? opts : key;
    },
    i18n: { language: "en" },
  }),
}));

function editor(editingIndex: number | null) {
  return {
    editingIndex,
    handleAdd: vi.fn(),
    handleSelect: vi.fn(),
    handleDelete: vi.fn(),
    handleDuplicate: vi.fn(),
  };
}

/** 60 youth players, so the list is capped and the 51st is past the edge. */
function youthSquad(): PlayerDef[] {
  return Array.from({ length: 60 }, (_, i) => ({
    ...emptyPlayer(),
    id: `y${i}`,
    name: `Youth ${i}`,
    youth: true,
  }));
}

function renderYouthSection(youthEditingIndex: number | null) {
  const youthEditor = editor(youthEditingIndex);
  render(
    <WorldEditorListContent
      selectedSection="youth"
      formPanel="youth"
      teams={[]}
      players={youthSquad()}
      staff={[]}
      confederations={[]}
      countries={[]}
      competitions={[]}
      names={{ version: 1, description: "", pools: {} }}
      teamEditor={editor(null)}
      playerEditor={editor(null)}
      youthEditor={youthEditor}
      staffEditor={editor(null)}
      confEditor={editor(null)}
      countryEditor={editor(null)}
      compEditor={editor(null)}
      namesEditor={{
        editingPoolKey: "",
        handleAddPool: vi.fn(),
        handleSelectPool: vi.fn(),
        handleDeletePool: vi.fn(),
      }}
    />,
  );
  return { youthEditor };
}

describe("WorldEditorListContent", () => {
  it("tells the youth list which row is selected", () => {
    // The youth editor opens its own form panel, but this list used to read
    // the selection only while the *player* panel was open — so the youth list
    // never knew what was selected. Duplicating the last visible youth player
    // selects the copy one row further down, and with the list capped the copy
    // would have been left off-screen with its form open.
    renderYouthSection(50);

    expect(screen.queryAllByRole("button", { name: "worldEditor.editPlayer" })).toHaveLength(51);
  });

  it("caps the youth list when nothing is selected", () => {
    renderYouthSection(null);

    expect(screen.queryAllByRole("button", { name: "worldEditor.editPlayer" })).toHaveLength(50);
  });
});
