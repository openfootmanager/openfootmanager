import { Briefcase, Building2, TrendingUp, UserRound, UsersRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ReactNode } from "react";

import type { GameStateData, ManagerData, TeamData } from "../../store/gameStore";
import { countryName } from "../../lib/countries";
import { Badge, Card, CardBody, CardHeader, CountryFlag } from "../ui";

interface ManagersWorldTabProps {
  gameState: GameStateData;
  onSelectTeam?: (id: string) => void;
}

interface VacancyEntry {
  kind: "vacancy";
  team: TeamData;
}

interface ManagerEntry {
  kind: "manager";
  manager: ManagerData;
  team: TeamData | null;
}

type DirectoryEntry = VacancyEntry | ManagerEntry;

function winRateLabel(manager: ManagerData): string {
  const matches = manager.career_stats.matches_managed;
  if (matches === 0) {
    return "0%";
  }

  return `${Math.round((manager.career_stats.wins / matches) * 100)}%`;
}

function managerEntries(gameState: GameStateData): ManagerEntry[] {
  const managers = gameState.managers ?? [gameState.manager];

  return managers.map((manager) => ({
    kind: "manager",
    manager,
    team: gameState.teams.find((team) => team.id === manager.team_id) ?? null,
  }));
}

function vacancyEntries(gameState: GameStateData): VacancyEntry[] {
  return gameState.teams
    .filter((team) => team.manager_id == null)
    .map((team) => ({ kind: "vacancy", team }));
}

function latestRecordedMatches(team: TeamData): string {
  const latestSeason = team.history.reduce<TeamData["history"][number] | null>(
    (latest, record) => {
      if (!latest || record.season > latest.season) {
        return record;
      }

      return latest;
    },
    null,
  );

  return latestSeason ? latestSeason.played.toString() : "-";
}

function sortDirectory(entries: DirectoryEntry[]): DirectoryEntry[] {
  return [...entries].sort((left, right) => {
    if (left.kind !== right.kind) {
      return left.kind === "vacancy" ? 1 : -1;
    }

    if (left.kind === "vacancy" && right.kind === "vacancy") {
      return left.team.name.localeCompare(right.team.name);
    }

    const leftManager = (left as ManagerEntry).manager;
    const rightManager = (right as ManagerEntry).manager;

    return rightManager.reputation - leftManager.reputation
      || leftManager.last_name.localeCompare(rightManager.last_name)
      || leftManager.first_name.localeCompare(rightManager.first_name);
  });
}

export default function ManagersWorldTab({
  gameState,
  onSelectTeam,
}: ManagersWorldTabProps) {
  const { t, i18n } = useTranslation();
  const entries = sortDirectory([
    ...managerEntries(gameState),
    ...vacancyEntries(gameState),
  ]);

  return (
    <div className="max-w-6xl mx-auto space-y-5">
      <Card accent="primary">
        <CardBody className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <p className="text-xs font-heading font-bold uppercase tracking-[0.25em] text-primary-500">
              {t("managersWorld.title")}
            </p>
            <h2 className="text-2xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-gray-100">
              {t("managersWorld.subtitle")}
            </h2>
          </div>
          <div className="flex flex-wrap gap-2 text-xs">
            <Badge variant="primary" size="md">{t("managersWorld.employed")}</Badge>
            <Badge variant="neutral" size="md">{t("managersWorld.unemployed")}</Badge>
            <Badge variant="accent" size="md">{t("managersWorld.vacancy")}</Badge>
          </div>
        </CardBody>
      </Card>

      {entries.length === 0 ? (
        <Card>
          <CardBody className="py-12 text-center text-sm text-gray-500 dark:text-gray-400">
            {t("managersWorld.noManagers")}
          </CardBody>
        </Card>
      ) : (
        <div className="grid grid-cols-1 gap-5 xl:grid-cols-2">
          {entries.map((entry) => {
            if (entry.kind === "vacancy") {
              return (
                <Card key={`vacancy-${entry.team.id}`} accent="accent">
                  <CardHeader
                    action={<Badge variant="accent">{t("managersWorld.vacancyBadge")}</Badge>}
                  >
                    {t("managersWorld.vacancy")}
                  </CardHeader>
                  <CardBody className="space-y-4">
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <p className="text-xs font-heading font-bold uppercase tracking-[0.22em] text-gray-400 dark:text-gray-500">
                          {t("managersWorld.currentClub")}
                        </p>
                        <button
                          type="button"
                          onClick={() => onSelectTeam?.(entry.team.id)}
                          className="mt-1 text-left text-xl font-heading font-bold uppercase tracking-wide text-accent-500 transition-colors hover:text-accent-400"
                        >
                          {t("managersWorld.openRole", { team: entry.team.name })}
                        </button>
                      </div>
                      <div className="rounded-xl bg-accent-100 p-3 text-accent-700 dark:bg-accent-900/40 dark:text-accent-300">
                        <Briefcase className="h-6 w-6" />
                      </div>
                    </div>

                    <div className="grid grid-cols-2 gap-3 text-sm">
                      <StatTile
                        icon={<Building2 className="h-4 w-4" />}
                        label={t("managersWorld.reputation")}
                        value={entry.team.reputation.toString()}
                      />
                      <StatTile
                        icon={<UsersRound className="h-4 w-4" />}
                        label={t("managersWorld.matches")}
                        value={latestRecordedMatches(entry.team)}
                      />
                    </div>
                  </CardBody>
                </Card>
              );
            }

            const { manager, team } = entry;
            const isEmployed = team != null;

            return (
              <Card
                key={manager.id}
                accent={isEmployed ? "primary" : "none"}
              >
                <CardHeader
                  action={
                    <Badge variant={isEmployed ? "primary" : "neutral"}>
                      {t(isEmployed ? "managersWorld.employed" : "managersWorld.unemployed")}
                    </Badge>
                  }
                >
                  {`${manager.first_name} ${manager.last_name}`}
                </CardHeader>
                <CardBody className="space-y-4">
                  <div className="flex items-start justify-between gap-4">
                    <div>
                      <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                        <CountryFlag
                          code={manager.nationality}
                          locale={i18n.language}
                          className="text-sm leading-none"
                        />
                        <span>{countryName(manager.nationality, i18n.language)}</span>
                      </div>
                      {team ? (
                        <button
                          type="button"
                          onClick={() => onSelectTeam?.(team.id)}
                          className="mt-2 text-left text-xl font-heading font-bold uppercase tracking-wide text-primary-500 transition-colors hover:text-primary-400"
                        >
                          {team.name}
                        </button>
                      ) : (
                        <p className="mt-2 text-xl font-heading font-bold uppercase tracking-wide text-gray-700 dark:text-gray-200">
                          {t("managersWorld.unemployed")}
                        </p>
                      )}
                    </div>
                    <div className="rounded-xl bg-primary-100 p-3 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300">
                      <UserRound className="h-6 w-6" />
                    </div>
                  </div>

                  <div className="grid grid-cols-3 gap-3 text-sm">
                    <StatTile
                      icon={<TrendingUp className="h-4 w-4" />}
                      label={t("managersWorld.reputation")}
                      value={manager.reputation.toString()}
                    />
                    <StatTile
                      icon={<UsersRound className="h-4 w-4" />}
                      label={t("managersWorld.matches")}
                      value={manager.career_stats.matches_managed.toString()}
                    />
                    <StatTile
                      icon={<Briefcase className="h-4 w-4" />}
                      label={t("managersWorld.winRate")}
                      value={winRateLabel(manager)}
                    />
                  </div>
                </CardBody>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

interface StatTileProps {
  icon: ReactNode;
  label: string;
  value: string;
}

function StatTile({ icon, label, value }: StatTileProps) {
  return (
    <div className="rounded-lg bg-gray-50 p-3 dark:bg-navy-800/70">
      <div className="mb-2 flex items-center gap-2 text-gray-400 dark:text-gray-500">
        {icon}
        <span className="text-[11px] font-heading font-bold uppercase tracking-[0.18em]">
          {label}
        </span>
      </div>
      <p className="text-lg font-heading font-bold text-gray-800 dark:text-gray-100">
        {value}
      </p>
    </div>
  );
}