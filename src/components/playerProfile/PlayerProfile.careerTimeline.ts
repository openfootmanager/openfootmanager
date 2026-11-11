import type {
  CareerEntry,
  PlayerMovementEntry,
  PlayerMovementKind,
} from "../../store/gameStore";

/** One club's share of one season: a closed season, or the season in progress. */
export interface CareerSeasonRow {
  kind: "season";
  key: string;
  /** The league's season number (the year it starts: 2026 is "2026/27"). */
  season: number;
  /** Empty for a spell at no club: the screen words it in the player's language. */
  clubId: string;
  clubName: string;
  appearances: number;
  goals: number;
  assists: number;
  /** The season in progress, from his current season stats. */
  inProgress: boolean;
  /** He was at the club on loan. */
  onLoan: boolean;
  /**
   * Only on the season in progress: he changed club since it began, and his season totals carry on
   * across the move, so this row counts the matches he played for his earlier clubs too.
   */
  includesEarlierClubs: boolean;
}

/** A move between clubs, written where it happened. */
export interface CareerMoveLine {
  kind: "move";
  key: string;
  date: string;
  move: PlayerMovementKind;
  fromClubName: string | null;
  toClubName: string | null;
  fee: number | null;
  loanEnd: string | null;
}

/** The same rows summed by club. */
export interface CareerClubRow {
  key: string;
  clubId: string;
  clubName: string;
  firstSeason: number;
  lastSeason: number;
  appearances: number;
  goals: number;
  assists: number;
}

export interface CareerTotals {
  appearances: number;
  goals: number;
  assists: number;
}

export interface CareerTimeline {
  /** Newest first: season rows with each move written as a line between them. */
  bySeason: (CareerSeasonRow | CareerMoveLine)[];
  /** Newest first by the last season at the club. */
  byClub: CareerClubRow[];
  totals: CareerTotals;
}

export interface CareerTimelineInput {
  /** The closed seasons, one row per club stint, oldest first (the order the rollover writes them). */
  career: CareerEntry[];
  /** His ledger in the order it was appended, club names already resolved. */
  movements: PlayerMovementEntry[];
  /** The season in progress, from his season stats; `null` when there is none to show. */
  current: {
    season: number;
    club: { id: string; name: string } | null;
    stats: CareerTotals;
    onLoan: boolean;
  } | null;
  /** The game's day, as `YYYY-MM-DD`. */
  today: string;
}

/**
 * The career as one list. Scaffold: the real model replaces it.
 */
export function buildCareerTimeline(_input: CareerTimelineInput): CareerTimeline {
  return { bySeason: [], byClub: [], totals: { appearances: 0, goals: 0, assists: 0 } };
}
