import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { LeagueData, TeamData, WorldRegionData } from "../../store/gameStore";
import { createTeam } from "../../test-utils/factories";
import SimulationDialog from "./SimulationDialog";
import type { ScopeDraft } from "./simulationScope";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (params?.defaultValue !== undefined) return String(params.defaultValue);
      if (params?.name !== undefined) return `${key}:${params.name}`;
      if (params?.regions !== undefined && params?.competitions === undefined)
        return `${key}:${params.regions}`;
      if (params?.regions !== undefined) return `${key}:${params.regions}/${params.competitions}`;
      if (params?.on !== undefined) return `${key}:${params.on}/${params.total}`;
      if (params?.clubs !== undefined && params?.countries === undefined)
        return `${key}:${params.clubs}`;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

const REGIONS: WorldRegionData[] = [
  { id: "scandinavia", name: "Scandinavia", country_codes: ["SE", "NO"] },
  { id: "western", name: "Western Europe", country_codes: ["DE", "FR"] },
];

function comp(overrides: Partial<LeagueData>): LeagueData {
  return {
    id: "c",
    name: "C",
    season: 2026,
    fixtures: [],
    standings: [],
    ...overrides,
  } as LeagueData;
}

const SE1 = comp({
  id: "se1",
  name: "Swedish First Division",
  scope: "Domestic",
  kind: "League",
  region_id: "scandinavia",
  country_id: "SE",
});
const SE3 = comp({
  id: "se3",
  name: "Swedish Third Division",
  scope: "Domestic",
  kind: "League",
  region_id: "scandinavia",
  country_id: "SE",
});
const DE1 = comp({
  id: "de1",
  name: "German First Division",
  scope: "Domestic",
  kind: "League",
  region_id: "western",
  country_id: "DE",
});
const CUP = comp({
  id: "cup",
  name: "Continental Cup",
  scope: "Continental",
  kind: "Cup",
  required_region_ids: ["scandinavia", "western"],
});
const COMPETITIONS = [SE1, SE3, DE1, CUP];
const TEAMS: TeamData[] = [
  createTeam({ id: "a", country: "SE" }),
  createTeam({ id: "b", country: "NO" }),
  createTeam({ id: "c", country: "DE" }),
];

const INITIAL: ScopeDraft = {
  homeRegionId: "scandinavia",
  homeCountryCode: "SE",
  regions: {},
  competitions: { se1: true, se3: true, de1: true, cup: true },
};

function renderDialog(overrides: Partial<Parameters<typeof SimulationDialog>[0]> = {}) {
  const handlers = { onApply: vi.fn(), onCancel: vi.fn() };
  render(
    <SimulationDialog
      regions={REGIONS}
      competitions={COMPETITIONS}
      teams={TEAMS}
      yourCompetitions={[]}
      initial={INITIAL}
      {...handlers}
      {...overrides}
    />,
  );
  return handlers;
}

const ASIA_CUP = comp({
  id: "asia_cup",
  name: "Asia Cup",
  scope: "Continental",
  kind: "Cup",
  required_region_ids: ["asia"],
});

describe("SimulationDialog", () => {
  /**
   * Given every competition the dialog can simulate switched off
   * When the footer is read, and then one competition is switched on again
   * Then Apply is disabled and a short reason says to switch one on, so the game is never asked to
   *      simulate nothing; with one on, Apply is enabled and the reason is gone.
   */
  it("disables Apply, with its reason, while no competition is simulated", () => {
    renderDialog();
    const apply = () => screen.getByRole("button", { name: "teamSelect.scope.apply" });
    expect(apply()).toBeEnabled();

    fireEvent.click(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:Swedish First Division" }),
    );
    expect(apply()).toBeEnabled();
    fireEvent.click(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:Swedish Third Division" }),
    );

    expect(apply()).toBeDisabled();
    expect(apply()).toHaveAccessibleDescription("teamSelect.scope.needOne");

    fireEvent.click(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:Swedish Third Division" }),
    );
    expect(apply()).toBeEnabled();
    expect(screen.queryByText("teamSelect.scope.needOne")).toBeNull();
  });

  /**
   * Given a cup that needs a region no club is in (Asia), which the world does not list
   * When the dialog opens, "Add" is pressed on the cup, and then the region's switch is turned off
   * Then the region has a switch of its own, off at first; Add turns it on and enables the cup;
   *      and the switch turns it off again, which blocks the cup: a region a competition needs is
   *      never simulated without a switch to say so.
   */
  it("lists a region no club is in when a competition needs it, and lets it be switched off", () => {
    renderDialog({ competitions: [...COMPETITIONS, ASIA_CUP] });

    const asia = screen.getByRole("switch", { name: "Asia" });
    expect(asia).not.toBeChecked();
    expect(asia).toBeEnabled();
    const cup = screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:Asia Cup" });
    expect(cup).toBeDisabled();

    fireEvent.click(within(cup.closest("li") as HTMLElement).getByRole("button", { name: /add/ }));
    expect(screen.getByRole("switch", { name: "Asia" })).toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:Asia Cup" }),
    ).toBeEnabled();

    fireEvent.click(screen.getByRole("switch", { name: "Asia" }));
    expect(screen.getByRole("switch", { name: "Asia" })).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:Asia Cup" }),
    ).toBeDisabled();
  });

  /**
   * Given the settings with the home region Scandinavia, and no club chosen yet
   * When the dialog opens
   * Then it is a dialog named by its title, the home region's switch is on and locked with the Home
   *      chip, the other region is off and free, and no competition is the club's (none is locked).
   */
  it("locks the home region and nothing else when no club is chosen", () => {
    renderDialog();

    expect(screen.getByRole("dialog", { name: "teamSelect.scope.title" })).toBeInTheDocument();
    const home = screen.getByRole("switch", { name: "Scandinavia" });
    expect(home).toBeChecked();
    expect(home).toBeDisabled();
    expect(screen.getByText("teamSelect.homeBadge")).toBeInTheDocument();
    // The lock's reason is the switch's description, not a chip beside it.
    expect(home).toHaveAccessibleDescription("teamSelect.homeBadge");
    const western = screen.getByRole("switch", { name: "Western Europe" });
    expect(western).not.toBeChecked();
    expect(western).toBeEnabled();
    expect(screen.queryByText("teamSelect.yourClubBadge")).toBeNull();
  });

  /**
   * Given a competition in a region that is off
   * When the dialog opens, and then "Add" is pressed on its Needs line
   * Then it is unchecked and disabled and says which region it needs; Add switches that region on,
   *      the competition becomes enabled and simulated, and focus moves to its checkbox (the button
   *      that held it is gone).
   */
  it("blocks a competition whose region is off, and Add switches the region on and moves focus", () => {
    renderDialog();

    const german = screen.getByRole("checkbox", {
      name: "teamSelect.scope.simulate:German First Division",
    });
    expect(german).toBeDisabled();
    expect(german).not.toBeChecked();
    // What it needs is its description, so a screen reader hears why it is unavailable.
    expect(german).toHaveAccessibleDescription(/teamSelect.scope.needs/);
    // Each "Add" says which competition it unblocks.
    expect(
      screen.getAllByRole("button", { name: /teamSelect.scope.add/ })[0],
    ).toHaveAccessibleDescription(/German First Division|Continental Cup/);
    expect(screen.getAllByText(/teamSelect.scope.needs/).length).toBeGreaterThan(0);

    fireEvent.click(screen.getAllByRole("button", { name: /teamSelect.scope.add/ })[0]);

    expect(screen.getByRole("switch", { name: "Western Europe" })).toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:German First Division" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:German First Division" }),
    ).toHaveFocus();
  });

  /**
   * Given a chosen club whose own league and cup need the home region and Western Europe
   * When the dialog opens
   * Then both regions are locked (Home, and Your club), and the club's league and cup are checked,
   *      disabled and marked Your club: they cannot be switched off.
   */
  it("locks the regions and competitions of the chosen club", () => {
    renderDialog({ yourCompetitions: [SE1, CUP] });

    expect(screen.getByRole("switch", { name: "Western Europe" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Western Europe" })).toBeChecked();
    const league = screen.getByRole("checkbox", {
      name: "teamSelect.scope.simulate:Swedish First Division",
    });
    expect(league).toBeChecked();
    expect(league).toBeDisabled();
    expect(league).toHaveAccessibleDescription("teamSelect.yourClubBadge");
    expect(screen.getAllByText("teamSelect.yourClubBadge").length).toBeGreaterThanOrEqual(3);
  });

  /**
   * Given a free competition that is simulated
   * When its box is unchecked and the dialog is applied
   * Then the settings handed on say it is not chosen; the region switches and the home choice are
   *      carried as they stand.
   */
  it("hands the draft on when applied", () => {
    const handlers = renderDialog();

    fireEvent.click(
      screen.getByRole("checkbox", { name: "teamSelect.scope.simulate:Swedish Third Division" }),
    );
    fireEvent.click(screen.getByRole("switch", { name: "Western Europe" }));
    fireEvent.click(screen.getByRole("button", { name: "teamSelect.scope.apply" }));

    expect(handlers.onApply).toHaveBeenCalledTimes(1);
    const draft = handlers.onApply.mock.calls[0][0] as ScopeDraft;
    expect(draft.competitions.se3).toBe(false);
    expect(draft.regions.western).toBe(true);
    expect(draft.homeRegionId).toBe("scandinavia");
  });

  /**
   * Given changes made in the dialog
   * When it is cancelled from the footer, the close button or Escape
   * Then Apply is never called: Cancel discards the draft.
   */
  it("applies nothing when cancelled, however it is closed", () => {
    const handlers = renderDialog();
    fireEvent.click(screen.getByRole("switch", { name: "Western Europe" }));

    fireEvent.click(screen.getByRole("button", { name: "common.cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "common.close" }));
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    expect(handlers.onApply).not.toHaveBeenCalled();
    expect(handlers.onCancel.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  /**
   * Given the home region changed to Western Europe in the dialog
   * When the dialog is applied
   * Then the home country is that region's first, and Scandinavia is no longer locked.
   */
  it("moves the home country with the home region, and frees the region it leaves", () => {
    const handlers = renderDialog();
    expect(screen.getByRole("switch", { name: "Scandinavia" })).toBeDisabled();

    fireEvent.click(screen.getByRole("combobox", { name: "teamSelect.homeRegion" }));
    fireEvent.click(screen.getByRole("option", { name: "Western Europe" }));

    expect(screen.getByRole("switch", { name: "Scandinavia" })).toBeEnabled();
    expect(screen.getByRole("switch", { name: "Scandinavia" })).not.toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "teamSelect.scope.apply" }));
    const draft = handlers.onApply.mock.calls[0][0] as ScopeDraft;
    expect(draft.homeRegionId).toBe("western");
    expect(draft.homeCountryCode).toBe("DE");
  });

  /**
   * Given the settings with one region and its competitions simulated
   * When the footer is read
   * Then it counts the regions and competitions simulated and the clubs in those regions.
   */
  it("counts the regions, competitions and clubs simulated in the footer", () => {
    renderDialog();

    // Scandinavia only: SE1 and SE3 are simulated; DE1 and the cup (needing Western Europe) are not.
    expect(screen.getByText("teamSelect.scope.summary:1/2")).toBeInTheDocument();
    expect(screen.getByText("teamSelect.scope.clubs:2")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Domestic" })).toBeInTheDocument();
    const domestic = screen.getByRole("region", { name: "Domestic" });
    expect(within(domestic).getByText("teamSelect.scope.groupCount:2/3")).toBeInTheDocument();
  });
});
