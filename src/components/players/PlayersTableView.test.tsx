import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerSummary, PlayersPageQuery } from "../../services/playersService";
import PlayersTableView from "./PlayersTableView";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));
vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});
vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  return cannedPositionsModule();
});

const query = { sort_key: "ovr", sort_asc: false, page: 1, page_size: 30 } as PlayersPageQuery;

function renderTable(onSort = vi.fn()) {
  render(
    <PlayersTableView
      items={[] as PlayerSummary[]}
      query={query}
      onSort={onSort}
      contextItemsFor={() => []}
      onSelectPlayer={vi.fn()}
      onSelectTeam={vi.fn()}
      total={0}
      fetchError={null}
      page={1}
      pageSize={30}
      totalPages={1}
      onPage={vi.fn()}
    />,
  );
  return onSort;
}

describe("PlayersTableView", () => {
  const SORTABLE = [
    ["common.position", "position"],
    ["common.name", "name"],
    ["common.age", "age"],
    ["common.team", "team"],
    ["common.value", "value"],
    ["common.ovr", "ovr"],
  ] as const;

  /**
   * Given the table
   * When each sortable header's button is pressed
   * Then the list is asked to sort by that column: position, name, age, team, value, rating.
   */
  it("sorts by the column whose header button is pressed", () => {
    const onSort = renderTable();

    for (const [label] of SORTABLE) {
      fireEvent.click(screen.getByRole("button", { name: label }));
    }

    expect(onSort.mock.calls.map(([key]) => key)).toEqual(SORTABLE.map(([, key]) => key));
  });

  /**
   * Given the table, whose headers a keyboard user must be able to reach
   * When the headers are read
   * Then each sortable header holds a real button (so Tab reaches it and Enter or Space press
   *      it), and a header that does not sort (nationality, status) holds none.
   */
  it("makes every sortable header a button and no other header one", () => {
    renderTable();

    for (const [label] of SORTABLE) {
      const header = screen.getByRole("columnheader", { name: label });
      const button = within(header).getByRole("button", { name: label });
      expect(button.tagName).toBe("BUTTON");
      expect(button).not.toHaveAttribute("tabindex", "-1");
    }
    for (const label of ["common.nationality", "common.status"]) {
      expect(
        within(screen.getByRole("columnheader", { name: label })).queryByRole("button"),
      ).toBeNull();
    }
  });

  /**
   * Given the list sorted by rating, highest first
   * When the headers are read
   * Then the rating header says "descending" and no other header carries a sort state.
   */
  it("states the sort on the sorted column's header", () => {
    renderTable();

    expect(screen.getByRole("columnheader", { name: "common.ovr" })).toHaveAttribute(
      "aria-sort",
      "descending",
    );
    for (const [label] of SORTABLE.filter(([, key]) => key !== "ovr")) {
      expect(screen.getByRole("columnheader", { name: label })).not.toHaveAttribute("aria-sort");
    }
  });

  /**
   * Given no players and no load error, and then a load error
   * When the table renders
   * Then it says no one matched in the first case, and not in the second.
   */
  it("says nothing matched only when the load did not fail", () => {
    const { unmount } = render(
      <PlayersTableView
        items={[]}
        query={query}
        onSort={vi.fn()}
        contextItemsFor={() => []}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        total={0}
        fetchError={null}
        page={1}
        pageSize={30}
        totalPages={1}
        onPage={vi.fn()}
      />,
    );
    expect(screen.getByText("players.noMatch")).toBeInTheDocument();
    unmount();

    render(
      <PlayersTableView
        items={[]}
        query={query}
        onSort={vi.fn()}
        contextItemsFor={() => []}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        total={0}
        fetchError="offline"
        page={1}
        pageSize={30}
        totalPages={1}
        onPage={vi.fn()}
      />,
    );
    expect(screen.queryByText("players.noMatch")).toBeNull();
  });
});
