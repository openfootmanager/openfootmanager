import type { MatchEvent, MatchSnapshot } from "./types";

/** Only the shootout's kicks: an in-match penalty lives in the same event log and is not one. */
const SHOOTOUT_KICKS = new Set(["ShootoutGoal", "ShootoutMiss"]);

/** The regular rounds; after them, a level shootout goes to sudden death a round at a time. */
export const REGULAR_ROUNDS = 5;

/** One side's kick in one round of the sheet. */
export type KickCell =
  | { state: "scored" | "missed"; kickerName: string | null }
  /** The shootout is still on and this kick hasn't been taken. */
  | { state: "toCome" }
  /**
   * Over, and this kick was never taken. The engine decides a shootout only at equal kick counts
   * (penalty.rs `decided`), so a finished sheet never has one today; it guards a snapshot that
   * ends mid-round rather than drawing a kick to come in a finished shootout.
   */
  | { state: "notTaken" };

interface KickSheetRow {
  /** 1-based, as the sheet numbers it. */
  round: number;
  home: KickCell;
  away: KickCell;
}

export interface KickSheet {
  rows: KickSheetRow[];
  /** The 1-based round being taken now; null once the shootout is over. */
  currentRound: number | null;
}

/** Whether an event is one of the shootout's kicks. */
export function isShootoutKick(evt: MatchEvent): boolean {
  return SHOOTOUT_KICKS.has(evt.event_type);
}

/** One side's shootout kicks, in the order taken. */
export function sideKicks(events: readonly MatchEvent[], side: "Home" | "Away"): MatchEvent[] {
  return events.filter((evt) => isShootoutKick(evt) && evt.side === side);
}

/** Every player the match knows, on the pitch or on the bench, by id: a kicker may have come on. */
function playerNames(snapshot: MatchSnapshot): Map<string, string> {
  const names = new Map<string, string>();
  for (const player of [
    ...snapshot.home_team.players,
    ...snapshot.away_team.players,
    ...snapshot.home_bench,
    ...snapshot.away_bench,
  ]) {
    names.set(player.id, player.name);
  }
  return names;
}

/**
 * How many rows the sheet shows. While the shootout is on: the five regular rounds, any round
 * under way, and in sudden death the next round once both sides are level on kicks. Once it is
 * over: exactly the rounds kicked.
 */
function roundCount(homeKicks: number, awayKicks: number, live: boolean): number {
  const kicked = Math.max(homeKicks, awayKicks);
  if (!live) return kicked;
  const nextSuddenDeathRound =
    homeKicks === awayKicks && homeKicks >= REGULAR_ROUNDS ? homeKicks + 1 : 0;
  return Math.max(REGULAR_ROUNDS, kicked, nextSuddenDeathRound);
}

/**
 * The shootout as a referee's sheet: a row per round, each side's kicker and result in it, drawn
 * from the kick events in the order they were taken.
 */
export function buildKickSheet(snapshot: MatchSnapshot): KickSheet {
  const live = snapshot.phase === "PenaltyShootout";
  const names = playerNames(snapshot);
  const home = sideKicks(snapshot.events, "Home");
  const away = sideKicks(snapshot.events, "Away");

  const cell = (kicks: MatchEvent[], index: number): KickCell => {
    const kick = kicks[index];
    if (kick) {
      return {
        state: kick.event_type === "ShootoutGoal" ? "scored" : "missed",
        kickerName: kick.player_id ? (names.get(kick.player_id) ?? null) : null,
      };
    }
    return live ? { state: "toCome" } : { state: "notTaken" };
  };

  const rows = Array.from({ length: roundCount(home.length, away.length, live) }, (_, index) => ({
    round: index + 1,
    home: cell(home, index),
    away: cell(away, index),
  }));

  return { rows, currentRound: live ? Math.min(home.length, away.length) + 1 : null };
}
