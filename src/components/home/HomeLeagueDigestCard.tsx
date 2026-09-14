import { useTranslation } from "react-i18next";

import { formatDateShort } from "../../lib/helpers";
import type { NewsArticle } from "../../store/gameStore";
import { Badge, Card, CardBody, CardHeader } from "../ui";

interface HomeLeagueDigestCardProps {
  articles: NewsArticle[];
  lang: string;
  onNavigate?: (tab: string) => void;
}

export default function HomeLeagueDigestCard({
  articles,
  lang,
  onNavigate,
}: HomeLeagueDigestCardProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("News")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("dashboard.news")}
          </button>
        }
      >
        {t("home.leagueDigest")}
      </CardHeader>
      <CardBody className="p-0">
        {articles.length === 0 ? (
          <p className="text-sm text-gray-500 dark:text-gray-400 p-6 text-center">
            {t("home.noLeagueDigest")}
          </p>
        ) : (
          <div className="divide-y divide-gray-100 dark:divide-navy-600">
            {articles.map((article) => (
              <button
                key={article.id}
                onClick={() => onNavigate?.("News")}
                className="w-full text-left px-4 py-3 hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors"
              >
                <div className="flex items-center gap-2 mb-1.5">
                  <Badge variant="neutral" size="sm">
                    {t(`news.categories.${article.category}`)}
                  </Badge>
                  <span className="text-[10px] text-gray-400 dark:text-gray-500">
                    {formatDateShort(article.date, lang)} - {article.source}
                  </span>
                </div>
                <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200 leading-snug">
                  {article.headline}
                </p>
              </button>
            ))}
          </div>
        )}
      </CardBody>
    </Card>
  );
}
