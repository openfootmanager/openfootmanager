import { Crosshair, Shield, Trophy, Users } from "lucide-react";

import { Card, CardBody, CardHeader } from "../ui";
import type { TeamData } from "../../store/gameStore";
import type { TeamProfileTranslate } from "./TeamProfile.types";
import { InfoRow } from "./TeamProfile.primitives";

interface TeamProfileClubDetailsCardProps {
  team: TeamData;
  t: TeamProfileTranslate;
}

export default function TeamProfileClubDetailsCard({ team, t }: TeamProfileClubDetailsCardProps) {
  return (
    <Card>
      <CardHeader>{t("teamProfile.clubInfo")}</CardHeader>
      <CardBody>
        <div className="flex flex-col gap-3">
          <InfoRow
            icon={<Shield className="w-4 h-4" />}
            label={t("teamProfile.stadium")}
            value={team.stadium_name}
          />
          <InfoRow
            icon={<Users className="w-4 h-4" />}
            label={t("teamProfile.capacity")}
            value={team.stadium_capacity.toLocaleString()}
          />
          <InfoRow
            icon={<Crosshair className="w-4 h-4" />}
            label={t("tactics.formation")}
            value={team.formation}
          />
          <InfoRow
            icon={<Trophy className="w-4 h-4" />}
            label={t("tactics.playStyle")}
            value={team.play_style}
          />
        </div>
      </CardBody>
    </Card>
  );
}
