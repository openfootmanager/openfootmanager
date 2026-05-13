import { useSettingsStore, type AppSettings } from "../store/settingsStore";

const CURRENCY_SYMBOLS: Record<AppSettings["currency"], string> = {
    EUR: "€",
    GBP: "£",
    USD: "$",
};

function getFormattingSettings(): Pick<AppSettings, "currency" | "language"> {
    const { settings } = useSettingsStore.getState();
    return {
        currency: settings.currency,
        language: settings.language || "en",
    };
}

export function getCurrencySymbol(
    currency: AppSettings["currency"] = getFormattingSettings().currency,
): string {
    return CURRENCY_SYMBOLS[currency] ?? CURRENCY_SYMBOLS.EUR;
}

function prefixCurrency(
    amount: string,
    value: number,
    currency: AppSettings["currency"],
): string {
    const sign = value < 0 ? "-" : "";
    return `${sign}${getCurrencySymbol(currency)}${amount}`;
}

export function calcAge(dob: string): number {
    return 2026 - new Date(dob).getFullYear();
}

export function formatExactMoney(value: number): string {
    const { currency, language } = getFormattingSettings();
    const absoluteValue = Math.abs(value);

    return prefixCurrency(
        absoluteValue.toLocaleString(language, {
            maximumFractionDigits: 0,
        }),
        value,
        currency,
    );
}

export function formatVal(value: number): string {
    const { currency, language } = getFormattingSettings();
    const absoluteValue = Math.abs(value);

    if (absoluteValue >= 1_000_000) {
        return `${prefixCurrency(
            (absoluteValue / 1_000_000).toLocaleString(language, {
                minimumFractionDigits: 1,
                maximumFractionDigits: 1,
            }),
            value,
            currency,
        )}M`;
    }

    if (absoluteValue >= 1_000) {
        return `${prefixCurrency(
            (absoluteValue / 1_000).toLocaleString(language, {
                maximumFractionDigits: 0,
            }),
            value,
            currency,
        )}K`;
    }

    return prefixCurrency(String(absoluteValue), value, currency);
}

export function formatWeeklyAmount(
    formattedAmount: string,
    weeklySuffix: string,
): string {
    return `${formattedAmount}${weeklySuffix}`;
}
