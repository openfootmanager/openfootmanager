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

export function getCurrencySymbol(
    currency: AppSettings["currency"] = getFormattingSettings().currency.code,
): string {
    const { supportedCurrencies } = useSettingsStore.getState();
    return (
        supportedCurrencies[currency]?.symbol
        ?? getFormattingSettings().currency.symbol
        ?? "€"
    );
}

function convertCurrencyValue(
    value: number,
    exchangeRate?: number,
): number {
    const resolvedExchangeRate =
        exchangeRate ?? getFormattingSettings().currency.exchange_rate;
    return Math.round(value * resolvedExchangeRate);
}

function prefixCurrency(
    amount: string,
    value: number,
    currency: AppSettings["currency"],
): string {
    const sign = value < 0 ? "-" : "";
    return `${sign}${getCurrencySymbol(currency)}${amount}`;
}

/**
 * Age as of the current in-game date (falls back to the real-world date when
 * no game is loaded). Previously hardcoded to `2026 - birth year`, which froze
 * every displayed age at the starting season as the game clock advanced.
 */
export function calcAge(dob: string): number {
    const gameDate = useGameStore.getState().gameState?.clock?.current_date;
    const asOfDate = gameDate ?? new Date().toISOString().slice(0, 10);
    return calcAgeOnDate(dob, asOfDate);
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
        currentDate.getUTCMonth() < birthMonth
        || (currentDate.getUTCMonth() === birthMonth && currentDate.getUTCDate() < birthDay)
    ) {
        age -= 1;
    }

    return age;
}

export function formatExactMoney(value: number): string {
    const { currency, language } = getFormattingSettings();
    const absoluteValue = convertCurrencyValue(
        Math.abs(value),
        currency.exchange_rate,
    );

    return prefixCurrency(
        absoluteValue.toLocaleString(language, {
            maximumFractionDigits: 0,
        }),
        value,
        currency.code,
    );
}

export function formatVal(value: number): string {
    const { currency, language } = getFormattingSettings();
    const absoluteValue = convertCurrencyValue(
        Math.abs(value),
        currency.exchange_rate,
    );

    if (absoluteValue >= 1_000_000) {
        return `${prefixCurrency(
            (absoluteValue / 1_000_000).toLocaleString(language, {
                minimumFractionDigits: 1,
                maximumFractionDigits: 1,
            }),
            value,
            currency.code,
        )}M`;
    }

    if (absoluteValue >= 1_000) {
        return `${prefixCurrency(
            (absoluteValue / 1_000).toLocaleString(language, {
                maximumFractionDigits: 0,
            }),
            value,
            currency.code,
        )}K`;
    }

    return prefixCurrency(
        absoluteValue.toLocaleString(language, { maximumFractionDigits: 0 }),
        value,
        currency.code,
    );
}

export function formatWeeklyAmount(
    formattedAmount: string,
    weeklySuffix: string,
): string {
    return `${formattedAmount}${weeklySuffix}`;
}

export function formatAnnualAmount(
    formattedAmount: string,
    annualSuffix: string,
): string {
    return `${formattedAmount}${annualSuffix}`;
}
