import { useSettingsStore } from "../store/settingsStore";

const LANG_LOCALE: Record<string, string> = {
    en: "en-US",
    es: "es-ES",
    pt: "pt-BR",
    fr: "fr-FR",
    de: "de-DE",
    it: "it-IT",
    zh: "zh-CN",
};

export function getLocale(lang?: string): string {
    if (!lang) {
        return "en-US";
    }
    return LANG_LOCALE[lang] || lang;
}

function parseDateInput(dateStr: string): Date | null {
    const dateOnly = dateStr.substring(0, 10);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(dateOnly)) {
        return null;
    }
    const value = new Date(`${dateOnly}T12:00:00`);
    if (Number.isNaN(value.getTime())) {
        return null;
    }
    return value;
}

export function formatMatchDate(dateStr: string, locale?: string): string {
    const date = parseDateInput(dateStr);
    if (!date) {
        return dateStr;
    }
    const resolvedLocale = locale ?? useSettingsStore.getState().settings.language;
    return date.toLocaleDateString(getLocale(resolvedLocale), {
        weekday: "short",
        month: "short",
        day: "numeric",
    });
}

export function formatDate(
    dateStr: string,
    locale?: string,
    opts?: Intl.DateTimeFormatOptions,
): string {
    const date = parseDateInput(dateStr);
    if (!date) {
        return dateStr;
    }
    return date.toLocaleDateString(
        getLocale(locale),
        opts || { year: "numeric", month: "long", day: "numeric" },
    );
}

export function formatDateFull(dateStr: string, locale?: string): string {
    const date = parseDateInput(dateStr);
    if (!date) {
        return dateStr;
    }
    return date.toLocaleDateString(getLocale(locale), {
        weekday: "long",
        year: "numeric",
        month: "long",
        day: "numeric",
    });
}

export function formatDateShort(dateStr: string, locale?: string): string {
    const date = parseDateInput(dateStr);
    if (!date) {
        return dateStr;
    }
    return date.toLocaleDateString(getLocale(locale), {
        month: "short",
        day: "numeric",
    });
}
