import { type ReactNode, useId, useState } from "react";

import type { KnowledgeAnswer } from "../../hooks/useKnowledgeAnswer";
import type { CareerClubTotals } from "../../store/generated/CareerClubTotals";
import type { CareerMoveLine } from "../../store/generated/CareerMoveLine";
import type { CareerSeasonRow } from "../../store/generated/CareerSeasonRow";
import type { CareerTotals } from "../../store/generated/CareerTotals";
import type { PlayerCareer } from "../../store/generated/PlayerCareer";
import { Badge, Card, CardBody, CardHeader, SegmentedControl } from "../ui";
import PlayerProfileCareerChart from "./PlayerProfileCareerChart";
import { ClubName, moveSummary, seasonLabel, type TranslateFn } from "./careerParts";
import ProfileLoadFailed from "./ProfileLoadFailed";

interface PlayerProfileCareerTableProps {
  career: KnowledgeAnswer<PlayerCareer | null>;
  language: string;
  /** Opens a club by its id; without it the club is plain text. */
  onOpenTeam?: (teamId: string) => void;
  t: TranslateFn;
}

type CareerView = "season" | "club" | "chart";

const SECTION = "playerProfile";

/** A move's line, spanning the table. */
function MoveRow({
  line,
  language,
  t,
}: {
  line: CareerMoveLine;
  language: string;
  t: TranslateFn;
}) {
  return (
    <tr className="bg-sunken">
      <td colSpan={5} className="px-3 py-2 text-caption text-fg-muted">
        {moveSummary(line, language, t)}
      </td>
    </tr>
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
  const viewSwitch = (
    <SegmentedControl
      labelledBy={titleId}
      options={[
        { value: "season", label: t(`${SECTION}.careerBySeason`) },
        { value: "club", label: t(`${SECTION}.careerByClub`) },
        { value: "chart", label: t(`${SECTION}.careerViewChart`) },
      ]}
      value={view}
      onChange={setView}
    />
  );
  if (view === "chart") {
    return (
      <Card>
        {header(viewSwitch)}
        <PlayerProfileCareerChart
          career={value}
          language={language}
          onOpenTeam={onOpenTeam}
          t={t}
        />
      </Card>
    );
  }
  return (
    <Card>
      {header(viewSwitch)}
      <CardBody className="px-0 pb-0">
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
      </CardBody>
    </Card>
  );
}
