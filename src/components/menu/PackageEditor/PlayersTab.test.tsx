import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installScrollGeometry } from "../../../test-utils/scrollGeometry";

import { PlayersTab } from "./PlayersTab";
import { emptyPlayer, emptyTeam } from "./helpers";
import type { PlayerDef, Position, TeamDef } from "./types";

// jsdom lays nothing out: the fixture gives the scroller a viewport and puts the list below a toolbar,
// moving the list's top with the scroll as a browser does, so a scroll test can fail.
const LIST_OFFSET = 120;
let restoreGeometry: () => void;
beforeEach(() => {
  restoreGeometry = installScrollGeometry({ viewportHeight: 640, listOffset: LIST_OFFSET });
});
afterEach(() => restoreGeometry());

const useAssetDataUrl = vi.fn(() => null);

vi.mock("../../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../../test-utils/positionTable");
  return cannedPositionsModule();
});

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

function players(
  count: number,
  overrides: (i: number) => Partial<PlayerDef> = () => ({}),
): PlayerDef[] {
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
    <div data-testid="scroller" data-scroll-container="">
      <PlayersTab
        players={players(500)}
        onAdd={() => {}}
        onEdit={onEdit}
        onDelete={() => {}}
        {...props}
      />
    </div>,
  );
  return { onEdit };
}

// The list is a window: a row far down is rendered only once the scroller has been moved to it
// (jsdom has no layout, so every row is taken to be 72 px: a 64 px row and its gap).
const ROW_STEP = 72;

async function scrollToPlayer(row: number) {
  const scroller = screen.getByTestId("scroller");
  scroller.scrollTop = LIST_OFFSET + row * ROW_STEP;
  fireEvent.scroll(scroller);
  await waitFor(() => expect(screen.getByText(`Player ${row}`)).toBeInTheDocument());
}

/** The edit button of the row that shows `text`. */
function editButtonOf(text: string): HTMLElement {
  const row = screen.getByText(text).closest('[role="listitem"]') as HTMLElement;
  return within(row).getByTitle("worldEditor.editPlayer");
}

function editButtons() {
  // Each row's edit button is named by its `title`. Counting them by title matches the attribute
  // directly; a role query with `name` computes the accessible name of every button in the list
  // (the first such query on a fresh DOM took over a second), which is what made this file time
  // out under load.
  return screen.queryAllByTitle("worldEditor.editPlayer");
}

/** Renders 500 players and hands back a way to move the selection, as opening a record does. */
function renderTabForRerender() {
  const props = {
    players: players(500),
    onAdd: () => {},
    onEdit: vi.fn(),
    onDelete: () => {},
  };
  const view = render(
    <div data-testid="scroller" data-scroll-container="">
      <PlayersTab {...props} selectedIndex={null} />
    </div>,
  );
  return {
    select: (selectedIndex: number) =>
      view.rerender(
        <div data-testid="scroller" data-scroll-container="">
          <PlayersTab {...props} selectedIndex={selectedIndex} />
        </div>,
      ),
  };
}

describe("PlayersTab", () => {
  it("renders a window of rows rather than every player", () => {
    // A package with a full pyramid holds tens of thousands of players, and every row on the page
    // is what made the list slow: 17,098 of them were 394,000 elements and a 5 GB web view.
    renderTab();

    expect(editButtons().length).toBeGreaterThan(5);
    expect(editButtons().length).toBeLessThan(40);
  });

  it("says how many players the list holds", () => {
    renderTab();

    expect(screen.getByText("common.nResults count=500")).toBeInTheDocument();
  });

  it("names the list and gives each row its place in the whole of it", () => {
    renderTab();

    const rows = screen.getAllByRole("listitem");
    expect(screen.getByRole("list", { name: "worldEditor.sectionPlayers" })).toBeInTheDocument();
    expect(rows[0]).toHaveAttribute("aria-setsize", "500");
    expect(rows[0]).toHaveAttribute("aria-posinset", "1");
  });

  it("offers no Load more: the rest of the list is a scroll away", () => {
    renderTab();

    expect(screen.queryByText("common.loadMore")).not.toBeInTheDocument();
  });

  it("addresses a row it scrolled to by its index in the whole list", async () => {
    // Edit, delete and duplicate all take the unfiltered index: a row that was only rendered once
    // the list scrolled to it must carry its real one.
    const { onEdit } = renderTab();

    await scrollToPlayer(300);
    fireEvent.click(editButtonOf("Player 300"));

    expect(onEdit).toHaveBeenCalledWith(300);
  });

  it("addresses a searched row by its index in the whole list", () => {
    const { onEdit } = renderTab({ players: players(500) });

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchPlayers" }), {
      target: { value: "Player 123" },
    });
    fireEvent.click(editButtons()[0]);

    expect(onEdit).toHaveBeenCalledWith(123);
  });

  it("goes back to the top of the list when the search changes", async () => {
    renderTab();
    await scrollToPlayer(300);

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchPlayers" }), {
      target: { value: "Player" },
    });

    await waitFor(() => expect(screen.getByText("Player 0")).toBeInTheDocument());
    expect(screen.getByTestId("scroller").scrollTop).toBe(LIST_OFFSET);
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

  it("reads only the photos of the rows in the window on mount", () => {
    useAssetDataUrl.mockClear();
    renderTab({ players: players(500, () => ({ photo: "photos/a.png" })), projectDir: "/pkg" });

    expect(useAssetDataUrl.mock.calls.length).toBeLessThanOrEqual(40);
  });

  it("keeps the youth and senior lists apart", () => {
    renderTab({
      players: players(4, (i) => ({ youth: i < 3 })),
      youthOnly: true,
    });

    expect(editButtons()).toHaveLength(3);
    expect(screen.getByText("common.nResults count=3")).toBeInTheDocument();
  });

  it("scrolls the selected row into view when the selection moves to a row out of the window", async () => {
    // Opening a record, or duplicating one (which selects the copy just below it), must show the row.
    const { select } = renderTabForRerender();

    select(450);

    await waitFor(() => expect(screen.getByText("Player 450")).toBeInTheDocument());
  });

  it("keeps showing the selected row as the selection walks down the list", async () => {
    const { select } = renderTabForRerender();

    select(99);
    await waitFor(() => expect(screen.getByText("Player 99")).toBeInTheDocument());
    select(100);
    await waitFor(() => expect(screen.getByText("Player 100")).toBeInTheDocument());
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

  it("goes back to the top of the list when the position changes", async () => {
    renderTab({ players: players(500, () => ({ position: "Striker" as Position })) });
    await scrollToPlayer(300);

    choosePosition("common.positions.Striker");

    await waitFor(() => expect(screen.getByText("Player 0")).toBeInTheDocument());
    expect(screen.getByTestId("scroller").scrollTop).toBe(LIST_OFFSET);
  });

  it("goes back to the whole list when the filter is cleared", () => {
    renderTab({ players: mixedSquad() });

    choosePosition("common.positions.Goalkeeper");
    choosePosition("worldEditor.allPositions");

    expect(editButtons()).toHaveLength(4);
  });
});
