import {
  ArrowLeft,
  Calendar as CalendarIcon,
  ChevronDown,
  ChevronRight,
  Loader2,
  Save,
  Search,
} from "lucide-react";
import { type JSX, type MouseEvent, type ReactNode, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { getTeamName } from "../../lib/helpers";
import { naturalPositionOf } from "../../lib/positions";
import type { PlayerData, TeamData } from "../../store/gameStore";
import type { MatchModeType } from "../../hooks/useAdvanceTime";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import {
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import { PlayerAvatar, TeamLogo, ThemeToggle } from "../ui";
import PlayerPositionBadge from "../PlayerPositionBadge";

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

const FOCUS_RING =
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel";
const ICON_BUTTON = `rounded-lg p-2 text-fg-muted transition-colors hover:bg-hover hover:text-fg ${FOCUS_RING}`;
// Inside a menu the ring is inset: an offset ring would be clipped by the menu's edge.
const MENU_FOCUS =
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus";
const MENU_ROW = `flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-hover ${MENU_FOCUS}`;

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
    return <Loader2 className="h-4 w-4 animate-spin" />;
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
 * A mouse press on a result must not take the focus from the field: WebKit doesn't focus a pressed
 * button, so the field would blur, the search would fold and the click would land on nothing.
 */
function keepFocusInSearch(event: MouseEvent): void {
  event.preventDefault();
}

function renderSearchResults(props: {
  matchedPlayers: PlayerData[];
  matchedTeams: TeamData[];
  onSelectSearchPlayer: (playerId: string) => void;
  onSelectSearchTeam: (teamId: string) => void;
  teams: TeamData[];
  t: (key: string) => string;
}): JSX.Element {
  const { matchedPlayers, matchedTeams, onSelectSearchPlayer, onSelectSearchTeam, t, teams } =
    props;

  if (matchedPlayers.length === 0 && matchedTeams.length === 0) {
    return <p className="p-3 text-caption text-fg-subtle">{t("dashboard.noResults")}</p>;
  }

  return (
    <>
      {matchedTeams.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-caption font-semibold text-fg-muted">
            {t("dashboard.searchTeams")}
          </p>
          {matchedTeams.map((team) => {
            const contextItems = [buildViewTeamMenuItem(t, () => onSelectSearchTeam(team.id))];

            return (
              <ContextMenu items={contextItems} key={team.id}>
                <button
                  type="button"
                  onMouseDown={keepFocusInSearch}
                  onClick={() => onSelectSearchTeam(team.id)}
                  className={MENU_ROW}
                  data-testid={`dashboard-search-team-${team.id}`}
                >
                  <TeamLogo
                    team={team}
                    className="flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-md"
                    imageClassName="h-5 w-5 object-contain"
                  />
                  <span className="text-body font-medium text-fg">{team.name}</span>
                  <span className="ml-auto text-caption text-fg-subtle">{team.city}</span>
                </button>
              </ContextMenu>
            );
          })}
        </div>
      )}
      {matchedPlayers.length > 0 && (
        <div>
          <p className="px-3 pb-1 pt-2 text-caption font-semibold text-fg-muted">
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
                  type="button"
                  onMouseDown={keepFocusInSearch}
                  onClick={() => onSelectSearchPlayer(player.id)}
                  className={MENU_ROW}
                  data-testid={`dashboard-search-player-${player.id}`}
                >
                  <PlayerAvatar
                    player={player}
                    className="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-md bg-sunken font-heading text-caption font-bold text-fg-muted"
                  />
                  <PlayerPositionBadge position={naturalPositionOf(player)} />
                  <span className="text-body font-medium text-fg">{player.full_name}</span>
                  <span className="ml-auto text-caption text-fg-subtle">
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

      {/* The search folds when the focus leaves the field and its results, not when it moves from
          the field into a result: a keyboard player tabs into the results to pick one. */}
      <div
        data-search-field
        className={`relative min-w-0 flex-1 ${searchExpanded ? "" : "hidden @3xl/header:block"}`}
        onBlur={(event) => {
          if (event.currentTarget.contains(event.relatedTarget as Node | null)) return;
          setSearchExpanded(false);
          onSearchBlur();
        }}
      >
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-subtle" />
        <input
          ref={searchInput}
          type="text"
          placeholder={t("dashboard.searchPlaceholder")}
          aria-label={t("common.search")}
          value={searchQuery}
          onChange={(event) => onSearchQueryChange(event.target.value)}
          onFocus={onSearchFocus}
          className={`w-full rounded-lg border border-line bg-sunken py-2 pl-9 pr-3 text-body text-fg placeholder:text-fg-subtle ${FOCUS_RING}`}
        />
        {showSearchResults && (
          <div className="absolute left-0 right-0 top-full z-30 mt-1 max-h-80 overflow-y-auto rounded-lg border border-line bg-panel shadow-lg">
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
