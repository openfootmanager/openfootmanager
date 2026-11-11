import type { PlayerMovementEntry } from "../../store/gameStore";
import type { ContractSource, ReleaseReason } from "../../store/types";

/** One contract the player has had, as the Contract tab lists it. */
export interface ContractHistoryRow {
  /** Stable across renders: the ledger entry's date, kind and place. */
  key: string;
  /** The club the contract is with; empty when the ledger names none. */
  clubName: string;
  /** `null` is an honest unknown (a save from before starts were recorded), never a date. */
  start: string | null;
  end: string | null;
  /** Per week. */
  weeklyWage: number;
  source: ContractSource;
  /** The release that closed it, when one did and said why; `null` for a contract that was renewed or is in force. */
  endedBy: ReleaseReason | null;
  /** The day a later contract took over from it before its own end (a renewal signed early). */
  replacedOn: string | null;
  /** The contract he is on now: the last one, not closed by a release, and not yet past its end. */
  inForce: boolean;
}

/**
 * The contracts a player has had, newest first, from his movement ledger.
 *
 * A row is a ledger entry that makes a contract. A release (`released`) closes the contract
 * before it; a renewal signed before the older contract ran out leaves the older one with the day
 * it was replaced. Entries on the same day keep the order they were recorded in, so the later one
 * is listed above the earlier. An unknown start stays null.
 */
export function buildContractHistory(
  movements: PlayerMovementEntry[],
  today: string,
): ContractHistoryRow[] {
  const ledger = movements
    .map((entry, index) => ({ entry, index }))
    .sort(
      (left, right) => left.entry.date.localeCompare(right.entry.date) || left.index - right.index,
    )
    .map(({ entry }) => entry);
  const rows: ContractHistoryRow[] = [];
  let closed = false;

  ledger.forEach((entry, position) => {
    if (entry.kind === "released") {
      const last = rows[rows.length - 1];
      if (last && !closed) {
        last.endedBy = entry.release_reason ?? null;
        last.inForce = false;
        closed = true;
      }
      return;
    }
    if (!entry.contract) return;
    const previous = rows[rows.length - 1];
    const takesOverOn = entry.contract.start ?? entry.date;
    if (previous?.end && takesOverOn && takesOverOn < previous.end) {
      previous.replacedOn = takesOverOn;
    }
    closed = false;
    rows.push({
      key: `${entry.date}-${entry.kind}-${position}`,
      clubName: entry.to_team_name || entry.to_team_id || "",
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
