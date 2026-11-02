import type { TFunction } from "i18next";

import type {
  GamePlayerData,
  GameStateData,
  LeagueData,
  WorldRegionData,
} from "../store/gameStore";
import { buildRegionLabel } from "../lib/teamRegions";

export function competitionScopeLabel(t: TFunction, scope?: string): string | null {
  if (!scope) {
    return null;
  }
  return t(`teamSelect.scopes.${scope}`, { defaultValue: scope });
}

export function competitionKindLabel(t: TFunction, kind?: string): string | null {
  if (!kind) {
    return null;
  }
  return t(`teamSelect.kinds.${kind}`, { defaultValue: kind });
}

/**
 * The regions a new career can be scoped to: each region a competition declares for a club's
 * country, with the countries it covers. The game itself carries no region list.
 */
export function regionsFromCompetitions(
  t: TFunction,
  gameState: GameStateData,
  competitions: LeagueData[],
): WorldRegionData[] {
  const byRegion = new Map<string, Set<string>>();

  for (const team of gameState.teams) {
    // Only a region an authored competition declares: guessing one from the country code filed
    // every club the guess did not know under Europe.
    const regionId = competitions.find(
      (competition) => competition.country_id === team.country,
    )?.region_id;
    if (!regionId) {
      continue;
    }

    const existing = byRegion.get(regionId) ?? new Set<string>();
    existing.add(team.country);
    byRegion.set(regionId, existing);
  }

  return Array.from(byRegion.entries())
    .map(([id, countryCodes]) => ({
      id,
      name: buildRegionLabel(t, id),
      country_codes: Array.from(countryCodes).sort(),
    }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

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

export function teamCompetitions(teamId: string, competitions: LeagueData[]): LeagueData[] {
  return competitions.filter(
    (competition) =>
      competition.participant_ids?.includes(teamId) ??
      competition.fixtures.some(
        (fixture) => fixture.home_team_id === teamId || fixture.away_team_id === teamId,
      ),
  );
}

/** A rating for sorting best first: a hidden one sorts last, below every known rating. */
function sortableRating(player: GamePlayerData): number {
  return player.ovr ?? Number.NEGATIVE_INFINITY;
}

/** The eleven best-rated players, best first; a player whose rating is hidden comes last. */
export function likelyXi<P extends GamePlayerData>(players: P[]): P[] {
  return [...players]
    .sort((left, right) => {
      const leftRating = sortableRating(left);
      const rightRating = sortableRating(right);
      if (leftRating === rightRating) return 0;
      return rightRating > leftRating ? 1 : -1;
    })
    .slice(0, 11);
}

export function sortCompetitions(competitions: LeagueData[]): LeagueData[] {
  return [...competitions].sort(
    (left, right) =>
      (left.priority ?? Number.MAX_SAFE_INTEGER) - (right.priority ?? Number.MAX_SAFE_INTEGER) ||
      left.name.localeCompare(right.name),
  );
}
