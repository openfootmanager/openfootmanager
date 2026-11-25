import {
  ArrowLeft,
  Calendar as CalendarIcon,
  ChevronDown,
  ChevronRight,
  Save,
  Search,
} from "lucide-react";
import { type JSX, type ReactNode, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { PlayerData, TeamData } from "../../store/gameStore";
import type { MatchModeType } from "../../hooks/useAdvanceTime";
import { Spinner, ThemeToggle } from "../ui";
import DashboardSearch from "./DashboardSearch";
import { FOCUS_RING, MENU_FOCUS } from "./dashboardStyles";

/** A match mode in the Continue menu. Its label and icon name it; it has no colour of its own. */
export interface DashboardMatchModeMeta {
  desc: string;
  icon: ReactNode;
  label: string;
}

interface DashboardHeaderProps {
  title: string;
  currentDate: string;
  /** The date without its weekday, for a narrow header. */
  currentDateShort: string;
  hasProfileHistory: boolean;
  hasMatchToday: boolean;
  isAdvancing: boolean;
  isUnemployed: boolean;
  isSaving: boolean;
  matchMode: MatchModeType;
  /** Every player in the world: what the header search looks through. */
  players: PlayerData[];
  modeMeta: Record<MatchModeType, DashboardMatchModeMeta>;
  onBack: () => void;
  onContinue: () => void;
  onSave: () => void;
  onSelectMatchMode: (mode: MatchModeType) => void;
  onSelectSearchPlayer: (playerId: string) => void;
  onSelectSearchTeam: (teamId: string) => void;
  onSkipToMatchDay: () => void;
  onToggleContinueMenu: () => void;
  saveFlash: boolean;
  seasonComplete: boolean;
  showContinueMenu: boolean;
  teams: TeamData[];
}

const ICON_BUTTON = `rounded-lg p-2 text-fg-muted transition-colors hover:bg-hover hover:text-fg ${FOCUS_RING}`;
function getSaveButtonClassName(saveFlash: boolean, isSaving: boolean): string {
  const state = saveFlash
    ? "bg-positive-tint text-positive"
    : "bg-sunken text-fg-muted ring-1 ring-inset ring-line hover:bg-hover hover:text-fg";
  return `flex shrink-0 items-center gap-1.5 rounded-lg px-3 py-2.5 font-heading text-body font-bold uppercase tracking-label transition-colors hover:cursor-pointer ${FOCUS_RING} ${state} ${
    isSaving ? "cursor-wait opacity-70" : ""
  }`;
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
    return <Spinner decorative className="h-4 w-4 border-2 border-current" />;
  }

  return <Save className="h-4 w-4" />;
}

// The game's one forward action (layout rule L3): the brand fill whatever the match mode, which
// its label and icon name.
const FORWARD_ACTION = `flex items-center gap-2 bg-primary-700 py-2.5 font-heading text-body font-bold uppercase tracking-label text-on-brand transition-colors hover:cursor-pointer hover:bg-primary-800 ${FOCUS_RING}`;

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
      <span className="whitespace-nowrap">
        {hasMatchToday ? matchModeMeta.label : t("dashboard.continue")}
      </span>
    </>
  );
}

/**
 * The global header: the page's only title, the date, search, save and the game's forward action.
 * It is a container, and shrinks by layout rule L5's steps as it narrows: the search field folds
 * to a button under 48rem, and the date drops its weekday under 42rem.
 */
export default function DashboardHeader({
  title,
  currentDate,
  currentDateShort,
  hasProfileHistory,
  hasMatchToday,
  isAdvancing,
  isUnemployed,
  isSaving,
  matchMode,
  players,
  modeMeta,
  onBack,
  onContinue,
  onSave,
  onSelectMatchMode,
  onSelectSearchPlayer,
  onSelectSearchTeam,
  onSkipToMatchDay,
  onToggleContinueMenu,
  saveFlash,
  seasonComplete,
  showContinueMenu,
  teams,
}: DashboardHeaderProps): JSX.Element {
  const { t } = useTranslation();
  const currentModeMeta = modeMeta[matchMode];
  // A narrow header shows search as a button; pressing it opens the field until it loses focus.
  const [searchExpanded, setSearchExpanded] = useState(false);
  const searchInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (searchExpanded) searchInput.current?.focus();
  }, [searchExpanded]);

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
    <header className="@container/header z-10 flex items-center gap-3 border-b border-line bg-panel px-6 py-3">
      <div className="flex min-w-0 items-center gap-3">
        {hasProfileHistory && (
          <button
            type="button"
            onClick={onBack}
            className={`-ml-2 shrink-0 ${ICON_BUTTON}`}
            title={t("common.back")}
            aria-label={t("common.back")}
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
        )}
        <div className="min-w-0">
          {/* Focusable from script only: after a page is chosen from the sidebar's drawer, the
              focus lands here, on the page, rather than going back to the sidebar. */}
          <h2
            data-page-title
            tabIndex={-1}
            className="max-w-64 truncate font-heading text-title font-bold uppercase tracking-label text-fg"
            title={title}
          >
            {title}
          </h2>
          <p className="mt-0.5 flex items-center gap-1.5 whitespace-nowrap text-caption text-fg-muted">
            <CalendarIcon className="h-3.5 w-3.5 shrink-0" />
            <span className="hidden font-medium @2xl/header:inline">{currentDate}</span>
            <span className="font-medium @2xl/header:hidden">{currentDateShort}</span>
          </p>
        </div>
      </div>

      <DashboardSearch
        players={players}
        teams={teams}
        expanded={searchExpanded}
        onCollapse={() => setSearchExpanded(false)}
        inputRef={searchInput}
        onSelectPlayer={onSelectSearchPlayer}
        onSelectTeam={onSelectSearchTeam}
      />

      <div className="ml-auto flex shrink-0 items-center gap-3">
        {searchExpanded ? null : (
          <button
            type="button"
            onClick={() => setSearchExpanded(true)}
            className={`${ICON_BUTTON} @3xl/header:hidden`}
            aria-label={t("common.search")}
            title={t("common.search")}
          >
            <Search className="h-5 w-5" />
          </button>
        )}
        <ThemeToggle />
        <button
          type="button"
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
            type="button"
            onClick={handleContinueClick}
            disabled={isAdvancing}
            className={`${FORWARD_ACTION} rounded-lg px-4 disabled:cursor-wait disabled:opacity-70`}
          >
            <span>{isAdvancing ? t("dashboard.simulating") : t("dashboard.continue")}</span>
            <ChevronRight className={`h-4 w-4 ${isAdvancing ? "animate-pulse" : ""}`} />
          </button>
        ) : (
          <div className="relative">
            <div className="flex">
              <button
                type="button"
                onClick={handleContinueClick}
                disabled={isAdvancing || seasonComplete}
                className={`${FORWARD_ACTION} rounded-l-lg pl-4 pr-3 disabled:cursor-wait disabled:opacity-70`}
              >
                {renderContinueButtonContent(
                  t,
                  hasMatchToday,
                  isAdvancing,
                  seasonComplete,
                  currentModeMeta,
                )}
                <ChevronRight className={`h-4 w-4 ${isAdvancing ? "animate-pulse" : ""}`} />
              </button>
              <button
                type="button"
                onClick={handleContinueMenuToggleClick}
                aria-label={t("dashboard.matchModeMenu")}
                aria-expanded={showContinueMenu}
                className={`${FORWARD_ACTION} rounded-r-lg border-l border-on-brand/30 px-2`}
              >
                <ChevronDown className="h-4 w-4" aria-hidden="true" />
              </button>
            </div>

            {showContinueMenu && (
              <div className="absolute right-0 top-full z-20 mt-1 w-64 rounded-lg border border-line bg-panel py-1 shadow-lg">
                {(["live", "spectator", "delegate"] as const).map((mode) => {
                  const isActive = matchMode === mode;
                  const optionMeta = modeMeta[mode];

                  return (
                    <button
                      type="button"
                      key={mode}
                      onClick={() => onSelectMatchMode(mode)}
                      className={`flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-hover ${MENU_FOCUS} ${
                        isActive ? "bg-hover" : ""
                      }`}
                    >
                      <span className={isActive ? "text-positive" : "text-fg-subtle"}>
                        {optionMeta.icon}
                      </span>
                      <div className="flex-1">
                        <span className="font-heading text-body font-bold uppercase tracking-label text-fg">
                          {optionMeta.label}
                        </span>
                        <p className="mt-0.5 text-caption text-fg-muted">{optionMeta.desc}</p>
                      </div>
                      {isActive && <span className="text-caption font-bold text-positive">✓</span>}
                    </button>
                  );
                })}
                <div className="my-1 border-t border-line" />
                <button
                  type="button"
                  onClick={handleSkipToMatchDayClick}
                  className={`w-full px-4 py-2.5 text-left transition-colors hover:bg-hover ${MENU_FOCUS}`}
                >
                  <span className="font-heading text-body font-bold uppercase tracking-label text-fg">
                    {t("continueMenu.skipToMatchDay")}
                  </span>
                  <p className="mt-0.5 text-caption text-fg-muted">
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
