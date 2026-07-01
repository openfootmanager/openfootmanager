import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Calendar as CalendarIcon,
  Globe,
  TableProperties,
  Trophy,
} from "lucide-react";

import {
  getActiveCompetitions,
  getNationalTeamFixtures,
  getNationalTeamName,
  getPromotionRelegationZones,
  getTeamName,
  formatMatchDate,
  getUserCalledUpPlayers,
} from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { competitionDisplayName } from "../../lib/competitionName";
import type { GameStateData, LeagueData } from "../../store/gameStore";
import {
  fetchSchedule,
  type MatchdayGroup,
  type ScheduleSlice,
} from "../../services/scheduleService";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import ContextMenu, { type ContextMenuItem } from "../ContextMenu";
import { Badge, Card, CardBody, Select } from "../ui";
import ScheduleCalendarGrid from "./ScheduleCalendarGrid";

interface ScheduleTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

const PAST_PAGE_SIZE = 4;

function sortStandings(competition: LeagueData | null): LeagueData["standings"] {
  if (!competition) return [];
  return [...competition.standings].sort(
    (a, b) =>
      b.points - a.points ||
      b.goals_for - b.goals_against - (a.goals_for - a.goals_against) ||
      b.goals_for - a.goals_for,
  );
}

type View = "calendar" | "fixtures" | "standings" | "international";

export default function ScheduleTab({ gameState, onSelectTeam }: ScheduleTabProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<View>("calendar");
  const [selectedCompetitionId, setSelectedCompetitionId] = useState<string | null>(null);
  const [slice, setSlice] = useState<ScheduleSlice | null>(null);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [visiblePastCount, setVisiblePastCount] = useState(PAST_PAGE_SIZE);
  const groupRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  const userTeamId = gameState.manager.team_id;
  const activeCompetitions = getActiveCompetitions(gameState);
  const userCompetitions = activeCompetitions.filter((c) =>
    c.participant_ids?.includes(userTeamId ?? ""),
  );
  const nationalFixtures = getNationalTeamFixtures(gameState);
  const hasInternational = nationalFixtures.length > 0;
  const calledUpPlayers = getUserCalledUpPlayers(gameState);
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";

  // Resolve which competition to show.
  const selectedCompetition =
    activeCompetitions.find((c) => c.id === selectedCompetitionId) ??
    userCompetitions.find((c) => c.standings.length > 0) ??
    activeCompetitions.find((c) => c.standings.length > 0) ??
    userCompetitions[0] ??
    activeCompetitions[0] ??
    null;

  // Initialise the competition selection.
  useEffect(() => {
    if (activeCompetitions.length === 0) {
      if (selectedCompetitionId !== null) setSelectedCompetitionId(null);
      return;
    }
    const hasSelection = activeCompetitions.some((c) => c.id === selectedCompetitionId);
    if (hasSelection) return;
    const preferred =
      userCompetitions.find((c) => c.standings.length > 0)?.id ??
      activeCompetitions.find((c) => c.standings.length > 0)?.id ??
      userCompetitions[0]?.id ??
      activeCompetitions[0].id;
    setSelectedCompetitionId(preferred);
  }, [activeCompetitions, selectedCompetitionId, userCompetitions]);

  // Fetch schedule slice whenever the competition changes.
  useEffect(() => {
    if (!selectedCompetition) {
      setSlice(null);
      return;
    }
    let cancelled = false;
    fetchSchedule({ competition_id: selectedCompetition.id })
      .then((result) => {
        if (cancelled) return;
        setSlice(result);
        setFetchError(null);
      })
      .catch((err) => {
        if (cancelled) return;
        setFetchError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
      });
    return () => {
      cancelled = true;
    };
  }, [selectedCompetition?.id, t]);

  // Reset past-group paging when competition changes.
  useEffect(() => {
    setVisiblePastCount(PAST_PAGE_SIZE);
  }, [selectedCompetition?.id]);

  const scrollToDate = useCallback((date: string) => {
    const el = groupRefs.current.get(date);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, []);

  // Auto-scroll to the next user fixture on load when mid-season (past groups present).
  useEffect(() => {
    if (!slice?.next_user_match_date || slice.past_groups.length === 0) return;
    const date = slice.next_user_match_date;
    const id = setTimeout(() => scrollToDate(date), 50);
    return () => clearTimeout(id);
  }, [slice?.competition_id, slice?.next_user_match_date, slice?.past_groups.length, scrollToDate]);

  const buildTeamMenuItem = (label: string, teamId: string): ContextMenuItem => ({
    label,
    onClick: () => onSelectTeam(teamId),
  });

  if (activeCompetitions.length === 0) {
    return (
      <p className="py-8 text-center text-gray-500 dark:text-gray-400">
        {t("schedule.noLeague")}
      </p>
    );
  }

  const competitionSwitcher = activeCompetitions.length > 1 && (
    <Select
      value={selectedCompetition?.id ?? ""}
      onChange={(e) => setSelectedCompetitionId(e.target.value)}
      wrapperClassName="ml-auto"
      aria-label={t("common.competition")}
    >
      {activeCompetitions.map((c) => (
        <option key={c.id} value={c.id}>
          {competitionDisplayName(c, t)}
        </option>
      ))}
    </Select>
  );

  return (
    <div>
      {isPreseason && (
        <Card accent="accent" className="mb-5">
          <CardBody>
            <div className="flex flex-col gap-1.5">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="accent" size="sm">
                  {t(`season.phases.${seasonContext.phase}`)}
                </Badge>
                <span className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                  {seasonContext.season_start
                    ? t("season.startsOn", { date: formatMatchDate(seasonContext.season_start) })
                    : t("season.noOpener")}
                </span>
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                {t("season.standingsLocked")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      <div className="mb-5 flex flex-wrap gap-2">
        <ViewButton active={view === "calendar"} onClick={() => setView("calendar")}>
          <CalendarIcon className="mr-1.5 inline h-4 w-4 -mt-0.5" />
          {t("schedule.calendar.title", "Calendar")}
        </ViewButton>
        <ViewButton active={view === "fixtures"} onClick={() => setView("fixtures")}>
          <TableProperties className="mr-1.5 inline h-4 w-4 -mt-0.5" />
          {t("schedule.fixtures")}
        </ViewButton>
        <ViewButton active={view === "standings"} onClick={() => setView("standings")}>
          <Trophy className="mr-1.5 inline h-4 w-4 -mt-0.5" />
          {t("schedule.standings")}
        </ViewButton>
        {hasInternational && (
          <ViewButton
            active={view === "international"}
            onClick={() => setView("international")}
          >
            <Globe className="mr-1.5 inline h-4 w-4 -mt-0.5" />
            {t("schedule.international")}
          </ViewButton>
        )}
        {competitionSwitcher}
      </div>

      {fetchError && (
        <p role="alert" className="mb-4 text-sm text-red-500">
          {fetchError}
        </p>
      )}

      {view === "calendar" && (
        <CalendarView
          slice={slice}
          userTeamId={userTeamId ?? null}
          groupRefs={groupRefs}
          visiblePastCount={visiblePastCount}
          onShowMorePast={() => setVisiblePastCount((n) => n + PAST_PAGE_SIZE)}
          onScrollToDate={scrollToDate}
          onSelectTeam={onSelectTeam}
          buildTeamMenuItem={buildTeamMenuItem}
          t={t}
        />
      )}

      {view === "fixtures" && (
        <FixturesListView
          slice={slice}
          userTeamId={userTeamId ?? null}
          groupRefs={groupRefs}
          onSelectTeam={onSelectTeam}
          buildTeamMenuItem={buildTeamMenuItem}
          t={t}
        />
      )}

      {view === "international" && (
        <InternationalView
          gameState={gameState}
          calledUpPlayers={calledUpPlayers}
          nationalFixtures={nationalFixtures}
          t={t}
        />
      )}

      {view === "standings" && (
        <StandingsView
          competition={selectedCompetition}
          userTeamId={userTeamId ?? null}
          isPreseason={isPreseason}
          seasonContext={seasonContext}
          onSelectTeam={onSelectTeam}
          buildTeamMenuItem={buildTeamMenuItem}
          gameState={gameState}
          t={t}
        />
      )}
    </div>
  );
}

// ─── Sub-views ──────────────────────────────────────────────────────────────

function ViewButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`rounded-lg px-4 py-2 font-heading text-sm font-bold uppercase tracking-wider transition-all ${
        active
          ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
          : "border border-gray-200 bg-white text-gray-500 hover:text-gray-700 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-400 dark:hover:text-gray-200"
      }`}
    >
      {children}
    </button>
  );
}

function CalendarView({
  slice,
  userTeamId,
  groupRefs,
  visiblePastCount,
  onShowMorePast,
  onScrollToDate,
  onSelectTeam,
  buildTeamMenuItem,
  t,
}: {
  slice: ScheduleSlice | null;
  userTeamId: string | null;
  groupRefs: React.MutableRefObject<Map<string, HTMLDivElement>>;
  visiblePastCount: number;
  onShowMorePast: () => void;
  onScrollToDate: (date: string) => void;
  onSelectTeam: (id: string) => void;
  buildTeamMenuItem: (label: string, teamId: string) => ContextMenuItem;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  if (!slice) return <LoadingPlaceholder />;

  const allGroups = [...slice.upcoming_groups, ...slice.past_groups];
  const visiblePastGroups = slice.past_groups.slice(0, visiblePastCount);

  return (
    <div className="flex flex-col gap-5">
      <ScheduleCalendarGrid
        groups={allGroups}
        userTeamId={userTeamId}
        today={slice.today}
        focusDate={slice.next_user_match_date}
        onSelectDate={onScrollToDate}
      />

      {/* Upcoming groups */}
      {slice.upcoming_groups.length > 0 ? (
        <div className="flex flex-col gap-4">
          {slice.upcoming_groups.map((group) => (
            <MatchdayGroupCard
              key={group.key}
              group={group}
              userTeamId={userTeamId}
              groupRefs={groupRefs}
              competitionName={slice.competition_name}
              onSelectTeam={onSelectTeam}
              buildTeamMenuItem={buildTeamMenuItem}
              t={t}
            />
          ))}
        </div>
      ) : (
        <p className="py-4 text-center text-sm text-gray-500 dark:text-gray-400">
          {t("schedule.noUpcoming", "No upcoming fixtures.")}
        </p>
      )}

      {/* Past groups */}
      {visiblePastGroups.length > 0 && (
        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-3 py-1">
            <div className="flex-1 h-px bg-gray-200 dark:bg-navy-600" />
            <span className="text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
              {t("schedule.pastResults", "Past results")}
            </span>
            <div className="flex-1 h-px bg-gray-200 dark:bg-navy-600" />
          </div>
          <div className="flex flex-col gap-4">
            {visiblePastGroups.map((group) => (
              <MatchdayGroupCard
                key={group.key}
                group={group}
                userTeamId={userTeamId}
                groupRefs={groupRefs}
                competitionName={slice.competition_name}
                dimmed
                onSelectTeam={onSelectTeam}
                buildTeamMenuItem={buildTeamMenuItem}
                t={t}
              />
            ))}
          </div>
          {slice.past_groups.length > visiblePastCount && (
            <button
              onClick={onShowMorePast}
              className="mx-auto mt-1 rounded-lg border border-gray-200 bg-white px-4 py-2 font-heading text-sm font-bold uppercase tracking-wider text-gray-500 transition-all hover:text-gray-700 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-400 dark:hover:text-gray-200"
            >
              {t("schedule.loadMore")}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

function FixturesListView({
  slice,
  userTeamId,
  groupRefs,
  onSelectTeam,
  buildTeamMenuItem,
  t,
}: {
  slice: ScheduleSlice | null;
  userTeamId: string | null;
  groupRefs: React.MutableRefObject<Map<string, HTMLDivElement>>;
  onSelectTeam: (id: string) => void;
  buildTeamMenuItem: (label: string, teamId: string) => ContextMenuItem;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  if (!slice) return <LoadingPlaceholder />;

  const allGroups = [...slice.upcoming_groups, ...slice.past_groups];

  if (allGroups.length === 0) {
    return (
      <p className="py-8 text-center text-gray-500 dark:text-gray-400">
        {t("schedule.noLeague")}
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      {allGroups.map((group) => (
        <MatchdayGroupCard
          key={group.key}
          group={group}
          userTeamId={userTeamId}
          groupRefs={groupRefs}
          competitionName={slice.competition_name}
          onSelectTeam={onSelectTeam}
          buildTeamMenuItem={buildTeamMenuItem}
          t={t}
        />
      ))}
    </div>
  );
}

function groupLabel(
  group: MatchdayGroup,
  competitionName: string,
  t: ReturnType<typeof useTranslation>["t"],
): string {
  if (group.competition === "League" && group.matchday > 0) {
    return `${competitionName} – ${t("schedule.matchday", { number: group.matchday })} – ${formatMatchDate(group.date)}`;
  }
  if (group.competition === "PreseasonTournament") {
    return `${competitionName} – ${t("season.preseasonTournament", "Pre-season")} – ${formatMatchDate(group.date)}`;
  }
  return `${competitionName} – ${formatMatchDate(group.date)}`;
}

function MatchdayGroupCard({
  group,
  userTeamId,
  groupRefs,
  competitionName,
  dimmed = false,
  onSelectTeam,
  buildTeamMenuItem,
  t,
}: {
  group: MatchdayGroup;
  userTeamId: string | null;
  groupRefs: React.MutableRefObject<Map<string, HTMLDivElement>>;
  competitionName: string;
  dimmed?: boolean;
  onSelectTeam: (id: string) => void;
  buildTeamMenuItem: (label: string, teamId: string) => ContextMenuItem;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  const setRef = (el: HTMLDivElement | null) => {
    if (el) {
      groupRefs.current.set(group.date, el);
    } else {
      groupRefs.current.delete(group.date);
    }
  };

  return (
    <div ref={setRef} data-group-key={group.key}>
      <Card className={dimmed ? "opacity-70" : undefined}>
        <div className="rounded-t-xl border-b border-gray-100 bg-gray-50 px-5 py-3 dark:border-navy-600 dark:bg-navy-800">
          <div className="flex items-center gap-2">
            {group.is_next_user_match && (
              <Badge variant="primary" size="sm">
                {t("schedule.nextMatch", "Next match")}
              </Badge>
            )}
            <h4 className="font-heading text-sm font-bold uppercase tracking-wider text-gray-600 dark:text-gray-300">
              {groupLabel(group, competitionName, t)}
            </h4>
          </div>
        </div>
        <CardBody className="p-0">
          <div className="divide-y divide-gray-100 dark:divide-navy-600">
            {group.fixtures.map((fixture) => {
              const isUserMatch =
                fixture.home_team_id === userTeamId ||
                fixture.away_team_id === userTeamId;
              const completed = fixture.status === "Completed";
              const contextItems = [
                buildTeamMenuItem(
                  `${t("common.viewTeam")}: ${fixture.home_team_name}`,
                  fixture.home_team_id,
                ),
                buildTeamMenuItem(
                  `${t("common.viewTeam")}: ${fixture.away_team_name}`,
                  fixture.away_team_id,
                ),
              ];

              return (
                <ContextMenu items={contextItems} key={fixture.id}>
                  <div
                    className={`flex items-center px-5 py-3 transition-colors ${
                      isUserMatch ? "bg-primary-50/50 dark:bg-primary-500/5" : ""
                    }`}
                    data-testid={`schedule-fixture-${fixture.id}`}
                  >
                    <span
                      onClick={() => onSelectTeam(fixture.home_team_id)}
                      className={`flex-1 cursor-pointer text-right text-sm font-semibold hover:underline ${
                        fixture.home_team_id === userTeamId
                          ? "text-primary-600 dark:text-primary-400"
                          : "text-gray-800 dark:text-gray-200"
                      }`}
                    >
                      {fixture.home_team_name}
                    </span>
                    <div className="mx-3 w-24 text-center">
                      {completed && fixture.result ? (
                        <span className="font-heading text-lg font-bold text-gray-800 dark:text-gray-100">
                          {fixture.result.home_goals} - {fixture.result.away_goals}
                        </span>
                      ) : (
                        <Badge variant="neutral" size="sm">
                          vs
                        </Badge>
                      )}
                    </div>
                    <span
                      onClick={() => onSelectTeam(fixture.away_team_id)}
                      className={`flex-1 cursor-pointer text-left text-sm font-semibold hover:underline ${
                        fixture.away_team_id === userTeamId
                          ? "text-primary-600 dark:text-primary-400"
                          : "text-gray-800 dark:text-gray-200"
                      }`}
                    >
                      {fixture.away_team_name}
                    </span>
                  </div>
                </ContextMenu>
              );
            })}
          </div>
        </CardBody>
      </Card>
    </div>
  );
}

function InternationalView({
  gameState,
  calledUpPlayers,
  nationalFixtures,
  t,
}: {
  gameState: GameStateData;
  calledUpPlayers: ReturnType<typeof getUserCalledUpPlayers>;
  nationalFixtures: ReturnType<typeof getNationalTeamFixtures>;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  const internationalByDate = new Map<
    string,
    typeof nationalFixtures
  >();
  [...nationalFixtures]
    .sort((a, b) => a.date.localeCompare(b.date) || a.id.localeCompare(b.id))
    .forEach((fixture) => {
      const list = internationalByDate.get(fixture.date) ?? [];
      list.push(fixture);
      internationalByDate.set(fixture.date, list);
    });

  return (
    <div className="flex flex-col gap-4">
      {calledUpPlayers.length > 0 && (
        <Card accent="accent">
          <div className="rounded-t-xl border-b border-gray-100 bg-gray-50 px-5 py-3 dark:border-navy-600 dark:bg-navy-800">
            <h4 className="flex items-center gap-2 font-heading text-sm font-bold uppercase tracking-wider text-gray-600 dark:text-gray-300">
              <Globe className="h-4 w-4 text-accent-500" />
              {t("schedule.internationalDuty")}
            </h4>
          </div>
          <CardBody className="p-0">
            <div className="divide-y divide-gray-100 dark:divide-navy-600">
              {calledUpPlayers.map(({ player, nationalTeamId, nationalTeamName, nationalTeamNameKey }) => (
                <div
                  key={`${player.id}-${nationalTeamId}`}
                  className="flex items-center justify-between px-5 py-2.5"
                  data-testid={`schedule-callup-${player.id}`}
                >
                  <span className="text-sm font-semibold text-gray-800 dark:text-gray-200">
                    {player.match_name}
                  </span>
                  <Badge variant="neutral" size="sm">
                    {nationalTeamNameKey
                      ? t("nations.nationalTeamTemplate", { name: t(nationalTeamNameKey) })
                      : nationalTeamName}
                  </Badge>
                </div>
              ))}
            </div>
          </CardBody>
        </Card>
      )}

      {Array.from(internationalByDate.entries()).map(([date, fixtures]) => (
        <Card key={date}>
          <div className="rounded-t-xl border-b border-gray-100 bg-gray-50 px-5 py-3 dark:border-navy-600 dark:bg-navy-800">
            <h4 className="font-heading text-sm font-bold uppercase tracking-wider text-gray-600 dark:text-gray-300">
              {t("schedule.international")} - {formatMatchDate(date)}
            </h4>
          </div>
          <CardBody className="p-0">
            <div className="divide-y divide-gray-100 dark:divide-navy-600">
              {fixtures.map((fixture) => {
                const completed = fixture.status === "Completed";
                return (
                  <div
                    key={fixture.id}
                    className="flex items-center px-5 py-3"
                    data-testid={`schedule-international-${fixture.id}`}
                  >
                    <span className="flex-1 text-right text-sm font-semibold text-gray-800 dark:text-gray-200">
                      {getNationalTeamName(gameState, fixture.home_team_id, t)}
                    </span>
                    <div className="mx-3 w-24 text-center">
                      {completed && fixture.result ? (
                        <span className="font-heading text-lg font-bold text-gray-800 dark:text-gray-100">
                          {fixture.result.home_goals} - {fixture.result.away_goals}
                        </span>
                      ) : (
                        <Badge variant="neutral" size="sm">
                          vs
                        </Badge>
                      )}
                    </div>
                    <span className="flex-1 text-left text-sm font-semibold text-gray-800 dark:text-gray-200">
                      {getNationalTeamName(gameState, fixture.away_team_id, t)}
                    </span>
                  </div>
                );
              })}
            </div>
          </CardBody>
        </Card>
      ))}
    </div>
  );
}

function StandingsView({
  competition,
  userTeamId,
  isPreseason,
  seasonContext,
  onSelectTeam,
  buildTeamMenuItem,
  gameState,
  t,
}: {
  competition: LeagueData | null;
  userTeamId: string | null;
  isPreseason: boolean;
  seasonContext: ReturnType<typeof resolveSeasonContext>;
  onSelectTeam: (id: string) => void;
  buildTeamMenuItem: (label: string, teamId: string) => ContextMenuItem;
  gameState: GameStateData;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  const standings = sortStandings(competition);
  const zones = competition
    ? getPromotionRelegationZones(getActiveCompetitions(gameState), competition)
    : { promotionSlots: 0, relegationSlots: 0 };

  if (isPreseason) {
    return (
      <Card>
        <CardBody>
          <div className="flex flex-col items-center gap-2 py-6 text-center">
            <Trophy className="h-8 w-8 text-gray-300 dark:text-navy-600" />
            <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
              {t("season.standingsLocked")}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-400">
              {seasonContext.season_start
                ? t("season.startsOn", { date: formatMatchDate(seasonContext.season_start) })
                : t("season.noOpener")}
            </p>
          </div>
        </CardBody>
      </Card>
    );
  }

  return (
    <Card>
      <div className="rounded-t-xl border-b border-gray-100 bg-gradient-to-r from-navy-700 to-navy-800 p-5 dark:border-navy-600">
        <h3 className="flex items-center gap-2 font-heading text-lg font-bold uppercase tracking-wide text-white">
          <Trophy className="h-5 w-5 text-accent-400" />
          {(competition && competitionDisplayName(competition, t)) ||
            t("schedule.fixtures")} –{" "}
          {t("schedule.season", { number: competition?.season ?? 0 })}
        </h3>
      </div>
      {standings.length === 0 ? (
        <CardBody>
          <p className="py-6 text-center text-sm text-gray-500 dark:text-gray-400">
            {t("schedule.standingsUnavailable")}
          </p>
        </CardBody>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-left">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50 text-xs dark:border-navy-600 dark:bg-navy-800">
                {["#", t("common.team"), t("common.played"), t("common.won"), t("common.drawn"), t("common.lost"), t("common.gf"), t("common.ga"), t("common.gd"), t("common.pts")].map(
                  (header, idx) => (
                    <th
                      key={idx}
                      className={`px-4 py-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 ${idx === 0 ? "w-8" : ""} ${idx >= 2 ? "text-center" : ""}`}
                    >
                      {header}
                    </th>
                  ),
                )}
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
              {standings.map((entry, index) => {
                const isUser = entry.team_id === userTeamId;
                const gd = entry.goals_for - entry.goals_against;
                const inPromotionZone = index < zones.promotionSlots;
                const inRelegationZone =
                  zones.relegationSlots > 0 &&
                  index >= standings.length - zones.relegationSlots;
                const contextItems = [buildTeamMenuItem(t("common.viewTeam"), entry.team_id)];

                return (
                  <ContextMenu items={contextItems} key={entry.team_id}>
                    <tr
                      className={`transition-colors ${
                        isUser
                          ? "bg-primary-50 dark:bg-primary-500/10"
                          : "hover:bg-gray-50 dark:hover:bg-navy-700/50"
                      }`}
                      data-testid={`schedule-standings-row-${entry.team_id}`}
                    >
                      <td
                        className={`px-4 py-3 font-heading text-sm font-bold ${
                          inPromotionZone
                            ? "border-l-2 border-primary-500 text-primary-500"
                            : inRelegationZone
                              ? "border-l-2 border-red-500 text-red-500"
                              : "text-gray-400 dark:text-gray-500"
                        }`}
                        data-testid={
                          inPromotionZone
                            ? `standings-promotion-${entry.team_id}`
                            : inRelegationZone
                              ? `standings-relegation-${entry.team_id}`
                              : undefined
                        }
                      >
                        {index + 1}
                      </td>
                      <td
                        onClick={() => onSelectTeam(entry.team_id)}
                        className={`cursor-pointer px-4 py-3 text-sm font-semibold hover:underline ${
                          isUser
                            ? "text-primary-600 dark:text-primary-400"
                            : "text-gray-800 dark:text-gray-200"
                        }`}
                      >
                        {getTeamName(gameState.teams, entry.team_id)}
                      </td>
                      {[entry.played, entry.won, entry.drawn, entry.lost, entry.goals_for, entry.goals_against].map(
                        (val, i) => (
                          <td key={i} className="px-4 py-3 text-center text-sm tabular-nums text-gray-600 dark:text-gray-400">
                            {val}
                          </td>
                        ),
                      )}
                      <td
                        className={`px-4 py-3 text-center text-sm font-semibold tabular-nums ${
                          gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500 dark:text-gray-400"
                        }`}
                      >
                        {gd > 0 ? `+${gd}` : gd}
                      </td>
                      <td className="px-4 py-3 text-center font-heading text-sm font-bold tabular-nums text-gray-800 dark:text-gray-100">
                        {entry.points}
                      </td>
                    </tr>
                  </ContextMenu>
                );
              })}
            </tbody>
          </table>
          {(zones.promotionSlots > 0 || zones.relegationSlots > 0) && (
            <div className="flex gap-5 border-t border-gray-100 px-4 py-2.5 text-xs text-gray-500 dark:border-navy-600 dark:text-gray-400">
              {zones.promotionSlots > 0 && (
                <span className="flex items-center gap-1.5">
                  <span className="h-2 w-2 rounded-full bg-primary-500" />
                  {t("schedule.promotionZone")}
                </span>
              )}
              {zones.relegationSlots > 0 && (
                <span className="flex items-center gap-1.5">
                  <span className="h-2 w-2 rounded-full bg-red-500" />
                  {t("schedule.relegationZone")}
                </span>
              )}
            </div>
          )}
        </div>
      )}
    </Card>
  );
}

function LoadingPlaceholder() {
  return (
    <div className="flex flex-col gap-4">
      {[1, 2, 3].map((n) => (
        <div
          key={n}
          className="h-32 rounded-xl bg-gray-100 dark:bg-navy-800 animate-pulse"
        />
      ))}
    </div>
  );
}
