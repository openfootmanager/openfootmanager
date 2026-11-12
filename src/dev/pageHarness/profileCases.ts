import type { PlayerAdvancedStatsSummary } from "../../components/playerProfile/PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "../../components/playerProfile/PlayerProfile.helpers";
import type {
  ContractTerminationPreviewData,
  RenewalProjection,
} from "../../services/contractService";
import type { PlayerKnowledge, VisibleWage } from "../../services/scoutingService";
import type { CareerLine } from "../../store/generated/CareerLine";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
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
  /** His career as the backend projects it; `null` where it answers null (a retiree). */
  career: PlayerCareer | null;
  /** Laid over the world's player of this id, to show a state the generated career has none of. */
  playerPatch?: Partial<PlayerData>;
  /**
   * What the Contract tab's two previews answer, for a contract the club holds: the backend refuses
   * both for another club's player, and the release preview for one on loan (so `termination` is
   * `null` there). `null` for every state whose contract is not the club's.
   */
  contractPreviews?: {
    termination: ContractTerminationPreviewData | null;
    projection: RenewalProjection;
  } | null;
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
      // The game records an unknown start with an empty date: an unknown, not a made-up day.
      date: "",
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

/**
 * The Contract tab's previews for the club's own player: what releasing him costs (a year's wages
 * over the weeks left, as a stand-in figure) and the club's wage bill from its players' wages. The
 * numbers are the fixture's own, enough for the layout; the backend's are `ofm_core::contracts`.
 */
function contractPreviewsOf(player: PlayerData, canBeReleased: boolean) {
  const wageBill = gameState.players
    .filter((candidate) => candidate.team_id === USER_CLUB)
    .reduce((sum, candidate) => sum + candidate.wage, 0);
  const budget = gameState.teams.find((team) => team.id === USER_CLUB)?.wage_budget ?? wageBill;
  return {
    termination: canBeReleased
      ? {
          player_id: player.id,
          player_name: player.match_name,
          severance_cost: player.wage * 26,
          squad_safety: {
            team_id: USER_CLUB ?? "",
            projected_roster_size: 24,
            healthy_players: 18,
            healthy_goalkeepers: 2,
            effective_xi_size: 11,
            can_field_matchday_squad: true,
            missing_reasons: [],
          },
        }
      : null,
    projection: {
      current_annual_wage_bill: wageBill,
      projected_annual_wage_bill: wageBill,
      annual_wage_budget: budget,
      annual_soft_cap: Math.round(budget * 1.1),
      current_weekly_wage_spend: wageBill,
      projected_weekly_wage_spend: wageBill,
      current_cash_runway_weeks: 38,
      projected_cash_runway_weeks: 38,
      currently_over_budget: wageBill > budget,
      policy_allows: true,
    },
  };
}

const clubNameOf = (teamId: string | null) =>
  gameState.teams.find((team) => team.id === teamId)?.name ?? "";

type SeasonLine = Extract<CareerLine, { kind: "season" }>;

/** The totals and the by-club sums of season rows, as the backend sums them (newest club first). */
function summed(rows: SeasonLine[]): Pick<PlayerCareer, "by_club" | "totals"> {
  const clubs = new Map<string, PlayerCareer["by_club"][number]>();
  for (const row of [...rows].reverse()) {
    const key = row.club_id || `name:${row.club_name}`;
    const club = clubs.get(key);
    if (club) {
      club.last_season = row.season;
      club.appearances += row.appearances;
      club.goals += row.goals;
      club.assists += row.assists;
    } else {
      clubs.set(key, {
        key,
        club_id: row.club_id,
        club_name: row.club_name,
        first_season: row.season,
        last_season: row.season,
        appearances: row.appearances,
        goals: row.goals,
        assists: row.assists,
      });
    }
  }
  return {
    by_club: [...clubs.values()].sort((left, right) => right.last_season - left.last_season),
    totals: rows.reduce(
      (sum, row) => ({
        appearances: sum.appearances + row.appearances,
        goals: sum.goals + row.goals,
        assists: sum.assists + row.assists,
      }),
      { appearances: 0, goals: 0, assists: 0 },
    ),
  };
}

/** The moves the backend writes as lines of a career (not a renewal, not a loan's return). */
const CAREER_LINE_KINDS: ReadonlySet<PlayerMovementEntry["kind"]> = new Set([
  "initial_contract",
  "permanent_transfer",
  "loan_start",
  "loan_to_buy",
  "free_agent_signing",
  "released",
  "retired",
]);

/** A career's move lines from a player's own movement history, the entries the backend reads. */
function moveLinesOf(history: PlayerMovementEntry[]): CareerLine[] {
  return history
    .filter((entry) => CAREER_LINE_KINDS.has(entry.kind) && entry.date !== "")
    .map((entry) => ({
      kind: "move" as const,
      key: `${entry.date}-${entry.kind}`,
      date: entry.date.slice(0, 10),
      movement: entry.kind,
      from_club_name: entry.from_team_name ?? null,
      to_club_name: entry.to_team_name ?? null,
      fee: entry.fee ? entry.fee : null,
      loan_end: entry.loan_end_date ?? null,
    }));
}

/**
 * A career the way the backend lists it, for the harness: the given season rows with the move
 * lines of the player's own movement history placed where they happened (a move before the stint
 * it began), newest first, and the sums by the backend's rule. The rows are hand-made, because the
 * harness has no Rust to project them; the lines are not invented, they are the history's.
 */
function careerFrom(rows: SeasonLine[], history: PlayerMovementEntry[]): PlayerCareer {
  const placed: { when: string; rank: number; line: CareerLine }[] = [
    ...rows.map((row) => ({ when: `${row.season}-07-01`, rank: 1, line: row as CareerLine })),
    ...moveLinesOf(history).map((line) => ({
      when: line.kind === "move" ? line.date : "",
      rank: 0,
      line,
    })),
  ];
  placed.sort((left, right) => left.when.localeCompare(right.when) || left.rank - right.rank);
  return { by_season: placed.reverse().map((item) => item.line), ...summed(rows) };
}

const clubAt = (teamId: string | null) => ({ id: teamId ?? "", name: clubNameOf(teamId) });

const seasonRow = (
  season: number,
  at: { id: string; name: string },
  figures: [number, number, number],
  extra: Partial<SeasonLine> = {},
): SeasonLine => ({
  kind: "season",
  key: `${season}-${at.id}`,
  season,
  club_id: at.id,
  club_name: at.name,
  appearances: figures[0],
  goals: figures[1],
  assists: figures[2],
  in_progress: false,
  on_loan: false,
  ...extra,
});

/** The club's own player: the season in progress and two seasons at the club, two before it elsewhere. */
function ownCareer(player: PlayerData, signedFrom: string | null): PlayerCareer {
  const home = clubAt(player.team_id);
  const before = clubAt(signedFrom);
  return careerFrom(
    [
      seasonRow(2031, home, [27, 9, 6], { in_progress: true }),
      seasonRow(2030, home, [33, 12, 8]),
      seasonRow(2029, home, [30, 7, 4]),
      seasonRow(2028, before, [14, 2, 1]),
      seasonRow(2027, before, [9, 1, 0]),
    ],
    ownHistory(player, signedFrom),
  );
}

/** A player known only by what the fixture holds: his club's season in progress and a closed one. */
function plainCareer(player: PlayerData): PlayerCareer {
  const home = clubAt(player.team_id);
  return careerFrom(
    [seasonRow(2031, home, [20, 6, 3], { in_progress: true }), seasonRow(2030, home, [31, 9, 5])],
    player.movement_history ?? [],
  );
}

/** A division rival: signed from the user's club in 2028 and at his club since. */
function rivalCareer(player: PlayerData, signedFrom: string | null): PlayerCareer {
  const home = clubAt(player.team_id);
  const before = clubAt(signedFrom);
  return careerFrom(
    [
      seasonRow(2031, home, [26, 8, 5], { in_progress: true }),
      seasonRow(2030, home, [34, 11, 6]),
      seasonRow(2029, home, [29, 7, 4]),
      seasonRow(2028, home, [18, 3, 2]),
      seasonRow(2027, before, [12, 1, 1]),
    ],
    rivalHistory(player, signedFrom),
  );
}

/** A player the club has loaned out: this season at the borrower, on loan; the parent's seasons before. */
function loanedCareer(player: PlayerData, loanClub: string | null): PlayerCareer {
  const parent = clubAt(USER_CLUB);
  return careerFrom(
    [
      seasonRow(2031, clubAt(loanClub), [9, 2, 1], { in_progress: true, on_loan: true }),
      seasonRow(2030, parent, [33, 12, 8]),
      seasonRow(2029, parent, [30, 7, 4]),
    ],
    loanedHistory(player, loanClub),
  );
}

/** A free agent: the seasons at his last club, signed in 2028 and released when the contract ran out. */
function freeAgentCareer(player: PlayerData, lastClub: string | null): PlayerCareer {
  const last = clubAt(lastClub);
  return careerFrom(
    [
      seasonRow(2030, last, [33, 12, 8]),
      seasonRow(2029, last, [30, 7, 4]),
      seasonRow(2028, last, [14, 2, 1]),
    ],
    freeAgentHistory(player, lastClub),
  );
}

/** An archived retiree's career: his saved seasons, newest first, and nothing in progress. */
function careerOfRetiree(entries: PlayerData["career"]): PlayerCareer {
  const rows: SeasonLine[] = [...entries]
    .sort((left, right) => right.season - left.season)
    .map((entry, index) => ({
      kind: "season",
      key: `${entry.season}-${entry.team_id}-${index}`,
      season: entry.season,
      club_id: entry.team_id,
      club_name: entry.team_name,
      appearances: entry.appearances,
      goals: entry.goals,
      assists: entry.assists,
      in_progress: false,
      on_loan: false,
    }));
  return { by_season: rows, ...summed(rows) };
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
    career: ownCareer(player, signedFrom),
    playerPatch: { movement_history: ownHistory(player, signedFrom) },
    contractPreviews: contractPreviewsOf(player, true),
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
    career: null,
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
        career: ownCareer(own, rival.team_id),
        playerPatch: { movement_history: ownHistory(own, rival.team_id) },
        contractPreviews: contractPreviewsOf(own, true),
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
        career: plainCareer(keeper),
        contractPreviews: contractPreviewsOf(keeper, true),
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
        // The backend refuses to release a player who is on loan.
        contractPreviews: contractPreviewsOf(loaned, false),
        // His season in progress is at the borrower, a loan; the parent's seasons are closed.
        career: loanedCareer(loaned, rival.team_id),
        // On loan from the user's club to a rival: the profile's loan banner.
        playerPatch: {
          movement_history: loanedHistory(loaned, rival.team_id),
          // The game derives a player's contract dates from the ledger's current record, whose
          // start here is unknown.
          contract_start: null,
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
        career: rivalCareer(rival, own.team_id),
        positionRatings: positionRatingsOf(rival, { kind: "live" }),
        // The current record is the renewal, which started on 1 July 2030.
        playerPatch: {
          movement_history: rivalHistory(rival, own.team_id),
          contract_start: "2030-07-01",
        },
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
        career: plainCareer(scouted),
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
        career: plainCareer(unknown),
      };
    case "freeAgent":
      return {
        ...none,
        playerId: freeAgent.id,
        knowledge: { kind: "unknown" },
        wage: { kind: "hidden" },
        career: freeAgentCareer(freeAgent, rival.team_id),
        playerPatch: { movement_history: freeAgentHistory(freeAgent, rival.team_id) },
      };
    case "retired":
      // Not among the world's players: an id of its own, so the dashboard has to fetch him.
      return {
        ...none,
        playerId: `${retiree.id}-retired`,
        knowledge: null,
        wage: null,
        career: careerOfRetiree(retiree.career),
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
