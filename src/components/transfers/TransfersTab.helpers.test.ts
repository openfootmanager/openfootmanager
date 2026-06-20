import { describe, expect, it } from "vitest";

import type { PlayerData, TransferOfferData } from "../../store/gameStore";
import {
  buildLoanPeriodOptions,
  buildResumedBidFeedback,
  buildResumedCounterFeedback,
  formatTransferFeeInput,
  getDefaultLoanPeriodId,
  getLoanPeriodIdForEndDate,
  getOutgoingNegotiationOffer,
  getTransferOfferBadgeVariant,
  getTransferOfferStatusLabel,
  mapTransferNegotiationError,
  normalizeTransferNegotiationFeedback,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";

function createOffer(
  overrides: Partial<TransferOfferData> = {},
): TransferOfferData {
  return {
    id: "offer-1",
    from_team_id: "team-2",
    fee: 1200000,
    wage_offered: 0,
    last_manager_fee: null,
    negotiation_round: 1,
    suggested_counter_fee: null,
    status: "Pending",
    date: "2026-08-01",
    ...overrides,
  };
}

function createPlayer(
  overrides: Partial<PlayerData> = {},
): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "England",
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
      handling: 30,
      reflexes: 30,
      aerial: 60,
    },
    condition: 90,
    morale: 70,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2028-06-30",
    wage: 1000,
    market_value: 1000000,
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
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [createOffer()],
    traits: [],
    ...overrides,
  };
}

const t = (key: string) => key;

describe("TransfersTab.helpers", () => {
  it("returns the pending outgoing offer for the user team", () => {
    const player = createPlayer({
      transfer_offers: [
        createOffer({ id: "offer-1", from_team_id: "team-3", status: "Rejected" }),
        createOffer({ id: "offer-2", from_team_id: "team-1", status: "Pending" }),
      ],
    });

    expect(getOutgoingNegotiationOffer(player, "team-1")?.id).toBe("offer-2");
    expect(getOutgoingNegotiationOffer(player, null)).toBeNull();
  });

  it("builds resumed bid feedback from the latest club signal", () => {
    const feedback = buildResumedBidFeedback(
      createOffer({ negotiation_round: 3, suggested_counter_fee: 1800000 }),
    );

    expect(feedback).toMatchObject({
      mood: "tense",
      round: 3,
      params: { fee: "€1,800,000" },
    });
  });

  it("builds resumed counter feedback from the current incoming offer", () => {
    const feedback = buildResumedCounterFeedback(
      createOffer({ negotiation_round: 2, fee: 1400000 }),
    );

    expect(feedback).toMatchObject({
      mood: "firm",
      round: 2,
      params: { fee: "€1,400,000" },
    });
  });

  it("formats and parses transfer fee input as exact money units", () => {
    expect(formatTransferFeeInput(691920)).toBe("691920");
    expect(parseTransferFeeInput("691920")).toBe(691920);
    expect(parseTransferFeeInput("€691,920")).toBe(691920);
  });

  it("builds loan period presets from the current game date", () => {
    const options = buildLoanPeriodOptions(
      "2026-08-01T12:00:00Z",
      "2028-06-30",
    );

    expect(options.map((option) => [option.id, option.endDate])).toEqual([
      ["three_months", "2026-10-30"],
      ["january_window", "2027-01-01"],
      ["end_of_season", "2027-06-30"],
      ["twelve_months", "2027-08-01"],
    ]);
    expect(options.every((option) => !option.disabled)).toBe(true);
    expect(getDefaultLoanPeriodId("2026-08-01T12:00:00Z", "2028-06-30")).toBe(
      "january_window",
    );
  });

  it("disables loan period presets that outlive the player contract", () => {
    const options = buildLoanPeriodOptions(
      "2026-08-01T12:00:00Z",
      "2026-11-15",
    );

    expect(options.find((option) => option.id === "three_months")).toMatchObject({
      disabled: false,
    });
    expect(options.find((option) => option.id === "january_window")).toMatchObject({
      disabled: true,
      disabledReasonKey: "transfers.loanPeriodUnavailableContract",
    });
    expect(getDefaultLoanPeriodId("2026-08-01T12:00:00Z", "2026-11-15")).toBe(
      "three_months",
    );
  });

  it("preserves unmatched incoming loan offer dates as counter periods", () => {
    const options = buildLoanPeriodOptions(
      "2026-08-01T12:00:00Z",
      "2028-06-30",
      "2027-01-28",
    );

    expect(options[0]).toMatchObject({
      id: "current_offer",
      labelKey: "transfers.loanPeriodCurrentOffer",
      endDate: "2027-01-28",
      disabled: false,
    });
    expect(
      getLoanPeriodIdForEndDate(
        "2026-08-01T12:00:00Z",
        "2028-06-30",
        "2027-01-28",
      ),
    ).toBe("current_offer");
  });

  it("normalizes transfer feedback fee params for display", () => {
    expect(
      normalizeTransferNegotiationFeedback({
        mood: "firm",
        headline_key: "headline",
        detail_key: "detail",
        tension: 40,
        patience: 60,
        round: 1,
        params: { fee: "691920" },
      }),
    ).toMatchObject({
      params: { fee: "€691,920" },
    });
  });

  it("maps statuses to transfer offer badge variants and labels", () => {
    expect(getTransferOfferBadgeVariant("Pending")).toBe("accent");
    expect(getTransferOfferBadgeVariant("Accepted")).toBe("success");
    expect(getTransferOfferStatusLabel(t, "Withdrawn")).toBe(
      "transfers.offerStatusWithdrawn",
    );
  });

  it("maps expired negotiation errors to the localized message", () => {
    expect(
      mapTransferNegotiationError(t, "Offer not found or not pending"),
    ).toBe("transfers.negotiationExpiredError");
    expect(mapTransferNegotiationError(t, "other error")).toBe("other error");
  });
});
