import {
  ArrowLeft,
  Calendar as CalendarIcon,
  ChevronDown,
  ChevronRight,
  Loader2,
  Save,
  Search,
} from "lucide-react";
import type { JSX, ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { getTeamName } from "../../lib/helpers";
import type { PlayerData, TeamData } from "../../store/gameStore";
import type { MatchModeType } from "../../hooks/useAdvanceTime";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import {
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import { Badge, ThemeToggle } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { getPlayerBadgeVariant } from "./dashboardHelpers";

export interface DashboardMatchModeMeta {
  buttonColorClass: string;
  desc: string;
  dropdownColorClass: string;
  icon: ReactNode;
  label: string;
}

interface DashboardHeaderProps {
  activeTabLabel: string;
  currentDate: string;
  hasProfileHistory: boolean;
  hasMatchToday: boolean;
  isAdvancing: boolean;
  isUnemployed: boolean;
  isSaving: boolean;
  matchMode: MatchModeType;
  matchedPlayers: PlayerData[];
  matchedTeams: TeamData[];
  modeMeta: Record<MatchModeType, DashboardMatchModeMeta>;
  onBack: () => void;
  onContinue: () => void;
  onSave: () => void;
  onSearchBlur: () => void;
  onSearchFocus: () => void;
  onSearchQueryChange: (query: string) => void;
  onSelectMatchMode: (mode: MatchModeType) => void;
  onSelectSearchPlayer: (playerId: string) => void;
  onSelectSearchTeam: (teamId: string) => void;
  onSkipToMatchDay: () => void;
  onToggleContinueMenu: () => void;
  saveFlash: boolean;
  searchOpen: boolean;
  searchQuery: string;
  seasonComplete: boolean;
  showContinueMenu: boolean;
  teams: TeamData[];
}

function getSaveButtonClassName(saveFlash: boolean, isSaving: boolean): string {
  let className =
    "flex items-center gap-1.5 rounded-lg px-3 py-2.5 text-sm font-heading font-bold uppercase tracking-wider transition-all hover:cursor-pointer";

  if (saveFlash) {
    className = `${className} bg-green-500 text-white`;
  } else {
    className = `${className} bg-gray-200 text-gray-600 hover:bg-gray-300 dark:bg-navy-700 dark:text-gray-300 dark:hover:bg-navy-600`;
  }

  if (isSaving) {
    className = `${className} cursor-wait opacity-70`;
  }

  return className;
}

function getSaveButtonLabel(
  t: (key: string) => string,
  saveFlash: boolean,
  isSaving: boolean,
): string {
  if (saveFlash) {
    return t("dashboard.saved");
  }

  if (isSaving) {
    return t("dashboard.saving");
  }

  return t("common.save");
}

function renderSaveButtonIcon(isSaving: boolean): JSX.Element {
  if (isSaving) {
    return <Loader2 className="h-4 w-4 animate-spin" />;
  }

  return <Save className="h-4 w-4" />;
}

function getContinueButtonClassName(
  modeMeta: DashboardMatchModeMeta,
  isAdvancing: boolean,
  seasonComplete: boolean,
): string {
  let className = `bg-linear-to-r ${modeMeta.buttonColorClass} flex items-center gap-2 rounded-l-lg pl-4 pr-3 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-white shadow-md transition-all hover:cursor-pointer hover:brightness-110 hover:shadow-lg`;

  if (isAdvancing || seasonComplete) {
    className = `${className} cursor-wait opacity-70`;
  }

  return className;
}

function getContinueDropdownButtonClassName(
  modeMeta: DashboardMatchModeMeta,
): string {
  return `bg-linear-to-r ${modeMeta.dropdownColorClass} rounded-r-lg border-l border-white/20 px-2 py-2.5 text-white transition-colors hover:brightness-110`;
}

function getModeOptionClassName(isActive: boolean): string {
  const baseClassName =
    "flex w-full items-center gap-3 px-4 py-2.5 text-left text-sm transition-colors hover:bg-gray-50 dark:hover:bg-navy-600";

  if (isActive) {
    return `${baseClassName} bg-gray-50 dark:bg-navy-600`;
  }

  return baseClassName;
}

function getModeOptionIconClassName(isActive: boolean): string {
  if (isActive) {
    return "text-primary-500";
  }

  return "text-gray-400";
}

function renderContinueButtonContent(
  t: (key: string) => string,
  hasMatchToday: boolean,
  isAdvancing: boolean,
  seasonComplete: boolean,
  matchModeMeta: DashboardMatchModeMeta,
): ReactNode {
  if (seasonComplete) {
    return <span>{t("endOfSeason.seasonComplete")}</span>;
  }

  if (isAdvancing) {
    return <span>{t("dashboard.simulating")}</span>;
  }

  return (
    <>
      {matchModeMeta.icon}
      <span>
        {hasMatchToday ? matchModeMeta.label : t("dashboard.continue")}
      </span>
    </>
  );
}

function renderSearchResults(props: {
  matchedPlayers: PlayerData[];
  matchedTeams: TeamData[];
  onSelectSearchPlayer: (playerId: string) => void;
  onSelectSearchTeam: (teamId: string) => void;
  teams: TeamData[];
  t: (key: string) => string;
}): JSX.Element {
  const {
    matchedPlayers,
    matchedTeams,
    onSelectSearchPlayer,
    onSelectSearchTeam,
    t,
    teams,
  } = props;

  if (matchedPlayers.length === 0 && matchedTeams.length === 0) {
    return (
      <p className="p-3 text-xs text-gray-400 dark:text-gray-500">
        {t("dashboard.noResults")}
      </p>
    );
  }

  return (
    <>
      {matchedTeams.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
            {t("dashboard.searchTeams")}
          </p>
          {matchedTeams.map((team) => {
            const contextItems = [
              buildViewTeamMenuItem(t, () => onSelectSearchTeam(team.id)),
            ];

            return (
              <ContextMenu items={contextItems} key={team.id}>
                <button
                  onMouseDown={() => onSelectSearchTeam(team.id)}
                  className="flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-gray-50 dark:hover:bg-navy-600"
                  data-testid={`dashboard-search-team-${team.id}`}
                >
                  <div
                    className="flex h-6 w-6 items-center justify-center rounded text-xs font-bold text-white"
                    style={{ backgroundColor: team.colors.primary }}
                  >
                    {team.short_name.charAt(0)}
                  </div>
                  <span className="text-sm font-medium text-gray-800 dark:text-gray-200">
                    {team.name}
                  </span>
                  <span className="ml-auto text-xs text-gray-400">{team.city}</span>
                </button>
              </ContextMenu>
            );
          })}
        </div>
      )}
      {matchedPlayers.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
            {t("dashboard.searchPlayers")}
          </p>
          {matchedPlayers.map((player) => {
            const contextItems: ContextMenuItem[] = [
              buildViewProfileMenuItem(t, () => onSelectSearchPlayer(player.id)),
            ];

            if (player.team_id) {
              contextItems.push(
                buildViewTeamMenuItem(t, () => onSelectSearchTeam(player.team_id!)),
              );
            }

            return (
              <ContextMenu items={contextItems} key={player.id}>
                <button
                  onMouseDown={() => onSelectSearchPlayer(player.id)}
                  className="flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-gray-50 dark:hover:bg-navy-600"
                  data-testid={`dashboard-search-player-${player.id}`}
                >
                  <Badge variant={getPlayerBadgeVariant(player.position)} size="sm">
                    {translatePositionAbbreviation(t, player.position)}
                  </Badge>
                  <span className="text-sm font-medium text-gray-800 dark:text-gray-200">
                    {player.full_name}
                  </span>
                  <span className="ml-auto text-xs text-gray-400">
                    {getTeamName(teams, player.team_id ?? "")}
                  </span>
                </button>
              </ContextMenu>
            );
          })}
        </div>
      )}
    </>
  );
}

export default function DashboardHeader({
  activeTabLabel,
  currentDate,
  hasProfileHistory,
  hasMatchToday,
  isAdvancing,
  isUnemployed,
  isSaving,
  matchMode,
  matchedPlayers,
  matchedTeams,
  modeMeta,
  onBack,
  onContinue,
  onSave,
  onSearchBlur,
  onSearchFocus,
  onSearchQueryChange,
  onSelectMatchMode,
  onSelectSearchPlayer,
  onSelectSearchTeam,
  onSkipToMatchDay,
  onToggleContinueMenu,
  saveFlash,
  searchOpen,
  searchQuery,
  seasonComplete,
  showContinueMenu,
  teams,
}: DashboardHeaderProps): JSX.Element {
  const { t } = useTranslation();
  const currentModeMeta = modeMeta[matchMode];
  const showSearchResults = searchOpen && searchQuery.length >= 2;

  function handleContinueClick(): void {
    console.info("[DashboardHeader] continueClick", {
      hasMatchToday,
      isAdvancing,
      matchMode,
      seasonComplete,
      showContinueMenu,
    });
    onContinue();
  }

  function handleContinueMenuToggleClick(): void {
    console.info("[DashboardHeader] continueMenuToggleClick", {
      hasMatchToday,
      isAdvancing,
      matchMode,
      seasonComplete,
      showContinueMenu,
    });
    onToggleContinueMenu();
  }

  function handleSkipToMatchDayClick(): void {
    console.info("[DashboardHeader] skipToMatchDayClick", {
      hasMatchToday,
      isAdvancing,
      matchMode,
      seasonComplete,
      showContinueMenu,
    });
    onSkipToMatchDay();
  }

  return (
    <header className="z-10 flex items-center justify-between border-b border-gray-200 bg-white px-6 py-3 shadow-sm transition-colors duration-300 dark:border-navy-700 dark:bg-navy-800">
      <div className="flex items-center gap-3">
        {hasProfileHistory && (
          <button
            onClick={onBack}
            className="-ml-2 rounded-lg p-2 text-gray-400 transition-colors hover:bg-gray-100 hover:text-gray-700 dark:hover:bg-navy-700 dark:hover:text-white"
            title={t("common.back")}
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
        )}
        <div>
          <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
            {activeTabLabel}
          </h2>
          <p className="mt-0.5 flex items-center gap-1.5 text-xs text-gray-500 dark:text-gray-400">
            <CalendarIcon className="h-3.5 w-3.5" />
            <span className="font-medium">{currentDate}</span>
          </p>
        </div>
      </div>

      <div className="relative mx-auto flex-1 max-w-md">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
        <input
          type="text"
          placeholder={t("dashboard.searchPlaceholder")}
          value={searchQuery}
          onChange={(event) => onSearchQueryChange(event.target.value)}
          onFocus={onSearchFocus}
          onBlur={onSearchBlur}
          className="w-full rounded-lg border border-gray-200 bg-gray-100 py-2 pl-9 pr-3 text-sm text-gray-800 transition-all placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500/50 dark:border-navy-600 dark:bg-navy-700 dark:text-gray-200 dark:placeholder-gray-500"
        />
        {showSearchResults && (
          <div className="absolute left-0 right-0 top-full z-30 mt-1 max-h-80 overflow-y-auto rounded-lg border border-gray-200 bg-white shadow-xl dark:border-navy-600 dark:bg-navy-700">
            {renderSearchResults({
              matchedPlayers,
              matchedTeams,
              onSelectSearchPlayer,
              onSelectSearchTeam,
              t,
              teams,
            })}
          </div>
        )}
      </div>

      <div className="flex items-center gap-3">
        <ThemeToggle />
        <button
          onClick={onSave}
          disabled={isSaving}
          className={getSaveButtonClassName(saveFlash, isSaving)}
          title={t("dashboard.saveGame")}
        >
          {renderSaveButtonIcon(isSaving)}
          {getSaveButtonLabel(t, saveFlash, isSaving)}
        </button>
        {isUnemployed ? (
          <button
            onClick={handleContinueClick}
            disabled={isAdvancing}
            className="bg-linear-to-r from-gray-600 to-gray-700 flex items-center gap-2 rounded-lg px-4 py-2.5 text-sm font-heading font-bold uppercase tracking-wider text-white shadow-md transition-all hover:cursor-pointer hover:brightness-110 hover:shadow-lg disabled:cursor-wait disabled:opacity-70"
          >
            <span>{isAdvancing ? t("dashboard.simulating") : t("dashboard.continue")}</span>
            <ChevronRight className={`h-4 w-4 ${isAdvancing ? "animate-pulse" : ""}`} />
          </button>
        ) : (
          <div className="relative">
            <div className="flex">
              <button
                onClick={handleContinueClick}
                disabled={isAdvancing || seasonComplete}
                className={getContinueButtonClassName(
                  currentModeMeta,
                  isAdvancing,
                  seasonComplete,
                )}
              >
                {renderContinueButtonContent(
                  t,
                  hasMatchToday,
                  isAdvancing,
                  seasonComplete,
                  currentModeMeta,
                )}
                <ChevronRight
                  className={`h-4 w-4 ${isAdvancing ? "animate-pulse" : ""}`}
                />
              </button>
              <button
                onClick={handleContinueMenuToggleClick}
                className={getContinueDropdownButtonClassName(currentModeMeta)}
              >
                <ChevronDown className="h-4 w-4" />
              </button>
            </div>

            {showContinueMenu && (
              <div className="absolute right-0 top-full z-20 mt-1 w-64 rounded-lg border border-gray-200 bg-white py-1 shadow-xl dark:border-navy-600 dark:bg-navy-700">
                {(["live", "spectator", "delegate"] as const).map((mode) => {
                  const isActive = matchMode === mode;
                  const optionMeta = modeMeta[mode];

                  return (
                    <button
                      key={mode}
                      onClick={() => onSelectMatchMode(mode)}
                      className={getModeOptionClassName(isActive)}
                    >
                      <span className={getModeOptionIconClassName(isActive)}>
                        {optionMeta.icon}
                      </span>
                      <div className="flex-1">
                        <span className="text-xs font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
                          {optionMeta.label}
                        </span>
                        <p className="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
                          {optionMeta.desc}
                        </p>
                      </div>
                      {isActive && (
                        <span className="text-xs font-bold text-primary-500">
                          ✓
                        </span>
                      )}
                    </button>
                  );
                })}
                <div className="my-1 border-t border-gray-200 dark:border-navy-600" />
                <button
                  onClick={handleSkipToMatchDayClick}
                  className="w-full px-4 py-2.5 text-left text-sm transition-colors hover:bg-gray-50 dark:hover:bg-navy-600"
                >
                  <span className="text-xs font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
                    {t("continueMenu.skipToMatchDay")}
                  </span>
                  <p className="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
                    {t("continueMenu.skipToMatchDayDesc")}
                  </p>
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </header>
  );
}
