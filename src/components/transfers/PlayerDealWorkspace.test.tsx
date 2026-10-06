import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { PlayerData, TeamData } from "../../store/gameStore";
import PlayerDealWorkspace from "./PlayerDealWorkspace";

import { createInstance } from "i18next";
import en from "../../i18n/locales/en.json";
import de from "../../i18n/locales/de.json";
import type { LoanOfferData, TransferOfferData } from "../../store/gameStore";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en }, de: { translation: de } },
  interpolation: { escapeValue: false },
});

vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({ t: translations.t.bind(translations), i18n: translations }),
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
        teams={[createTeam(), createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" })]}
        myTeam={createTeam()}
        weeklySuffix="/wk"
        transferWindowBlocksRegistration={false}
        transferWindowSummary="Window open"
        loanNoticeDetail={null}
        selectedKind="transfer"
        onSelectKind={vi.fn()}
        onClose={onClose}
        renderDealPanel={() => <div>Deal panel</div>}
      />,
    );

    expect(screen.queryByRole("button", { name: "Cancel" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("separates current wage from the offered wage so a free agent's €0 does not contradict the offer (#305)", () => {
    render(
      <PlayerDealWorkspace
        player={createPlayer({ team_id: null, wage: 0 })}
        teams={[createTeam()]}
        myTeam={createTeam()}
        weeklySuffix="/wk"
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
        teams={[createTeam(), createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" })]}
        myTeam={createTeam()}
        weeklySuffix="/wk"
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
    expect(screen.getByText("Permanent move with a transfer fee.")).toBeInTheDocument();
    expect(screen.getByText("The club is open to a permanent sale.")).toBeInTheDocument();
  });

  it("shows the reason (not the description or metadata) for a disabled route (#305)", () => {
    render(
      <PlayerDealWorkspace
        player={createPlayer({ transfer_listed: false, loan_listed: true })}
        teams={[createTeam(), createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" })]}
        myTeam={createTeam()}
        weeklySuffix="/wk"
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
    expect(screen.getByText("Player is not transfer listed.")).toBeInTheDocument();
    expect(screen.queryByText("Permanent move with a transfer fee.")).not.toBeInTheDocument();
  });
});

function transferOffer(overrides: Partial<TransferOfferData> = {}): TransferOfferData {
  return {
    id: "offer-1",
    from_team_id: "team-1",
    fee: 700001,
    wage_offered: 12000,
    last_manager_fee: null,
    negotiation_round: 1,
    suggested_counter_fee: null,
    status: "PendingRegistration",
    date: "2026-12-20",
    registration_date: "2027-01-02",
    ...overrides,
  };
}

function loanOffer(overrides: Partial<LoanOfferData> = {}): LoanOfferData {
  return {
    id: "loan-1",
    from_team_id: "team-1",
    parent_team_id: "team-2",
    start_date: "2027-01-02",
    end_date: "2027-06-30",
    wage_contribution_pct: 75,
    buy_option_fee: 800001,
    status: "PendingRegistration",
    date: "2026-12-20",
    ...overrides,
  };
}

function workspace(
  player: PlayerData,
  overrides: Partial<React.ComponentProps<typeof PlayerDealWorkspace>> = {},
) {
  return (
    <PlayerDealWorkspace
      player={player}
      teams={[createTeam(), createTeam({ id: "team-2", name: "Beta FC" })]}
      myTeam={createTeam()}
      weeklySuffix="/wk"
      transferWindowBlocksRegistration={false}
      transferWindowSummary="Window open"
      loanNoticeDetail={null}
      selectedKind="transfer"
      onSelectKind={vi.fn()}
      onClose={vi.fn()}
      renderDealPanel={() => <div>Deal panel</div>}
      {...overrides}
    />
  );
}

describe("saved player offers", () => {
  /** Given a saved first-round rejection, when the workspace opens, then its fee, round, date and outcome are visible. */
  it("shows a first round rejection from the saved offer", () => {
    render(
      workspace(
        createPlayer({
          transfer_offers: [transferOffer({ status: "Rejected", registration_date: null })],
        }),
      ),
    );
    const history = screen.getByRole("region", { name: "Offer history" });
    expect(within(history).getByText("Rejected")).toBeInTheDocument();
    expect(within(history).getByText("Round 1")).toBeInTheDocument();
    expect(within(history).getByText("€700,001")).toBeInTheDocument();
    expect(within(history).getByText("Offer dated December 20, 2026.")).toBeInTheDocument();
    expect(screen.getByText("Deal panel")).toBeInTheDocument();
  });

  /** Given each persisted outcome, when a save is reloaded, then history presents the saved status independently. */
  it.each([
    ["Pending", "Live"],
    ["PendingRegistration", "Pending registration"],
    ["Accepted", "Accepted"],
    ["Rejected", "Rejected"],
    ["Withdrawn", "Talks cooled off"],
  ] as const)("keeps the %s outcome visible after reload", (status, label) => {
    const restored: PlayerData = JSON.parse(
      JSON.stringify(createPlayer({ transfer_offers: [transferOffer({ status })] })),
    );
    render(workspace(restored));
    expect(
      within(screen.getByRole("region", { name: "Offer history" })).getByText(label),
    ).toBeInTheDocument();
  });

  /** Given a future transfer agreement, when the window has no known opening, then the saved agreement remains visible and all new approaches are blocked. */
  it("keeps a transfer agreement visible and blocks new approaches", () => {
    render(
      workspace(createPlayer({ loan_listed: true, transfer_offers: [transferOffer()] }), {
        transferWindowBlocksRegistration: true,
        transferWindowSummary: "Window closed",
      }),
    );
    expect(screen.queryByText("Deal panel")).not.toBeInTheDocument();
    const history = screen.getByRole("region", { name: "Offer history" });
    expect(
      within(history).getByText(
        "The terms are accepted. Registration is scheduled for January 2, 2027.",
      ),
    ).toBeInTheDocument();
    expect(within(history).getByText("€700,001")).toBeInTheDocument();
    for (const name of ["Make Transfer Bid", "Make Loan Offer", "Offer Contract"]) {
      expect(screen.getByRole("button", { name })).toBeDisabled();
    }
  });

  /** Given a deferred loan, when its workspace opens, then final loan dates, wage split and option are shown while further approaches are disabled. */
  it("shows agreed loan terms while blocking another deal", () => {
    render(
      workspace(createPlayer({ loan_listed: true, loan_offers: [loanOffer()] }), {
        selectedKind: "loan",
      }),
    );
    const history = screen.getByRole("region", { name: "Offer history" });
    expect(within(history).getByText("Loan 75% wages until June 30, 2027")).toBeInTheDocument();
    expect(within(history).getByText("Loan starts on January 2, 2027.")).toBeInTheDocument();
    expect(within(history).getByText("Option €800,001")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Make Loan Offer" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Make Transfer Bid" })).toBeDisabled();
    expect(screen.queryByText("Deal panel")).not.toBeInTheDocument();
  });

  /** Given an agreement between other clubs, when this manager inspects the player, then new bids are blocked without showing unrelated negotiation records. */
  it("blocks approaches to an agreed player without exposing other clubs offers", () => {
    render(
      workspace(createPlayer({ transfer_offers: [transferOffer({ from_team_id: "other-club" })] })),
    );
    expect(screen.getByRole("button", { name: "Make Transfer Bid" })).toBeDisabled();
    expect(screen.queryByRole("region", { name: "Offer history" })).not.toBeInTheDocument();
    expect(screen.queryByText("€700,001")).not.toBeInTheDocument();
  });

  /** Given an incoming offer for the manager's player, when its workspace is inspected, then the offer is shown but buying the manager's own player is unavailable. */
  it("shows an incoming agreement for the current club", () => {
    render(
      workspace(
        createPlayer({
          team_id: "team-1",
          transfer_offers: [transferOffer({ from_team_id: "team-2" })],
        }),
      ),
    );
    expect(
      within(screen.getByRole("region", { name: "Offer history" })).getByText("Beta FC"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Make Transfer Bid" })).toBeDisabled();
  });

  /** Given a parent club's player already away on loan, when the parent inspects the workspace, then the accepted loan remains in history and new deals are blocked. */
  it("shows the parent clubs accepted loan after the player moves", () => {
    render(
      workspace(
        createPlayer({
          team_id: "team-2",
          loan_listed: true,
          loan_offers: [
            loanOffer({ from_team_id: "team-2", parent_team_id: "team-1", status: "Accepted" }),
          ],
          active_loan: {
            parent_team_id: "team-1",
            loan_team_id: "team-2",
            start_date: "2027-01-02",
            end_date: "2027-06-30",
            wage_contribution_pct: 75,
          },
        }),
      ),
    );
    expect(
      within(screen.getByRole("region", { name: "Offer history" })).getByText("Accepted"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Make Transfer Bid" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Make Loan Offer" })).toBeDisabled();
  });

  /** Given an old completed transfer and a later listing at another club, when approached, then the historical acceptance does not block a new bid. */
  it("allows another approach after a historical completed transfer", () => {
    render(
      workspace(
        createPlayer({
          transfer_offers: [transferOffer({ status: "Accepted", registration_date: null })],
        }),
      ),
    );
    expect(
      within(screen.getByRole("region", { name: "Offer history" })).getByText("Accepted"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Make Transfer Bid" })).toBeEnabled();
    expect(screen.getByText("Deal panel")).toBeInTheDocument();
  });

  /** Given several persisted attempts, when opened, then every own offer is shown newest first and other clubs records are excluded. */
  it("lists all saved attempts newest first", () => {
    render(
      workspace(
        createPlayer({
          transfer_offers: [
            transferOffer({ id: "old", status: "Rejected", date: "2026-12-01", fee: 200001 }),
            transferOffer({ id: "other", from_team_id: "other", status: "Rejected", fee: 400001 }),
            transferOffer({
              id: "recent",
              status: "Pending",
              date: "2026-12-20",
              fee: 600001,
              negotiation_round: 3,
            }),
          ],
        }),
      ),
    );
    const history = screen.getByRole("region", { name: "Offer history" });
    const entries = within(history).getAllByRole("listitem");
    expect(entries).toHaveLength(2);
    expect(within(entries[0]).getByText("€600,001")).toBeInTheDocument();
    expect(within(entries[0]).getByText("Round 3")).toBeInTheDocument();
    expect(within(entries[1]).getByText("€200,001")).toBeInTheDocument();
  });

  /** Given a closed window and a rejected bid, when inspected, then history survives the route's unavailable state. */
  it("shows history when the selected route is unavailable", () => {
    render(
      workspace(
        createPlayer({
          transfer_listed: false,
          transfer_offers: [transferOffer({ status: "Rejected" })],
        }),
      ),
    );
    expect(
      within(screen.getByRole("region", { name: "Offer history" })).getByText("Rejected"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Deal panel")).not.toBeInTheDocument();
  });

  /** Given a legacy player with no loan records, when opened, then the usual deal panel remains usable without an empty history section. */
  it("keeps the approach panel usable for a player without offers", () => {
    render(workspace(createPlayer()));
    expect(screen.queryByRole("region", { name: "Offer history" })).not.toBeInTheDocument();
    expect(screen.getByText("Deal panel")).toBeInTheDocument();
  });

  /** Given an own listed player without offers, when inspected, then none of the buy or loan routes can open. */
  it("blocks buying the managers own listed player", () => {
    render(workspace(createPlayer({ team_id: "team-1", loan_listed: true })));
    expect(screen.getByRole("button", { name: "Make Transfer Bid" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Make Loan Offer" })).toBeDisabled();
    expect(screen.queryByText("Deal panel")).not.toBeInTheDocument();
  });

  /** Given a retired listed player, when inspected, then no approach is available. */
  it("blocks approaches to a retired player", () => {
    render(workspace(createPlayer({ retired: true, team_id: null, loan_listed: true })));
    for (const name of ["Make Transfer Bid", "Make Loan Offer", "Offer Contract"])
      expect(screen.getByRole("button", { name })).toBeDisabled();
  });

  /** Given a refreshed player after acceptance, when the open workspace rerenders, then the persistent agreement replaces the bid form. */
  it("updates the open workspace when the saved player changes", () => {
    const { rerender } = render(workspace(createPlayer()));
    expect(screen.getByText("Deal panel")).toBeInTheDocument();
    rerender(workspace(createPlayer({ transfer_offers: [transferOffer()] })));
    expect(screen.getByRole("region", { name: "Offer history" })).toBeInTheDocument();
    expect(screen.queryByText("Deal panel")).not.toBeInTheDocument();
  });

  /** Given a focused workspace trigger, when the modal opens and Escape closes it, then focus moves inside and returns to that trigger. */
  it("restores keyboard focus after Escape closes the workspace", () => {
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();
    const onClose = vi.fn();
    const { unmount } = render(workspace(createPlayer(), { onClose }));
    expect(screen.getByRole("button", { name: "Back" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
    unmount();
    expect(trigger).toHaveFocus();
    trigger.remove();
  });

  /** Given the workspace's last enabled control, when Tab or Shift Tab reaches an edge, then focus stays inside the dialog. */
  it("keeps keyboard focus inside the workspace", () => {
    render(workspace(createPlayer()));
    const back = screen.getByRole("button", { name: "Back" });
    const bid = screen.getByRole("button", { name: "Make Transfer Bid" });
    bid.focus();
    fireEvent.keyDown(bid, { key: "Tab" });
    expect(back).toHaveFocus();
    fireEvent.keyDown(back, { key: "Tab", shiftKey: true });
    expect(bid).toHaveFocus();
  });
});

/** Given the bid control has keyboard focus, when accepted data replaces the form, then focus stays inside and the new agreement is announced. */
it("keeps focus inside when agreement data replaces the form", () => {
  const renderDealPanel = () => <button type="button">Submit a deal</button>;
  const { rerender } = render(workspace(createPlayer(), { renderDealPanel }));
  screen.getByRole("button", { name: "Submit a deal" }).focus();
  rerender(workspace(createPlayer({ transfer_offers: [transferOffer()] }), { renderDealPanel }));
  expect(screen.getByRole("button", { name: "Back" })).toHaveFocus();
  expect(screen.getByRole("status")).toHaveTextContent(
    "This player has an agreed move and is awaiting registration.",
  );
});

/** Given a saved offer whose club no longer exists, when history renders, then the unknown-club fallback uses a translation. */
it("translates a missing offer club", async () => {
  await translations.changeLanguage("de");
  render(
    workspace(
      createPlayer({
        team_id: "team-1",
        transfer_offers: [transferOffer({ from_team_id: "missing-club" })],
      }),
    ),
  );
  expect(
    within(screen.getByRole("region", { name: "Angebotsverlauf" })).getByText("Unbekannt"),
  ).toBeInTheDocument();
});

afterEach(async () => {
  await translations.changeLanguage("en");
});

/** Given a legacy saved offer with round zero, when history opens, then the first round uses the same normalization as the live negotiation panels. */
it.each(["transfer", "loan"] as const)("shows the first round for a legacy %s offer", (kind) => {
  render(
    workspace(
      createPlayer({
        transfer_offers: kind === "transfer" ? [transferOffer({ negotiation_round: 0 })] : [],
        loan_offers: kind === "loan" ? [loanOffer({ negotiation_round: 0 })] : [],
      }),
    ),
  );
  const history = screen.getByRole("region", { name: "Offer history" });
  expect(within(history).getByText("Round 1")).toBeInTheDocument();
  expect(within(history).queryByText("Round 0")).not.toBeInTheDocument();
});
