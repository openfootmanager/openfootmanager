import { Medal, Trophy, UserRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ReactNode } from "react";

import type { GameStateData } from "../../store/gameStore";
import { countryName } from "../../lib/countries";
import { Badge, Card, CardBody, CardHeader, CountryFlag } from "../ui";
import {
  deriveHallOfFameLegends,
  derivePastChampions,
} from "./HallOfFameWorldTab.model";

interface HallOfFameWorldTabProps {
  gameState: GameStateData;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam?: (id: string) => void;
}

export default function HallOfFameWorldTab({
  gameState,
  onSelectPlayer,
  onSelectTeam,
}: HallOfFameWorldTabProps) {
  const { t, i18n } = useTranslation();
  const legends = deriveHallOfFameLegends(gameState);
  const champions = derivePastChampions(gameState);

  return (
    <div className="mx-auto max-w-6xl space-y-5">
      <Card accent="primary">
        <CardBody className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div>
            <p className="text-xs font-heading font-bold uppercase tracking-[0.25em] text-primary-500">
              {t("hallOfFameWorld.title")}
            </p>
            <h2 className="text-2xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-gray-100">
              {t("hallOfFameWorld.subtitle")}
            </h2>
          </div>
          <div className="grid grid-cols-2 gap-3 text-sm md:min-w-[20rem]">
            <SummaryTile
              icon={<UserRound className="h-4 w-4" />}
              label={t("hallOfFameWorld.legendsCount", { count: legends.length })}
              value={legends.length.toString()}
            />
            <SummaryTile
              icon={<Trophy className="h-4 w-4" />}
              label={t("hallOfFameWorld.championsCount", { count: champions.length })}
              value={champions.length.toString()}
            />
          </div>
        </CardBody>
      </Card>

      <section className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-gray-100">
            {t("hallOfFameWorld.legends")}
          </h3>
          <Badge variant="primary" size="md">{legends.length}</Badge>
        </div>

        {legends.length === 0 ? (
          <Card>
            <CardBody className="py-10 text-center text-sm text-gray-500 dark:text-gray-400">
              {t("hallOfFameWorld.noLegends")}
            </CardBody>
          </Card>
        ) : (
          <div className="grid grid-cols-1 gap-5 xl:grid-cols-2">
            {legends.map((legend) => {
              const footballNation = legend.player.football_nation ?? legend.player.nationality;

              return (
                <Card key={legend.player.id} accent="primary">
                  <CardHeader
                    action={<Badge variant="accent">{`${t("hallOfFameWorld.titles")}: ${legend.titles}`}</Badge>}
                  >
                    <button
                      type="button"
                      onClick={() => onSelectPlayer?.(legend.player.id)}
                      className="text-left transition-colors hover:text-primary-500"
                    >
                      {legend.player.full_name}
                    </button>
                  </CardHeader>
                  <CardBody className="space-y-4">
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                          <CountryFlag
                            code={footballNation}
                            locale={i18n.language}
                            className="text-sm leading-none"
                          />
                          <span>{countryName(footballNation, i18n.language)}</span>
                        </div>
                        <p className="mt-2 text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-gray-100">
                          {legend.player.position}
                        </p>
                        {legend.lastClubName ? (
                          <p className="mt-1 text-sm text-primary-500">
                            {`${t("hallOfFameWorld.lastClub")}: ${legend.lastClubName}`}
                          </p>
                        ) : null}
                      </div>
                      <div className="rounded-xl bg-primary-100 p-3 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300">
                        <Medal className="h-6 w-6" />
                      </div>
                    </div>

                    <div className="grid grid-cols-2 gap-3 text-sm">
                      <StatTile
                        label={t("hallOfFameWorld.appearances")}
                        value={legend.appearances.toString()}
                      />
                      <StatTile
                        label={t("hallOfFameWorld.goals")}
                        value={legend.goals.toString()}
                      />
                      <StatTile
                        label={t("hallOfFameWorld.assists")}
                        value={legend.assists.toString()}
                      />
                      <StatTile
                        label={t("hallOfFameWorld.finalSeason")}
                        value={legend.finalSeason?.toString() ?? "-"}
                      />
                    </div>
                  </CardBody>
                </Card>
              );
            })}
          </div>
        )}
      </section>

      <section className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-gray-100">
            {t("hallOfFameWorld.pastChampions")}
          </h3>
          <Badge variant="accent" size="md">{champions.length}</Badge>
        </div>

        {champions.length === 0 ? (
          <Card>
            <CardBody className="py-10 text-center text-sm text-gray-500 dark:text-gray-400">
              {t("hallOfFameWorld.noChampions")}
            </CardBody>
          </Card>
        ) : (
          <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
            {champions.map((champion) => (
              <Card key={`${champion.team.id}-${champion.season}`} accent="accent">
                <CardBody className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
                  <div>
                    <p className="text-xs font-heading font-bold uppercase tracking-[0.2em] text-gray-400 dark:text-gray-500">
                      {t("hallOfFameWorld.seasonLabel", { season: champion.season })}
                    </p>
                    <button
                      type="button"
                      onClick={() => onSelectTeam?.(champion.team.id)}
                      className="mt-1 text-left text-xl font-heading font-bold uppercase tracking-wide text-accent-500 transition-colors hover:text-accent-400"
                    >
                      {champion.team.name}
                    </button>
                    <div className="mt-2 flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                      <CountryFlag
                        code={champion.team.country}
                        locale={i18n.language}
                        className="text-sm leading-none"
                      />
                      <span>{countryName(champion.team.country, i18n.language)}</span>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-3 text-sm md:min-w-[16rem]">
                    <StatTile
                      label={t("hallOfFameWorld.played")}
                      value={champion.record.played.toString()}
                    />
                    <StatTile
                      label={t("hallOfFameWorld.goals")}
                      value={champion.record.goals_for.toString()}
                    />
                    <StatTile
                      label={t("hallOfFameWorld.record")}
                      value={`${champion.record.won}-${champion.record.drawn}-${champion.record.lost}`}
                    />
                    <StatTile
                      label={t("hallOfFameWorld.goalDifference")}
                      value={(champion.record.goals_for - champion.record.goals_against).toString()}
                    />
                  </div>
                </CardBody>
              </Card>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

interface SummaryTileProps {
  icon: ReactNode;
  label: string;
  value: string;
}

function SummaryTile({ icon, label, value }: SummaryTileProps) {
  return (
    <div className="rounded-xl bg-gray-50 p-3 dark:bg-navy-800/70">
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

interface StatTileProps {
  label: string;
  value: string;
}

function StatTile({ label, value }: StatTileProps) {
  return (
    <div className="rounded-lg bg-gray-50 p-3 dark:bg-navy-800/70">
      <p className="text-[11px] font-heading font-bold uppercase tracking-[0.18em] text-gray-400 dark:text-gray-500">
        {label}
      </p>
      <p className="mt-2 text-lg font-heading font-bold text-gray-800 dark:text-gray-100">
        {value}
      </p>
    </div>
  );
}
