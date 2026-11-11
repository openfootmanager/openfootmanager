import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useSettingsStore } from "../../store/settingsStore";
import {
  getRenewalStatusClassName,
  getRenewalStatusMessage,
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
});
