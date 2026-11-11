import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import PlayerProfileContractSheet, {
  type PlayerProfileContractSheetProps,
} from "./PlayerProfileContractSheet";
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
    contract_end: "2033-06-30",
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
   * Given his own player with a contract to 30 June 2033, on 10,000 a week
   * When the sheet is drawn
   * Then it shows when the contract ends and the years left, his exact wage, and the club's weekly
   *      wage bill with its budget use and his share of it (the previews loaded with the tab)
   */
  it("shows his contract, his wage and the club's weekly wage bill", () => {
    renderSheet();

    expect(screen.getByText("2033-06-30")).toBeInTheDocument();
    expect(screen.getByText("playerProfile.yearsRemaining")).toBeInTheDocument();
    expect(screen.getByText(/playerProfile\.contractSheet\.weeklyWage/)).toBeInTheDocument();
    expect(screen.getByText(/playerProfile\.contractSheet\.billWeekly/)).toHaveTextContent(
      "300,000",
    );
    expect(screen.getByText(/playerProfile\.contractSheet\.budgetUse/)).toHaveTextContent(
      '"percent":75',
    );
    expect(screen.getByText(/playerProfile\.contractSheet\.share/)).toHaveTextContent(
      '"percent":3',
    );
  });

  /**
   * Given his own player, with the previews answered
   * When the sheet is drawn
   * Then each decision says what it does and costs before a click: Terminate names the severance and
   *      the healthy players left, Renew says it is at his current wage, Let expire names the day he
   *      leaves
   */
  it("says what each decision does and costs before the click", () => {
    renderSheet();

    expect(screen.getByText(/playerProfile\.contractSheet\.terminateNote/)).toHaveTextContent(
      "250,000",
    );
    expect(screen.getByText(/playerProfile\.contractSheet\.terminateNote/)).toHaveTextContent(
      '"healthy":18',
    );
    expect(screen.getByText(/playerProfile\.contractSheet\.renewNote/)).toHaveTextContent("10,000");
    expect(screen.getByText(/playerProfile\.contractSheet\.letExpireNote/)).toHaveTextContent(
      "2033-06-30",
    );
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
    expect(screen.getByText(/playerProfile\.contractSheet\.letExpireSetNote/)).toHaveTextContent(
      "2033-06-30",
    );
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
    expect(screen.queryByText(/playerProfile\.contractSheet\.terminateNote/)).toBeNull();
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
    expect(screen.queryByText(/billWeekly/)).toBeNull();
  });

  it("offers a retry for a preview that failed", () => {
    const retry = vi.fn();
    renderSheet({ data: { ...ANSWERED, projection: { status: "failed" }, retry } });

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));

    expect(retry).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(/billWeekly/)).toBeNull();
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

    expect(screen.getByText("2033-06-30")).toBeInTheDocument();
    expect(screen.queryByText(/weeklyWage/)).toBeNull();
    expect(screen.queryByText(/billWeekly/)).toBeNull();
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
