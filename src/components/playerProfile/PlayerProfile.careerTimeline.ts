import type { CareerEntry, PlayerMovementEntry, PlayerMovementKind } from "../../store/gameStore";

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

/** The seasons turn over on 1 July: contracts end on 30 June, and a season runs from one to the next. */
const seasonStart = (season: number) => `${season}-07-01`;
const seasonEnd = (season: number) => `${season + 1}-06-30`;

const dayOf = (date: string) => date.slice(0, 10);
const isDay = (date: string) => /^\d{4}-\d{2}-\d{2}$/.test(dayOf(date));

/** The moves that are a line in the career: a renewal is a contract matter, a loan's return is carried by the loan's line. */
const LINE_KINDS: ReadonlySet<PlayerMovementKind> = new Set([
  "initial_contract",
  "permanent_transfer",
  "loan_start",
  "loan_to_buy",
  "free_agent_signing",
  "released",
  "retired",
]);

/** The moves that put him at a different club than before. */
const CLUB_CHANGE_KINDS: ReadonlySet<PlayerMovementKind> = new Set([
  "permanent_transfer",
  "loan_start",
  "loan_return",
  "loan_to_buy",
  "free_agent_signing",
]);

/** A spell at one club, from the day he arrived to the day he left (`null` while he is there). */
interface Visit {
  clubId: string;
  start: string;
  end: string | null;
  loan: boolean;
}

/** The clubs he has been at, in order, from the ledger. A move with no date cannot place a visit. */
function buildVisits(movements: PlayerMovementEntry[]): Visit[] {
  const visits: Visit[] = [];
  const openAt = (clubId: string) =>
    [...visits].reverse().find((visit) => visit.clubId === clubId && visit.end === null);

  for (const entry of movements) {
    if (entry.kind === "renewal" || !isDay(entry.date)) continue;
    const date = dayOf(entry.date);
    const left = entry.from_team_id ? openAt(entry.from_team_id) : undefined;
    if (left) left.end = date;
    const joined = entry.to_team_id;
    if (joined && entry.kind !== "released" && entry.kind !== "retired") {
      // A club buying the player he had on loan: the loan spell ends and his own begins.
      const open = openAt(joined);
      if (open) open.end = date;
      visits.push({ clubId: joined, start: date, end: null, loan: entry.kind === "loan_start" });
    }
  }
  return visits;
}

/**
 * Where a season row falls in time and whether it was a loan: the day he arrived at the club for
 * that spell, or the start of the season if he was already there. A club he was at twice in a
 * season takes its visits in order. With no ledger for it, the start of the season.
 */
function placeSeasonRow(
  entry: CareerEntry,
  visits: Visit[],
  taken: Map<string, number>,
): { start: string; loan: boolean } {
  const first = seasonStart(entry.season);
  const last = seasonEnd(entry.season);
  const overlapping = visits.filter(
    (visit) =>
      visit.clubId === entry.team_id &&
      visit.start <= last &&
      (visit.end === null || visit.end >= first),
  );
  const slot = `${entry.team_id}|${entry.season}`;
  const nth = taken.get(slot) ?? 0;
  taken.set(slot, nth + 1);
  const visit = overlapping[nth] ?? overlapping[overlapping.length - 1];
  if (!visit) return { start: first, loan: false };
  return { start: visit.start > first ? visit.start : first, loan: visit.loan };
}

function moveLine(entry: PlayerMovementEntry, position: number): CareerMoveLine {
  return {
    kind: "move",
    key: `${entry.date}-${entry.kind}-${position}`,
    date: dayOf(entry.date),
    move: entry.kind,
    fromClubName: entry.from_team_name || null,
    toClubName: entry.to_team_name || null,
    fee: entry.fee ? entry.fee : null,
    loanEnd: entry.loan_end_date ?? null,
  };
}

/** Rows and lines in the order they happened: on one day the move comes first, then the stint it began. */
interface Placed {
  when: string;
  rank: 0 | 1;
  position: number;
  item: CareerSeasonRow | CareerMoveLine;
}

function byClubRows(rows: CareerSeasonRow[]): CareerClubRow[] {
  const clubs = new Map<string, CareerClubRow>();
  for (const row of rows) {
    const key = row.clubId || `name:${row.clubName}`;
    const club = clubs.get(key);
    if (club) {
      club.firstSeason = Math.min(club.firstSeason, row.season);
      club.lastSeason = Math.max(club.lastSeason, row.season);
      club.appearances += row.appearances;
      club.goals += row.goals;
      club.assists += row.assists;
    } else {
      clubs.set(key, {
        key,
        clubId: row.clubId,
        clubName: row.clubName,
        firstSeason: row.season,
        lastSeason: row.season,
        appearances: row.appearances,
        goals: row.goals,
        assists: row.assists,
      });
    }
  }
  return [...clubs.values()].sort(
    (left, right) => right.lastSeason - left.lastSeason || right.firstSeason - left.firstSeason,
  );
}

/**
 * The career as one list, newest first: a row for each club he played for in each season, the
 * season in progress on top, and each move written as a line where it happened (the line sits
 * between the season he arrived and the one he left). The same rows are summed by club.
 *
 * The closed seasons are the rollover's rows, one per club stint and oldest first. The season in
 * progress is built from his current season stats and the club he is with now; his season totals
 * carry across a move, so after a move this season the row is flagged as counting his earlier
 * club's matches too (the wire does not split the season in progress by club).
 */
export function buildCareerTimeline({
  career,
  movements,
  current,
  today,
}: CareerTimelineInput): CareerTimeline {
  const visits = buildVisits(movements);
  const taken = new Map<string, number>();
  const placed: Placed[] = [];

  career.forEach((entry, position) => {
    const { start, loan } = placeSeasonRow(entry, visits, taken);
    placed.push({
      when: start,
      rank: 1,
      position,
      item: {
        kind: "season",
        key: `${entry.season}-${entry.team_id}-${position}`,
        season: entry.season,
        clubId: entry.team_id,
        clubName: entry.team_name,
        appearances: entry.appearances,
        goals: entry.goals,
        assists: entry.assists,
        inProgress: false,
        onLoan: loan,
        includesEarlierClubs: false,
      },
    });
  });
  movements.forEach((entry, position) => {
    if (!LINE_KINDS.has(entry.kind) || !isDay(entry.date)) return;
    placed.push({ when: dayOf(entry.date), rank: 0, position, item: moveLine(entry, position) });
  });
  placed.sort(
    (left, right) =>
      left.when.localeCompare(right.when) ||
      left.rank - right.rank ||
      left.position - right.position,
  );

  const chronological = placed.map(({ item }) => item);
  const seasonRows = chronological.filter(
    (item): item is CareerSeasonRow => item.kind === "season",
  );

  if (current && (current.club || current.stats.appearances > 0)) {
    const since = seasonStart(current.season);
    const movedThisSeason = movements.some(
      (entry) =>
        CLUB_CHANGE_KINDS.has(entry.kind) &&
        isDay(entry.date) &&
        dayOf(entry.date) >= since &&
        dayOf(entry.date) <= today,
    );
    const now: CareerSeasonRow = {
      kind: "season",
      key: `${current.season}-now`,
      season: current.season,
      clubId: current.club?.id ?? "",
      clubName: current.club?.name ?? "",
      appearances: current.stats.appearances,
      goals: current.stats.goals,
      assists: current.stats.assists,
      inProgress: true,
      onLoan: current.onLoan,
      includesEarlierClubs: movedThisSeason,
    };
    chronological.push(now);
    seasonRows.push(now);
  }

  const totals = seasonRows.reduce<CareerTotals>(
    (sum, row) => ({
      appearances: sum.appearances + row.appearances,
      goals: sum.goals + row.goals,
      assists: sum.assists + row.assists,
    }),
    { appearances: 0, goals: 0, assists: 0 },
  );

  return {
    bySeason: chronological.reverse(),
    byClub: byClubRows(seasonRows),
    totals,
  };
}
