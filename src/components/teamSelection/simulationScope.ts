import type { LeagueData, TeamData, WorldRegionData } from "../../store/gameStore";

/**
 * The regions a competition needs simulated: those it names, and its own region for a domestic or
 * regional one. Sorted, without repeats.
 */
export function competitionRequiredRegions(competition: LeagueData): string[] {
  const regionIds = new Set(competition.required_region_ids ?? []);
  if (
    (competition.scope === "Domestic" || competition.scope === "Regional") &&
    competition.region_id
  ) {
    regionIds.add(competition.region_id);
  }
  return Array.from(regionIds).sort();
}

/**
 * What the simulation dialog edits, as a draft until Apply: where the career starts, the regions the
 * player switched on, and each competition's remembered choice (a competition keeps its choice while
 * its region is off, and is simulated again when the region is back).
 */
export interface ScopeDraft {
  homeRegionId: string | null;
  homeCountryCode: string | null;
  regions: Record<string, boolean>;
  competitions: Record<string, boolean>;
}

/** Why a region cannot be switched off: the career starts in it, or one of the club's competitions needs it. */
export type RegionLock = "home" | "yourClub";

/**
 * The regions that are always simulated: the home region, and every region one of the chosen club's
 * competitions needs. With no club chosen there is only the home region.
 */
export function lockedRegions(
  homeRegionId: string | null,
  yourCompetitions: readonly LeagueData[],
): Map<string, RegionLock> {
  const locks = new Map<string, RegionLock>();
  for (const competition of yourCompetitions) {
    for (const regionId of competitionRequiredRegions(competition)) {
      locks.set(regionId, "yourClub");
    }
  }
  if (homeRegionId) locks.set(homeRegionId, "home");
  return locks;
}

/**
 * The regions simulated: the locked ones and those switched on, sorted. A region need not be one
 * the world lists a club in: a competition may need one that has none, and "Add" turns it on.
 */
export function activeRegionIds(
  draft: ScopeDraft,
  locks: ReadonlyMap<string, RegionLock>,
): string[] {
  const switchedOn = Object.entries(draft.regions)
    .filter(([, on]) => on)
    .map(([regionId]) => regionId);
  return Array.from(new Set([...locks.keys(), ...switchedOn])).sort();
}

export interface CompetitionState {
  /** Simulated: its regions are all on, and it is the club's or the player's choice. */
  on: boolean;
  /** One of the chosen club's competitions: always simulated, never a choice. */
  yours: boolean;
  /** The regions it needs that are off. */
  missingRegionIds: string[];
  /** It needs a region that is off, so it cannot be chosen until that region is on. */
  blocked: boolean;
}

export function competitionState(
  competition: LeagueData,
  draft: ScopeDraft,
  yourCompetitionIds: ReadonlySet<string>,
  active: ReadonlySet<string>,
): CompetitionState {
  const yours = yourCompetitionIds.has(competition.id);
  const missingRegionIds = competitionRequiredRegions(competition).filter(
    (regionId) => !active.has(regionId),
  );
  const blocked = missingRegionIds.length > 0;
  return {
    on: !blocked && (yours || draft.competitions[competition.id] === true),
    yours,
    missingRegionIds,
    blocked,
  };
}

/** The competition ids the simulation runs: what `select_team` is sent. A blocked one is left out. */
export function enabledCompetitionIds(
  competitions: readonly LeagueData[],
  draft: ScopeDraft,
  yourCompetitionIds: ReadonlySet<string>,
  active: ReadonlySet<string>,
): string[] {
  return competitions
    .filter((competition) => competitionState(competition, draft, yourCompetitionIds, active).on)
    .map((competition) => competition.id);
}

/** The order the dialog lists competitions in; a scope it does not know follows them. */
const SCOPE_ORDER = ["Domestic", "Regional", "Continental", "International"];

export interface CompetitionGroup {
  scope: string;
  competitions: LeagueData[];
}

/** The competitions by scope (domestic first), each group in the order given, empty groups left out. */
export function groupCompetitions(competitions: readonly LeagueData[]): CompetitionGroup[] {
  const byScope = new Map<string, LeagueData[]>();
  for (const competition of competitions) {
    const scope = competition.scope ?? "Domestic";
    byScope.set(scope, [...(byScope.get(scope) ?? []), competition]);
  }
  const rank = (scope: string) => {
    const index = SCOPE_ORDER.indexOf(scope);
    return index === -1 ? SCOPE_ORDER.length : index;
  };
  return Array.from(byScope.entries())
    .sort(([left], [right]) => rank(left) - rank(right))
    .map(([scope, grouped]) => ({ scope, competitions: grouped }));
}

/** The clubs in the regions simulated, for the dialog's footer. */
export function clubsInRegions(
  regions: readonly WorldRegionData[],
  activeIds: ReadonlySet<string>,
  teams: readonly TeamData[],
): number {
  const countries = new Set(
    regions.filter((region) => activeIds.has(region.id)).flatMap((region) => region.country_codes),
  );
  return teams.filter((team) => countries.has(team.country)).length;
}
