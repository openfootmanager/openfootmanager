import type { TFunction } from "i18next";

import type { GameStateData, LeagueData, WorldRegionData } from "../store/gameStore";
import { buildRegionLabel } from "../lib/teamRegions";

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

export function sortCompetitions(competitions: LeagueData[]): LeagueData[] {
  return [...competitions].sort(
    (left, right) =>
      (left.priority ?? Number.MAX_SAFE_INTEGER) - (right.priority ?? Number.MAX_SAFE_INTEGER) ||
      left.name.localeCompare(right.name),
  );
}
