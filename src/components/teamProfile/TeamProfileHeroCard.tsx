import { Calendar, Users } from "lucide-react";

import { Card, TeamLocation, TeamLogo } from "../ui";
import type { TeamProfileTranslate } from "./TeamProfile.types";
import { QuickStat } from "./TeamProfile.primitives";
import type { TeamProfileViewModel } from "./TeamProfile.types";
import type { TeamData } from "../../store/gameStore";

interface TeamProfileHeroCardProps {
  team: TeamData;
  viewModel: TeamProfileViewModel;
  locale: string;
  t: TeamProfileTranslate;
}

export default function TeamProfileHeroCard({
  team,
  viewModel,
  locale,
  t,
}: TeamProfileHeroCardProps) {
  return (
    <Card className="mb-5 overflow-hidden">
      <div
        className="p-8 relative"
        style={{
          background: `linear-gradient(135deg, ${team.colors.primary}, ${team.colors.secondary}40)`,
        }}
      >
        <div className="flex items-start gap-6">
          <TeamLogo
            team={team}
            className="w-24 h-24 rounded-2xl flex items-center justify-center font-heading font-bold text-3xl text-white border-2 border-white/30 overflow-hidden"
            imageClassName="h-20 w-20 object-contain drop-shadow"
            style={{ backgroundColor: team.colors.primary }}
          />
          <div className="flex-1">
            <h2 className="text-3xl font-heading font-bold text-white uppercase tracking-wide drop-shadow">
              {team.name}
            </h2>
            <div className="flex items-center gap-4 mt-2 text-white/80 text-sm">
              <TeamLocation
                city={team.city}
                countryCode={team.country}
                locale={locale}
                className="text-white/80"
              />
              <span className="flex items-center gap-1.5">
                <Calendar className="w-4 h-4" /> {t("teams.est")} {team.founded_year}
              </span>
            </div>
            {viewModel.manager && (
              <p className="text-white/70 text-sm mt-1 flex items-center gap-1.5">
                <Users className="w-4 h-4" /> {t("teamProfile.managerLabel")} {viewModel.manager.first_name} {viewModel.manager.last_name}
              </p>
            )}
          </div>

          <div className="hidden md:grid grid-cols-2 gap-3">
            <QuickHeroStat label={t("teams.avgOvr")} value={String(viewModel.avgOvr)} />
            <QuickHeroStat
              label={t("manager.reputation")}
              value={String(team.reputation)}
              valueClassName="text-accent-300"
            />
            <QuickHeroStat
              label={t("teamProfile.leaguePos")}
              value={viewModel.leaguePos > 0 ? `#${viewModel.leaguePos}` : "—"}
            />
            <QuickHeroStat
              label={t("teams.squad")}
              value={String(viewModel.roster.length)}
            />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-px bg-gray-200 dark:bg-navy-600 md:hidden">
        <QuickStat
          label={t("teams.avgOvr")}
          value={String(viewModel.avgOvr)}
          color="text-primary-500"
        />
        <QuickStat
          label={t("teams.rep")}
          value={String(team.reputation)}
          color="text-accent-500"
        />
        <QuickStat
          label={t("common.position")}
          value={viewModel.leaguePos > 0 ? `#${viewModel.leaguePos}` : "—"}
          color="text-gray-700 dark:text-gray-200"
        />
        <QuickStat
          label={t("teams.squad")}
          value={String(viewModel.roster.length)}
          color="text-gray-700 dark:text-gray-200"
        />
      </div>
    </Card>
  );
}

function QuickHeroStat({
  label,
  value,
  valueClassName = "text-white",
}: {
  label: string;
  value: string;
  valueClassName?: string;
}) {
  return (
    <div className="bg-black/20 backdrop-blur rounded-xl px-5 py-3 text-center min-w-[100px]">
      <p className="text-xs text-white/60 font-heading uppercase tracking-wider">
        {label}
      </p>
      <p className={`font-heading font-bold text-2xl mt-0.5 ${valueClassName}`}>
        {value}
      </p>
    </div>
  );
}
