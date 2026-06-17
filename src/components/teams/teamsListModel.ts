import type { TFunction } from "i18next";

import { getPlayerOvr } from "../../lib/helpers";
import { buildRegionLabel, inferRegionId } from "../../lib/teamRegions";
import type {
  GameStateData,
  LeagueData,
  PlayerData,
  StandingData,
  TeamData,
} from "../../store/gameStore";

export interface TeamCard {
  team: TeamData;
  rosterSize: number;
  avgOvr: number;
  totalValue: number;
  /** 1-based league position, or 0 when the team isn't in a ranked league. */
  leaguePos: number;
  standing: StandingData | null;
}

export interface LeagueGroup {
  id: string;
  name: string;
  teams: TeamCard[];
}

export interface RegionGroup {
  id: string;
  name: string;
  leagues: LeagueGroup[];
  teamCount: number;
}

const UNGROUPED_LEAGUE_ID = "__ungrouped";

function sortStandings(standings: StandingData[]): StandingData[] {
  return [...standings].sort(
    (left, right) =>
      right.points - left.points ||
      right.goals_for -
        right.goals_against -
        (left.goals_for - left.goals_against) ||
      right.goals_for - left.goals_for,
  );
}

/** Domestic leagues only — the competitions clubs are grouped under. */
function domesticLeagues(gameState: GameStateData): LeagueData[] {
  const competitions = (gameState.competitions ?? []).filter(
    (competition) =>
      competition.kind === "League" && competition.scope === "Domestic",
  );
  if (competitions.length > 0) {
    return competitions;
  }
  // Legacy single-league saves: treat game.league as the one domestic league.
  return gameState.league ? [gameState.league] : [];
}

function leagueMemberIds(league: LeagueData): string[] {
  if (league.participant_ids && league.participant_ids.length > 0) {
    return league.participant_ids;
  }
  return league.standings.map((entry) => entry.team_id);
}

/**
 * Group every club under its domestic league and region, computing per-team
 * roster stats and league standing. Builds a single players-by-team index so
 * the work stays O(players + teams) rather than O(teams × players).
 */
export function buildTeamRegionGroups(
  gameState: GameStateData,
  t: TFunction,
): RegionGroup[] {
  const playersByTeam = new Map<string, PlayerData[]>();
  for (const player of gameState.players) {
    if (!player.team_id) {
      continue;
    }
    const roster = playersByTeam.get(player.team_id);
    if (roster) {
      roster.push(player);
    } else {
      playersByTeam.set(player.team_id, [player]);
    }
  }

  const leagueByTeam = new Map<string, LeagueData>();
  const standingByTeam = new Map<string, { pos: number; entry: StandingData }>();
  for (const league of domesticLeagues(gameState)) {
    for (const teamId of leagueMemberIds(league)) {
      if (!leagueByTeam.has(teamId)) {
        leagueByTeam.set(teamId, league);
      }
    }
    sortStandings(league.standings).forEach((entry, index) => {
      if (!standingByTeam.has(entry.team_id)) {
        standingByTeam.set(entry.team_id, { pos: index + 1, entry });
      }
    });
  }

  const toCard = (team: TeamData): TeamCard => {
    const roster = playersByTeam.get(team.id) ?? [];
    const avgOvr =
      roster.length > 0
        ? Math.round(
            roster.reduce((sum, player) => sum + getPlayerOvr(player), 0) /
              roster.length,
          )
        : 0;
    const standing = standingByTeam.get(team.id);
    return {
      team,
      rosterSize: roster.length,
      avgOvr,
      totalValue: roster.reduce((sum, player) => sum + player.market_value, 0),
      leaguePos: standing?.pos ?? 0,
      standing: standing?.entry ?? null,
    };
  };

  // region id -> league id -> group
  const regions = new Map<string, Map<string, LeagueGroup>>();
  const ensureLeague = (
    regionId: string,
    leagueId: string,
    leagueName: string,
  ): LeagueGroup => {
    const leagues = regions.get(regionId) ?? new Map<string, LeagueGroup>();
    regions.set(regionId, leagues);
    const existing = leagues.get(leagueId);
    if (existing) {
      return existing;
    }
    const group: LeagueGroup = { id: leagueId, name: leagueName, teams: [] };
    leagues.set(leagueId, group);
    return group;
  };

  for (const team of gameState.teams) {
    const league = leagueByTeam.get(team.id);
    const regionId =
      league?.region_id ?? inferRegionId(team.country);
    const group = league
      ? ensureLeague(regionId, league.id, league.name)
      : ensureLeague(regionId, UNGROUPED_LEAGUE_ID, t("teams.otherClubs"));
    group.teams.push(toCard(team));
  }

  return Array.from(regions.entries())
    .map(([regionId, leagues]) => {
      const leagueGroups = Array.from(leagues.values())
        .map((group) => ({
          ...group,
          teams: group.teams.sort(
            (left, right) =>
              (left.leaguePos || Number.MAX_SAFE_INTEGER) -
                (right.leaguePos || Number.MAX_SAFE_INTEGER) ||
              left.team.name.localeCompare(right.team.name),
          ),
        }))
        // Ungrouped clubs sink to the bottom of their region.
        .sort((left, right) => {
          if (left.id === UNGROUPED_LEAGUE_ID) return 1;
          if (right.id === UNGROUPED_LEAGUE_ID) return -1;
          return left.name.localeCompare(right.name);
        });
      return {
        id: regionId,
        name: buildRegionLabel(t, regionId),
        leagues: leagueGroups,
        teamCount: leagueGroups.reduce(
          (sum, group) => sum + group.teams.length,
          0,
        ),
      };
    })
    .sort((left, right) => left.name.localeCompare(right.name));
}

/** Case-insensitive filter on club name and city, preserving group structure. */
export function filterTeamRegionGroups(
  groups: RegionGroup[],
  query: string,
): RegionGroup[] {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return groups;
  }
  const matches = (card: TeamCard): boolean =>
    card.team.name.toLowerCase().includes(needle) ||
    card.team.city.toLowerCase().includes(needle);

  return groups
    .map((region) => {
      const leagues = region.leagues
        .map((league) => ({
          ...league,
          teams: league.teams.filter(matches),
        }))
        .filter((league) => league.teams.length > 0);
      return {
        ...region,
        leagues,
        teamCount: leagues.reduce((sum, league) => sum + league.teams.length, 0),
      };
    })
    .filter((region) => region.leagues.length > 0);
}
