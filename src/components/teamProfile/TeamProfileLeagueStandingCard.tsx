import { Card, CardBody, CardHeader } from "../ui";

import { StatBox } from "./TeamProfile.primitives";
import type { LeagueStanding, TeamProfileTranslate } from "./TeamProfile.types";

interface TeamProfileLeagueStandingCardProps {
  standings: LeagueStanding | null;
  t: TeamProfileTranslate;
}

export default function TeamProfileLeagueStandingCard({
  standings,
  t,
}: TeamProfileLeagueStandingCardProps) {
  if (!standings) {
    return null;
  }

  return (
    <Card>
      <CardHeader>{t("teamProfile.leagueStanding")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-4 gap-2 text-center">
          <StatBox label={t("common.played")} value={standings.played} />
          <StatBox label={t("common.won")} value={standings.won} />
          <StatBox label={t("common.drawn")} value={standings.drawn} />
          <StatBox label={t("common.lost")} value={standings.lost} />
          <StatBox label={t("common.gf")} value={standings.goals_for} />
          <StatBox label={t("common.ga")} value={standings.goals_against} />
          <StatBox label={t("common.gd")} value={standings.goals_for - standings.goals_against} />
          <StatBox label={t("common.pts")} value={standings.points} highlight />
        </div>
      </CardBody>
    </Card>
  );
}
