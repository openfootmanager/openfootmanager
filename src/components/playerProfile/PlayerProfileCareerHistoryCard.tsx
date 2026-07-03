import type { CareerEntry } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";

type TranslateFn = (
    key: string,
    options?: Record<string, string | number>,
) => string;

interface PlayerProfileCareerHistoryCardProps {
    career: CareerEntry[];
    t: TranslateFn;
}

export default function PlayerProfileCareerHistoryCard({
    career,
    t,
}: PlayerProfileCareerHistoryCardProps) {
    return (
        <Card>
            <CardHeader>{t("playerProfile.careerHistory")}</CardHeader>
            <CardBody>
                {career.length > 0 ? (
                    <table className="w-full table-fixed text-xs">
                        <thead>
                            <tr className="border-b border-gray-200 dark:border-navy-500 text-gray-400 dark:text-gray-500 font-heading font-bold uppercase tracking-wider">
                                <th className="pb-2 pr-4 text-left font-bold">{t("common.team")}</th>
                                <th className="pb-2 w-[14%] text-right font-bold">{t("playerProfile.season")}</th>
                                <th className="pb-2 w-[14%] text-right font-bold">{t("playerProfile.apps")}</th>
                                <th className="pb-2 w-[14%] text-right font-bold">{t("playerProfile.goals")}</th>
                                <th className="pb-2 w-[14%] text-right font-bold">{t("playerProfile.assists")}</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                            {career.map((entry, index) => (
                                <tr key={`${entry.team_id}-${entry.season}-${index}`}>
                                    <td
                                        className="py-2 pr-4 font-semibold text-gray-800 dark:text-gray-200 truncate"
                                        title={entry.team_name}
                                    >
                                        {entry.team_name}
                                    </td>
                                    <td className="py-2 text-right text-gray-400 dark:text-gray-500 tabular-nums">
                                        {entry.season}/{entry.season + 1}
                                    </td>
                                    <td className="py-2 text-right text-gray-500 dark:text-gray-400 tabular-nums">
                                        {entry.appearances}
                                    </td>
                                    <td className="py-2 text-right text-gray-500 dark:text-gray-400 tabular-nums">
                                        {entry.goals}
                                    </td>
                                    <td className="py-2 text-right text-gray-500 dark:text-gray-400 tabular-nums">
                                        {entry.assists}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                ) : (
                    <p className="text-sm text-gray-400 dark:text-gray-500 text-center py-4">
                        {t("playerProfile.noCareer")}
                    </p>
                )}
            </CardBody>
        </Card>
    );
}