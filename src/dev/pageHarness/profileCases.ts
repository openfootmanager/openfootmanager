import type { PlayerAdvancedStatsSummary } from "../../components/playerProfile/PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "../../components/playerProfile/PlayerProfile.helpers";
import type { PlayerKnowledge, VisibleWage } from "../../services/scoutingService";
import type { PositionRatings } from "../../store/generated/PositionRatings";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import type { Position } from "../../store/generated/Position";
import type { PlayerData, ScoutedPlayerView } from "../../store/types";
import type { AttributeWeight } from "../../store/generated/AttributeWeight";
import { gameState, positionWeights } from "./fixture.generated";
import type { HarnessProfile } from "./harnessPage";

/** What the backend answers about the one player a profile page is open on. */
export interface ProfileCase {
  playerId: string;
  /** `null`: the answer leaves him out, as Rust does for an id no player has (a retiree). */
  knowledge: PlayerKnowledge | null;
  wage: VisibleWage | null;
  opinion: PotentialOpinion | null;
  positionRatings: PositionRatings | null;
  scoutedReport: ScoutedPlayerView | null;
  /** Set for a player no longer in the world's players: the profile fetches him on his own. */
  retiredPlayer: PlayerData | null;
  /** Latest first, as the backend returns them. Only the club's own player has played. */
  recentMatches: PlayerRecentMatchEntry[];
  seasonStats: PlayerAdvancedStatsSummary;
}

const USER_CLUB = gameState.manager.team_id;
/** The day a scout wrote the report the scouted case carries: before the fixture's clock. */
const SCOUTED_ON = "2031-05-12";

const ofClub = (players: PlayerData[]) => players.filter((player) => player.team_id === USER_CLUB);
const rivals = (players: PlayerData[]) =>
  players.filter((player) => player.team_id !== null && player.team_id !== USER_CLUB);
const freeAgents = (players: PlayerData[]) =>
  players.filter((player) => player.team_id === null && player.position !== "Goalkeeper");

/**
 * The players the cases use, by their place in the generated career (fixed by its seed): an
 * outfield player of the club's own and of a rival, and three outfield free agents, who are the players
 * outside the club's division.
 */
function pickPlayers(players: PlayerData[]) {
  const outfield = (list: PlayerData[]) => list.filter((p) => p.position !== "Goalkeeper");
  const [scouted, unknown, retiree] = freeAgents(players);
  return {
    own: outfield(ofClub(players))[0],
    rival: outfield(rivals(players))[0],
    scouted,
    unknown,
    retiree,
  };
}

const metric = (total: number) => ({ total, per90: null, percentile: null });

/** A season before a minute is played: what every state but the own player's shows. */
const NO_SEASON_YET: PlayerAdvancedStatsSummary = {
  percentileEligible: false,
  peerGroup: null,
  seasonRatingRated: false,
  metrics: {
    shots: metric(0),
    shotsOnTarget: metric(0),
    passes: { completed: 0, attempted: 0, accuracy: null, percentile: null },
    tacklesWon: metric(0),
    interceptions: metric(0),
    foulsCommitted: metric(0),
  },
};

/** The same season with something in every cell, so the layout sees populated stats. */
const A_SEASON: PlayerAdvancedStatsSummary = {
  percentileEligible: false,
  peerGroup: null,
  seasonRatingRated: true,
  metrics: {
    shots: { total: 14, per90: 2.1, percentile: null },
    shotsOnTarget: { total: 7, per90: 1.1, percentile: null },
    passes: { completed: 212, attempted: 251, accuracy: 84.5, percentile: null },
    tacklesWon: { total: 18, per90: 2.7, percentile: null },
    interceptions: { total: 11, per90: 1.6, percentile: null },
    foulsCommitted: { total: 6, per90: 0.9, percentile: null },
  },
};

/** Five matches against the rivals, latest first, with ratings enough for a trend line. */
function fiveMatches(): PlayerRecentMatchEntry[] {
  const clubs = gameState.teams.filter((team) => team.id !== USER_CLUB).slice(0, 5);
  return clubs.map((club, index) => ({
    fixture_id: `harness-match-${index + 1}`,
    date: `2031-06-${String(28 - index * 7).padStart(2, "0")}`,
    competition: "League",
    matchday: 5 - index,
    opponent_team_id: club.id,
    opponent_name: club.name,
    team_goals: index % 3,
    opponent_goals: (index + 1) % 3,
    minutes_played: 90,
    goals: index === 0 ? 1 : 0,
    assists: index === 2 ? 1 : 0,
    shots: 3,
    shots_on_target: 1 + (index % 2),
    rating: 7.4 - index * 0.3,
    rated: true,
  }));
}

/**
 * The wage a scout reports: two significant figures, as `player_knowledge::VisibleWage` rounds it.
 * A free agent has no contract and so no wage in the fixture; the figure is a stand-in, one 220th of
 * his market value, so the layout sees a real number.
 */
const scoutedWage = (player: PlayerData) =>
  Number(Math.round(player.market_value / 220).toPrecision(2));

/**
 * His ratings at his natural position and at each alternate. The numbers are the fixture's own
 * rating and a step below it: enough rows for the layout, not the backend's `ovr_from_attributes`.
 */
function positionRatingsOf(player: PlayerData, source: PositionRatings["source"]): PositionRatings {
  const top = Object.entries(player.attributes)
    .sort(([, a], [, b]) => b - a)
    .slice(0, 3)
    .map(([attribute, value]) => ({ attribute, value }));
  const rows = [player.natural_position, ...player.alternate_positions].map((position, index) => ({
    position: position as Position,
    rating: (player.ovr ?? 60) - index * 3,
    familiarity: index === 0 ? ("natural" as const) : ("can_play" as const),
    deployed: false,
    key_attributes: top as PositionRatings["rows"][number]["key_attributes"],
    // Rust writes the table into the fixture (`position_weights`): the harness holds no copy.
    weights: (positionWeights as Record<string, AttributeWeight[]>)[position] ?? [],
  }));
  return { source, rows };
}

/** A range around his rating, in the shape of an opinion; the band and the certainty are for show. */
function opinionOf(player: PlayerData, source: PotentialOpinion["source"]): PotentialOpinion {
  const ovr = player.ovr ?? 60;
  return {
    low: ovr + 2,
    high: ovr + 8,
    band: "B",
    certainty: "fairly_sure",
    source,
    wonderkid: false,
  };
}

/**
 * The backend's answers about the player a profile is open on, in the state `variant` names, as
 * `ofm_core::player_knowledge` decides them: its own player in full; a division rival's rating but
 * not his wage, condition or potential; a scouted player through his report; a stranger's public
 * facts only; a retiree fetched on his own.
 */
export function profileCase(variant: HarnessProfile): ProfileCase {
  const { own, rival, scouted, unknown, retiree } = pickPlayers(gameState.players);
  const none = {
    opinion: null,
    positionRatings: null,
    scoutedReport: null,
    retiredPlayer: null,
    recentMatches: [],
    seasonStats: NO_SEASON_YET,
  };
  switch (variant) {
    case "own":
      return {
        ...none,
        playerId: own.id,
        knowledge: { kind: "own" },
        wage: { kind: "exact", weekly: own.wage },
        opinion: opinionOf(own, { kind: "coaches" }),
        positionRatings: positionRatingsOf(own, { kind: "live" }),
        recentMatches: fiveMatches(),
        seasonStats: A_SEASON,
      };
    case "rival":
      return {
        ...none,
        playerId: rival.id,
        knowledge: { kind: "known" },
        wage: { kind: "hidden" },
        positionRatings: positionRatingsOf(rival, { kind: "live" }),
      };
    case "scouted":
      return {
        ...none,
        playerId: scouted.id,
        knowledge: { kind: "scouted", scouted_on: SCOUTED_ON, current: true },
        wage: { kind: "approximate", weekly: scoutedWage(scouted) },
        opinion: opinionOf(scouted, {
          kind: "scout",
          scout_name: null,
          scouted_on: SCOUTED_ON,
          current: true,
        }),
        positionRatings: positionRatingsOf(scouted, {
          kind: "report",
          scouted_on: SCOUTED_ON,
          current: true,
        }),
        scoutedReport: {
          player_id: scouted.id,
          scouted_on: SCOUTED_ON,
          attributes: scouted.attributes,
          out_of_date: false,
        },
      };
    case "unknown":
      return {
        ...none,
        playerId: unknown.id,
        knowledge: { kind: "unknown" },
        wage: { kind: "hidden" },
      };
    case "retired":
      // Not among the world's players: an id of its own, so the dashboard has to fetch him.
      return {
        ...none,
        playerId: `${retiree.id}-retired`,
        knowledge: null,
        wage: null,
        retiredPlayer: {
          ...retiree,
          id: `${retiree.id}-retired`,
          retired: true,
          team_id: null,
          // He left from a rival club, as the archive records it.
          movement_history: [
            {
              date: "2031-06-30",
              kind: "retired",
              from_team_id: rival.team_id,
              from_team_name:
                gameState.teams.find((team) => team.id === rival.team_id)?.name ?? null,
              to_team_id: null,
              to_team_name: null,
            },
          ],
        },
      };
  }
}
