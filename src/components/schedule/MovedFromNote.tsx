import { useTranslation } from "react-i18next";

import { formatMatchDate } from "../../lib/dateFormatting";

interface MovedFromNoteProps {
  originalDate: string;
  testId: string;
}

/** Where a postponed match was first scheduled, under its score. */
export default function MovedFromNote({ originalDate, testId }: MovedFromNoteProps) {
  const { t } = useTranslation();

  return (
    <span data-testid={testId} className="mt-1 block text-xs text-gray-600 dark:text-gray-300">
      {t("schedule.movedFrom", { date: formatMatchDate(originalDate) })}
    </span>
  );
}
