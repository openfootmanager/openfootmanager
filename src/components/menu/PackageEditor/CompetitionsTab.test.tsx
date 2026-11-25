import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installScrollGeometry } from "../../../test-utils/scrollGeometry";

import { CompetitionsTab } from "./CompetitionsTab";
import { emptyCompetition, entityRowKey } from "./helpers";
import type { CompetitionDef } from "./types";

// jsdom lays nothing out: the fixture gives the scroller a viewport and puts the list below a toolbar,
// moving the list's top with the scroll as a browser does, so a scroll test can fail.
const LIST_OFFSET = 120;
const ROW_STEP = 72;
let restoreGeometry: () => void;
beforeEach(() => {
  restoreGeometry = installScrollGeometry({ viewportHeight: 640, listOffset: LIST_OFFSET });
});
afterEach(() => restoreGeometry());

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../hooks/useAssetDataUrl", () => ({
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

function competitions(count: number): CompetitionDef[] {
  return Array.from({ length: count }, (_, i) => ({
    ...emptyCompetition(),
    id: `c${i}`,
    name: `Competition ${i}`,
  }));
}

function tab(
  rows: CompetitionDef[],
  extra: Partial<React.ComponentProps<typeof CompetitionsTab>> = {},
) {
  return (
    <div role="region" aria-label="Competitions scroller" data-scroll-container="">
      <CompetitionsTab
        competitions={rows}
        onAdd={() => {}}
        onEdit={() => {}}
        onDelete={() => {}}
        {...extra}
      />
    </div>
  );
}

const scroller = () => screen.getByRole("region", { name: "Competitions scroller" });
const editButtons = () => screen.queryAllByRole("button", { name: "worldEditor.editCompetition" });

describe("CompetitionsTab", () => {
  /**
   * Given 500 competitions (more than a window)
   * When the tab is shown
   * Then a window of rows is in the page, not all 500, and the list says how long it is
   */
  it("renders a window of rows rather than every competition", () => {
    render(tab(competitions(500)));

    expect(editButtons().length).toBeGreaterThan(5);
    expect(editButtons().length).toBeLessThan(40);
    expect(
      screen.getByRole("list", { name: "worldEditor.sectionCompetitions" }),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")[0]).toHaveAttribute("aria-setsize", "500");
  });

  /**
   * Given a long list and the selection far below the first window (a record opened from elsewhere)
   * When the selection is set
   * Then the list scrolls to the selected row and renders it, and the rows at the top are gone
   */
  it("scrolls a distant selection into view", async () => {
    const rows = competitions(500);
    const view = render(tab(rows, { selectedIndex: null }));
    expect(screen.queryByText("Competition 300")).not.toBeInTheDocument();

    view.rerender(tab(rows, { selectedIndex: 300 }));

    await waitFor(() => expect(screen.getByText("Competition 300")).toBeInTheDocument());
    expect(screen.queryByText("Competition 0")).not.toBeInTheDocument();
  });

  /**
   * Given the list scrolled to a row far down
   * When that row's edit button is clicked
   * Then the callback gets the row's index in the whole list, not its place in the window
   */
  it("addresses a row it scrolled to by its index in the whole list", async () => {
    const onEdit = vi.fn();
    render(tab(competitions(500), { onEdit }));

    scroller().scrollTop = LIST_OFFSET + 300 * ROW_STEP;
    fireEvent.scroll(scroller());
    await waitFor(() => expect(screen.getByText("Competition 300")).toBeInTheDocument());
    const row = screen.getByText("Competition 300").closest('[role="listitem"]') as HTMLElement;
    fireEvent.click(within(row).getByRole("button", { name: "worldEditor.editCompetition" }));

    expect(onEdit).toHaveBeenCalledWith(300);
  });

  /**
   * Given a competition with no id yet (blank until it is named) and an imported one whose id is
   *   exactly the key that blank record is given
   * When the list is rendered and then updated (the rows swap places, as an undo or a sort would)
   * Then React sees two distinct keys (no "same key" warning), and each row keeps its own element:
   *   one React and focus identity per row, not two rows collapsed into one
   */
  it("keeps a blank-id row and an imported id equal to its generated key apart", () => {
    const blank = { ...emptyCompetition(), id: "", name: "Unnamed" };
    const imported = { ...emptyCompetition(), id: entityRowKey(blank), name: "Imported" };
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});

    const view = render(tab([blank, imported]));
    const unnamedBefore = screen.getByText("Unnamed").closest('[role="listitem"]');
    const importedBefore = screen.getByText("Imported").closest('[role="listitem"]');
    view.rerender(tab([imported, blank]));

    expect(errors.mock.calls.flat().join(" ")).not.toMatch(/same key/);
    expect(screen.getByText("Unnamed").closest('[role="listitem"]')).toBe(unnamedBefore);
    expect(screen.getByText("Imported").closest('[role="listitem"]')).toBe(importedBefore);
    errors.mockRestore();
  });
});
