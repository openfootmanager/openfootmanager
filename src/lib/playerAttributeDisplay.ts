type AttributeBarVariant = "success" | "accent" | "danger";

interface AttributeColors {
  barVariant: AttributeBarVariant;
  textClass: string;
}

/** Returns bar variant and text class from a single threshold table (70/40), matching ProgressBar's "auto" thresholds. */
export function getAttributeColors(value: number): AttributeColors {
  if (value >= 70) {
    return { barVariant: "success", textClass: "text-success-600 dark:text-success-400" };
  }
  if (value >= 40) {
    return { barVariant: "accent", textClass: "text-accent-700 dark:text-accent-300" };
  }
  return { barVariant: "danger", textClass: "text-red-600 dark:text-red-300" };
}

export function getAttributeValueClassName(value: number): string {
  return getAttributeColors(value).textClass;
}
