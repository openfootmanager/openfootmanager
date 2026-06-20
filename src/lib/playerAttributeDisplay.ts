export function getAttributeValueClassName(value: number): string {
    if (value >= 80) {
        return "text-primary-600 dark:text-primary-300";
    }

    if (value >= 60) {
        return "text-accent-700 dark:text-accent-300";
    }

    if (value >= 40) {
        return "text-gray-700 dark:text-gray-300";
    }

    return "text-red-600 dark:text-red-300";
}
