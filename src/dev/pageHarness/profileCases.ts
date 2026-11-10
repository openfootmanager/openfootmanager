import type { PlayerKnowledge, VisibleWage } from "../../services/scoutingService";
import type { PositionRatings } from "../../store/generated/PositionRatings";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import type { Position } from "../../store/generated/Position";
import type { PlayerData, ScoutedPlayerView } from "../../store/types";
import { gameState } from "./fixture.generated";
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
  const none = { opinion: null, positionRatings: null, scoutedReport: null, retiredPlayer: null };
  switch (variant) {
    case "own":
      return {
        ...none,
        playerId: own.id,
        knowledge: { kind: "own" },
        wage: { kind: "exact", weekly: own.wage },
        opinion: opinionOf(own, { kind: "coaches" }),
        positionRatings: positionRatingsOf(own, { kind: "live" }),
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
        retiredPlayer: { ...retiree, id: `${retiree.id}-retired`, retired: true, team_id: null },
      };
  }
}
