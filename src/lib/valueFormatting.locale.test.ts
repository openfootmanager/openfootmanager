import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { useSettingsStore } from "../store/settingsStore";
import { formatExactMoney, formatVal } from "./valueFormatting";

const SUPPORTED_CURRENCIES = {
  EUR: { code: "EUR", symbol: "€", exchange_rate: 1 },
  GBP: { code: "GBP", symbol: "£", exchange_rate: 0.86 },
  USD: { code: "USD", symbol: "$", exchange_rate: 1.08 },
} as const;

const speak = (language: string, currency: "EUR" | "GBP" | "USD" = "EUR") =>
  useSettingsStore.setState((state) => ({
    settings: { ...state.settings, language, currency },
    currency: SUPPORTED_CURRENCIES[currency],
    supportedCurrencies: SUPPORTED_CURRENCIES,
  }));

describe("money in the player's language", () => {
  beforeEach(() => speak("en"));
  afterEach(() => speak("en"));

  /**
   * Given a German player
   * When an exact amount is shown
   * Then the symbol follows the number after a no-break space, with dot grouping.
   */
  it("puts the symbol after the number where the language does (de)", () => {
    speak("de");

    expect(formatExactMoney(12345)).toBe("12.345 €");
  });

  /**
   * Given a French player on euros
   * When exact and compact amounts are shown
   * Then digits group with a narrow no-break space, the symbol trails, and the million is "M".
   */
  it("uses French grouping and a trailing symbol (fr)", () => {
    speak("fr");

    expect(formatExactMoney(12345)).toBe("12 345 €");
    expect(formatVal(1_250_000)).toBe("1,3 M €");
  });

  /**
   * Given a German player on pounds
   * When a negative amount is shown
   * Then the sign leads the number and the pound sign trails it.
   */
  it("keeps the sign before the number and the symbol after it (de, GBP)", () => {
    speak("de", "GBP");

    expect(formatExactMoney(-500)).toBe(
      `-${Math.round(500 * SUPPORTED_CURRENCIES.GBP.exchange_rate)} £`,
    );
  });

  /**
   * Given a Chinese player
   * When compact amounts are shown
   * Then the unit is the Chinese myriad (万), not an English K or M.
   */
  it("uses the language's own compact units (zh-CN)", () => {
    speak("zh-CN");

    expect(formatVal(1_250_000)).toBe("€125万");
  });

  /**
   * Given a Chinese player, whose compact unit (万) is ten thousand
   * When amounts of different sizes are shown compactly
   * Then no digit is lost to the unit: €12,345 is €1.2万 and €941,250 is €94.1万, not €1万 and €94万.
   */
  it("keeps a digit after the decimal where the myriad unit needs it (zh-CN)", () => {
    speak("zh-CN");

    expect(formatVal(12_345)).toBe("\u20ac1.2\u4e07");
    expect(formatVal(941_250)).toBe("\u20ac94.1\u4e07");
    expect(formatVal(1_250_000)).toBe("\u20ac125\u4e07");
  });

  /**
   * Given English
   * When tens of millions are shown compactly
   * Then they keep their one decimal, as before the move to Intl.
   */
  it("keeps one decimal on tens of millions in thousand-based languages", () => {
    expect(formatVal(12_500_000)).toBe("\u20ac12.5M");
  });

  /**
   * Given a display currency worth 0.86 of the euro
   * When 25 and -25 are converted
   * Then both round the same way (22), the sign reapplied after rounding the magnitude.
   */
  it("rounds a negative amount the way it rounds the positive one", () => {
    speak("en", "GBP");

    expect(formatExactMoney(25)).toBe("\u00a322");
    expect(formatExactMoney(-25)).toBe("-\u00a322");
  });

  /**
   * Given English
   * When the same amounts are shown
   * Then nothing changes from the established forms: €12,345, €941K, €1.3M.
   */
  it("leaves English as it was", () => {
    expect(formatExactMoney(12345)).toBe("€12,345");
    expect(formatVal(941_250)).toBe("€941K");
    expect(formatVal(1_250_000)).toBe("€1.3M");
    expect(formatExactMoney(-500)).toBe("-€500");
  });
});
