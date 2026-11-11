import type { PlayerAdvancedStatsSummary } from "../../components/playerProfile/PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "../../components/playerProfile/PlayerProfile.helpers";
import type { PlayerKnowledge, VisibleWage } from "../../services/scoutingService";
import type { PositionRatings } from "../../store/generated/PositionRatings";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import type { Position } from "../../store/generated/Position";
import type { PlayerData, PlayerMovementEntry, ScoutedPlayerView } from "../../store/types";
import type { AttributeKey } from "../../store/generated/AttributeKey";
import type { AttributeWeight } from "../../store/generated/AttributeWeight";
import { gameState, positionKeyAttributes, positionWeights } from "./fixture.generated";
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
  /** Laid over the world's player of this id, to show a state the generated career has none of. */
  playerPatch?: Partial<PlayerData>;
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
  const [scouted, unknown, retiree, freeAgent] = freeAgents(players);
  const club = outfield(ofClub(players));
  if (club.length < 3) {
    throw new Error(
      `the generated career's club has ${club.length} outfield players; the own, injured and loaned cases need three`,
    );
  }
  return {
    own: club[0],
    injured: club[1],
    loaned: club[2],
    keeper: ofClub(players).find((p) => p.position === "Goalkeeper") ?? ofClub(players)[0],
    rival: outfield(rivals(players))[0],
    scouted,
    unknown,
    freeAgent,
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
  const rows = [player.natural_position, ...player.alternate_positions].map((position, index) => ({
    position: position as Position,
    rating: (player.ovr ?? 60) - index * 3,
    familiarity: index === 0 ? ("natural" as const) : ("can_play" as const),
    deployed: false,
    // Rust writes the game's three key attributes per position into the fixture
    // (`key_attributes`); each carries this player's own value.
    key_attributes: ((positionKeyAttributes as Record<string, AttributeKey[]>)[position] ?? []).map(
      (attribute) => ({ attribute, value: player.attributes[attribute] }),
    ),
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

/** A club as a movement entry names it: its id and its name, as the ledger stores both. */
function clubRef(teamId: string | null | undefined) {
  return {
    id: teamId ?? null,
    name: gameState.teams.find((team) => team.id === teamId)?.name ?? null,
  };
}

/**
 * The own player's contracts: signed from a rival for a fee, then renewed into the one he has now
 * (its end and wage are his own, so the history and the profile agree about the contract in force).
 */
function ownHistory(player: PlayerData, fromClub: string | null): PlayerMovementEntry[] {
  const from = clubRef(fromClub);
  const club = clubRef(USER_CLUB);
  return [
    {
      date: "2029-07-01",
      kind: "permanent_transfer",
      from_team_id: from.id,
      from_team_name: from.name,
      to_team_id: club.id,
      to_team_name: club.name,
      fee: 1_250_000,
      contract: {
        start: "2029-07-01",
        end: "2031-06-30",
        weekly_wage: Math.round(player.wage * 0.7),
        source: "transfer",
      },
    },
    {
      date: "2031-07-01",
      kind: "renewal",
      to_team_id: club.id,
      to_team_name: club.name,
      contract: {
        start: player.contract_start ?? "2031-07-01",
        end: player.contract_end,
        weekly_wage: player.wage,
        source: "renewal",
      },
    },
  ];
}

/**
 * A player loaned out: a contract from a save made before starts were recorded (so its start is an
 * honest unknown), and the loan that began this season.
 */
function loanedHistory(player: PlayerData, loanClub: string | null): PlayerMovementEntry[] {
  const parent = clubRef(USER_CLUB);
  const borrower = clubRef(loanClub);
  return [
    {
      date: "2029-07-01",
      kind: "initial_contract",
      to_team_id: parent.id,
      to_team_name: parent.name,
      contract: {
        start: null,
        end: player.contract_end,
        weekly_wage: player.wage,
        source: "legacy_migrated",
      },
    },
    {
      date: "2031-07-01",
      kind: "loan_start",
      from_team_id: parent.id,
      from_team_name: parent.name,
      to_team_id: borrower.id,
      to_team_name: borrower.name,
      loan_end_date: "2032-06-30",
    },
  ];
}

/**
 * A rival's player: signed for a fee and renewed. His dates and fee are public; his wage is in the
 * ledger too, and stays hidden because the knowledge answer says so, not because it is left out.
 */
function rivalHistory(player: PlayerData, fromClub: string | null): PlayerMovementEntry[] {
  const from = clubRef(fromClub);
  const club = clubRef(player.team_id);
  return [
    {
      date: "2028-07-01",
      kind: "permanent_transfer",
      from_team_id: from.id,
      from_team_name: from.name,
      to_team_id: club.id,
      to_team_name: club.name,
      fee: 800_001,
      contract: {
        start: "2028-07-01",
        end: "2030-06-30",
        weekly_wage: Math.round(player.wage * 0.8),
        source: "transfer",
      },
    },
    {
      date: "2030-07-01",
      kind: "renewal",
      to_team_id: club.id,
      to_team_name: club.name,
      contract: {
        start: "2030-07-01",
        end: player.contract_end,
        weekly_wage: player.wage,
        source: "renewal",
      },
    },
  ];
}

/** A free agent whose last contract ran out on its end date: he was released when it did. */
function freeAgentHistory(player: PlayerData, lastClub: string | null): PlayerMovementEntry[] {
  const from = clubRef(lastClub);
  return [
    {
      date: "2028-07-01",
      kind: "permanent_transfer",
      from_team_id: null,
      from_team_name: null,
      to_team_id: from.id,
      to_team_name: from.name,
      fee: 500_000,
      contract: {
        start: "2028-07-01",
        end: "2031-06-30",
        weekly_wage: Math.max(1, Math.round(player.market_value / 220)),
        source: "transfer",
      },
    },
    {
      date: "2031-06-30",
      kind: "released",
      from_team_id: from.id,
      from_team_name: from.name,
      to_team_id: null,
      to_team_name: null,
      release_reason: "expired",
    },
  ];
}

/** The club's own player as the club sees him: known, his wage exact, with ratings and a season. */
function ownCase(player: PlayerData, signedFrom: string | null): ProfileCase {
  return {
    playerId: player.id,
    knowledge: { kind: "own" },
    wage: { kind: "exact", weekly: player.wage },
    opinion: opinionOf(player, { kind: "coaches" }),
    positionRatings: positionRatingsOf(player, { kind: "live" }),
    scoutedReport: null,
    retiredPlayer: null,
    recentMatches: fiveMatches(),
    seasonStats: A_SEASON,
    playerPatch: { movement_history: ownHistory(player, signedFrom) },
  };
}

/**
 * The backend's answers about the player a profile is open on, in the state `variant` names, as
 * `ofm_core::player_knowledge` decides them: its own player in full; a division rival's rating but
 * not his wage, condition or potential; a scouted player through his report; a stranger's public
 * facts only; a retiree fetched on his own.
 */
export function profileCase(variant: HarnessProfile): ProfileCase {
  const { own, keeper, injured, loaned, rival, scouted, unknown, freeAgent, retiree } = pickPlayers(
    gameState.players,
  );
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
        playerPatch: { movement_history: ownHistory(own, rival.team_id) },
      };
    case "keeper":
      return {
        ...none,
        playerId: keeper.id,
        knowledge: { kind: "own" },
        wage: { kind: "exact", weekly: keeper.wage },
        opinion: opinionOf(keeper, { kind: "coaches" }),
        positionRatings: positionRatingsOf(keeper, { kind: "live" }),
        recentMatches: fiveMatches(),
        seasonStats: A_SEASON,
      };
    case "injured":
      return {
        ...ownCase(injured, rival.team_id),
        // A knee injury with twelve days to go: the profile's injury banner.
        playerPatch: {
          movement_history: ownHistory(injured, rival.team_id),
          injury: { name: "common.injuries.kneeBruise", days_remaining: 12 },
        },
      };
    case "loaned":
      return {
        ...ownCase(loaned, rival.team_id),
        // The coaches watch a player the club has loaned out no longer, so no opinion of him
        // (`player_knowledge::trains_with_club_since`).
        opinion: null,
        // On loan from the user's club to a rival: the profile's loan banner.
        playerPatch: {
          movement_history: loanedHistory(loaned, rival.team_id),
          team_id: rival.team_id,
          active_loan: {
            parent_team_id: USER_CLUB,
            loan_team_id: rival.team_id ?? "",
            start_date: "2031-07-01",
            end_date: "2032-06-30",
            wage_contribution_pct: 50,
            buy_option_fee: null,
          },
        },
      };
    case "rival":
      return {
        ...none,
        playerId: rival.id,
        knowledge: { kind: "known" },
        wage: { kind: "hidden" },
        positionRatings: positionRatingsOf(rival, { kind: "live" }),
        playerPatch: { movement_history: rivalHistory(rival, own.team_id) },
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
    case "freeAgent":
      return {
        ...none,
        playerId: freeAgent.id,
        knowledge: { kind: "unknown" },
        wage: { kind: "hidden" },
        playerPatch: { movement_history: freeAgentHistory(freeAgent, rival.team_id) },
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
