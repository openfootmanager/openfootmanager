export type LocaleTree = Record<string, unknown>;

export function collectMissingKeys(
    reference: LocaleTree,
    candidate: LocaleTree,
    path: string[] = [],
): string[] {
    return Object.entries(reference).flatMap(([key, value]) => {
        const nextPath = [...path, key];
        const candidateValue = candidate[key];

        if (value !== null && typeof value === "object" && !Array.isArray(value)) {
            if (
                candidateValue === null ||
                typeof candidateValue !== "object" ||
                Array.isArray(candidateValue)
            ) {
                return [nextPath.join(".")];
            }

            return collectMissingKeys(
                value as LocaleTree,
                candidateValue as LocaleTree,
                nextPath,
            );
        }

        return candidateValue == null || typeof candidateValue !== "string"
            ? [nextPath.join(".")]
            : [];
    });
}

export function hasLocaleKey(locale: LocaleTree, keyPath: string): boolean {
    const segments = keyPath.split(".");
    let current: unknown = locale;

    for (const segment of segments) {
        if (
            current === null ||
            typeof current !== "object" ||
            Array.isArray(current) ||
            !(segment in current)
        ) {
            return false;
        }

        current = (current as LocaleTree)[segment];
    }

    return typeof current === "string";
}
