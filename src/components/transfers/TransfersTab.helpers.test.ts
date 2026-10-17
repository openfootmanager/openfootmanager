import { describe, expect, it } from "vitest";

import en from "../../i18n/locales/en.json";
import type { TransferOfferData } from "../../store/gameStore";
import {
  buildLoanPeriodOptions,
  buildResumedBidFeedback,
  buildResumedCounterFeedback,
  formatTransferFeeInput,
  getDefaultLoanPeriodId,
  getLoanPeriodIdForEndDate,
  getTransferOfferBadgeVariant,
  getTransferOfferStatusLabel,
  mapTransferNegotiationError,
  normalizeTransferNegotiationFeedback,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";

function createOffer(overrides: Partial<TransferOfferData> = {}): TransferOfferData {
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

const t = (key: string) => key;

function translateEnglishOfferLabel(key: string): string {
  const name = key.slice("transfers.".length) as keyof typeof en.transfers;
  return en.transfers[name] ?? key;
}

describe("TransfersTab.helpers", () => {
  // Given a funds-related registration void, when displaying it, then its label explains the funds failure.
  it("labels an unaffordable registration with its funds reason", () => {
    expect(
      getTransferOfferStatusLabel(translateEnglishOfferLabel, "Withdrawn", "InsufficientFunds"),
    ).toBe("Registration failed: insufficient buyer funds");
  });

  // Given a loan conflict, when displaying the voided transfer, then the label names the conflict.
  it("labels a loan conflict as a registration failure", () => {
    expect(
      getTransferOfferStatusLabel(translateEnglishOfferLabel, "Withdrawn", "LoanConflict"),
    ).toBe("Registration failed: loan conflict");
  });

  // Given an unavailable player, when displaying the voided transfer, then the label names that reason.
  it("labels an unavailable player as a registration failure", () => {
    expect(
      getTransferOfferStatusLabel(translateEnglishOfferLabel, "Withdrawn", "PlayerUnavailable"),
    ).toBe("Registration failed: player unavailable");
  });

  // Given a final execution refusal, when displaying the voided transfer, then it is shown as blocked registration.
  it("labels a final refusal as blocked registration", () => {
    expect(
      getTransferOfferStatusLabel(translateEnglishOfferLabel, "Withdrawn", "RegistrationBlocked"),
    ).toBe("Registration blocked");
  });

  // Given ordinary withdrawn talks without a registration reason, when displaying them, then the cooled-off label remains.
  it("keeps the cooled-off label for ordinary or legacy withdrawals", () => {
    expect(getTransferOfferStatusLabel((key) => key, "Withdrawn")).toBe(
      "transfers.offerStatusWithdrawn",
    );
  });

  // Given a successful offer, when displaying its status, then a stale reason cannot override acceptance.
  it("keeps accepted offers labelled as accepted", () => {
    expect(getTransferOfferStatusLabel((key) => key, "Accepted", "InsufficientFunds")).toBe(
      "transfers.offerStatusAccepted",
    );
  });

  // Given an unknown backend failure reason, when displaying it, then a localized registration label replaces the raw key.
  it("falls back to a localized label for an unknown registration reason", () => {
    expect(
      getTransferOfferStatusLabel(translateEnglishOfferLabel, "Withdrawn", "FutureReason"),
    ).toBe("Registration blocked");
  });

  /**
   * Given a negotiation the club refused (a rejected offer, round 2)
   * When the panel resumes it
   * Then the feedback says the club turned the bid down rather than that it is waiting for the next
   *      move; a live offer keeps the waiting copy.
   */
  it("words a refused negotiation as refused, not as waiting", () => {
    const refused = buildResumedBidFeedback(
      createOffer({ status: "Rejected", negotiation_round: 2 }),
    );
    const live = buildResumedBidFeedback(createOffer({ status: "Pending", negotiation_round: 2 }));

    expect(refused?.headline_key).toBe("transfers.resumeRefusedHeadline");
    expect(refused?.detail_key).toBe("transfers.resumeRefusedDetail");
    expect(live?.headline_key).toBe("transfers.resumeNegotiationHeadline");
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
    const options = buildLoanPeriodOptions("2026-08-01T12:00:00Z", "2028-06-30");

    expect(options.map((option) => [option.id, option.endDate])).toEqual([
      ["three_months", "2026-10-30"],
      ["january_window", "2027-01-01"],
      ["end_of_season", "2027-06-30"],
      ["twelve_months", "2027-08-01"],
    ]);
    expect(options.every((option) => !option.disabled)).toBe(true);
    expect(getDefaultLoanPeriodId("2026-08-01T12:00:00Z", "2028-06-30")).toBe("january_window");
  });

  it("disables loan period presets that outlive the player contract", () => {
    const options = buildLoanPeriodOptions("2026-08-01T12:00:00Z", "2026-11-15");

    expect(options.find((option) => option.id === "three_months")).toMatchObject({
      disabled: false,
    });
    expect(options.find((option) => option.id === "january_window")).toMatchObject({
      disabled: true,
      disabledReasonKey: "transfers.loanPeriodUnavailableContract",
    });
    expect(getDefaultLoanPeriodId("2026-08-01T12:00:00Z", "2026-11-15")).toBe("three_months");
  });

  // Given one year left on a contract, when choosing a year-long loan,
  // then the preset ending on the contract date remains available.
  it("allows a twelve month loan ending on the contract date", () => {
    const options = buildLoanPeriodOptions("2026-08-01", "2027-08-01");

    expect(options.find((option) => option.id === "twelve_months")).toMatchObject({
      endDate: "2027-08-01",
      disabled: false,
      disabledReasonKey: null,
    });
  });

  // Given a contract ending this season, when choosing the season-end loan,
  // then the matching preset is available and becomes the default in January.
  it("allows a season end loan ending on the contract date", () => {
    const options = buildLoanPeriodOptions("2027-01-01", "2027-06-30");

    expect(options.find((option) => option.id === "end_of_season")).toMatchObject({
      disabled: false,
      disabledReasonKey: null,
    });
    expect(getDefaultLoanPeriodId("2027-01-01", "2027-06-30")).toBe("end_of_season");
  });

  // Given an incoming offer ending with the contract, when resuming its counter,
  // then its unmatched current period remains selectable.
  it("allows an incoming loan period ending on the contract date", () => {
    const options = buildLoanPeriodOptions("2026-08-01", "2027-01-28", "2027-01-28");

    expect(options[0]).toMatchObject({
      id: "current_offer",
      disabled: false,
      disabledReasonKey: null,
    });
  });

  // Given a matching contract date outside the permitted loan duration,
  // when presenting the current offer, then duration rules still block it.
  it("keeps duration limits for loans ending on the contract date", () => {
    for (const endDate of ["2026-08-30", "2027-08-07"]) {
      const options = buildLoanPeriodOptions("2026-08-01", endDate, endDate);
      expect(options[0]).toMatchObject({
        id: "current_offer",
        disabled: true,
        disabledReasonKey: "transfers.loanPeriodUnavailableRules",
      });
    }
  });

  it("preserves unmatched incoming loan offer dates as counter periods", () => {
    const options = buildLoanPeriodOptions("2026-08-01T12:00:00Z", "2028-06-30", "2027-01-28");

    expect(options[0]).toMatchObject({
      id: "current_offer",
      labelKey: "transfers.loanPeriodCurrentOffer",
      endDate: "2027-01-28",
      disabled: false,
    });
    expect(getLoanPeriodIdForEndDate("2026-08-01T12:00:00Z", "2028-06-30", "2027-01-28")).toBe(
      "current_offer",
    );
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
    expect(getTransferOfferStatusLabel(t, "Withdrawn")).toBe("transfers.offerStatusWithdrawn");
  });

  it("maps expired negotiation errors to the localized message", () => {
    expect(mapTransferNegotiationError(t, "Offer not found or not pending")).toBe(
      "transfers.negotiationExpiredError",
    );
    expect(mapTransferNegotiationError(t, "other error")).toBe("other error");
  });
});
