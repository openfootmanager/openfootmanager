import { Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";

interface TournamentsPreseasonNoteProps {
  /** The overview shows this inside a shared card, so the padding differs. */
  className?: string;
}

/** Why there is no table yet: the season has not started. */
export default function TournamentsPreseasonNote({
  className = "px-6 py-8",
}: TournamentsPreseasonNoteProps) {
  const { t } = useTranslation();

  return (
    <div className={`flex flex-col items-center gap-2 text-center ${className}`}>
      <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
      <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
        {t("season.standingsLocked")}
      </p>
      <p className="text-xs text-gray-500 dark:text-gray-400 max-w-md">
        {t("season.tournamentsPreseasonHint")}
      </p>
    </div>
  );
}
