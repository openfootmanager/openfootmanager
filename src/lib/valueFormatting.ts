import { useSettingsStore, type AppSettings } from "../store/settingsStore";

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

export function calcAge(dob: string): number {
    return 2026 - new Date(dob).getFullYear();
}

export function calcAgeOnDate(dob: string, asOfDate: string): number {
    const birthDate = new Date(dob);
    const currentDate = new Date(asOfDate);

    if (Number.isNaN(birthDate.getTime()) || Number.isNaN(currentDate.getTime())) {
        return calcAge(dob);
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
