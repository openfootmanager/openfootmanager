import { useTranslation } from "react-i18next";
import type { CupRoundSummary } from "./types";

interface CupRoundSectionProps {
  summary: CupRoundSummary;
}

/** The user's cup ties of the day, shown beside the league round on a cup day. */
export default function CupRoundSection({ summary }: CupRoundSectionProps) {
  const { t } = useTranslation();
  const heading = summary.round_name
    ? `${summary.competition_name} · ${summary.round_name}`
    : summary.competition_name;

  return (
    <section
      aria-label={t("match.cupResults", { competition: heading })}
      className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300"
    >
      <h2 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
        {heading}
      </h2>
      <ul className="flex flex-col gap-2">
        {summary.results.map((tie) => (
          <li key={tie.fixture_id} className="text-sm text-gray-800 dark:text-gray-200">
            <span className="font-heading font-bold">
              {tie.home_team_name} {tie.home_goals} – {tie.away_goals} {tie.away_team_name}
            </span>
            {tie.home_penalties !== null && tie.away_penalties !== null && (
              <span className="ml-2 text-xs text-gray-600 dark:text-gray-300">
                {t("match.cupPenalties", { home: tie.home_penalties, away: tie.away_penalties })}
              </span>
            )}
            {tie.advancing_team_name && (
              <span className="ml-2 text-xs font-heading font-bold uppercase tracking-wider text-primary-700 dark:text-primary-400">
                {t("match.cupAdvances", { team: tie.advancing_team_name })}
              </span>
            )}
          </li>
        ))}
      </ul>
      {!summary.is_complete && (
        <p className="mt-3 text-xs text-gray-600 dark:text-gray-300">{t("match.cupTiesPending")}</p>
      )}
    </section>
  );
}
