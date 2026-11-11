import type { PlayerMovementEntry } from "../../store/gameStore";
import type { ContractSource, ReleaseReason } from "../../store/types";

/** One contract the player has had, as the Contract tab lists it. */
export interface ContractHistoryRow {
  /** Stable across renders: the ledger entry's date, kind and place. */
  key: string;
  /** The club the contract is with; empty when the ledger names none. */
  clubName: string;
  /** Its id, for a link to the club; `null` when the ledger names none. */
  clubId: string | null;
  /** The club he came from, when the contract is a transfer's; `null` otherwise. */
  fromClubName: string | null;
  /** `null` is an honest unknown (a save from before starts were recorded), never a date. */
  start: string | null;
  end: string | null;
  /** Per week. */
  weeklyWage: number;
  source: ContractSource;
  /** The release that closed it, when one did and said why; `null` for a contract that was renewed or is in force. */
  endedBy: ReleaseReason | "retired" | null;
  /** The day a later contract took over from it before its own end (a renewal signed early). */
  replacedOn: string | null;
  /** The contract he is on now: the last one, not closed by a release, and not yet past its end. */
  inForce: boolean;
}

/**
 * The contracts a player has had, newest first, from his movement ledger.
 *
 * The ledger is read in the order it was appended, never by date: the game's current contract is
 * the last one appended (a legacy save appends its current contract, with an unknown start and no
 * date, after the history that came before it). A row is an entry that makes a contract. A release
 * or a retirement closes the contract before it; a renewal signed before the older contract ran
 * out leaves the older one with the day it was replaced. The later appended is listed above the
 * earlier. An unknown start stays null.
 */
export function buildContractHistory(
  movements: PlayerMovementEntry[],
  today: string,
): ContractHistoryRow[] {
  const rows: ContractHistoryRow[] = [];
  let closed = false;

  movements.forEach((entry, position) => {
    if (entry.kind === "released" || entry.kind === "retired") {
      const last = rows[rows.length - 1];
      if (last && !closed) {
        last.endedBy = entry.kind === "retired" ? "retired" : (entry.release_reason ?? null);
        last.inForce = false;
        closed = true;
      }
      return;
    }
    if (!entry.contract) return;
    const previous = rows[rows.length - 1];
    const takesOverOn = entry.contract.start ?? entry.date;
    // Only an open contract is taken over: a released one was already closed, and says so itself.
    if (!closed && previous?.end && takesOverOn && takesOverOn < previous.end) {
      previous.replacedOn = takesOverOn;
    }
    closed = false;
    rows.push({
      key: `${entry.date}-${entry.kind}-${position}`,
      clubName: entry.to_team_name || "",
      clubId: entry.to_team_id ?? null,
      // Only a transfer comes from another club; a renewal or a first contract does not.
      fromClubName:
        entry.contract.source === "transfer"
          ? (entry.from_team_name ?? entry.from_team_id ?? null)
          : null,
      start: entry.contract.start ?? null,
      end: entry.contract.end ?? null,
      weeklyWage: entry.contract.weekly_wage,
      source: entry.contract.source,
      endedBy: null,
      replacedOn: null,
      inForce: false,
    });
  });

  const current = rows[rows.length - 1];
  if (current && !closed && (current.end === null || current.end >= today)) {
    current.inForce = true;
  }
  return rows.reverse();
}
