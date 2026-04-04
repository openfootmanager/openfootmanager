const DAY_IN_MS = 24 * 60 * 60 * 1000;

export function parseGameDate(input: string | null | undefined): Date | null {
    if (!input) {
        return null;
    }

    if (/^\d{4}-\d{2}-\d{2}$/.test(input)) {
        const [year, month, day] = input.split("-").map(Number);
        return new Date(Date.UTC(year, month - 1, day));
    }

    const parsed = new Date(input);
    if (Number.isNaN(parsed.getTime())) {
        return null;
    }

    return new Date(
        Date.UTC(parsed.getUTCFullYear(), parsed.getUTCMonth(), parsed.getUTCDate()),
    );
}

export function formatGameDate(date: Date | null): string | null {
    if (!date) {
        return null;
    }

    return date.toISOString().slice(0, 10);
}

export function addGameDays(date: Date, days: number): Date {
    return new Date(date.getTime() + days * DAY_IN_MS);
}

export function diffGameDays(startDate: Date, endDate: Date): number {
    return Math.round((endDate.getTime() - startDate.getTime()) / DAY_IN_MS);
}

export function positiveGameDayDiff(
    startDate: Date,
    endDate: Date,
): number | null {
    const difference = diffGameDays(startDate, endDate);
    return difference >= 0 ? difference : null;
}

export function getAgeOnDate(
    dateOfBirth: string,
    asOfDate: string,
): number {
    const birthDate = parseGameDate(dateOfBirth);
    const referenceDate = parseGameDate(asOfDate);

    if (!birthDate || !referenceDate) {
        return 0;
    }

    let age = referenceDate.getUTCFullYear() - birthDate.getUTCFullYear();

    if (
        referenceDate.getUTCMonth() < birthDate.getUTCMonth() ||
        (referenceDate.getUTCMonth() === birthDate.getUTCMonth() &&
            referenceDate.getUTCDate() < birthDate.getUTCDate())
    ) {
        age -= 1;
    }

    return age;
}