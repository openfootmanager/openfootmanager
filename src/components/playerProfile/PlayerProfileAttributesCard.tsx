import { useState } from "react";
import { Shield } from "lucide-react";
import { getAttributeColorClass } from "./PlayerProfile.helpers";
import type { PlayerAttributeGroup } from "./PlayerProfile.attributes";
import { Card, CardBody, CardHeader, ProgressBar } from "../ui";
import { PlayerAttributeRadarChart } from "./PlayerAttributeRadarChart";

interface PlayerProfileAttributesCardProps {
    attrGroups: PlayerAttributeGroup[];
    isOwnClub: boolean;
    isGk?: boolean;
    title: string;
    averageLabel: string;
    hiddenTitle: string;
    hiddenBody: string;
    listLabel: string;
    radarLabel: string;
}

export default function PlayerProfileAttributesCard({
    attrGroups,
    isOwnClub,
    isGk = false,
    title,
    averageLabel,
    hiddenTitle,
    hiddenBody,
    listLabel,
    radarLabel,
}: PlayerProfileAttributesCardProps) {
    const [view, setView] = useState<"list" | "radar">("list");

    return (
        <Card className="lg:col-span-2">
            <CardHeader
                action={
                    isOwnClub ? (
                        <div className="flex rounded-lg overflow-hidden border border-gray-200 dark:border-navy-600 text-[10px] font-heading font-bold uppercase tracking-wider">
                            <button
                                onClick={() => setView("list")}
                                className={`px-3 py-1 transition-colors ${view === "list" ? "bg-primary-500 text-white" : "text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700"}`}
                            >
                                {listLabel}
                            </button>
                            <button
                                onClick={() => setView("radar")}
                                className={`px-3 py-1 transition-colors ${view === "radar" ? "bg-primary-500 text-white" : "text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700"}`}
                            >
                                {radarLabel}
                            </button>
                        </div>
                    ) : null
                }
            >
                {title}
            </CardHeader>
            <CardBody>
                {isOwnClub && view === "radar" ? (
                    <PlayerAttributeRadarChart attrGroups={attrGroups} isGk={isGk} />
                ) : isOwnClub ? (
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                        {attrGroups.map((group) => (
                            <div key={group.label}>
                                <h4 className="font-heading font-bold text-xs uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3 pb-2 border-b border-gray-100 dark:border-navy-600">
                                    {group.label}
                                </h4>
                                <div className="flex flex-col gap-2.5">
                                    {group.attrs.map((attr) => (
                                        <div key={attr.name} className="flex items-center gap-3">
                                            <span className="text-sm text-gray-600 dark:text-gray-400 w-24">
                                                {attr.name}
                                            </span>
                                            <ProgressBar
                                                value={attr.value}
                                                variant="auto"
                                                size="sm"
                                                className="flex-1"
                                            />
                                            <span
                                                className={`font-heading font-bold text-sm w-8 text-right tabular-nums ${getAttributeColorClass(attr.value)}`}
                                            >
                                                {attr.value}
                                            </span>
                                        </div>
                                    ))}
                                    <div className="pt-1 border-t border-gray-100 dark:border-navy-600 flex items-center gap-3">
                                        <span className="text-sm text-gray-500 dark:text-gray-400 w-24 font-semibold">
                                            {averageLabel}
                                        </span>
                                        <span className="flex-1" />
                                        <span className="font-heading font-bold text-sm w-8 text-right tabular-nums text-gray-700 dark:text-gray-200">
                                            {group.average}
                                        </span>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                ) : (
                    <div className="text-center py-8">
                        <div className="w-14 h-14 rounded-full bg-gray-100 dark:bg-navy-700 flex items-center justify-center mx-auto mb-4">
                            <Shield className="w-7 h-7 text-gray-400 dark:text-gray-500" />
                        </div>
                        <p className="text-sm text-gray-500 dark:text-gray-400 font-medium">
                            {hiddenTitle}
                        </p>
                        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1 max-w-xs mx-auto">
                            {hiddenBody}
                        </p>
                        <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-6 text-left">
                            {attrGroups.map((group) => (
                                <div key={group.label}>
                                    <h4 className="font-heading font-bold text-xs uppercase tracking-wider text-gray-400 dark:text-gray-500 mb-2">
                                        {group.label}
                                    </h4>
                                    {group.attrs.map((attr) => (
                                        <div key={attr.name} className="flex items-center gap-3 mb-1.5">
                                            <span className="text-xs text-gray-400 dark:text-gray-500 w-24">
                                                {attr.name}
                                            </span>
                                            <div className="flex-1 h-2 bg-gray-200 dark:bg-navy-600 rounded-full overflow-hidden">
                                                <div
                                                    className="h-full bg-gray-300 dark:bg-navy-500 rounded-full"
                                                    style={{ width: `${Math.random() * 60 + 20}%` }}
                                                />
                                            </div>
                                            <span className="text-xs text-gray-400 dark:text-gray-500 w-6 text-right">
                                                ??
                                            </span>
                                        </div>
                                    ))}
                                </div>
                            ))}
                        </div>
                    </div>
                )}
            </CardBody>
        </Card>
    );
}