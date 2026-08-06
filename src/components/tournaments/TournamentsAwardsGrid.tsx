import { Award, Briefcase, Shield, Star, Trophy, Users, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";

import TournamentsAwardCard from "./TournamentsAwardCard";
import type {
  SeasonAwardEntryData,
  SeasonManagerAwardEntryData,
  SeasonAwardsData,
} from "../../store/gameStore";

interface TournamentsAwardsGridProps {
  awards: SeasonAwardsData | null;
  awardsLoadState: "idle" | "loading" | "error";
  retryAwards: () => void;
  seasonComplete: boolean;
  onSelectTeam: (id: string) => void;
  onSelectPlayer?: (id: string) => void;
}

interface AwardCardSpec {
  id: string;
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  entries: SeasonAwardEntryData[] | SeasonManagerAwardEntryData[];
  unit: string;
  decimal?: boolean;
}

/**
 * The season awards board: seven trophies, or the state of trying to load them.
 *
 * Every `t()` argument here is a literal on purpose. `frontendKeyCoverage.test.ts`
 * walks the AST and only sees keys passed to `t()` directly, so folding these
 * into a table of key strings would take them out of that gate's reach.
 */
export default function TournamentsAwardsGrid({
  awards,
  awardsLoadState,
  retryAwards,
  seasonComplete,
  onSelectTeam,
  onSelectPlayer,
}: TournamentsAwardsGridProps) {
  const { t } = useTranslation();

  const cards: AwardCardSpec[] = awards
    ? [
        {
          id: "managerOfSeason",
          icon: <Briefcase className="w-5 h-5 text-accent-500" />,
          title: t("tournaments.awards.managerOfSeasonTitle"),
          subtitle: t("tournaments.awards.managerOfSeasonSubtitle"),
          entries: awards.manager_of_season,
          unit: t("tournaments.awards.units.winRate"),
          decimal: false,
        },
        {
          id: "goldenBoot",
          icon: <Zap className="w-5 h-5 text-accent-500" />,
          title: t("tournaments.awards.goldenBootTitle"),
          subtitle: t("tournaments.awards.goldenBootSubtitle"),
          entries: awards.golden_boot,
          unit: t("tournaments.awards.units.goals"),
        },
        {
          id: "assistKing",
          icon: <Star className="w-5 h-5 text-purple-500" />,
          title: t("tournaments.awards.assistKingTitle"),
          subtitle: t("tournaments.awards.assistKingSubtitle"),
          entries: awards.assist_king,
          unit: t("tournaments.awards.units.assists"),
        },
        {
          id: "playerOfYear",
          icon: <Trophy className="w-5 h-5 text-primary-500" />,
          title: t("tournaments.awards.playerOfYearTitle"),
          subtitle: t("tournaments.awards.playerOfYearSubtitle"),
          entries: awards.player_of_year,
          unit: t("tournaments.awards.units.rating"),
          decimal: true,
        },
        {
          id: "goldenGlove",
          icon: <Shield className="w-5 h-5 text-blue-500" />,
          title: t("tournaments.awards.goldenGloveTitle"),
          subtitle: t("tournaments.awards.goldenGloveSubtitle"),
          entries: awards.clean_sheet_king,
          unit: t("tournaments.awards.units.cleanSheets"),
        },
        {
          id: "everPresent",
          icon: <Users className="w-5 h-5 text-green-500" />,
          title: t("tournaments.awards.everPresentTitle"),
          subtitle: t("tournaments.awards.everPresentSubtitle"),
          entries: awards.most_appearances,
          unit: t("tournaments.awards.units.apps"),
        },
        {
          id: "youngPlayer",
          icon: <Star className="w-5 h-5 text-amber-500" />,
          title: t("tournaments.awards.youngPlayerTitle"),
          subtitle: t("tournaments.awards.youngPlayerSubtitle"),
          entries: awards.young_player,
          unit: t("tournaments.awards.units.rating"),
          decimal: true,
        },
      ]
    : [];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
      {!seasonComplete && (
        <div className="md:col-span-2 lg:col-span-3 rounded-xl border border-amber-300/50 bg-amber-50 px-4 py-3 text-sm text-amber-700 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-300">
          {t("tournaments.awards.currentLeaders")}
        </div>
      )}
      {awards ? (
        cards.map((card) => (
          <TournamentsAwardCard
            key={card.id}
            icon={card.icon}
            title={card.title}
            subtitle={card.subtitle}
            entries={card.entries}
            unit={card.unit}
            emptyText={t("tournaments.awards.noDataYet")}
            decimal={card.decimal ?? false}
            onSelectPlayer={onSelectPlayer}
            onSelectTeam={onSelectTeam}
          />
        ))
      ) : awardsLoadState === "error" ? (
        <div className="col-span-full text-center py-12">
          <Award className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
          <p className="text-sm text-gray-400 dark:text-gray-500 mb-4">
            {t("tournaments.awards.noDataYet")}
          </p>
          <button
            onClick={retryAwards}
            className="px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider bg-primary-500 text-white hover:bg-primary-600 transition-colors"
          >
            {t("common.retry")}
          </button>
        </div>
      ) : (
        <div className="col-span-full text-center py-12">
          <Award className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
          <p className="text-sm text-gray-400 dark:text-gray-500">
            {t("tournaments.loadingAwards")}
          </p>
        </div>
      )}
    </div>
  );
}
