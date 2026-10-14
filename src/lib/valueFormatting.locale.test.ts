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
    expect(formatVal(12_345)).toBe("€1万");
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
