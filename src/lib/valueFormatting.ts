import i18n from "i18next";
import { useSettingsStore, type AppSettings } from "../store/settingsStore";
import { useGameStore } from "../store/gameStore";

function getFormattingSettings(): {
  currency: ReturnType<typeof useSettingsStore.getState>["currency"];
  language: AppSettings["language"];
} {
  const { settings, currency } = useSettingsStore.getState();
  return {
    currency,
    language: settings.language || "en",
  };
}

/** Rounds the magnitude and reapplies the sign, so 25 and -25 convert to the same size. */
function convertCurrencyValue(value: number, exchangeRate?: number): number {
  const resolvedExchangeRate = exchangeRate ?? getFormattingSettings().currency.exchange_rate;
  return Math.sign(value) * Math.round(Math.abs(value) * resolvedExchangeRate);
}

/**
 * Age as of the current in-game date (falls back to the real-world date when
 * no game is loaded). Previously hardcoded to `2026 - birth year`, which froze
 * every displayed age at the starting season as the game clock advanced.
 */
export function calcAge(dob: string): number {
  const gameDate = useGameStore.getState().gameState?.clock?.current_date;
  const asOfDate = gameDate ?? localDateIso();
  return calcAgeOnDate(dob, asOfDate);
}

/** The player's own calendar day as YYYY-MM-DD; `toISOString` is the UTC day, which near midnight is another one. */
export function localDateIso(now: Date = new Date()): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export function calcAgeOnDate(dob: string, asOfDate: string): number {
  const birthDate = new Date(dob);
  const currentDate = new Date(asOfDate);

  if (Number.isNaN(birthDate.getTime())) {
    // Naive year difference, mirroring the old fallback behavior
    // (NaN in, NaN out) without re-entering calcAge.
    return new Date().getUTCFullYear() - birthDate.getUTCFullYear();
  }
  if (Number.isNaN(currentDate.getTime())) {
    // Valid DOB but unusable reference date: keep the birthday-aware
    // logic, anchored to the real-world date.
    return calcAgeOnDate(dob, new Date().toISOString().slice(0, 10));
  }

  let age = currentDate.getUTCFullYear() - birthDate.getUTCFullYear();
  const birthMonth = birthDate.getUTCMonth();
  const birthDay = birthDate.getUTCDate();

  if (
    currentDate.getUTCMonth() < birthMonth ||
    (currentDate.getUTCMonth() === birthMonth && currentDate.getUTCDate() < birthDay)
  ) {
    age -= 1;
  }

  return age;
}

/**
 * Presentation only: the amount is converted by the app's own exchange rate (the game keeps one
 * display currency, chosen in settings), and Intl then places the symbol, groups the digits and
 * picks the compact units the way the player's language does.
 */
function presentMoney(
  value: number,
  options: Pick<Intl.NumberFormatOptions, "notation" | "maximumFractionDigits">,
): string {
  const { currency, language } = getFormattingSettings();
  const converted = convertCurrencyValue(value, currency.exchange_rate);

  return new Intl.NumberFormat(language, {
    style: "currency",
    currency: currency.code,
    currencyDisplay: "narrowSymbol",
    minimumFractionDigits: 0,
    ...options,
  }).format(converted === 0 ? 0 : converted);
}

export function formatExactMoney(value: number): string {
  return presentMoney(value, { maximumFractionDigits: 0 });
}

/**
 * How many decimals a compact amount keeps. Languages whose units step by a thousand (K, M) keep
 * the established rule: thousands whole, millions to one decimal. A language whose unit steps by
 * ten thousand (万) would lose a fifth of €12,345 that way, so it keeps one decimal until the
 * number in front of the unit reaches three digits.
 */
function compactFractionDigits(amount: number, language: string): number {
  if (amount === 0) return 0;
  const digits = new Intl.NumberFormat(language, { notation: "compact", maximumFractionDigits: 3 })
    .formatToParts(amount)
    .filter((part) => part.type === "integer" || part.type === "fraction");
  const integer = digits.filter((part) => part.type === "integer").map((part) => part.value);
  const fraction = digits.filter((part) => part.type === "fraction").map((part) => part.value);
  const scaled = Number(`${integer.join("")}.${fraction.join("") || "0"}`);
  const unitExponent = Math.round(Math.log10(amount / scaled));

  if (unitExponent % 3 === 0) {
    return amount >= 1_000_000 ? 1 : 0;
  }
  return scaled < 100 ? 1 : 0;
}

export function formatVal(value: number): string {
  const { currency, language } = getFormattingSettings();
  const converted = Math.abs(convertCurrencyValue(value, currency.exchange_rate));

  return presentMoney(value, {
    notation: "compact",
    maximumFractionDigits: compactFractionDigits(converted, language),
  });
}

/** Exact on the surfaces where a decision is made, compact in tiles and list columns. */
export type MoneyPrecision = "exact" | "compact";

/** The per-week unit, from the active language; "/wk" only before i18n has started. */
function weeklyUnit(): string {
  const unit = i18n.isInitialized ? i18n.t("finances.perWeekSuffix", { defaultValue: "/wk" }) : "";
  return unit || "/wk";
}

/** Puts the weekly unit on an already formatted amount, for the signed figures a caller builds itself. */
export function formatWeeklyAmount(formattedAmount: string): string {
  return `${formattedAmount}${weeklyUnit()}`;
}

/**
 * A weekly wage. Stored wages are weekly, so there is no annual counterpart: the unit belongs to
 * the formatter, and no caller fetches or drills it.
 */
export function formatWeeklyWage(weeklyWage: number, precision: MoneyPrecision = "exact"): string {
  return formatWeeklyAmount(
    precision === "exact" ? formatExactMoney(weeklyWage) : formatVal(weeklyWage),
  );
}
