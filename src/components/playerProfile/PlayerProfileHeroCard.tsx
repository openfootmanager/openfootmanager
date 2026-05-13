import { Shield } from "lucide-react";
import { countryName } from "../../lib/countries";
import { positionBadgeVariant } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import { buildViewTeamMenuItem } from "../playerActions/playerContextMenuItems";
import { translatePositionLabel } from "../squad/SquadTab.helpers";
import { formatPlayerMarketValue, formatPlayerWage } from "./PlayerProfile.helpers";
import type {
    PlayerProfileScoutStatus,
    ScoutAvailability,
} from "./PlayerProfile.scouting";
import PlayerProfileScoutAction from "./PlayerProfileScoutAction";
import { TraitList } from "../TraitBadge";
import { Badge, Card, CountryFlag } from "../ui";

type TranslateFn = (
    key: string,
    options?: Record<string, string | number>,
) => string;

interface PlayerProfileHeroCardProps {
    player: PlayerData;
    ovr: number;
    primaryPosition: string;
    age: number;
    teamName: string;
    footednessLabel: string;
    weakFootValue: number;
    weeklySuffix: string;
    language: string;
    isOwnClub: boolean;
    scoutAvailability: ScoutAvailability;
    scoutStatus: PlayerProfileScoutStatus;
    scoutError: string | null;
    onScout: () => void;
    onSelectTeam?: (id: string) => void;
    t: TranslateFn;
}

export default function PlayerProfileHeroCard({
    player,
    ovr,
    primaryPosition,
    age,
    teamName,
    footednessLabel,
    weakFootValue,
    weeklySuffix,
    language,
    isOwnClub,
    scoutAvailability,
    scoutStatus,
    scoutError,
    onScout,
    onSelectTeam,
    t,
}: PlayerProfileHeroCardProps) {
    const teamContextItems = player.team_id && onSelectTeam
        ? [buildViewTeamMenuItem(t, () => onSelectTeam(player.team_id!))]
        : [];

    return (
        <Card accent="primary" className="mb-5">
            <div className="bg-linear-to-r from-navy-700 to-navy-800 p-8 rounded-t-xl">
                <div className="flex items-start gap-6">
                    <div
                        className={`w-24 h-24 rounded-2xl flex items-center justify-center font-heading font-bold text-4xl border-2 ${ovr >= 75
                            ? "bg-primary-500/20 text-primary-400 border-primary-500/30"
                            : ovr >= 55
                                ? "bg-accent-500/20 text-accent-400 border-accent-500/30"
                                : "bg-gray-500/20 text-gray-400 border-gray-500/30"
                            }`}
                    >
                        {ovr}
                    </div>
                    <div className="flex-1">
                        <h2 className="text-3xl font-heading font-bold text-white uppercase tracking-wide">
                            {player.full_name}
                        </h2>
                        <div className="flex items-center gap-3 mt-2">
                            <Badge variant={positionBadgeVariant(primaryPosition)}>
                                {translatePositionLabel(t, primaryPosition)}
                            </Badge>
                            {player.alternate_positions?.map((alternatePosition) => (
                                <Badge key={alternatePosition} variant="neutral">
                                    {translatePositionLabel(t, alternatePosition)}
                                </Badge>
                            ))}
                            <span className="text-gray-400 text-sm">
                                <CountryFlag
                                    code={player.nationality}
                                    locale={language}
                                    className="mr-1 text-sm leading-none"
                                />
                                {countryName(player.nationality, language)}
                            </span>
                            <span className="text-gray-500">•</span>
                            <span className="text-gray-400 text-sm">
                                {t("common.age")} {age}
                            </span>
                            <span className="text-gray-500">•</span>
                            <span className="text-gray-400 text-sm">
                                {t("common.footednessLabel")}: {" "}
                                {footednessLabel}
                            </span>
                            <span className="text-gray-500">•</span>
                            <span className="text-gray-400 text-sm">
                                {t("common.weakFoot")}: {" "}
                                {weakFootValue}/5
                            </span>
                        </div>
                        <p className="text-gray-400 text-sm mt-2 flex items-center gap-1.5">
                            <Shield className="w-4 h-4" />
                            {player.team_id && onSelectTeam ? (
                                <ContextMenu items={teamContextItems}>
                                    <button
                                        data-testid="player-profile-team-link"
                                        onClick={() => onSelectTeam(player.team_id!)}
                                        className="hover:text-primary-400 transition-colors underline underline-offset-2"
                                    >
                                        {teamName}
                                    </button>
                                </ContextMenu>
                            ) : (
                                <span>{teamName}</span>
                            )}
                        </p>
                        {player.traits && player.traits.length > 0 ? (
                            <div className="mt-3">
                                <TraitList traits={player.traits} size="sm" />
                            </div>
                        ) : null}
                    </div>

                    {!isOwnClub ? (
                        <div className="mt-3">
                            <PlayerProfileScoutAction
                                availability={scoutAvailability}
                                scoutStatus={scoutStatus}
                                scoutError={scoutError}
                                onScout={onScout}
                            />
                        </div>
                    ) : null}

                    <div className="hidden md:grid grid-cols-2 gap-3">
                        <QuickStat
                            label={t("common.condition")}
                            value={`${player.condition}%`}
                            color={player.condition >= 70 ? "text-primary-400" : "text-red-400"}
                        />
                        <QuickStat
                            label={t("common.morale")}
                            value={`${player.morale}%`}
                            color={player.morale >= 70 ? "text-primary-400" : "text-accent-400"}
                        />
                        <QuickStat
                            label={t("common.value")}
                            value={formatPlayerMarketValue(player.market_value)}
                            color="text-white"
                        />
                        <QuickStat
                            label={t("common.wage")}
                            value={formatPlayerWage(player.wage, weeklySuffix)}
                            color="text-white"
                        />
                    </div>
                </div>
            </div>

            <div className="grid grid-cols-4 gap-px bg-gray-200 dark:bg-navy-600 md:hidden">
                <MobileQuickStat
                    label={t("common.condition")}
                    value={`${player.condition}%`}
                    color={player.condition >= 70 ? "text-primary-500" : "text-red-500"}
                />
                <MobileQuickStat
                    label={t("common.morale")}
                    value={`${player.morale}%`}
                    color={player.morale >= 70 ? "text-primary-500" : "text-accent-500"}
                />
                <MobileQuickStat
                    label={t("common.value")}
                    value={formatPlayerMarketValue(player.market_value)}
                    color="text-gray-700 dark:text-gray-200"
                />
                <MobileQuickStat
                    label={t("common.wage")}
                    value={formatPlayerWage(player.wage, weeklySuffix)}
                    color="text-gray-700 dark:text-gray-200"
                />
            </div>
        </Card>
    );
}

function QuickStat({
    label,
    value,
    color,
}: {
    label: string;
    value: string;
    color: string;
}) {
    return (
        <div className="bg-white/5 rounded-xl px-5 py-3 text-center min-w-25">
            <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                {label}
            </p>
            <p className={`font-heading font-bold text-xl mt-0.5 ${color}`}>
                {value}
            </p>
        </div>
    );
}

function MobileQuickStat({
    label,
    value,
    color,
}: {
    label: string;
    value: string;
    color: string;
}) {
    return (
        <div className="bg-white dark:bg-navy-800 p-3 text-center">
            <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {label}
            </p>
            <p className={`font-heading font-bold text-lg mt-0.5 ${color}`}>
                {value}
            </p>
        </div>
    );
}
