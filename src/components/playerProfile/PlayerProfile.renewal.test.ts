import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useSettingsStore } from "../../store/settingsStore";
import type { PlayerData } from "../../store/gameStore";
import {
  getRenewalStatusClassName,
  getRenewalStatusMessage,
  hasActiveManagerRenewalBlock,
  shouldDisableRenewalSubmit,
} from "./PlayerProfile.renewal";

const originalSettings = useSettingsStore.getState().settings;
const originalCurrency = useSettingsStore.getState().currency;
const originalSupportedCurrencies = useSettingsStore.getState().supportedCurrencies;

beforeEach(() => {
  useSettingsStore.setState({
    settings: { ...originalSettings, currency: "EUR", language: "en" },
    currency: { code: "EUR", symbol: "€", exchange_rate: 1 },
    supportedCurrencies: {
      EUR: { code: "EUR", symbol: "€", exchange_rate: 1 },
      GBP: { code: "GBP", symbol: "£", exchange_rate: 0.86 },
      USD: { code: "USD", symbol: "$", exchange_rate: 1.08 },
    },
  });
});

afterEach(() => {
  useSettingsStore.setState({
    settings: originalSettings,
    currency: originalCurrency,
    supportedCurrencies: originalSupportedCurrencies,
  });
});

const t = (key: string, params?: Record<string, string | number>): string => {
  if (key === "playerProfile.renewalBlocked") {
    return "Talks blocked";
  }

  if (key === "playerProfile.renewalAccepted") {
    return "Offer accepted";
  }

  if (key === "playerProfile.renewalRejected") {
    return "Offer rejected";
  }

  if (key === "playerProfile.renewalCounter") {
    return `Counter ${params?.wage}/${params?.years}`;
  }

  return key;
};

describe("PlayerProfile renewal helpers", () => {
  it("prefers the blocked message when the session is blocked", () => {
    expect(
      getRenewalStatusMessage(
        {
          renewalSessionStatus: "blocked",
          renewalStatus: "idle",
          renewalSuggestedWage: null,
          renewalSuggestedYears: null,
          renewalError: "raw error",
        },
        t,
      ),
    ).toBe("Talks blocked");
  });

  it("formats counter-offer messaging with suggested terms", () => {
    expect(
      getRenewalStatusMessage(
        {
          renewalSessionStatus: "open",
          renewalStatus: "counter_offer",
          renewalSuggestedWage: 16000,
          renewalSuggestedYears: 4,
          renewalError: null,
        },
        t,
      ),
    ).toBe("Counter €16,000/4");
  });

  it("falls back to the raw error when no translated state applies", () => {
    expect(
      getRenewalStatusMessage(
        {
          renewalSessionStatus: "idle",
          renewalStatus: "error",
          renewalSuggestedWage: null,
          renewalSuggestedYears: null,
          renewalError: "backend timeout",
        },
        t,
      ),
    ).toBe("backend timeout");
  });

  it("maps renewal statuses to their display class names", () => {
    expect(getRenewalStatusClassName("accepted")).toBe("text-positive");
    expect(getRenewalStatusClassName("rejected")).toBe("text-danger");
    expect(getRenewalStatusClassName("counter_offer")).toBe("text-warning");
    expect(getRenewalStatusClassName("idle")).toBe("text-fg-muted");
  });

  it("disables submit when any blocking condition is present", () => {
    expect(
      shouldDisableRenewalSubmit({
        renewalSubmitting: false,
        renewalIsTerminal: false,
        isRenewalWageValid: true,
        isRenewalLengthValid: true,
        renewalViolatesSoftCap: false,
      }),
    ).toBe(false);

    expect(
      shouldDisableRenewalSubmit({
        renewalSubmitting: false,
        renewalIsTerminal: false,
        isRenewalWageValid: true,
        isRenewalLengthValid: true,
        renewalViolatesSoftCap: true,
      }),
    ).toBe(true);
  });

  const blockedPlayer = (status: string, until?: string) =>
    ({
      morale_core: { renewal_state: { status, manager_blocked_until: until } },
    }) as unknown as PlayerData;

  /**
   * Given a player whose renewal state is Blocked until 1 September
   * When it is read on 1 August, on 1 September (the clock's date written with a time of day), and
   *      on 2 September
   * Then the block is active up to and including the day it names, and over after it
   */
  it.each([
    ["1 August", "2026-08-01T00:00:00Z", true],
    ["1 September, the day it ends", "2026-09-01T12:00:00Z", true],
    ["2 September, the day after", "2026-09-02T00:00:00Z", false],
  ])("a block until 1 September is read on %s: active is %s", (_label, today, active) => {
    expect(hasActiveManagerRenewalBlock(blockedPlayer("Blocked", "2026-09-01"), today)).toBe(
      active,
    );
  });

  /**
   * Given a Blocked state with no end date
   * When it is read
   * Then the block is active
   */
  it("treats an undated block as active", () => {
    expect(hasActiveManagerRenewalBlock(blockedPlayer("Blocked"), "2026-08-01")).toBe(true);
  });

  /**
   * Given a renewal state that is not Blocked, even with an end date far ahead
   * When it is read
   * Then there is no block
   */
  it("treats a state that is not Blocked as no block", () => {
    expect(hasActiveManagerRenewalBlock(blockedPlayer("Open", "2099-01-01"), "2026-08-01")).toBe(
      false,
    );
  });

  /**
   * Given a player with no morale information at all
   * When it is read
   * Then there is no block
   */
  it("treats a player with no morale as no block", () => {
    expect(hasActiveManagerRenewalBlock({} as PlayerData, "2026-08-01")).toBe(false);
  });
});
