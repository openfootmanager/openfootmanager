import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PlayersTab } from "./PlayersTab";
import { emptyPlayer, emptyTeam } from "./helpers";
import type { PlayerDef, Position, TeamDef } from "./types";

const useAssetDataUrl = vi.fn(() => null);

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../hooks/useAssetDataUrl", () => ({
  useAssetDataUrl: (...args: unknown[]) => useAssetDataUrl(...(args as [])),
  evictAssetDataUrl: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Echo the key back, interpolating counts so a test can assert on the
    // numbers without depending on the English wording.
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

function players(count: number, overrides: (i: number) => Partial<PlayerDef> = () => ({})): PlayerDef[] {
  return Array.from({ length: count }, (_, i) => ({
    ...emptyPlayer(),
    id: `p${i}`,
    name: `Player ${i}`,
    ...overrides(i),
  }));
}

function renderTab(props: Partial<React.ComponentProps<typeof PlayersTab>> = {}) {
  const onEdit = vi.fn();
  render(
    <PlayersTab
      players={players(500)}
      onAdd={() => {}}
      onEdit={onEdit}
      onDelete={() => {}}
      {...props}
    />,
  );
  return { onEdit };
}

function editButtons() {
  return screen.queryAllByRole("button", { name: "worldEditor.editPlayer" });
}

/**
 * Renders 500 players and hands back a way to move the selection, which is how
 * duplicating the last visible row over and over looks from the tab's side.
 */
function renderTabForRerender() {
  const list = players(500);
  const props = {
    players: list,
    onAdd: () => {},
    onEdit: vi.fn(),
    onDelete: () => {},
  };
  const view = render(<PlayersTab {...props} selectedIndex={null} />);
  return {
    rerender: (selectedIndex: number) =>
      view.rerender(<PlayersTab {...props} selectedIndex={selectedIndex} />),
  };
}

describe("PlayersTab", () => {
  it("renders one page of rows rather than every player", () => {
    // A package with a full pyramid holds tens of thousands of players, and
    // rendering all of them is what made opening the list slow.
    renderTab();

    expect(editButtons()).toHaveLength(50);
  });

  it("says how much of the list it is showing", () => {
    renderTab();

    expect(screen.getByText("worldEditor.showingEntries shown=50 total=500")).toBeInTheDocument();
  });

  it("reveals another page on request", () => {
    renderTab();

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));

    expect(editButtons()).toHaveLength(100);
  });

  it("offers nothing more to load once the whole list fits", () => {
    renderTab({ players: players(3) });

    expect(screen.queryByRole("button", { name: "common.loadMore" })).not.toBeInTheDocument();
    expect(screen.getByText("common.nResults count=3")).toBeInTheDocument();
  });

  it("still addresses a revealed row by its index in the whole list", () => {
    // Edit, delete and duplicate all take the unfiltered index, so a row that
    // only became visible after a reveal must not be renumbered.
    const { onEdit } = renderTab();

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));
    fireEvent.click(editButtons()[50]);

    expect(onEdit).toHaveBeenCalledWith(50);
  });

  it("addresses a searched row by its index in the whole list", () => {
    const { onEdit } = renderTab({ players: players(500) });

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchPlayers" }), {
      target: { value: "Player 123" },
    });
    fireEvent.click(editButtons()[0]);

    expect(onEdit).toHaveBeenCalledWith(123);
  });

  it("goes back to one page when the search changes", () => {
    renderTab();

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));
    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchPlayers" }), {
      target: { value: "Player" },
    });

    expect(editButtons()).toHaveLength(50);
  });

  it("says so when a search matches nothing, instead of going blank", () => {
    renderTab();

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchPlayers" }), {
      target: { value: "nobody by that name" },
    });

    expect(editButtons()).toHaveLength(0);
    expect(screen.getByText("common.noResults")).toBeInTheDocument();
  });

  it("finds a player by the club name shown in the row", () => {
    const teams: TeamDef[] = [{ ...emptyTeam(), id: "nsfc", name: "Northshire FC" }];
    const { onEdit } = renderTab({
      players: players(3, (i) => (i === 1 ? { club: "nsfc" } : {})),
      teams,
    });

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchPlayers" }), {
      target: { value: "northshire" },
    });
    fireEvent.click(editButtons()[0]);

    expect(onEdit).toHaveBeenCalledWith(1);
  });

  it("reads at most one page of player photos on mount", () => {
    useAssetDataUrl.mockClear();
    renderTab({ players: players(500, () => ({ photo: "photos/a.png" })), projectDir: "/pkg" });

    expect(useAssetDataUrl.mock.calls.length).toBeLessThanOrEqual(50);
  });

  it("keeps the youth and senior lists apart", () => {
    renderTab({
      players: players(4, (i) => ({ youth: i < 3 })),
      youthOnly: true,
    });

    expect(editButtons()).toHaveLength(3);
    expect(screen.getByText("common.nResults count=3")).toBeInTheDocument();
  });

  it("keeps the load-more button in place once it has nothing left to reveal", () => {
    // Removing it on the last press would unmount the focused control and
    // drop focus to the top of the document, hundreds of rows above.
    renderTab({ players: players(60) });

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));

    expect(editButtons()).toHaveLength(60);
    // aria-disabled rather than disabled, so it keeps focus and its place in
    // the tab order instead of handing focus back to the document.
    expect(screen.getByRole("button", { name: "common.loadMore" }))
      .toHaveAttribute("aria-disabled", "true");
  });

  it("does nothing when the exhausted load-more button is pressed again", () => {
    renderTab({ players: players(60) });

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));
    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));

    expect(editButtons()).toHaveLength(60);
  });

  it("keeps rows revealed once a stretch has shown them", () => {
    // The stretch is recomputed from visibleCount every render, so on its own
    // it does not accumulate: walk the selection down one row at a time and it
    // eventually reaches visibleCount + pageSize, the stretch is refused, and
    // the list snaps back from 100 rows to 50 — hiding rows that were on
    // screen, along with the copy the user just made.
    const { rerender } = renderTabForRerender();

    // Row 99 is inside the stretch window, so the list grows to 100 rows.
    rerender(99);
    expect(editButtons()).toHaveLength(100);

    // Row 100 is one past that — inside the window only if the previous
    // stretch stuck. Measured from the original 50 it is exactly one page
    // away, which the bound rejects, and the list collapses.
    rerender(100);
    expect(editButtons()).toHaveLength(101);
  });

  it("keeps a duplicated row visible when it lands just past the page edge", () => {
    // handleDuplicate selects index + 1, so duplicating the last visible row
    // would otherwise open a form for a record the list cannot show.
    renderTab({ selectedIndex: 50 });

    expect(editButtons()).toHaveLength(51);
  });
});

describe("PlayersTab position column", () => {
  it("shows the position of each rendered player", () => {
    renderTab({ players: players(2, () => ({ position: "Striker" as Position })) });

    expect(screen.getAllByText(/common\.positions\.Striker/)).toHaveLength(2);
  });
});

describe("PlayersTab position filter", () => {
  // Select is a portal-backed combobox, not a native <select>, so an option is
  // reached by opening the list and clicking it.
  //
  // The trigger is named `aria-labelledby="<caption> <self>"`, so a browser
  // reads it as "Filter by position, All positions". jsdom's accessible-name
  // computation drops the self-reference and returns the caption alone, which
  // is why the exact-string query below works — if a library upgrade starts
  // honouring it, this query needs a substring matcher, not a fix to the app.
  function choosePosition(optionName: string) {
    fireEvent.click(screen.getByRole("combobox", { name: "worldEditor.filterByPosition" }));
    fireEvent.click(screen.getByRole("option", { name: optionName }));
  }

  const mixedSquad = () => [
    { ...emptyPlayer(), id: "gk", name: "Ana", position: "Goalkeeper" as Position },
    { ...emptyPlayer(), id: "cb", name: "Bo", position: "CenterBack" as Position },
    { ...emptyPlayer(), id: "lb", name: "Cyd", position: "LeftBack" as Position },
    { ...emptyPlayer(), id: "st", name: "Dee", position: "Striker" as Position },
  ];

  it("narrows the list to one exact position", () => {
    // The reported use: how many goalkeepers does this package have?
    renderTab({ players: mixedSquad() });

    choosePosition("common.positions.Goalkeeper");

    expect(editButtons()).toHaveLength(1);
    expect(screen.getByText("common.nResults count=1")).toBeInTheDocument();
  });

  it("narrows the list to a whole group", () => {
    renderTab({ players: mixedSquad() });

    choosePosition("worldEditor.positionGroupFilter.Defender");

    expect(editButtons()).toHaveLength(2);
  });

  it("still addresses a filtered row by its index in the whole list", () => {
    const { onEdit } = renderTab({ players: mixedSquad() });

    choosePosition("common.positions.Striker");
    fireEvent.click(editButtons()[0]);

    expect(onEdit).toHaveBeenCalledWith(3);
  });

  it("applies the position and the search together", () => {
    renderTab({ players: mixedSquad() });

    choosePosition("worldEditor.positionGroupFilter.Defender");
    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchPlayers" }), {
      target: { value: "cyd" },
    });

    expect(editButtons()).toHaveLength(1);
  });

  it("goes back to one page when the position changes", () => {
    renderTab({ players: players(500, () => ({ position: "Striker" as Position })) });

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));
    choosePosition("common.positions.Striker");

    expect(editButtons()).toHaveLength(50);
  });

  it("goes back to the whole list when the filter is cleared", () => {
    renderTab({ players: mixedSquad() });

    choosePosition("common.positions.Goalkeeper");
    choosePosition("worldEditor.allPositions");

    expect(editButtons()).toHaveLength(4);
  });
});
