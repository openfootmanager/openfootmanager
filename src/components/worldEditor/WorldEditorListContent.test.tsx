import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installScrollGeometry } from "../../test-utils/scrollGeometry";

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

/** 60 youth players: more than a window holds, so the 51st is far below the first screenful. */
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
    <div data-scroll-container="">
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
      />
    </div>,
  );
  return { youthEditor };
}

// jsdom lays nothing out: the fixture gives the scroller a viewport, so a row 50 deep is outside the
// first window unless the list scrolls to it.
let restoreGeometry: () => void;
beforeEach(() => {
  restoreGeometry = installScrollGeometry({ viewportHeight: 640, listOffset: 120 });
});
afterEach(() => restoreGeometry());

describe("WorldEditorListContent", () => {
  /**
   * Given 60 youth players and the youth form open on the 51st
   * When the list is shown
   * Then the list scrolls to the selected row and renders it: the youth list reads the selection while
   *      the *youth* panel is open (it used to read it only for the player panel, so the copy a
   *      duplicate makes further down was left off-screen with its form open)
   */
  it("tells the youth list which row is selected", () => {
    renderYouthSection(50);

    expect(screen.getByText("Youth 50")).toBeInTheDocument();
    expect(screen.queryByText("Youth 0")).not.toBeInTheDocument();
  });

  /**
   * Given 60 youth players and nothing selected
   * When the list is shown
   * Then it starts at the first row and the 51st is not rendered
   */
  it("starts at the first row when nothing is selected", () => {
    renderYouthSection(null);

    expect(screen.getByText("Youth 0")).toBeInTheDocument();
    expect(screen.queryByText("Youth 50")).not.toBeInTheDocument();
  });
});
