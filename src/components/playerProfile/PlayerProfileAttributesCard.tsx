import { Fragment, useState } from "react";
import { Shield } from "lucide-react";
import { getAttributeColorClass } from "./PlayerProfile.helpers";
import { getAttributeColors } from "../../lib/playerAttributeDisplay";
import type { PlayerAttributeGroup } from "./PlayerProfile.attributes";
import { Card, CardBody, CardHeader, ProgressBar } from "../ui";
import { PlayerAttributeRadarChart } from "./PlayerAttributeRadarChart";
import PlayerProfileStatCard from "./PlayerProfileStatCard";

// Deterministic placeholder bar width (20-79%) for hidden attributes, derived
// from the attribute name. Stable across renders, unlike Math.random().
function placeholderWidth(name: string): number {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
        hash = (hash * 31 + name.charCodeAt(i)) % 60;
    }
    return hash + 20;
}

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
                                type="button"
                                aria-pressed={view === "list"}
                                onClick={() => setView("list")}
                                className={`px-3 py-1 transition-colors ${view === "list" ? "bg-primary-500 text-white" : "text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700"}`}
                            >
                                {listLabel}
                            </button>
                            <button
                                type="button"
                                aria-pressed={view === "radar"}
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
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 sm:auto-rows-fr">
                        {attrGroups.map((group) => (
                            <PlayerProfileStatCard
                                key={group.label}
                                label={group.label}
                                headerRight={
                                    <span
                                        title={averageLabel}
                                        className={`font-heading font-bold text-sm tabular-nums ${getAttributeColorClass(group.average)}`}
                                    >
                                        {group.average}
                                    </span>
                                }
                            >
                                <div className="grid grid-cols-[auto_1fr_1.75rem] items-center gap-x-3 gap-y-2.5">
                                    {group.attrs.map((attr) => (
                                        <Fragment key={attr.name}>
                                            <span className="text-xs text-gray-600 dark:text-gray-400 whitespace-nowrap">
                                                {attr.name}
                                            </span>
                                            <ProgressBar
                                                value={attr.value}
                                                variant={getAttributeColors(attr.value).barVariant}
                                                size="sm"
                                                className="min-w-0"
                                            />
                                            <span
                                                className={`font-heading font-bold text-xs text-right tabular-nums ${getAttributeColorClass(attr.value)}`}
                                            >
                                                {attr.value}
                                            </span>
                                        </Fragment>
                                    ))}
                                </div>
                            </PlayerProfileStatCard>
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
                        <div className="mt-6 grid grid-cols-1 sm:grid-cols-2 gap-4 sm:auto-rows-fr text-left">
                            {attrGroups.map((group) => (
                                <PlayerProfileStatCard
                                    key={group.label}
                                    label={group.label}
                                    labelClassName="text-gray-400 dark:text-gray-500"
                                    headerRight={
                                        <span className="font-heading font-bold text-sm text-gray-400 dark:text-gray-500">
                                            ??
                                        </span>
                                    }
                                >
                                    <div className="grid grid-cols-[auto_1fr_1.75rem] items-center gap-x-3 gap-y-2.5">
                                        {group.attrs.map((attr) => (
                                            <Fragment key={attr.name}>
                                                <span className="text-xs text-gray-400 dark:text-gray-500 whitespace-nowrap">
                                                    {attr.name}
                                                </span>
                                                <ProgressBar
                                                    value={placeholderWidth(attr.name)}
                                                    variant="muted"
                                                    size="sm"
                                                    className="min-w-0"
                                                />
                                                <span className="text-xs text-gray-400 dark:text-gray-500 text-right">
                                                    ??
                                                </span>
                                            </Fragment>
                                        ))}
                                    </div>
                                </PlayerProfileStatCard>
                            ))}
                        </div>
                    </div>
                )}
            </CardBody>
        </Card>
    );
}
