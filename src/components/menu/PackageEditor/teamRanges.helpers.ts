/**
 * Interpretation for the two club range fields in the World Editor.
 *
 * Both are bare numbers with no natural units, and both are easy to get wrong:
 * reputation runs 0–1000 rather than the 0–100 an author reasonably assumes,
 * and finance silently determines the club's transfer budget.
 */

export const REPUTATION_MIN = 0;
export const REPUTATION_MAX = 1000;

/** Share of a club's finance that becomes its transfer budget at world build. */
export const TRANSFER_BUDGET_SHARE = 0.15;

export type ReputationBandKey =
  | "amateur"
  | "lowerLeague"
  | "midTable"
  | "topFlight"
  | "elite";

/**
 * Lower bound of each band, highest first. These match the guidance already
 * shown in the field help, so the readout and the help cannot disagree.
 */
export const REPUTATION_BANDS: ReadonlyArray<{ from: number; key: ReputationBandKey }> = [
  { from: 850, key: "elite" },
  { from: 720, key: "topFlight" },
  { from: 550, key: "midTable" },
  { from: 300, key: "lowerLeague" },
  { from: REPUTATION_MIN, key: "amateur" },
];

/** The band a reputation value falls in. Out-of-range input is clamped. */
export function reputationBandKey(value: number): ReputationBandKey {
  const clamped = Math.min(REPUTATION_MAX, Math.max(REPUTATION_MIN, value));
  const band = REPUTATION_BANDS.find((candidate) => clamped >= candidate.from);
  return band?.key ?? "amateur";
}

/**
 * The transfer budget a club will open with, given its finance.
 * Mirrors `team.transfer_budget = (finance as f64 * 0.15) as i64` in world
 * generation — the cast truncates, so this floors too.
 */
export function deriveTransferBudget(finance: number): number {
  if (!Number.isFinite(finance) || finance <= 0) return 0;
  return Math.floor(finance * TRANSFER_BUDGET_SHARE);
}

/**
 * Compact amount ("3.5M", "450K") so a range fits on one line.
 *
 * Delegates to `Intl` rather than appending hardcoded `K`/`M`, which would ship
 * English magnitude suffixes to every locale — Chinese groups by 万, German
 * writes "Mio.". It also rounds before deciding whether a fraction is needed,
 * so 1,999,999 reads "2M" rather than "2.0M".
 */
export function formatCompactAmount(value: number, locale = "en"): string {
  if (!Number.isFinite(value)) return "0";
  return new Intl.NumberFormat(locale, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}
