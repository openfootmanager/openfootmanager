import { GitCompareArrows } from "lucide-react";
import { useTranslation } from "react-i18next";

import { calcAge, calcOvr, positionBadgeVariant } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { Badge, Button, CountryFlag } from "../ui";
import { translatePositionLabel } from "../squad/SquadTab.helpers";
import {
    getAttributeComparisonState,
    getNormalizedPlayerPosition,
    getVisibleAttributeGroups,
    valueBarTone,
    valueTone,
} from "./TacticsPlayerFocusPanel.helpers";

export function PlayerSummaryCard({
    currentDate,
    label,
    player,
}: {
    currentDate: string;
    label: string;
    player: PlayerData;
}) {
    const { t } = useTranslation();
    const normalizedPosition = getNormalizedPlayerPosition(player);
    const displayPosition = player.natural_position || player.position;
    const overallRating = calcOvr(player, displayPosition);

    return (
        <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800/70 px-4 py-4">
            <div className="flex items-start justify-between gap-3">
                <div>
                    <p className="text-sm font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                        {label}
                    </p>
                    <p className="mt-1 text-base font-heading font-bold text-gray-900 dark:text-gray-100">
                        {player.full_name}
                    </p>
                    <div className="mt-2 flex flex-wrap items-center gap-2">
                        <Badge variant={positionBadgeVariant(normalizedPosition)} size="sm">
                            {translatePositionLabel(t, displayPosition)}
                        </Badge>
                        <span className="text-xs text-gray-500 dark:text-gray-400">
                            <CountryFlag
                                code={player.nationality}
                                className="mr-1 text-xs leading-none"
                            />
                            {t("common.age", "Age")} {calcAge(player.date_of_birth, currentDate)}
                        </span>
                    </div>
                </div>
                <div className="shrink-0 text-right">
                    <div className="text-sm font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                        {t("common.ovr", "OVR")}
                    </div>
                    <div className="text-3xl font-heading font-bold text-primary-500 dark:text-primary-400">
                        {overallRating}
                    </div>
                </div>
            </div>
        </div>
    );
}

export function SinglePlayerAttributeGroups({
    player,
}: {
    player: PlayerData;
}) {
    const { t } = useTranslation();

    return (
        <div className="space-y-4">
            {getVisibleAttributeGroups([player]).map((group) => (
                <div key={group.labelKey}>
                    <h4 className="mb-2 text-sm font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                        {t(group.labelKey)}
                    </h4>
                    <div className="space-y-2">
                        {group.attrs.map((attr) => {
                            const value = player.attributes[attr];
                            return (
                                <div
                                    key={attr}
                                    className="grid grid-cols-[minmax(0,1fr)_40px] items-center gap-3"
                                >
                                    <div>
                                        <div className="mb-1 text-xs text-gray-600 dark:text-gray-300">
                                            {t(`common.attributes.${attr}`)}
                                        </div>
                                        <div className="h-2 overflow-hidden rounded-full bg-gray-100 dark:bg-navy-800">
                                            <div
                                                className={`h-full rounded-full ${valueBarTone(value)}`}
                                                style={{ width: `${value}%` }}
                                            />
                                        </div>
                                    </div>
                                    <span
                                        className={`text-sm font-heading font-bold tabular-nums ${valueTone(value)}`}
                                    >
                                        {value}
                                    </span>
                                </div>
                            );
                        })}
                    </div>
                </div>
            ))}
        </div>
    );
}

export function ComparePlayerAttributes({
    canConfirmSwap,
    comparePlayer,
    currentDate,
    onConfirmSwap,
    selectedPlayer,
}: {
    canConfirmSwap: boolean;
    comparePlayer: PlayerData;
    currentDate: string;
    onConfirmSwap: () => void;
    selectedPlayer: PlayerData;
}) {
    const { t } = useTranslation();

    return (
        <div className="space-y-4">
            <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
                <PlayerSummaryCard
                    currentDate={currentDate}
                    label={t("tactics.selectedPlayer", "Selected player")}
                    player={selectedPlayer}
                />
                <PlayerSummaryCard
                    currentDate={currentDate}
                    label={t("tactics.comparePlayer", "Comparison player")}
                    player={comparePlayer}
                />
            </div>
            <div className="flex flex-col gap-3 rounded-xl border border-gray-200 bg-gray-50 px-4 py-3 dark:border-navy-600 dark:bg-navy-800/70 sm:flex-row sm:items-center sm:justify-between">
                <p className="text-sm text-gray-600 dark:text-gray-300">
                    {t(
                        "tactics.compareSelectionHint",
                        "Review both players below, then confirm the swap when you're ready.",
                    )}
                </p>
                <Button
                    type="button"
                    size="sm"
                    onClick={onConfirmSwap}
                    disabled={!canConfirmSwap}
                >
                    {t("tactics.confirmSwap", "Confirm swap")}
                </Button>
            </div>
            {getVisibleAttributeGroups([selectedPlayer, comparePlayer]).map((group) => (
                <div key={group.labelKey}>
                    <h4 className="mb-2 text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                        {t(group.labelKey)}
                    </h4>
                    <div className="space-y-2">
                        {group.attrs.map((attr) => {
                            const left = selectedPlayer.attributes[attr];
                            const right = comparePlayer.attributes[attr];
                            const { leftWins, rightWins } = getAttributeComparisonState(
                                left,
                                right,
                            );
                            return (
                                <div
                                    key={attr}
                                    className="grid grid-cols-[minmax(0,1fr)_90px_minmax(0,1fr)] items-center gap-2"
                                >
                                    <div
                                        className={`rounded-lg px-2 py-2 ${leftWins ? "bg-primary-500/10 ring-1 ring-primary-500/20" : "bg-gray-50 dark:bg-navy-800/70"}`}
                                    >
                                        <div className="mb-1 flex items-center justify-between gap-2 text-xs">
                                            <span className={`font-heading font-bold ${valueTone(left)}`}>
                                                {left}
                                            </span>
                                        </div>
                                        <div className="h-2 overflow-hidden rounded-full bg-white dark:bg-navy-700">
                                            <div
                                                className={`h-full rounded-full ${valueBarTone(left)}`}
                                                style={{ width: `${left}%` }}
                                            />
                                        </div>
                                    </div>
                                    <div className="text-center text-[10px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                                        {t(`common.attributes.${attr}`)}
                                    </div>
                                    <div
                                        className={`rounded-lg px-2 py-2 ${rightWins ? "bg-primary-500/10 ring-1 ring-primary-500/20" : "bg-gray-50 dark:bg-navy-800/70"}`}
                                    >
                                        <div className="mb-1 flex items-center justify-between gap-2 text-xs">
                                            <span className={`font-heading font-bold ${valueTone(right)}`}>
                                                {right}
                                            </span>
                                        </div>
                                        <div className="h-2 overflow-hidden rounded-full bg-white dark:bg-navy-700">
                                            <div
                                                className={`h-full rounded-full ${valueBarTone(right)}`}
                                                style={{ width: `${right}%` }}
                                            />
                                        </div>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                </div>
            ))}
        </div>
    );
}

export function EmptyFocusState() {
    const { t } = useTranslation();

    return (
        <div className="rounded-xl border border-dashed border-gray-200 px-4 py-8 text-center dark:border-navy-600">
            <GitCompareArrows className="mx-auto mb-3 h-10 w-10 text-gray-300 dark:text-navy-600" />
            <p className="text-sm text-gray-600 dark:text-gray-300">
                {t(
                    "tactics.selectPitchPlayer",
                    "Select a player in the lineup view to inspect their attributes.",
                )}
            </p>
            <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                {t(
                    "tactics.selectAnotherToSwap",
                    "Select a second player to compare them and confirm the swap.",
                )}
            </p>
        </div>
    );
}