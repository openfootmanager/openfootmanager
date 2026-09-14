import { Newspaper } from "lucide-react";
import { useTranslation } from "react-i18next";

import { formatDateShort, getTeamName } from "../../lib/helpers";
import type { NewsArticle, TeamData } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";

interface HomeLatestNewsCardProps {
  articles: NewsArticle[];
  teams: TeamData[];
  lang: string;
  onNavigate?: (tab: string) => void;
}

export default function HomeLatestNewsCard({
  articles,
  teams,
  lang,
  onNavigate,
}: HomeLatestNewsCardProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("News")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("home.allNews")}
          </button>
        }
      >
        {t("home.latestNews")}
      </CardHeader>
      <CardBody className="p-0">
        {articles.length === 0 ? (
          <div className="flex flex-col items-center gap-2 py-6">
            <Newspaper className="w-8 h-8 text-gray-300 dark:text-navy-600" />
            <p className="text-xs text-gray-500 dark:text-gray-400">{t("home.noNews")}</p>
          </div>
        ) : (
          <div className="divide-y divide-gray-100 dark:divide-navy-600">
            {articles.map((article) => (
              <button
                key={article.id}
                onClick={() => onNavigate?.("News")}
                className="w-full text-left px-4 py-3 hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors"
              >
                <p className="text-xs text-gray-400 dark:text-gray-500 mb-0.5">
                  {formatDateShort(article.date, lang)} - {article.source}
                </p>
                <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200 leading-snug line-clamp-2">
                  {article.headline}
                </p>
                {article.match_score && (
                  <div className="flex items-center gap-1.5 mt-1">
                    <span className="text-[10px] text-gray-500 dark:text-gray-400">
                      {getTeamName(teams, article.match_score.home_team_id)}
                    </span>
                    <span className="text-[10px] font-heading font-bold text-primary-500">
                      {article.match_score.home_goals}-{article.match_score.away_goals}
                    </span>
                    <span className="text-[10px] text-gray-500 dark:text-gray-400">
                      {getTeamName(teams, article.match_score.away_team_id)}
                    </span>
                  </div>
                )}
              </button>
            ))}
          </div>
        )}
      </CardBody>
    </Card>
  );
}
