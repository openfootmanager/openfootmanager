import { fireEvent, render, screen } from "@testing-library/react";
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
  /**
   * Given the table
   * When each sortable header is pressed
   * Then the list is asked to sort by that column: position, name, age, team, value, rating.
   */
  it("sorts by the column whose header is pressed", () => {
    const onSort = renderTable();

    for (const label of [
      "common.position",
      "common.name",
      "common.age",
      "common.team",
      "common.value",
      "common.ovr",
    ]) {
      fireEvent.click(screen.getByRole("columnheader", { name: label }));
    }

    expect(onSort.mock.calls.map(([key]) => key)).toEqual([
      "position",
      "name",
      "age",
      "team",
      "value",
      "ovr",
    ]);
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
