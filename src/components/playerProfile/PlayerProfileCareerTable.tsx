import { type ReactNode, useId, useState } from "react";

import type { KnowledgeAnswer } from "../../hooks/useKnowledgeAnswer";
import { formatExactMoney } from "../../lib/helpers";
import { storedTeamLabel } from "../../lib/team";
import type { CareerClubTotals } from "../../store/generated/CareerClubTotals";
import type { CareerMoveLine } from "../../store/generated/CareerMoveLine";
import type { CareerSeasonRow } from "../../store/generated/CareerSeasonRow";
import type { CareerTotals } from "../../store/generated/CareerTotals";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import type { PlayerMovementKind } from "../../store/generated/PlayerMovementKind";
import { Badge, Card, CardBody, CardHeader, SegmentedControl } from "../ui";
import { formatProfileDay } from "./PlayerProfile.helpers";
import ProfileLoadFailed from "./ProfileLoadFailed";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileCareerTableProps {
  career: KnowledgeAnswer<PlayerCareer | null>;
  language: string;
  /** Opens a club by its id; without it the club is plain text. */
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}

type CareerView = "season" | "club";

const SECTION = "playerProfile";

/** A season as the game writes it: the year it starts and the two digits of the year it ends. */
const seasonLabel = (season: number) => `${season}/${String(season + 1).slice(-2)}`;

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

/** A move's line: the date, what happened, the fee and, for a loan, when it ends. */
function MoveRow({
  line,
  language,
  t,
}: {
  line: CareerMoveLine;
  language: string;
  t: TranslateFn;
}) {
  const parts = [formatProfileDay(line.date, language), moveText(line, t)];
  if (line.loan_end) {
    parts[1] = `${parts[1]} ${t(`${SECTION}.careerMove.loanUntil`, {
      date: formatProfileDay(line.loan_end, language),
    })}`;
  }
  if (line.fee) parts.push(formatExactMoney(line.fee));
  return (
    <tr className="bg-sunken">
      <td colSpan={5} className="px-3 py-2 text-caption text-fg-muted">
        {parts.join(" · ")}
      </td>
    </tr>
  );
}

function ClubName({
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

const NUMBER_CELL = "px-3 py-2 text-right tabular-nums text-fg-muted";

function Figures({ row }: { row: Pick<CareerTotals, "appearances" | "goals" | "assists"> }) {
  return (
    <>
      <td className={NUMBER_CELL}>{row.appearances}</td>
      <td className={`${NUMBER_CELL} font-semibold text-fg`}>{row.goals}</td>
      <td className={NUMBER_CELL}>{row.assists}</td>
    </>
  );
}

function SeasonRow({
  row,
  onOpenTeam,
  t,
}: {
  row: CareerSeasonRow;
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}) {
  return (
    <tr className={row.in_progress ? "bg-positive-tint" : undefined}>
      <td className="px-3 py-2 tabular-nums text-fg-muted">{seasonLabel(row.season)}</td>
      <td className="px-3 py-2">
        <span className="inline-flex flex-wrap items-center gap-2">
          <ClubName id={row.club_id} name={row.club_name} onOpenTeam={onOpenTeam} t={t} />
          {row.in_progress ? (
            <Badge variant="success">{t(`${SECTION}.careerInProgress`)}</Badge>
          ) : null}
          {row.on_loan ? <Badge variant="neutral">{t(`${SECTION}.careerOnLoan`)}</Badge> : null}
        </span>
      </td>
      <Figures row={row} />
    </tr>
  );
}

function ClubRow({
  club,
  onOpenTeam,
  t,
}: {
  club: CareerClubTotals;
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}) {
  const span =
    club.first_season === club.last_season
      ? seasonLabel(club.first_season)
      : `${seasonLabel(club.first_season)} – ${seasonLabel(club.last_season)}`;
  return (
    <tr>
      <td className="px-3 py-2">
        <ClubName id={club.club_id} name={club.club_name} onOpenTeam={onOpenTeam} t={t} />
      </td>
      <td className="px-3 py-2 tabular-nums text-fg-muted">{span}</td>
      <Figures row={club} />
    </tr>
  );
}

/**
 * The career as one table, drawn as the backend lists it (`ofm_core::player_career`): seasons newest
 * first with each move a line where it happened, or the same rows summed by club, and the career
 * total. The screen decides nothing about the order or the sums.
 */
export default function PlayerProfileCareerTable({
  career,
  language,
  onOpenTeam,
  t,
}: PlayerProfileCareerTableProps) {
  const titleId = useId();
  const [view, setView] = useState<CareerView>("season");

  const title = <span id={titleId}>{t(`${SECTION}.careerHistory`)}</span>;
  const header = (action?: ReactNode) => <CardHeader action={action}>{title}</CardHeader>;

  if (career.status === "loading") {
    return (
      <Card>
        {header()}
        <CardBody>
          <p className="text-body text-fg-muted">{t("common.loading")}</p>
        </CardBody>
      </Card>
    );
  }
  if (career.status === "failed") {
    return (
      <Card>
        {header()}
        <CardBody>
          <ProfileLoadFailed t={t} onRetry={career.retry} />
        </CardBody>
      </Card>
    );
  }
  const value = career.value;
  if (!value || value.by_season.length === 0) {
    return (
      <Card>
        {header()}
        <CardBody>
          <p className="py-4 text-center text-body text-fg-subtle">{t(`${SECTION}.noCareer`)}</p>
        </CardBody>
      </Card>
    );
  }

  const bySeason = view === "season";
  return (
    <Card>
      {header(
        <SegmentedControl
          labelledBy={titleId}
          options={[
            { value: "season", label: t(`${SECTION}.careerBySeason`) },
            { value: "club", label: t(`${SECTION}.careerByClub`) },
          ]}
          value={view}
          onChange={setView}
        />,
      )}
      <CardBody className="px-0 pb-0">
        <div className="overflow-x-auto">
          <table aria-label={t(`${SECTION}.careerHistory`)} className="w-full text-body">
            <thead>
              <tr className="border-b border-line text-caption text-fg-muted">
                <th scope="col" className="px-3 py-2 text-left font-normal">
                  {bySeason ? t(`${SECTION}.season`) : t("common.team")}
                </th>
                <th scope="col" className="px-3 py-2 text-left font-normal">
                  {bySeason ? t("common.team") : t(`${SECTION}.season`)}
                </th>
                <th scope="col" className="px-3 py-2 text-right font-normal">
                  {t(`${SECTION}.apps`)}
                </th>
                <th scope="col" className="px-3 py-2 text-right font-normal">
                  {t(`${SECTION}.goals`)}
                </th>
                <th scope="col" className="px-3 py-2 text-right font-normal">
                  {t(`${SECTION}.assists`)}
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-line">
              {bySeason
                ? value.by_season.map((line) =>
                    line.kind === "season" ? (
                      <SeasonRow key={line.key} row={line} onOpenTeam={onOpenTeam} t={t} />
                    ) : (
                      <MoveRow key={line.key} line={line} language={language} t={t} />
                    ),
                  )
                : value.by_club.map((club) => (
                    <ClubRow key={club.key} club={club} onOpenTeam={onOpenTeam} t={t} />
                  ))}
            </tbody>
            <tfoot>
              <tr className="border-t border-line-strong bg-sunken font-semibold text-fg">
                <td className="px-3 py-2">{t(`${SECTION}.careerTotal`)}</td>
                <td className="px-3 py-2" />
                <Figures row={value.totals} />
              </tr>
            </tfoot>
          </table>
        </div>
      </CardBody>
    </Card>
  );
}
