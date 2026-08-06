import { Award, Calendar, GitBranch, TableProperties, Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";

export type TournamentsView = "overview" | "standings" | "fixtures" | "awards";

interface TournamentsViewTabsProps {
  view: TournamentsView;
  onSelectView: (view: TournamentsView) => void;
  /** A knockout competition shows a bracket where a league shows a table. */
  isKnockout: boolean;
}

export default function TournamentsViewTabs({
  view,
  onSelectView,
  isKnockout,
}: TournamentsViewTabsProps) {
  const { t } = useTranslation();

  const tabs: { id: TournamentsView; icon: React.ReactNode; label: string }[] = [
    {
      id: "overview",
      icon: <Trophy className="w-4 h-4 inline mr-1.5 -mt-0.5" />,
      label: t("tournaments.overview"),
    },
    isKnockout
      ? {
          id: "standings",
          icon: <GitBranch className="w-4 h-4 inline mr-1.5 -mt-0.5" />,
          label: t("tournaments.bracket"),
        }
      : {
          id: "standings",
          icon: <TableProperties className="w-4 h-4 inline mr-1.5 -mt-0.5" />,
          label: t("schedule.standings"),
        },
    {
      id: "fixtures",
      icon: <Calendar className="w-4 h-4 inline mr-1.5 -mt-0.5" />,
      label: t("schedule.fixtures"),
    },
    {
      id: "awards",
      icon: <Award className="w-4 h-4 inline mr-1.5 -mt-0.5" />,
      label: t("tournaments.awardsTab"),
    },
  ];

  return (
    <div className="flex gap-2 mb-5">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onSelectView(tab.id)}
          className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all ${
            view === tab.id
              ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
              : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 border border-gray-200 dark:border-navy-600"
          }`}
        >
          {tab.icon}
          {tab.label}
        </button>
      ))}
    </div>
  );
}
