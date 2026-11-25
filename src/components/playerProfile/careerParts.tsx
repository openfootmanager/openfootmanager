import { formatExactMoney } from "../../lib/helpers";
import { storedTeamLabel } from "../../lib/team";
import type { CareerMoveLine } from "../../store/generated/CareerMoveLine";
import type { PlayerMovementKind } from "../../store/generated/PlayerMovementKind";
import { formatProfileDay } from "./PlayerProfile.helpers";

export type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

const SECTION = "playerProfile";

/** A season as the game writes it: the year it starts and the two digits of the year it ends. */
export const seasonLabel = (season: number) => `${season}/${String(season + 1).slice(-2)}`;

/**
 * What a move says, as the sentence its line starts with: who it was with, from the backend's
 * names. A kind the career has no line for (a renewal, a loan's return) never reaches here.
 */
function moveText(line: CareerMoveLine, t: TranslateFn): string {
  const unknown = t("common.unknown");
  const from = line.from_club_name ?? unknown;
  const to = line.to_club_name ?? unknown;
  const kinds: Partial<Record<PlayerMovementKind, string>> = {
    initial_contract: t(`${SECTION}.careerMove.initial`, { club: to }),
    permanent_transfer: line.from_club_name
      ? t(`${SECTION}.careerMove.transferFrom`, { club: from })
      : t(`${SECTION}.careerMove.transferTo`, { club: to }),
    loan_start: t(`${SECTION}.careerMove.loanTo`, { club: to }),
    loan_to_buy: t(`${SECTION}.careerMove.boughtBy`, { club: to }),
    free_agent_signing: t(`${SECTION}.careerMove.freeAgent`, { club: to }),
    released: t(`${SECTION}.careerMove.releasedBy`, { club: from }),
    retired: t(`${SECTION}.careerMove.retiredFrom`, { club: from }),
  };
  return kinds[line.movement] ?? "";
}

/** A move as one line of words: the date, what happened, the loan's end, and the fee. */
export function moveSummary(line: CareerMoveLine, language: string, t: TranslateFn): string {
  const parts = [formatProfileDay(line.date, language), moveText(line, t)];
  if (line.loan_end) {
    parts[1] = `${parts[1]} ${t(`${SECTION}.careerMove.loanUntil`, {
      date: formatProfileDay(line.loan_end, language),
    })}`;
  }
  if (line.fee) parts.push(formatExactMoney(line.fee));
  return parts.join(" · ");
}

export function ClubName({
  id,
  name,
  onOpenTeam,
  t,
}: {
  id: string;
  name: string;
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}) {
  const label = storedTeamLabel(name, t);
  if (!onOpenTeam || !id) return <span className="font-semibold text-fg">{label}</span>;
  return (
    <button
      type="button"
      onClick={() => onOpenTeam(id)}
      className="rounded font-semibold text-positive underline underline-offset-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
    >
      {label}
    </button>
  );
}
