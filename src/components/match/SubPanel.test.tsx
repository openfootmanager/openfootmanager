import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SubPanel } from "./SubPanel";
import type { EnginePlayerData, EngineTeamData, MatchSnapshot } from "./types";

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  return cannedPositionsModule();
});

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, arg?: unknown) => {
      if (typeof arg === "string") {
        return arg;
      }

      if (typeof arg === "object" && arg !== null && "used" in arg && "max" in arg) {
        return `${key}:${String((arg as { used: number }).used)}/${String((arg as { max: number }).max)}`;
      }

      if (key === "common.cancel") {
        return "Cancel";
      }

      if (key === "match.selectToTakeOff") {
        return "Select to take off";
      }

      if (key === "match.clearReplacementSelection") {
        return "Clear replacement";
      }

      if (key === "match.selectReplacementMenu") {
        return "Select replacement";
      }

      if (key === "match.selectPlayerToTakeOffFirst") {
        return "Select player to take off first";
      }

      if (key === "match.confirmSubstitution") {
        return "Confirm substitution";
      }

      if (key === "match.removeQueuedSubstitution" && typeof arg === "object" && arg !== null) {
        const { off, on } = arg as { off: string; on: string };
        return `Remove ${off} → ${on} from the queue`;
      }

      if (key === "match.substitutionsPartlyMade" && typeof arg === "object" && arg !== null) {
        const { applied, total } = arg as { applied: number; total: number };
        return `${applied} of ${total} substitutions made`;
      }

      if (key === "match.substitutionRefused" && typeof arg === "object" && arg !== null) {
        const { off, on, reason } = arg as { off: string; on: string; reason: string };
        return `Refused ${off} for ${on}: ${reason}`;
      }

      return key;
    },
  }),
}));

const makePlayer = (overrides: Partial<EnginePlayerData> = {}): EnginePlayerData => {
  const { ovr = 70, ...rest } = overrides;

  return {
    id: "player-1",
    name: "Player One",
    position: "Midfielder",
    ovr,
    condition: 78,
    pace: 70,
    stamina: 70,
    strength: 70,
    agility: 70,
    passing: 70,
    shooting: 70,
    tackling: 70,
    dribbling: 70,
    defending: 70,
    positioning: 70,
    vision: 70,
    decisions: 70,
    composure: 70,
    aggression: 60,
    teamwork: 70,
    leadership: 60,
    handling: 20,
    reflexes: 20,
    aerial: 60,
    traits: [],
    role: "Standard",
    ...rest,
  };
};

const makeTeam = (overrides: Partial<EngineTeamData> = {}): EngineTeamData => ({
  id: "team-1",
  name: "Alpha FC",
  formation: "4-4-2",
  play_style: "Balanced",
  players: [
    makePlayer({ id: "starter-1", name: "Starter One", position: "Midfielder" }),
    makePlayer({ id: "starter-2", name: "Starter Two", position: "Forward", shooting: 80 }),
  ],
  ...overrides,
});

function createSnapshot(): MatchSnapshot {
  return {
    phase: "first_half",
    current_minute: 32,
    home_score: 1,
    away_score: 0,
    possession: "Home",
    ball_zone: "MiddleThird",
    home_team: makeTeam(),
    away_team: makeTeam({
      id: "team-2",
      name: "Beta FC",
      players: [makePlayer({ id: "opp-1", name: "Opponent One" })],
    }),
    home_bench: [
      makePlayer({ id: "bench-1", name: "Bench One", position: "Midfielder", condition: 92 }),
      makePlayer({ id: "bench-2", name: "Bench Two", position: "Forward", shooting: 76 }),
    ],
    away_bench: [makePlayer({ id: "opp-bench-1", name: "Opponent Bench" })],
    home_possession_pct: 56,
    away_possession_pct: 44,
    events: [],
    home_subs_made: 0,
    away_subs_made: 0,
    max_subs: 5,
    home_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    away_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    substitutions: [],
    allows_extra_time: false,
    home_yellows: {},
    away_yellows: {},
    sent_off: [],
  };
}

describe("SubPanel", () => {
  const createProps = () => ({
    snapshot: createSnapshot(),
    side: "Home" as const,
    onSubstitute: vi.fn().mockResolvedValue({ applied: true }),
    onFormationChange: vi.fn(),
    onPlayStyleChange: vi.fn(),
    onChangeRole: vi.fn().mockResolvedValue({ applied: true }),
    onClose: vi.fn(),
  });

  it("shows a disabled bench context menu action until a player is selected to come off", () => {
    const props = createProps();

    render(<SubPanel {...props} />);

    fireEvent.contextMenu(screen.getByTestId("sub-panel-bench-bench-1"));

    expect(
      screen.getByRole("menuitem", { name: "Select player to take off first" }),
    ).toBeDisabled();
  });

  it("supports the substitution selection flow through context menus", () => {
    const props = createProps();

    render(<SubPanel {...props} />);

    fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Select to take off" }));

    fireEvent.contextMenu(screen.getByTestId("sub-panel-bench-bench-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Select replacement" }));

    fireEvent.click(screen.getByRole("button", { name: "Confirm substitution" }));

    expect(props.onSubstitute).toHaveBeenCalledWith([{ offId: "starter-1", onId: "bench-1" }]);
  });

  it("allows clearing the selected off-player through the context menu", () => {
    const props = createProps();

    render(<SubPanel {...props} />);

    fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Select to take off" }));

    expect(screen.getByText("match.selectBenchToCompare")).toBeInTheDocument();

    fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Cancel" }));

    expect(screen.queryByText("match.selectBenchToCompare")).not.toBeInTheDocument();
  });

  it("surfaces recommendations and applies the recommended play style", () => {
    const props = createProps();

    render(<SubPanel {...props} />);

    expect(screen.getByTestId("recommended-sub-starter-1-bench-1")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("recommended-plan-cta"));

    expect(props.onPlayStyleChange).toHaveBeenCalledWith("Balanced");
  });

  it("applies formation from quick tactical tweaks", () => {
    const props = createProps();

    render(<SubPanel {...props} />);

    fireEvent.click(screen.getByRole("combobox", { name: "tactics.formation" }));
    fireEvent.click(screen.getByRole("option", { name: "4-3-3" }));

    expect(props.onFormationChange).toHaveBeenCalledWith("4-3-3");
  });

  /**
   * Given the substitution panel is open beside the formation and play-style controls
   * When a starter is given a new role from the roles section
   * Then the panel asks its owner for that change.
   */
  it("hands a role change to its owner", () => {
    const props = createProps();
    render(<SubPanel {...props} />);

    fireEvent.click(screen.getByText("match.playerRoles"));
    // Listed by position group, so the forward is the second starter.
    const [, forwardPicker] = screen.getAllByRole("combobox", { name: "match.roleFor" });
    fireEvent.click(forwardPicker);
    fireEvent.click(screen.getByRole("option", { name: "tactics.playerRoles.Poacher" }));

    expect(props.onChangeRole).toHaveBeenCalledWith("starter-2", "Poacher");
  });

  /**
   * Given a role picker's list is open inside the substitution dialog
   * When Escape is pressed
   * Then only the list closes; a second Escape closes the dialog.
   */
  it("closes an open role list on the first Escape and the dialog on the second", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    fireEvent.click(screen.getByText("match.playerRoles"));
    const [picker] = screen.getAllByRole("combobox", { name: "match.roleFor" });
    fireEvent.click(picker);
    expect(screen.getByRole("listbox")).toBeInTheDocument();

    fireEvent.keyDown(picker, { key: "Escape" });
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(props.onClose).not.toHaveBeenCalled();

    fireEvent.keyDown(picker, { key: "Escape" });
    expect(props.onClose).toHaveBeenCalledTimes(1);
  });

  it("lets a recommendation prefill the swap flow", () => {
    const props = createProps();

    render(<SubPanel {...props} />);

    fireEvent.click(screen.getByTestId("recommended-sub-starter-1-bench-1"));

    expect(screen.getAllByText("Starter One").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Bench One").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Confirm substitution" })).toBeInTheDocument();
  });

  function pickSwap(offId: string, benchId: string) {
    fireEvent.click(screen.getByTestId(`sub-panel-off-${offId}`));
    fireEvent.click(screen.getByTestId(`sub-panel-bench-${benchId}`));
  }

  /**
   * Given a player to come off and a replacement are chosen
   * When the manager queues the swap
   * Then the panel stays open with the swap listed, and nothing has been committed.
   */
  it("keeps the panel open when a swap is queued", () => {
    const props = createProps();
    render(<SubPanel {...props} />);

    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    expect(props.onClose).not.toHaveBeenCalled();
    expect(props.onSubstitute).not.toHaveBeenCalled();
    expect(screen.getByTestId("queued-sub-starter-1-bench-1")).toBeInTheDocument();
  });

  /**
   * Given two swaps are queued
   * When the manager confirms all
   * Then both are committed in the order they were queued, in one call.
   */
  it("commits every queued swap, in order, with Confirm all", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    pickSwap("starter-2", "bench-2");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    fireEvent.click(screen.getByRole("button", { name: "match.confirmAllSubstitutions" }));

    expect(props.onSubstitute).toHaveBeenCalledTimes(1);
    expect(props.onSubstitute).toHaveBeenCalledWith([
      { offId: "starter-1", onId: "bench-1" },
      { offId: "starter-2", onId: "bench-2" },
    ]);
  });

  /**
   * Given a swap is queued
   * When the manager looks for its players
   * Then the queued bench player is gone from the bench and the queued player cannot be taken off
   *      again.
   */
  it("takes the queued players out of play for further swaps", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    expect(screen.queryByTestId("sub-panel-bench-bench-1")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId("sub-panel-off-starter-1"));
    expect(screen.queryByText("match.selectBenchToCompare")).not.toBeInTheDocument();
  });

  /**
   * Given a swap is queued
   * When the manager removes it from the queue
   * Then it leaves the queue and its players are available again.
   */
  it("lets a queued swap be removed", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    fireEvent.click(screen.getByTestId("remove-queued-sub-starter-1-bench-1"));

    expect(screen.queryByTestId("queued-sub-starter-1-bench-1")).not.toBeInTheDocument();
    expect(screen.getByTestId("sub-panel-bench-bench-1")).toBeInTheDocument();
  });

  /**
   * Given one swap is queued and a second pair is selected
   * When the manager confirms
   * Then the queue and the selected pair are committed together.
   */
  it("commits the queue together with the pair being chosen", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    pickSwap("starter-2", "bench-2");

    fireEvent.click(screen.getByRole("button", { name: "match.confirmSubstitutions" }));

    expect(props.onSubstitute).toHaveBeenCalledWith([
      { offId: "starter-1", onId: "bench-1" },
      { offId: "starter-2", onId: "bench-2" },
    ]);
  });

  /**
   * Given the club has one substitution left and one is already queued
   * When another pair is chosen
   * Then it cannot be queued; Confirm commits the queue alone.
   */
  it("stops queuing once the substitutions left are spoken for", () => {
    const props = createProps();
    props.snapshot.home_subs_made = 4;
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    pickSwap("starter-2", "bench-2");

    expect(screen.getByRole("button", { name: "match.queueSubstitution" })).toBeDisabled();
    // The pair can no longer be added, so Confirm now commits the queue alone.
    expect(screen.getByRole("button", { name: "match.confirmAllSubstitutions" })).toBeEnabled();
  });

  /**
   * Given a swap was queued and the backend has since applied it
   * When the panel is shown the new snapshot
   * Then the swap leaves the queue, so a partly applied batch shows only what is left.
   */
  it("drops a queued swap once the snapshot shows it applied", () => {
    const props = createProps();
    const { rerender } = render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    const applied = createSnapshot();
    applied.substitutions = [
      { minute: 32, side: "Home", player_off_id: "starter-1", player_on_id: "bench-1" },
    ] as unknown as MatchSnapshot["substitutions"];
    applied.home_subs_made = 1;

    rerender(<SubPanel {...props} snapshot={applied} />);

    expect(screen.queryByTestId("queued-sub-starter-1-bench-1")).not.toBeInTheDocument();
  });

  /**
   * Given a confirm that is still running
   * When the manager clicks it again
   * Then the batch is not sent twice, and the buttons are disabled meanwhile.
   */
  it("ignores a second confirm while the batch is being applied", () => {
    const props = createProps();
    props.onSubstitute = vi.fn(() => new Promise<never>(() => {}));
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    const confirm = screen.getByRole("button", { name: "match.confirmAllSubstitutions" });

    fireEvent.click(confirm);
    fireEvent.click(confirm);

    expect(props.onSubstitute).toHaveBeenCalledTimes(1);
    expect(confirm).toBeDisabled();
  });

  /**
   * Given every substitution left is already queued and another pair is selected
   * When the manager confirms
   * Then the queue alone is committed, without first clearing the selection.
   */
  it("commits a full queue even while another pair is selected", () => {
    const props = createProps();
    props.snapshot.home_subs_made = 4;
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    pickSwap("starter-2", "bench-2");

    const confirm = screen.getByRole("button", { name: "match.confirmAllSubstitutions" });
    expect(confirm).toBeEnabled();
    fireEvent.click(confirm);

    expect(props.onSubstitute).toHaveBeenCalledWith([{ offId: "starter-1", onId: "bench-1" }]);
  });

  /**
   * Given two queued swaps and focus on the first one's remove button
   * When it is removed
   * Then focus moves to the remaining swap's remove button, and when that is removed too, to the
   *      first player who can still come off, never to the page body.
   */
  it("keeps keyboard focus inside the panel when queued swaps are removed", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    pickSwap("starter-2", "bench-2");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    screen.getByTestId("remove-queued-sub-starter-1-bench-1").focus();
    fireEvent.click(screen.getByTestId("remove-queued-sub-starter-1-bench-1"));
    expect(screen.getByTestId("remove-queued-sub-starter-2-bench-2")).toHaveFocus();

    fireEvent.click(screen.getByTestId("remove-queued-sub-starter-2-bench-2"));
    expect(screen.getByTestId("sub-panel-off-starter-1")).toHaveFocus();
  });

  /**
   * Given a pair is chosen
   * When the manager adds it to the queue (the button unmounts)
   * Then focus moves to the first player who can still come off.
   */
  it("moves focus to the next player to take off after queuing a swap", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    const add = screen.getByRole("button", { name: "match.queueSubstitution" });
    add.focus();

    fireEvent.click(add);

    expect(screen.getByTestId("sub-panel-off-starter-2")).toHaveFocus();
  });

  /**
   * Given the engine refuses a queued swap
   * When the batch comes back refused
   * Then the panel stays open and says which swap was refused and why.
   */
  it("tells the manager which swap the engine refused and why", async () => {
    const props = createProps();
    props.onSubstitute = vi.fn().mockResolvedValue({
      applied: false,
      refused: { offId: "starter-1", onId: "bench-1" },
      reason: "be.error.liveMatch.playerAlreadySubstitutedOff",
      appliedCount: 0,
    });
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    fireEvent.click(screen.getByRole("button", { name: "match.confirmAllSubstitutions" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Starter One");
    expect(alert).toHaveTextContent("Bench One");
    expect(props.onClose).not.toHaveBeenCalled();
  });

  /**
   * Given two queued swaps
   * When a screen reader reads the remove buttons
   * Then each is named by the swap it removes, not by the same generic label.
   */
  it("names each remove button by the swap it removes", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    pickSwap("starter-2", "bench-2");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    expect(
      screen.getByRole("button", { name: "Remove Starter One → Bench One from the queue" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Remove Starter Two → Bench Two from the queue" }),
    ).toBeInTheDocument();
  });

  /**
   * Given a swap is queued
   * When the formation pitch is read
   * Then the queued player's token is marked unavailable and cannot be picked from the pitch.
   */
  it("marks a queued player as unavailable on the pitch too", () => {
    const props = createProps();
    const { container } = render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    const token = container.querySelector<HTMLElement>('[aria-label="Starter One"]');
    expect(token).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(token as HTMLElement);
    expect(screen.queryByText("match.selectBenchToCompare")).not.toBeInTheDocument();
  });

  /**
   * Given a swap is queued
   * When the context menu of the queued player's row is opened
   * Then "Select to take off" is not offered; a disabled "already queued" entry stands in.
   */
  it("offers no take-off action in the context menu of a queued player", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));

    expect(screen.queryByRole("menuitem", { name: "Select to take off" })).not.toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "match.alreadyQueued" })).toBeDisabled();
  });

  /**
   * Given a swap is queued
   * When the queued row's direction glyph is read
   * Then it is hidden from assistive technology, since the row already says it is unavailable.
   */
  it("hides the direction glyph of a queued player from assistive technology", () => {
    const props = createProps();
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    const row = screen.getByTestId("sub-panel-off-starter-1");
    expect(within(row).getByText("▼")).toHaveAttribute("aria-hidden", "true");
  });

  /**
   * Given a batch where the first swap was made and the second refused
   * When the outcome comes back
   * Then a polite status says how many of the substitutions were made.
   */
  it("announces how much of a partly refused batch was made", async () => {
    const props = createProps();
    props.onSubstitute = vi.fn().mockResolvedValue({
      applied: false,
      refused: { offId: "starter-2", onId: "bench-2" },
      reason: "be.error.liveMatch.maxSubstitutionsReached",
      appliedCount: 1,
    });
    render(<SubPanel {...props} />);
    pickSwap("starter-1", "bench-1");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));
    pickSwap("starter-2", "bench-2");
    fireEvent.click(screen.getByRole("button", { name: "match.queueSubstitution" }));

    fireEvent.click(screen.getByRole("button", { name: "match.confirmAllSubstitutions" }));

    expect(await screen.findByRole("status")).toHaveTextContent("1 of 2 substitutions made");
  });
});

describe("SubPanel as a dialog", () => {
  const renderPanel = () => {
    const onClose = vi.fn();
    render(
      <SubPanel
        snapshot={createSnapshot()}
        side="Home"
        onSubstitute={vi.fn().mockResolvedValue({ applied: true })}
        onFormationChange={vi.fn()}
        onPlayStyleChange={vi.fn()}
        onChangeRole={vi.fn().mockResolvedValue({ applied: true })}
        onClose={onClose}
      />,
    );
    return onClose;
  };

  /**
   * Given the substitution panel is open over the match
   * When assistive technology looks for it
   * Then it is a modal dialog named by its heading, and focus is inside it.
   */
  it("is a named modal dialog that holds focus", () => {
    renderPanel();

    const dialog = screen.getByRole("dialog", { name: "match.substitutionsTitle" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveFocus();
  });

  /**
   * Given the panel is open
   * When the player presses Escape
   * Then the panel closes.
   */
  it("closes on Escape", () => {
    const onClose = renderPanel();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a player's context menu is open inside the panel
   * When the player presses Escape
   * Then only the menu closes and the panel stays.
   */
  it("keeps the panel open when Escape closes a context menu", () => {
    const onClose = renderPanel();
    fireEvent.contextMenu(screen.getByTestId("sub-panel-off-starter-1"));
    expect(screen.getByRole("menuitem", { name: "Select to take off" })).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(screen.queryByRole("menuitem")).not.toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });

  /**
   * Given the panel is open
   * When the player activates the close control by its accessible name
   * Then the panel closes.
   */
  it("has a close button a screen reader can name", () => {
    const onClose = renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "common.close" }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
