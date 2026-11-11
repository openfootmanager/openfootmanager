import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import PlayerProfileContractSheet, {
  type PlayerProfileContractSheetProps,
} from "./PlayerProfileContractSheet";
import type { ContractHistoryRow } from "./PlayerProfile.contractHistory";
import type { ContractSheetData } from "./useContractSheetData";

/** Keys and their params: what the sheet asks to say is what the tests read. */
const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key} ${JSON.stringify(options)}` : key;

const LOAN = {
  parent_team_id: "t1",
  loan_team_id: "t2",
  start_date: "2031-07-01",
  end_date: "2032-06-30",
  wage_contribution_pct: 50,
  buy_option_fee: null,
};

const player = (patch: Partial<PlayerData> = {}) =>
  ({
    id: "p1",
    match_name: "P. Player",
    wage: 10_000,
    contract_end: "2034-06-30",
    active_loan: null,
    team_id: "t1",
    ...patch,
  }) as PlayerData;

const ANSWERED: ContractSheetData = {
  termination: {
    status: "answered",
    value: {
      player_id: "p1",
      player_name: "P. Player",
      severance_cost: 250_000,
      squad_safety: {
        team_id: "t1",
        projected_roster_size: 24,
        healthy_players: 18,
        healthy_goalkeepers: 2,
        effective_xi_size: 11,
        can_field_matchday_squad: true,
        missing_reasons: [],
      },
    },
  },
  projection: {
    status: "answered",
    value: {
      current_annual_wage_bill: 300_000,
      projected_annual_wage_bill: 300_000,
      annual_wage_budget: 400_000,
      annual_soft_cap: 440_000,
      current_weekly_wage_spend: 300_000,
      projected_weekly_wage_spend: 300_000,
      current_cash_runway_weeks: 40,
      projected_cash_runway_weeks: 40,
      currently_over_budget: false,
      policy_allows: true,
    },
  },
  retry: () => {},
};

const LAST_SIGNED: ContractHistoryRow = {
  key: "a",
  clubName: "North FC",
  clubId: "north",
  fromClubName: null,
  start: "2031-07-01",
  end: "2034-06-30",
  weeklyWage: 10_000,
  source: "renewal",
  endedBy: null,
  replacedOn: null,
  inForce: true,
};

function renderSheet(patch: Partial<PlayerProfileContractSheetProps> = {}) {
  const handlers = {
    onOpenRenewal: vi.fn(),
    onMarkLetExpire: vi.fn(),
    onClearLetExpire: vi.fn(),
    onOpenTermination: vi.fn(),
    onOpenFreeAgentContract: vi.fn(),
  };
  render(
    <PlayerProfileContractSheet
      player={player()}
      currentDate="2031-07-10"
      isOwnContract
      isFreeAgent={false}
      hasLetExpireIntent={false}
      exactWeeklyWage={10_000}
      language="en"
      contractRisk={{ level: "stable", label: "Stable" }}
      lastSigned={LAST_SIGNED}
      below={<p>the-contracts-he-has-had</p>}
      data={ANSWERED}
      actionSubmitting={false}
      t={t}
      {...handlers}
      {...patch}
    />,
  );
  return handlers;
}

describe("PlayerProfileContractSheet: his own player", () => {
  /**
   * Given his own player with a contract to 30 June 2034, on 10,000 a week, last signed as a renewal
   *       on 1 July 2031, and a contract risk the backend rates stable
   * When the sheet is drawn
   * Then his contract card shows the end as a date in the player's language, the years left with
   *      the risk beside them, his exact wage, and when and how he last signed
   */
  it("shows his contract: the end, the years left and their risk, his wage, when he last signed", () => {
    renderSheet();

    const row = (label: string) => screen.getByText(label).closest("div");
    expect(row("playerProfile.contractSheet.ends")).toHaveTextContent("Jun 30, 2034");
    expect(row("playerProfile.contractSheet.timeLeft")).toHaveTextContent("Stable");
    expect(row("playerProfile.contractSheet.wage")).toHaveTextContent("€10,000");
    const lastSigned = row("playerProfile.contractSheet.lastSigned");
    expect(lastSigned).toHaveTextContent("Jul 1, 2031");
    expect(lastSigned).toHaveTextContent("playerProfile.contractSheet.source.renewal");
  });

  /**
   * Given a contract from a save made before starts were recorded
   * When the sheet is drawn
   * Then "last signed" says so in words, never a date it does not have
   */
  it("says a legacy contract has no signing date, not a made-up one", () => {
    renderSheet({
      lastSigned: { ...LAST_SIGNED, start: null, source: "legacy_migrated" },
    });

    expect(screen.getByText("playerProfile.movementLegacyContract")).toBeInTheDocument();
    expect(screen.queryByText("playerProfile.contractSheet.lastSigned")).toBeNull();
  });

  /**
   * Given the projection the backend sent (the weekly bill 300,000 against a weekly budget of
   *       400,000, 40 weeks of runway)
   * When the sheet is drawn
   * Then the wage bill card shows the bill, the budget in use as a named bar and a figure, the
   *      runway, and his share of the bill, all read from that projection
   */
  it("shows the club's wage bill, the budget in use as a bar, the runway and his share", () => {
    renderSheet();

    expect(
      screen.getByText("playerProfile.contractSheet.billLabel").closest("div"),
    ).toHaveTextContent("300,000");
    const bar = screen.getByRole("progressbar", {
      name: /playerProfile\.contractSheet\.budgetLabel/,
    });
    expect(bar).toHaveAttribute("aria-valuenow", "75");
    expect(screen.getByText(/playerProfile\.contractSheet\.runwayLabel/)).toBeInTheDocument();
    expect(screen.getByText("playerProfile.contractSheet.shareLabel")).toBeInTheDocument();
    expect(screen.getByText("3.3%")).toBeInTheDocument();
  });

  /**
   * Given a club the backend says is over its wage budget
   * When the sheet is drawn
   * Then the bar is drawn in the danger fill: the backend's flag decides it, the sheet compares
   *      no figure against a threshold
   */
  it("fills the budget bar in the danger colour when the backend says the club is over budget", () => {
    const projection = ANSWERED.projection;
    if (projection.status !== "answered") throw new Error("the fixture answers the projection");
    renderSheet({
      data: {
        ...ANSWERED,
        projection: {
          status: "answered",
          value: {
            ...projection.value,
            currently_over_budget: true,
            current_weekly_wage_spend: 500_000,
          },
        },
      },
    });

    const bar = screen.getByRole("progressbar", { name: /budgetLabel/ });
    expect(bar.firstElementChild?.className).toContain("bg-danger-solid");
  });

  /**
   * Given his own player, with the previews answered
   * When the sheet is drawn
   * Then the three decisions are side by side in one card, each saying what it does and carrying
   *      its own panel of what it costs before a click: Renew "at his current wage", Let expire the
   *      day he would leave, Terminate the severance and the healthy players left
   */
  it("gives each decision a panel of what it costs before the click", () => {
    renderSheet();

    const decisions = screen.getByRole("region", { name: "playerProfile.contractSheet.decide" });
    const renew = within(decisions).getByRole("group", { name: "common.renewContract" });
    const expire = within(decisions).getByRole("group", {
      name: "playerProfile.letContractExpire",
    });
    const terminate = within(decisions).getByRole("group", {
      name: "playerProfile.terminateContract",
    });

    expect(renew).toHaveTextContent("€10,000");
    expect(renew).toHaveTextContent("Jun 30, 2034");
    expect(renew).toHaveTextContent("playerProfile.contractSheet.billAtThisWage");
    expect(expire).toHaveTextContent("playerProfile.contractSheet.leaveOn");
    expect(expire).toHaveTextContent("Jun 30, 2034");
    expect(terminate).toHaveTextContent("playerProfile.contractSheet.severance");
    expect(terminate).toHaveTextContent("250,000");
    expect(terminate).toHaveTextContent('"healthy":18');
  });

  /**
   * Given his own player
   * When the sheet is drawn
   * Then Renew is the filled button and Terminate the danger-outlined one, in the design system's
   *      token variants
   */
  it("draws Renew filled and Terminate danger-outlined", () => {
    renderSheet();

    expect(screen.getByRole("button", { name: "common.renewContract" }).className).toContain(
      "bg-primary",
    );
    expect(
      screen.getByRole("button", { name: "playerProfile.terminateContract" }).className,
    ).toContain("border-danger");
  });

  /**
   * Given the three decisions of his own player
   * When the sheet is drawn
   * Then they share their rows (title, what it does, the panel, the button) so the panels sit at
   *      the same height at the bottom of each column, the buttons fill their column, and the
   *      columns after the first are divided from the one before
   */
  it("aligns the panels to the column bottom, fills the buttons and divides the columns", () => {
    renderSheet();

    const decisions = screen.getByRole("region", { name: "playerProfile.contractSheet.decide" });
    const groups = within(decisions).getAllByRole("group");
    expect(groups).toHaveLength(3);
    for (const group of groups) {
      expect(group.className).toContain("grid-rows-subgrid");
      expect(within(group).getByRole("button").className).toContain("w-full");
    }
    expect(groups[0].className).not.toContain("border-l");
    expect(groups[1].className).toContain("border-l");
    expect(groups[2].className).toContain("border-l");
  });

  /**
   * Given the contracts he has had, handed to the sheet to place
   * When the sheet is drawn
   * Then they sit under the decisions, in the same column
   */
  it("places the contracts he has had under the decisions", () => {
    renderSheet();

    const decisions = screen.getByRole("region", { name: "playerProfile.contractSheet.decide" });
    const slot = screen.getByText("the-contracts-he-has-had");

    expect(decisions.compareDocumentPosition(slot) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(decisions.parentElement).toBe(slot.parentElement);
  });

  /**
   * Given the three decisions
   * When each button is pressed
   * Then each calls its own handler
   */
  it("hands each decision to its own action", () => {
    const handlers = renderSheet();

    fireEvent.click(screen.getByRole("button", { name: "common.renewContract" }));
    fireEvent.click(screen.getByRole("button", { name: "playerProfile.letContractExpire" }));
    fireEvent.click(screen.getByRole("button", { name: "playerProfile.terminateContract" }));

    expect(handlers.onOpenRenewal).toHaveBeenCalledTimes(1);
    expect(handlers.onMarkLetExpire).toHaveBeenCalledTimes(1);
    expect(handlers.onOpenTermination).toHaveBeenCalledTimes(1);
  });

  /**
   * Given his contract is set to run down (let expire)
   * When the sheet is drawn
   * Then Renew and Let expire give way to Reopen contract talks, and the note names the day he leaves
   */
  it("offers to reopen the talks once the contract is let to expire", () => {
    const handlers = renderSheet({ hasLetExpireIntent: true });

    expect(screen.queryByRole("button", { name: "common.renewContract" })).toBeNull();
    expect(screen.queryByRole("button", { name: "playerProfile.letContractExpire" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "playerProfile.reopenContractTalks" }));
    expect(handlers.onClearLetExpire).toHaveBeenCalledTimes(1);
    expect(
      screen.getByRole("group", { name: "playerProfile.reopenContractTalks" }),
    ).toHaveTextContent("Jun 30, 2034");
  });

  /**
   * Given a player with no contract end
   * When the sheet is drawn
   * Then Terminate is disabled and the card says why, in words
   */
  it("disables Terminate with the reason when he has no contract to end", () => {
    renderSheet({ player: player({ contract_end: null }) });

    expect(screen.getByRole("button", { name: "playerProfile.terminateContract" })).toBeDisabled();
    expect(screen.getByText("playerProfile.contractSheet.terminateNoContract")).toBeInTheDocument();
  });

  /**
   * Given his own player on loan at another club (the backend refuses to release him)
   * When the sheet is drawn
   * Then Terminate is disabled with the reason, and no severance is promised
   */
  it("disables Terminate with the reason when he is on loan", () => {
    renderSheet({ player: player({ active_loan: LOAN }) });

    expect(screen.getByRole("button", { name: "playerProfile.terminateContract" })).toBeDisabled();
    expect(screen.getByText("playerProfile.contractSheet.terminateOnLoan")).toBeInTheDocument();
    expect(screen.queryByText("playerProfile.contractSheet.severance")).toBeNull();
  });

  /**
   * Given a release that would leave the club unable to field a matchday XI
   * When the sheet is drawn
   * Then Terminate is disabled and the warning the dialog uses is shown before the click
   */
  it("disables Terminate and warns when the squad could no longer field a matchday XI", () => {
    const safe = ANSWERED.termination;
    if (safe.status !== "answered") throw new Error("the fixture answers the preview");
    const unsafe: ContractSheetData = {
      ...ANSWERED,
      termination: {
        status: "answered",
        value: {
          ...safe.value,
          squad_safety: {
            ...safe.value.squad_safety,
            projected_roster_size: 14,
            healthy_players: 9,
            can_field_matchday_squad: false,
          },
        },
      },
    };
    renderSheet({ data: unsafe });

    expect(screen.getByRole("button", { name: "playerProfile.terminateContract" })).toBeDisabled();
    expect(screen.getByText("playerProfile.terminationUnsafe")).toBeInTheDocument();
  });

  /**
   * Given previews still loading, and one that failed
   * When the sheet is drawn
   * Then loading says so, a failure offers a retry, and neither is drawn as a figure of 0
   */
  it("says when the previews are loading and offers a retry when one failed", () => {
    renderSheet({
      data: {
        termination: { status: "loading" },
        projection: { status: "loading" },
        retry: () => {},
      },
    });
    expect(screen.getAllByText("common.loading").length).toBeGreaterThan(0);
    expect(screen.queryByText(/billLabel/)).toBeNull();
  });

  it("offers a retry for a preview that failed", () => {
    const retry = vi.fn();
    renderSheet({ data: { ...ANSWERED, projection: { status: "failed" }, retry } });

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));

    expect(retry).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/billLabel/)).toBeNull();
  });
});

describe("PlayerProfileContractSheet: another club's player and a free agent", () => {
  /**
   * Given a rival's player, whose contract dates are public and whose wage is not
   * When the sheet is drawn
   * Then it shows when his contract ends and the years left, and no wage, wage bill or decision
   */
  it("shows a rival's contract dates and nothing of his wage or the decisions", () => {
    renderSheet({
      player: player({ team_id: "t9" }),
      isOwnContract: false,
      exactWeeklyWage: null,
      data: { termination: { status: "idle" }, projection: { status: "idle" }, retry: () => {} },
    });

    expect(screen.getByText("Jun 30, 2034")).toBeInTheDocument();
    expect(screen.getByText("the-contracts-he-has-had")).toBeInTheDocument();
    expect(screen.queryByText("playerProfile.contractSheet.wage")).toBeNull();
    expect(screen.queryByText(/billLabel/)).toBeNull();
    expect(screen.queryByRole("button")).toBeNull();
  });

  /**
   * Given a free agent (no club, no contract end)
   * When the sheet is drawn
   * Then it says he has no contract, offers one, and offers none of the decisions
   */
  it("shows a free agent as having no contract, with an offer to make one", () => {
    const handlers = renderSheet({
      player: player({ team_id: null, contract_end: null }),
      isOwnContract: false,
      isFreeAgent: true,
      exactWeeklyWage: null,
      data: { termination: { status: "idle" }, projection: { status: "idle" }, retry: () => {} },
    });

    expect(screen.getByText("playerProfile.contractSheet.noContract")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "transfers.offerContract" }));
    expect(handlers.onOpenFreeAgentContract).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "playerProfile.terminateContract" })).toBeNull();
    expect(screen.queryByRole("button", { name: "common.renewContract" })).toBeNull();
  });
});
