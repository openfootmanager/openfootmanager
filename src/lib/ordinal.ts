type Translate = (key: string, options: { n: number }) => string;

const WRITTEN_FORMS = new Set(["one", "two", "few"]);

/**
 * "2nd", "2.º", "2e" ...: the language's own plural rule picks the form and the locale file
 * writes it, so no screen builds an English suffix itself.
 */
export function formatOrdinal(value: number, language: string, translate: Translate): string {
  const category = new Intl.PluralRules(language, { type: "ordinal" }).select(value);
  const form = WRITTEN_FORMS.has(category) ? category : "other";
  return translate(`common.ordinal.${form}`, { n: value });
}
