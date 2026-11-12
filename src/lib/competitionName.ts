import type { CompetitionSeasonPhase } from "../store/generated/CompetitionSeasonPhase";

type TranslateFn = (key: string, options?: Record<string, unknown>) => string;
type NamedDivisionIdentity = {
  familyId?: string;
  tier?: number | null;
  phase?: CompetitionSeasonPhase;
};

/** The fields needed to render a competition's display name. */
export interface NamedCompetition {
  id?: string;
  name?: string;
  name_key?: string | null;
  season?: number;
  country_id?: string | null;
  division_tier?: number | null;
  calendar?: {
    definition_id?: string;
    season?: {
      division?: NamedDivisionIdentity | null;
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
  const division = comp.calendar?.season?.division;
  const nameKey = comp.name_key ?? legacySplitSeasonNameKey(comp, division);
  if (!nameKey) {
    return comp.name ?? "";
  }
  const country = comp.country_id
    ? t(`nations.${comp.country_id.toLowerCase()}`, {
        defaultValue: comp.country_id,
      })
    : "";
  const tier = comp.division_tier ?? division?.tier;
  if (nameKey.endsWith(".tierDivision") && (!tier || tier < 1)) {
    return comp.name ?? "";
  }
  // Keep using name_key even without a country (e.g. the World Cup, which has a
  // key but no country_id) so its localized name still resolves. A missing key
  // or an absent `{{country}}` value falls back to the stored name and trims the
  // stray space a blank country would otherwise leave.
  const localizedName = t(nameKey, {
    year: comp.season,
    country,
    tier,
    defaultValue: comp.name ?? "",
  }).trim();
  const hasTierTemplate = [
    "tournaments.competitions.league",
    "tournaments.competitions.firstDivision",
    "tournaments.competitions.secondDivision",
    "tournaments.competitions.tierDivision",
  ].includes(nameKey);
  const phaseKey = hasTierTemplate
    ? division?.phase === "opening"
      ? "tournaments.competitions.phaseOpening"
      : division?.phase === "closing"
        ? "tournaments.competitions.phaseClosing"
        : undefined
    : undefined;
  return phaseKey ? `${localizedName} ${t(phaseKey)}`.trim() : localizedName;
}

function legacySplitSeasonNameKey(
  comp: NamedCompetition,
  division: NamedDivisionIdentity | null | undefined,
): string | undefined {
  const definitionId = comp.calendar?.definition_id;
  const countrySlug = comp.country_id?.toLowerCase();
  if (
    !countrySlug ||
    division?.familyId !== `${countrySlug}-league` ||
    (!definitionId?.endsWith("-apertura") && !definitionId?.endsWith("-clausura")) ||
    (division.phase !== "opening" && division.phase !== "closing")
  ) {
    return undefined;
  }

  if (division.tier === 1) {
    return comp.name?.includes(" League ")
      ? "tournaments.competitions.league"
      : "tournaments.competitions.firstDivision";
  }
  return division.tier === 2
    ? "tournaments.competitions.secondDivision"
    : "tournaments.competitions.tierDivision";
}

/** Resolve only the identified competition; names are not unique. */
export function localizeCompetitionName(
  name: string,
  competitions: readonly NamedCompetition[] | undefined,
  t: TranslateFn,
  competitionId?: string | null,
): string {
  const competition = competitionId
    ? competitions?.find((item) => item.id === competitionId)
    : undefined;
  return competition ? competitionDisplayName(competition, t) : name;
}
