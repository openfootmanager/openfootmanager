import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { useSettingsStore } from "../store/settingsStore";
import { formatPreciseCompact } from "./valueFormatting";

const SUPPORTED_CURRENCIES = {
  EUR: { code: "EUR", symbol: "€", exchange_rate: 1 },
} as const;

const speak = (language: string) =>
  useSettingsStore.setState((state) => ({
    settings: { ...state.settings, language, currency: "EUR" },
    currency: SUPPORTED_CURRENCIES.EUR,
    supportedCurrencies: SUPPORTED_CURRENCIES,
  }));

describe("compact but precise money", () => {
  beforeEach(() => speak("en"));
  afterEach(() => speak("en"));

  /**
   * Given amounts a message might carry
   * When they are shown compact and precise
   * Then compact is used when three significant digits say it all, and the exact figure when they would not.
   */
  it.each([
    [1_250_000, "€1.25M"],
    [1_700, "€1.7K"],
    [250_000, "€250K"],
    [941_250, "€941,250"],
    [800_001, "€800,001"],
    [12_345, "€12,345"],
    [12_500_000, "€12.5M"],
    [999, "€999"],
    [1_000, "€1K"],
    [0, "€0"],
  ])("shows %d as %s", (amount, shown) => {
    expect(formatPreciseCompact(amount)).toBe(shown);
  });

  /**
   * Given a negative amount
   * When it is shown
   * Then it is the positive one with the sign in front.
   */
  it("keeps the sign of a debt", () => {
    expect(formatPreciseCompact(-1_250_000)).toBe("-€1.25M");
  });

  /**
   * Given a German player
   * When 1,250,000 is shown
   * Then the unit and the symbol follow the language.
   */
  it("follows the player's language (de)", () => {
    speak("de");

    expect(formatPreciseCompact(1_250_000)).toBe("1,25\u00a0Mio.\u00a0€");
  });
});
