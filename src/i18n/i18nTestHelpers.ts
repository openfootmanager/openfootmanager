export type LocaleTree = Record<string, unknown>;

type LeafResult = string[];

function traverseLocaleTree(
    reference: LocaleTree,
    candidate: LocaleTree,
    path: string[],
    onLeaf: (
        key: string,
        refValue: unknown,
        candidateValue: unknown,
        path: string[],
    ) => LeafResult,
): LeafResult {
    return Object.entries(reference).flatMap(([key, value]) => {
        const nextPath = [...path, key];
        const candidateValue = candidate[key];

        if (value !== null && typeof value === "object" && !Array.isArray(value)) {
            if (
                candidateValue !== null &&
                typeof candidateValue === "object" &&
                !Array.isArray(candidateValue)
            ) {
                return traverseLocaleTree(
                    value as LocaleTree,
                    candidateValue as LocaleTree,
                    nextPath,
                    onLeaf,
                );
            }
            return onLeaf(key, value, candidateValue, nextPath);
        }

        return onLeaf(key, value, candidateValue, nextPath);
    });
}

export function collectMissingKeys(
    reference: LocaleTree,
    candidate: LocaleTree,
    path: string[] = [],
): string[] {
    return traverseLocaleTree(reference, candidate, path, (_key, value, candidateValue, nextPath) => {
        if (value !== null && typeof value === "object" && !Array.isArray(value)) {
            return [nextPath.join(".")];
        }
        return candidateValue == null || typeof candidateValue !== "string"
            ? [nextPath.join(".")]
            : [];
    });
}

export function collectUntranslatedKeys(
    reference: LocaleTree,
    candidate: LocaleTree,
    path: string[] = [],
): string[] {
    return traverseLocaleTree(reference, candidate, path, (_key, value, candidateValue, nextPath) => {
        if (value !== null && typeof value === "object" && !Array.isArray(value)) {
            return [];
        }
        return typeof value === "string" &&
            typeof candidateValue === "string" &&
            candidateValue === value
            ? [nextPath.join(".")]
            : [];
    });
}

/**
 * Keys the candidate locale carries that `en.json` does not — the direction
 * `collectMissingKeys` cannot see. A key here is dead weight at best: a
 * translator keeps maintaining a string nothing renders, and a rename that
 * leaves the old key behind still looks translated long after it stopped
 * being reachable.
 */
export function collectOrphanKeys(
    reference: LocaleTree,
    candidate: LocaleTree,
    path: string[] = [],
): string[] {
    return Object.entries(candidate).flatMap(([key, value]) => {
        const nextPath = [...path, key];
        const referenceValue = reference[key];

        if (value !== null && typeof value === "object" && !Array.isArray(value)) {
            if (
                referenceValue !== null &&
                typeof referenceValue === "object" &&
                !Array.isArray(referenceValue)
            ) {
                return collectOrphanKeys(
                    referenceValue as LocaleTree,
                    value as LocaleTree,
                    nextPath,
                );
            }

            // English has no table here, so every leaf beneath it is orphaned.
            return collectOrphanKeys({}, value as LocaleTree, nextPath);
        }

        return referenceValue === undefined ? [nextPath.join(".")] : [];
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
