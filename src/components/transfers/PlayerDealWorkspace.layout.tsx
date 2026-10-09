import type { ReactNode } from "react";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PlayerData, TeamData } from "../../store/gameStore";
import { countryName } from "../../lib/countries";
import {
  calcAge,
  formatAnnualAmount,
  formatVal,
  getPlayerOvr,
  getTeamName,
  positionBadgeVariant,
} from "../../lib/helpers";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { Badge, CountryFlag, PlayerAvatar } from "../ui";
import type { DealKind } from "./PlayerDealWorkspace";

export interface DealOption {
  kind: DealKind;
  title: string;
  description: string;
  detail: string;
  disabledReason: string | null;
  icon: ReactNode;
}

function routeButtonClass(isSelected: boolean, isDisabled: boolean): string {
  const base =
    "min-h-[88px] w-full rounded-lg border px-3 py-3 text-left transition-colors duration-150 motion-reduce:transition-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800";
  if (isDisabled)
    return `${base} border-gray-200 bg-gray-50 text-gray-500 opacity-80 dark:border-navy-600 dark:bg-navy-900/50 dark:text-gray-300`;
  if (isSelected)
    return `${base} border-primary-600 bg-primary-50 text-gray-900 dark:border-primary-400 dark:bg-primary-900/50 dark:text-white`;
  return `${base} border-gray-200 bg-white text-gray-700 hover:bg-gray-50 hover:text-gray-950 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-300 dark:hover:bg-navy-700 dark:hover:text-white`;
}

function factLabelClass(): string {
  return "text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-300";
}

function factValueClass(): string {
  return "mt-1 text-sm font-semibold text-gray-900 dark:text-gray-100";
}

export function DealWorkspaceHeader({
  player,
  teams,
  transferWindowSummary,
  onClose,
}: {
  player: PlayerData;
  teams: TeamData[];
  transferWindowSummary: string;
  onClose: () => void;
}) {
  const { t, i18n } = useTranslation();
  const teamName = player.team_id ? getTeamName(teams, player.team_id) : t("common.freeAgent");
  const age = calcAge(player.date_of_birth);
  return (
    <header className="shrink-0 border-b border-gray-200 bg-white px-4 py-3 shadow-sm dark:shadow-sm dark:border-navy-600 dark:bg-navy-800">
      <div className="flex items-center gap-4">
        <button
          type="button"
          onClick={onClose}
          className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 -ml-2 flex shrink-0 items-center gap-2 rounded-lg px-2 py-2 text-sm text-gray-500 transition-colors duration-150 motion-reduce:transition-none hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-navy-700 dark:hover:text-white"
          aria-label={t("common.back")}
        >
          <ArrowLeft className="h-5 w-5" />
          <span className="hidden font-heading font-bold uppercase tracking-wider sm:inline">
            {t("common.back")}
          </span>
        </button>
        <div className="flex min-w-0 flex-1 items-center gap-4">
          <PlayerAvatar
            player={player}
            className="flex h-16 w-16 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-gray-200 dark:border-navy-600 bg-gray-100 text-sm font-heading font-bold text-gray-500  dark:bg-navy-700 dark:text-gray-300"
            imageClassName="h-full w-full object-cover object-top"
          />
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h2
                id="player-deal-workspace-title"
                className="truncate font-heading text-2xl font-bold uppercase tracking-wide text-gray-950 dark:text-white"
              >
                {player.full_name}
              </h2>
              <Badge
                variant={positionBadgeVariant(player.natural_position || player.position)}
                size="sm"
              >
                {translatePositionAbbreviation(t, player.natural_position || player.position)}
              </Badge>
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-gray-500 dark:text-gray-300">
              <span>{age}</span>
              <span className="flex items-center gap-1">
                <CountryFlag
                  code={player.nationality}
                  locale={i18n.language}
                  className="text-sm leading-none"
                />
                {countryName(player.nationality, i18n.language)}
              </span>
              <span>{teamName}</span>
              <span>{transferWindowSummary}</span>
            </div>
          </div>
        </div>
      </div>
    </header>
  );
}

export function DealWorkspaceRoutes({
  options,
  selectedKind,
  onSelectKind,
}: {
  options: DealOption[];
  selectedKind: DealKind;
  onSelectKind: (kind: DealKind) => void;
}) {
  const { t } = useTranslation();
  return (
    <nav aria-label={t("transfers.dealType")} className="space-y-3 lg:min-h-0 lg:overflow-y-auto">
      {options.map((option) => {
        const disabled = Boolean(option.disabledReason);
        const selected = option.kind === selectedKind;

        return (
          <button
            key={option.kind}
            type="button"
            disabled={disabled}
            onClick={() => onSelectKind(option.kind)}
            className={routeButtonClass(selected, disabled)}
            aria-pressed={selected}
            aria-label={option.title}
          >
            <span className="flex items-start gap-3">
              <span
                className={`mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md ${
                  disabled
                    ? "bg-gray-200 text-gray-500 dark:bg-navy-700 dark:text-gray-300"
                    : selected
                      ? "bg-primary-700 text-white dark:bg-primary-700 dark:text-white"
                      : "bg-gray-100 text-gray-600 dark:bg-navy-700 dark:text-gray-300"
                }`}
              >
                {option.icon}
              </span>
              <span className="min-w-0">
                <span className="font-heading text-sm font-bold uppercase tracking-wider">
                  {option.title}
                </span>
                {/* Subtitle describes the route action, not availability, so a
                          selected route never reads as a confirmed deal (#305). */}
                <span className="mt-1 block text-xs text-gray-500 dark:text-gray-300">
                  {option.disabledReason ?? option.description}
                </span>
                {!disabled ? (
                  <span className="mt-1 block text-xs font-medium uppercase tracking-wide text-gray-500 dark:text-gray-300">
                    {option.detail}
                  </span>
                ) : null}
              </span>
            </span>
          </button>
        );
      })}
    </nav>
  );
}

export function DealWorkspaceFacts({
  player,
  myTeam,
  offeredWage,
  weeklySuffix,
}: {
  player: PlayerData;
  myTeam: TeamData | null;
  offeredWage?: number | null;
  weeklySuffix: string;
}) {
  const { t } = useTranslation();
  const ovr = getPlayerOvr(player);
  return (
    <aside className="min-h-0 space-y-4 lg:overflow-y-auto">
      <div className="rounded-lg border border-gray-200 dark:border-navy-600 bg-white p-4  dark:bg-navy-800 ">
        <p className={factLabelClass()}>{t("common.ovr")}</p>
        <p className="mt-1 font-heading text-3xl font-bold tabular-nums text-primary-700 dark:text-primary-400">
          {ovr}
        </p>
        <div className="mt-4 grid grid-cols-2 gap-3">
          <div>
            <p className={factLabelClass()}>{t("common.value")}</p>
            <p className={`${factValueClass()} tabular-nums`}>{formatVal(player.market_value)}</p>
          </div>
          <div>
            <p className={factLabelClass()}>{t("common.currentWage")}</p>
            <p className={`${factValueClass()} tabular-nums`}>
              {formatAnnualAmount(formatVal(player.wage), weeklySuffix)}
            </p>
          </div>
        </div>
        {offeredWage != null && offeredWage > 0 ? (
          <div className="mt-3 border-t border-gray-100 pt-3 dark:border-navy-700">
            <p className={factLabelClass()}>{t("playerProfile.renewalWage")}</p>
            <p className={`${factValueClass()} tabular-nums`}>
              {formatAnnualAmount(formatVal(offeredWage), weeklySuffix)}
            </p>
          </div>
        ) : null}
      </div>

      {myTeam ? (
        <div className="rounded-lg border border-gray-200 dark:border-navy-600 bg-white p-4  dark:bg-navy-800 ">
          <p className={factLabelClass()}>{t("finances.transferBudget")}</p>
          <p className={`${factValueClass()} tabular-nums`}>{formatVal(myTeam.transfer_budget)}</p>
          <div className="mt-4">
            <p className={factLabelClass()}>{t("finances.wageBudget")}</p>
            <p className={`${factValueClass()} tabular-nums`}>
              {formatAnnualAmount(formatVal(myTeam.wage_budget), weeklySuffix)}
            </p>
          </div>
        </div>
      ) : null}
    </aside>
  );
}
