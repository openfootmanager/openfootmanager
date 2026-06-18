import { useTranslation } from "react-i18next";
import { Trophy, Users } from "lucide-react";
import type { LeagueData } from "../../store/types";
import { Card, CardHeader, CardBody, Badge } from "../ui";

interface Props {
  competitions: LeagueData[];
  userTeamId: string | null;
  onSelect: (id: string) => void;
}

type CompetitionScope = "Domestic" | "Regional" | "Continental" | "International";

const SCOPE_ORDER: CompetitionScope[] = [
  "Domestic",
  "Regional",
  "Continental",
  "International",
];

function getCompetitionStatus(
  comp: LeagueData,
): "notStarted" | "inProgress" | "completed" {
  const { fixtures } = comp;
  if (fixtures.length === 0) return "notStarted";
  const completed = fixtures.filter((f) => f.status === "Completed").length;
  if (completed === 0) return "notStarted";
  if (completed >= fixtures.length) return "completed";
  return "inProgress";
}

export default function CompetitionsOverview({
  competitions,
  userTeamId,
  onSelect,
}: Props) {
  const { t } = useTranslation();

  if (competitions.length === 0) {
    return (
      <Card>
        <CardBody>
          <div className="flex flex-col items-center gap-2 py-6 text-center">
            <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {t("tournaments.noActive")}
            </p>
          </div>
        </CardBody>
      </Card>
    );
  }

  const byScope = SCOPE_ORDER.reduce<Record<string, LeagueData[]>>(
    (acc, scope) => {
      const group = competitions.filter(
        (c) => (c.scope ?? "Domestic") === scope,
      );
      if (group.length > 0) acc[scope] = group;
      return acc;
    },
    {},
  );

  return (
    <Card>
      <CardHeader>{t("tournaments.competitions.title")}</CardHeader>
      <CardBody className="p-0">
        {Object.entries(byScope).map(([scope, comps]) => (
          <div key={scope}>
            <div className="px-4 py-2 border-b border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800">
              <h5 className="font-heading font-bold text-xs uppercase tracking-wider text-gray-600 dark:text-gray-300">
                {t(`teamSelect.scopes.${scope}`)}
              </h5>
            </div>
            <div className="divide-y divide-gray-100 dark:divide-navy-600">
              {comps.map((comp) => {
                const status = getCompetitionStatus(comp);
                const isParticipating =
                  userTeamId != null &&
                  (comp.participant_ids?.includes(userTeamId) ?? false);

                return (
                  <button
                    key={comp.id}
                    onClick={() => onSelect(comp.id)}
                    className="w-full flex items-center gap-3 px-4 py-3 hover:bg-gray-50 dark:hover:bg-navy-700 text-left transition-colors"
                    data-testid={`competitions-overview-row-${comp.id}`}
                  >
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
                          {comp.name}
                        </span>
                        <Badge variant="neutral" size="sm">
                          {t(`teamSelect.kinds.${comp.kind ?? "League"}`)}
                        </Badge>
                        {isParticipating && (
                          <Badge variant="primary" size="sm">
                            <Users className="w-3 h-3 inline mr-0.5" />
                            {t("tournaments.competitions.participating")}
                          </Badge>
                        )}
                      </div>
                      <p className="text-xs text-gray-400 dark:text-gray-500 mt-0.5">
                        {t("schedule.season", { number: comp.season })}
                      </p>
                    </div>
                    <Badge
                      variant={
                        status === "completed"
                          ? "accent"
                          : status === "inProgress"
                            ? "primary"
                            : "neutral"
                      }
                      size="sm"
                    >
                      {status === "notStarted"
                        ? t("tournaments.competitions.statusNotStarted")
                        : status === "inProgress"
                          ? t("tournaments.competitions.statusInProgress")
                          : t("tournaments.competitions.statusCompleted")}
                    </Badge>
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
