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