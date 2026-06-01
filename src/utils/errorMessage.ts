type TranslateFn = (
    key: string,
    params?: Record<string, string | number>,
) => string;

function extractErrorMessage(error: unknown): string {
    if (typeof error === "string") {
        return error;
    }

    if (error instanceof Error) {
        return error.message;
    }

    if (
        error &&
        typeof error === "object" &&
        "message" in error &&
        typeof (error as { message?: unknown }).message === "string"
    ) {
        return (error as { message: string }).message;
    }

    return String(error);
}

function parseEncodedErrorMessage(message: string): {
    key: string;
    params?: Record<string, string>;
} | null {
    const trimmed = message.trim();
    const separatorIndex = trimmed.indexOf("?");
    const key = separatorIndex === -1 ? trimmed : trimmed.slice(0, separatorIndex);

    if (!key.includes(".")) {
        return null;
    }

    return {
        key,
        params: separatorIndex === -1
            ? undefined
            : Object.fromEntries(new URLSearchParams(trimmed.slice(separatorIndex + 1)).entries()),
    };
}

export function getErrorMessage(error: unknown): string {
    return extractErrorMessage(error);
}

export function resolveTranslatedErrorMessage(
    error: unknown,
    t: TranslateFn,
): string {
    const message = extractErrorMessage(error);
    const parsed = parseEncodedErrorMessage(message);

    if (!parsed) {
        return message;
    }

    const translated = t(parsed.key, parsed.params);
    return translated === parsed.key ? message : translated;
}