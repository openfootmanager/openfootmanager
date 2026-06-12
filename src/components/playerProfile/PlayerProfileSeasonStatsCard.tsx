import type { PlayerSeasonStats } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";

type TranslateFn = (key: string) => string;

interface PlayerProfileSeasonStatsCardProps {
    stats: PlayerSeasonStats;
    t: TranslateFn;
}

export default function PlayerProfileSeasonStatsCard({
    stats,
    t,
}: PlayerProfileSeasonStatsCardProps) {
    return (
        <Card>
            <CardHeader>{t("playerProfile.seasonStats")}</CardHeader>
            <CardBody>
                <div className="grid grid-cols-4 md:grid-cols-8 gap-3">
                    <StatBox label={t("playerProfile.apps")} value={stats.appearances} />
                    <StatBox label={t("playerProfile.goals")} value={stats.goals} />
                    <StatBox label={t("playerProfile.assists")} value={stats.assists} />
                    <StatBox label={t("playerProfile.mins")} value={stats.minutes_played} />
                    <StatBox
                        label={t("playerProfile.cleanSheets")}
                        value={stats.clean_sheets}
                    />
                    <StatBox
                        label={t("playerProfile.yellows")}
                        value={stats.yellow_cards}
                    />
                    <StatBox label={t("playerProfile.reds")} value={stats.red_cards} />
                    <StatBox
                        label={t("playerProfile.avgRating")}
                        value={stats.avg_rating > 0 ? stats.avg_rating.toFixed(1) : "-"}
                    />
                </div>
            </CardBody>
        </Card>
    );
}

function StatBox({ label, value }: { label: string; value: number | string }) {
    return (
        <div className="text-center p-2.5 bg-gray-50 dark:bg-navy-700 rounded-lg">
            <p className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100 tabular-nums">
                {value}
            </p>
            <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {label}
            </p>
        </div>
    );
}