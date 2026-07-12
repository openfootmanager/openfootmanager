import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData, TeamData } from "../../store/gameStore";
import PlayerDealWorkspace from "./PlayerDealWorkspace";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "common.back") return "Back";
      if (key === "common.cancel") return "Cancel";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "common.ovr") return "OVR";
      if (key === "common.value") return "Value";
      if (key === "common.wage") return "Wage";
      if (key === "common.currentWage") return "Current Wage";
      if (key === "playerProfile.renewalWage") return "Offered Wage";
      if (key === "finances.transferBudget") return "Transfer Budget";
      if (key === "finances.wageBudget") return "Wage Budget";
      if (key === "transfers.dealType") return "Deal Type";
      if (key === "transfers.makeBid") return "Make Transfer Bid";
      if (key === "transfers.makeLoanOffer") return "Make Loan Offer";
      if (key === "transfers.offerContract") return "Offer Contract";
      if (key === "transfers.dealTransferDescription")
        return "Open a transfer negotiation.";
      if (key === "transfers.dealLoanDescription")
        return "Open a loan negotiation.";
      if (key === "transfers.dealContractDescription")
        return "Offer a contract.";
      if (key === "transfers.dealAvailableTransfer")
        return "Available for transfer.";
      if (key === "transfers.dealUnavailableTransfer")
        return "Not available for transfer.";
      if (key === "transfers.dealAvailableLoan") return "Available for loan.";
      if (key === "transfers.dealUnavailableLoan")
        return "Not available for loan.";
      if (key === "transfers.dealAvailableContract")
        return "Available on a free transfer.";
      if (key === "transfers.dealUnavailableContract")
        return "Already contracted.";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-2",
    retired: false,
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: true,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

describe("PlayerDealWorkspace", () => {
  it("uses a page-style back affordance instead of a close icon", () => {
    const onClose = vi.fn();

    render(
      <PlayerDealWorkspace
        player={createPlayer()}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" }),
        ]}
        myTeam={createTeam()}
        annualSuffix="/yr"
        transferWindowBlocksRegistration={false}
        transferWindowSummary="Window open"
        loanNoticeDetail={null}
        selectedKind="transfer"
        onSelectKind={vi.fn()}
        onClose={onClose}
        renderDealPanel={() => <div>Deal panel</div>}
      />,
    );

    expect(
      screen.queryByRole("button", { name: "Cancel" }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("separates current wage from the offered wage so a free agent's €0 does not contradict the offer (#305)", () => {
    render(
      <PlayerDealWorkspace
        player={createPlayer({ team_id: null, wage: 0 })}
        teams={[createTeam()]}
        myTeam={createTeam()}
        annualSuffix="/yr"
        transferWindowBlocksRegistration={false}
        transferWindowSummary="Window open"
        loanNoticeDetail={null}
        selectedKind="contract"
        offeredWage={9000}
        onSelectKind={vi.fn()}
        onClose={vi.fn()}
        renderDealPanel={() => <div>Deal panel</div>}
      />,
    );

    // Both labels render as distinct rows — no lone ambiguous "Wage".
    expect(screen.getByText("Current Wage")).toBeInTheDocument();
    expect(screen.getByText("Offered Wage")).toBeInTheDocument();
    expect(screen.queryByText("Wage")).not.toBeInTheDocument();
  });

  it("labels the selected route with its action, not availability copy (#305)", () => {
    render(
      <PlayerDealWorkspace
        player={createPlayer()}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" }),
        ]}
        myTeam={createTeam()}
        annualSuffix="/yr"
        transferWindowBlocksRegistration={false}
        transferWindowSummary="Window open"
        loanNoticeDetail={null}
        selectedKind="transfer"
        onSelectKind={vi.fn()}
        onClose={vi.fn()}
        renderDealPanel={() => <div>Deal panel</div>}
      />,
    );

    // Subtitle describes the route; availability is only secondary metadata.
    expect(screen.getByText("Open a transfer negotiation.")).toBeInTheDocument();
    expect(screen.getByText("Available for transfer.")).toBeInTheDocument();
  });

  it("shows the reason (not the description or metadata) for a disabled route (#305)", () => {
    render(
      <PlayerDealWorkspace
        player={createPlayer({ transfer_listed: false, loan_listed: true })}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" }),
        ]}
        myTeam={createTeam()}
        annualSuffix="/yr"
        transferWindowBlocksRegistration={false}
        transferWindowSummary="Window open"
        loanNoticeDetail={null}
        selectedKind="loan"
        onSelectKind={vi.fn()}
        onClose={vi.fn()}
        renderDealPanel={() => <div>Deal panel</div>}
      />,
    );

    // The disabled (and unselected) transfer route shows its disabledReason as the
    // subtitle — not the route description, and with no availability metadata line.
    expect(screen.getByText("Not available for transfer.")).toBeInTheDocument();
    expect(
      screen.queryByText("Open a transfer negotiation."),
    ).not.toBeInTheDocument();
  });
});
