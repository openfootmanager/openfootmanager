type TranslateFn = (key: string, options?: Record<string, unknown>) => string;

/** The fields needed to render a competition's display name. */
export interface NamedCompetition {
  name?: string;
  name_key?: string | null;
  season?: number;
  country_id?: string | null;
  division_tier?: number | null;
  calendar?: {
    season?: {
      division?: {
        tier?: number | null;
        phase?: "annual" | "opening" | "closing";
      } | null;
    } | null;
  } | null;
}

/**
 * Display name for a competition. When a `name_key` is present it is translated
 * with the competition's season and nation interpolated, so generic templates
 * like `"{{country}} First Division"` render as "Argentina First Division".
 * Falls back to the raw `name` when there is no key.
 */
export function competitionDisplayName(comp: NamedCompetition, t: TranslateFn): string {
  if (!comp.name_key) {
    return comp.name ?? "";
  }
  const country = comp.country_id
    ? t(`nations.${comp.country_id.toLowerCase()}`, {
        defaultValue: comp.country_id,
      })
    : "";
  const division = comp.calendar?.season?.division;
  const tier = comp.division_tier ?? division?.tier;
  const hasInvalidTier = comp.name_key.endsWith(".tierDivision") && (!tier || tier < 1);
  // Keep using name_key even without a country (e.g. the World Cup, which has a
  // key but no country_id) so its localized name still resolves. A missing key
  // or an absent `{{country}}` value falls back to the stored name and trims the
  // stray space a blank country would otherwise leave.
  const displayName = hasInvalidTier
    ? (comp.name ?? comp.name_key)
    : t(comp.name_key, {
        year: comp.season,
        country,
        tier,
        defaultValue: comp.name ?? comp.name_key,
      }).trim();
  const phase = division?.phase;
  if (phase === "opening" || phase === "closing") {
    return t(`tournaments.competitions.splitSeason.${phase}`, {
      competition: displayName,
      defaultValue: displayName,
    }).trim();
  }
  return displayName;
}
