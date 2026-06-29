import type { TFunction } from "i18next";

import {
  GameStateData,
  LeagueData,
  PlayerData,
  WorldRegionData,
} from "../store/gameStore";
import { getPlayerOvr } from "../lib/helpers";
import { buildRegionLabel, inferRegionId } from "../lib/teamRegions";

export function competitionScopeLabel(
  t: TFunction,
  scope?: string,
): string | null {
  if (!scope) {
    return null;
  }
  return t(`teamSelect.scopes.${scope}`, { defaultValue: scope });
}

export function competitionKindLabel(
  t: TFunction,
  kind?: string,
): string | null {
  if (!kind) {
    return null;
  }
  return t(`teamSelect.kinds.${kind}`, { defaultValue: kind });
}

export function buildFallbackRegions(
  t: TFunction,
  gameState: GameStateData,
  competitions: LeagueData[],
): WorldRegionData[] {
  const byRegion = new Map<string, Set<string>>();

  for (const team of gameState.teams) {
    const regionId =
      competitions.find((competition) => competition.country_id === team.country)?.region_id ??
      inferRegionId(team.country);
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

export function teamCompetitions(
  teamId: string,
  competitions: LeagueData[],
): LeagueData[] {
  return competitions.filter(
    (competition) =>
      competition.participant_ids?.includes(teamId) ??
      competition.fixtures.some(
        (fixture) => fixture.home_team_id === teamId || fixture.away_team_id === teamId,
      ),
  );
}

export function likelyXi(players: PlayerData[]): PlayerData[] {
  return [...players]
    .sort((left, right) => getPlayerOvr(right) - getPlayerOvr(left))
    .slice(0, 11);
}

export function sortCompetitions(competitions: LeagueData[]): LeagueData[] {
  return [...competitions].sort(
    (left, right) =>
      (left.priority ?? Number.MAX_SAFE_INTEGER) -
        (right.priority ?? Number.MAX_SAFE_INTEGER) ||
      left.name.localeCompare(right.name),
  );
}
