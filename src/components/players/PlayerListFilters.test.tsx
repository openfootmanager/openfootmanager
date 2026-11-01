import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import type { PlayersPageQuery } from "../../services/playersService";
import PlayerListFilters from "./PlayerListFilters";

const positionTableState = vi.hoisted(() => ({ arrived: true }));

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  const { createPositionTable } = await import("../../lib/positionTable");
  const canned = cannedPositionsModule();
  return {
    ...canned,
    usePositionTable: () => {
      const arrived = canned.usePositionTable();
      return positionTableState.arrived ? arrived : createPositionTable([]);
    },
  };
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts && "group" in opts && "selected" in opts
        ? `${key}:${String(opts.group)}:${String(opts.selected)}`
        : opts && "group" in opts
          ? `${key}:${String(opts.group)}`
          : key,
  }),
}));

const baseQuery: PlayersPageQuery = {
  search: null,
  position: null,
  positions: [],
  age_min: null,
  age_max: null,
  ovr_min: null,
  ovr_max: null,
  value_min: null,
  value_max: null,
  in_budget: false,
  scope: "all",
  team_id: null,
  status: "all",
  sort_key: "ovr",
  sort_asc: false,
  page: 1,
  page_size: 30,
};

const renderFilters = (
  query: Partial<PlayersPageQuery> = {},
  props: Partial<React.ComponentProps<typeof PlayerListFilters>> = {},
) => {
  const onChange = vi.fn();
  render(
    <PlayerListFilters
      query={{ ...baseQuery, ...query }}
      onChange={onChange}
      statusOptions={["all", "transfer", "loan"]}
      {...props}
    />,
  );
  return onChange;
};

/** The filters owned by a parent that keeps the query, as every screen does, so a press sees the last one. */
const renderOwned = (initial: Partial<PlayersPageQuery> = {}) => {
  const seen: PlayersPageQuery[] = [];
  function Owner() {
    const [query, setQuery] = useState<PlayersPageQuery>({ ...baseQuery, ...initial });
    seen.push(query);
    return (
      <PlayerListFilters
        query={query}
        onChange={(patch) => setQuery((current) => ({ ...current, ...patch }))}
        statusOptions={["all"]}
      />
    );
  }
  render(<Owner />);
  return () => seen[seen.length - 1].positions;
};

describe("PlayerListFilters", () => {
  /**
   * Given the search box
   * When something is typed
   * Then it is reported as the search, and an emptied box as null.
   */
  it("reports the search text, and null when it is cleared", () => {
    const onChange = renderFilters({ search: "ron" });
    const box = screen.getByPlaceholderText("players.searchPlaceholder");

    fireEvent.change(box, { target: { value: "ronal" } });
    fireEvent.change(box, { target: { value: "" } });

    expect(onChange).toHaveBeenNthCalledWith(1, { search: "ronal" });
    expect(onChange).toHaveBeenNthCalledWith(2, { search: null });
  });

  /**
   * Given no position chosen
   * When the defenders chip is pressed
   * Then every defender position is chosen, sent as the group shorthand so a player stored under
   *      the broad "Defender" still matches.
   */
  it("sends a whole group as the group shorthand", () => {
    const onChange = renderFilters();

    fireEvent.click(screen.getByRole("button", { name: "common.positionGroups.Defender" }));

    expect(onChange).toHaveBeenCalledWith({ positions: [{ group: "Defender" }] });
  });

  /**
   * Given the defenders group chosen
   * When the refinement panel opens and Centre Back is switched off
   * Then the remaining four are sent as exact positions.
   */
  it("narrows a group to exact positions in its refinement panel", () => {
    const positionsNow = renderOwned();

    fireEvent.click(screen.getByRole("button", { name: "common.positionGroups.Defender" }));
    fireEvent.click(screen.getByRole("button", { name: "common.positions.CenterBack" }));

    expect(positionsNow()).toEqual([
      { exact: "LeftBack" },
      { exact: "RightBack" },
      { exact: "LeftWingBack" },
      { exact: "RightWingBack" },
    ]);
  });

  /**
   * Given defenders and midfielders chosen, with no refinement panel open
   * When the defenders chip is pressed
   * Then the defenders are un-chosen and the midfielders stay, rather than the panel opening and
   *      the whole group being chosen again on the next press.
   */
  it("un-chooses a fully chosen group with its chip and leaves the others alone", () => {
    const positionsNow = renderOwned({
      positions: [{ group: "Defender" }, { group: "Midfielder" }],
    });

    fireEvent.click(screen.getByRole("button", { name: "common.positionGroups.Defender" }));

    expect(positionsNow()).toEqual([{ group: "Midfielder" }]);
  });

  /**
   * Given only some defenders chosen
   * When the defenders chip is pressed
   * Then the whole group is chosen and its panel opens, as before.
   */
  it("chooses the whole group when only part of it was chosen", () => {
    const positionsNow = renderOwned({ positions: [{ exact: "CenterBack" }] });

    fireEvent.click(screen.getByRole("button", { name: /positionGroupPartialSelection/ }));

    expect(positionsNow()).toEqual([{ group: "Defender" }]);
    expect(
      screen.getByRole("group", {
        name: "transfers.refinePositionGroup:common.positionGroups.Defender",
      }),
    ).toBeTruthy();
  });

  /**
   * Given the refinement panel is open with focus inside it
   * When Escape is pressed
   * Then the panel closes and focus returns to its chip.
   */
  it("closes the refinement on Escape and returns focus to its chip", () => {
    renderOwned();
    const chip = screen.getByRole("button", { name: "common.positionGroups.Midfielder" });
    chip.focus();
    fireEvent.click(chip);
    const inside = screen.getByRole("button", { name: "common.positions.CentralMidfielder" });
    inside.focus();

    fireEvent.keyDown(inside, { key: "Escape" });

    expect(screen.queryByRole("group", { name: /transfers.refinePositionGroup/ })).toBeNull();
    expect(chip).toHaveFocus();
  });

  /**
   * Given the range inputs
   * When a minimum age is typed and then cleared
   * Then it is reported as a number, then as null.
   */
  it("reports a range bound as a number, and null when cleared", () => {
    const typed = renderFilters();
    fireEvent.change(screen.getByLabelText("players.filterAgeMin"), { target: { value: "21" } });
    expect(typed).toHaveBeenCalledWith({ age_min: 21 });
  });

  /**
   * Given the range inputs
   * When a decimal age, an out-of-range rating and a negative number are typed
   * Then the request carries whole numbers inside the valid range: the backend reads ages as
   *      integers and ratings as 0-255, and a stray value would fail the whole page request.
   */
  it("sends whole numbers inside the valid range, whatever is typed", () => {
    const onChange = renderFilters();

    fireEvent.change(screen.getByLabelText("players.filterAgeMin"), { target: { value: "21.5" } });
    expect(onChange).toHaveBeenLastCalledWith({ age_min: 21 });

    fireEvent.change(screen.getByLabelText("players.filterOvrMin"), { target: { value: "300" } });
    expect(onChange).toHaveBeenLastCalledWith({ ovr_min: 100 });

    fireEvent.change(screen.getByLabelText("players.filterAgeMax"), { target: { value: "-4" } });
    expect(onChange).toHaveBeenLastCalledWith({ age_max: 0 });

    fireEvent.change(screen.getByLabelText("players.filterOvrMax"), { target: { value: "1e3" } });
    expect(onChange).toHaveBeenLastCalledWith({ ovr_max: 100 });
  });

  it("reports an emptied range bound as null", () => {
    const cleared = renderFilters({ age_min: 21 });
    fireEvent.change(screen.getByLabelText("players.filterAgeMin"), { target: { value: "" } });
    expect(cleared).toHaveBeenCalledWith({ age_min: null });
  });

  /**
   * Given the "in budget" toggle is offered
   * When it is pressed
   * Then it is reported, and it shows whether it is on.
   */
  it("reports the in-budget toggle when offered, and hides it when not", () => {
    const onChange = renderFilters({ in_budget: false }, { showInBudget: true });
    const toggle = screen.getByRole("button", { name: "transfers.affordableOnly" });
    expect(toggle).toHaveAttribute("aria-pressed", "false");

    fireEvent.click(toggle);

    expect(onChange).toHaveBeenCalledWith({ in_budget: true });
  });

  it("does not offer the in-budget toggle unless asked", () => {
    renderFilters();

    expect(screen.queryByRole("button", { name: "transfers.affordableOnly" })).toBeNull();
  });

  /**
   * Given the status chips of a screen
   * When one is pressed
   * Then it is reported as the status.
   */
  it("reports the chosen status", () => {
    const onChange = renderFilters();

    fireEvent.click(screen.getByRole("button", { name: "transfers.loan" }));

    expect(onChange).toHaveBeenCalledWith({ status: "loan" });
  });

  /**
   * Given the position table has not arrived, so a group has no detailed members to select
   * When the filters render
   * Then the position chips are disabled (pressing one would write a selection back without the
   *      detailed positions already chosen), and the All chip still works.
   */
  it("waits for the position table before the position chips work", () => {
    positionTableState.arrived = false;
    try {
      const onChange = renderFilters({ positions: [{ exact: "CenterBack" }] });

      expect(screen.getByRole("button", { name: "common.positionGroups.Defender" })).toBeDisabled();
      const all = screen.getByRole("button", { name: "transfers.allPositions" });
      expect(all).toBeEnabled();
      fireEvent.click(all);
      expect(onChange).toHaveBeenCalledWith({ positions: [] });
    } finally {
      positionTableState.arrived = true;
    }
  });
});
