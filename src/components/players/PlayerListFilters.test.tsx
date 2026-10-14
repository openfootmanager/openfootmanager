import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayersPageQuery } from "../../services/playersService";
import PlayerListFilters from "./PlayerListFilters";

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
    const onChange = renderFilters({ positions: [{ group: "Defender" }] });

    fireEvent.click(screen.getByRole("button", { name: "common.positionGroups.Defender" }));
    fireEvent.click(screen.getByRole("button", { name: "common.positions.CenterBack" }));

    expect(onChange).toHaveBeenLastCalledWith({
      positions: [
        { exact: "LeftBack" },
        { exact: "RightBack" },
        { exact: "LeftWingBack" },
        { exact: "RightWingBack" },
      ],
    });
  });

  /**
   * Given the refinement panel is open with focus inside it
   * When Escape is pressed
   * Then the panel closes and focus returns to its chip.
   */
  it("closes the refinement on Escape and returns focus to its chip", () => {
    renderFilters({ positions: [{ group: "Midfielder" }] });
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
});
