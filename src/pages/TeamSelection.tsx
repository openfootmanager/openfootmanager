import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useEffect, useMemo, useState } from "react";
import {
  GameStateData,
  LeagueData,
  PlayerData,
  TeamData,
  useGameStore,
} from "../store/gameStore";
import { getActiveCompetitions, getPlayerOvr } from "../lib/helpers";
import { buildRegionLabel, inferRegionId } from "../lib/teamRegions";
import { Badge, Card, CardBody, ThemeToggle } from "../components/ui";
import { ArrowLeft, ChevronRight, Loader2 } from "lucide-react";
import { resolveBackendError } from "../utils/backendI18n";
import { prewarmManagerSquadPortraits } from "../services/portraitService";
import {
  buildFallbackRegions,
  competitionRequiredRegions,
  likelyXi,
  sortCompetitions,
  teamCompetitions,
} from "./TeamSelection.helpers";
import TeamSelectionScopePanel from "./TeamSelectionScopePanel";
import TeamSelectionGrid from "./TeamSelectionGrid";
import TeamSelectionSidebar from "./TeamSelectionSidebar";

type CompetitionSelection = Record<string, boolean>;
type RegionSelection = Record<string, boolean>;
type ScopeMessage = {
  key: string;
  values?: Record<string, string | number>;
};

export default function TeamSelection() {
  const { t } = useTranslation();
  const compName = (c: LeagueData) =>
    c.name_key ? t(c.name_key, { year: c.season }) : c.name;
  const navigate = useNavigate();
  const { gameState, setGameState, setGameActive } = useGameStore();
  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [clubSearch, setClubSearch] = useState("");
  const [scopeExpanded, setScopeExpanded] = useState(true);
  const [selectedHomeRegionId, setSelectedHomeRegionId] = useState<string | null>(null);
  const [selectedCountryCode, setSelectedCountryCode] = useState<string | null>(null);
  const [regionSelection, setRegionSelection] = useState<RegionSelection>({});
  const [competitionSelection, setCompetitionSelection] = useState<CompetitionSelection>({});
  const [scopeMessage, setScopeMessage] = useState<ScopeMessage | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);

  const competitions = useMemo(
    () => (gameState ? sortCompetitions(getActiveCompetitions(gameState)) : []),
    [gameState],
  );

  const regions = useMemo(
    () =>
      gameState
        ? gameState.regions && gameState.regions.length > 0
          ? gameState.regions
          : buildFallbackRegions(t, gameState, competitions)
        : [],
    [competitions, gameState, t],
  );

  useEffect(() => {
    if (regions.length === 0) {
      if (selectedHomeRegionId !== null) {
        setSelectedHomeRegionId(null);
      }
      return;
    }

    const hasCurrentSelection = regions.some((region) => region.id === selectedHomeRegionId);
    if (!hasCurrentSelection) {
      setSelectedHomeRegionId(regions[0].id);
    }
  }, [regions, selectedHomeRegionId]);

  const regionCountries = useMemo(() => {
    const region = regions.find((candidate) => candidate.id === selectedHomeRegionId);
    return region?.country_codes ?? [];
  }, [regions, selectedHomeRegionId]);

  useEffect(() => {
    if (regionCountries.length === 0) {
      if (selectedCountryCode !== null) {
        setSelectedCountryCode(null);
      }
      return;
    }

    if (!selectedCountryCode || !regionCountries.includes(selectedCountryCode)) {
      setSelectedCountryCode(regionCountries[0]);
    }
  }, [regionCountries, selectedCountryCode]);

  useEffect(() => {
    if (regions.length === 0) {
      return;
    }

    setRegionSelection((current) => {
      const next = Object.fromEntries(
        regions.map((region) => [region.id, current[region.id] ?? false]),
      );
      if (selectedHomeRegionId) {
        next[selectedHomeRegionId] = true;
      }
      return next;
    });
  }, [regions, selectedHomeRegionId]);

  useEffect(() => {
    if (competitions.length === 0) {
      return;
    }

    setCompetitionSelection((current) =>
      Object.fromEntries(
        competitions.map((competition) => [
          competition.id,
          current[competition.id] ?? true,
        ]),
      ),
    );
  }, [competitions]);

  if (!gameState) {
    navigate("/");
    return null;
  }

  const activeRegionIds = regions
    .filter(
      (region) =>
        region.id === selectedHomeRegionId || Boolean(regionSelection[region.id]),
    )
    .map((region) => region.id);

  const homeRegionTeamIds = new Set(
    gameState.teams
      .filter((team) => regionCountries.includes(team.country))
      .map((team) => team.id),
  );

  const availableCompetitions = competitions.filter((competition) => {
    if (!selectedHomeRegionId) {
      return true;
    }

    const requiredRegions = competitionRequiredRegions(competition);
    return (
      requiredRegions.includes(selectedHomeRegionId) ||
      competition.region_id === selectedHomeRegionId ||
      (competition.country_id ? regionCountries.includes(competition.country_id) : false) ||
      competition.participant_ids?.some((teamId) => homeRegionTeamIds.has(teamId)) ||
      competition.scope === "Continental" ||
      competition.scope === "International"
    );
  });

  const teams = gameState.teams.filter((team) => {
    if (selectedCountryCode) {
      return team.country === selectedCountryCode;
    }
    if (selectedHomeRegionId) {
      return inferRegionId(team.country) === selectedHomeRegionId;
    }
    return true;
  });

  // Free-text search over the country/region-filtered clubs (name or city).
  const clubSearchQuery = clubSearch.trim().toLowerCase();
  const filteredTeams = clubSearchQuery
    ? teams.filter(
        (team) =>
          team.name.toLowerCase().includes(clubSearchQuery) ||
          team.city.toLowerCase().includes(clubSearchQuery),
      )
    : teams;

  // Group the visible clubs by their domestic league/division (strongest first),
  // with any club not in a league falling into an "other" bucket.
  const teamGroups = useMemo(() => {
    const leagueByTeam = new Map<string, LeagueData>();
    for (const competition of competitions) {
      if (competition.kind !== "League" || competition.scope !== "Domestic") {
        continue;
      }
      for (const teamId of competition.participant_ids ?? []) {
        if (!leagueByTeam.has(teamId)) {
          leagueByTeam.set(teamId, competition);
        }
      }
    }

    const groups = new Map<
      string,
      { id: string; name: string; order: number; teams: TeamData[] }
    >();
    const ungrouped: TeamData[] = [];
    for (const team of filteredTeams) {
      const league = leagueByTeam.get(team.id);
      if (!league) {
        ungrouped.push(team);
        continue;
      }
      const group = groups.get(league.id) ?? {
        id: league.id,
        name: league.name,
        order: league.priority ?? 0,
        teams: [],
      };
      group.teams.push(team);
      groups.set(league.id, group);
    }

    const ordered = Array.from(groups.values()).sort(
      (left, right) =>
        left.order - right.order || left.name.localeCompare(right.name),
    );
    for (const group of ordered) {
      group.teams.sort((left, right) => right.reputation - left.reputation);
    }
    if (ungrouped.length > 0) {
      ungrouped.sort((left, right) => right.reputation - left.reputation);
      ordered.push({
        id: "__ungrouped",
        name: t("teamSelect.otherClubs"),
        order: Number.MAX_SAFE_INTEGER,
        teams: ungrouped,
      });
    }
    return ordered;
  }, [competitions, filteredTeams, t]);

  useEffect(() => {
    if (teams.length === 0) {
      if (selectedTeamId !== null) {
        setSelectedTeamId(null);
      }
      return;
    }

    if (!selectedTeamId || !teams.some((team) => team.id === selectedTeamId)) {
      setSelectedTeamId(teams[0].id);
    }
  }, [selectedTeamId, teams]);

  const getTeamPlayers = (teamId: string): PlayerData[] =>
    gameState.players.filter((player) => player.team_id === teamId);

  const getTeamAvgOvr = (teamId: string): number => {
    const players = getTeamPlayers(teamId);
    if (players.length === 0) return 0;
    return Math.round(
      players.reduce((sum, player) => sum + getPlayerOvr(player), 0) / players.length,
    );
  };

  const selectedTeam = teams.find((team) => team.id === selectedTeamId) ?? teams[0] ?? null;
  const selectedTeamPlayers = selectedTeam ? getTeamPlayers(selectedTeam.id) : [];
  const selectedTeamXi = likelyXi(selectedTeamPlayers);
  const selectedTeamCompetitions = selectedTeam
    ? teamCompetitions(selectedTeam.id, competitions)
    : [];
  const mandatoryCompetitionIds = new Set(
    selectedTeamCompetitions.map((competition) => competition.id),
  );
  const enabledCompetitionIds = Array.from(
    new Set(
      Object.entries(competitionSelection)
        .filter(([, enabled]) => enabled)
        .map(([competitionId]) => competitionId)
        .concat(Array.from(mandatoryCompetitionIds)),
    ),
  );

  const handleRegionToggle = (regionId: string) => {
    if (regionId === selectedHomeRegionId) {
      setScopeMessage({ key: "teamSelect.scopeMessages.homeRegionAlwaysActive" });
      return;
    }

    const nextEnabled = !regionSelection[regionId];
    if (nextEnabled) {
      setRegionSelection((current) => ({
        ...current,
        [regionId]: true,
      }));
      setScopeMessage(null);
      return;
    }

    const blockedMandatoryCompetition = selectedTeamCompetitions.find((competition) =>
      competitionRequiredRegions(competition).includes(regionId),
    );
    if (blockedMandatoryCompetition) {
      setScopeMessage(
        {
          key: "teamSelect.scopeMessages.regionRequiredByCompetition",
          values: {
            competition: compName(blockedMandatoryCompetition),
            club: selectedTeam?.short_name ?? t("teamSelect.yourClub"),
            region: buildRegionLabel(t, regionId),
          },
        },
      );
      return;
    }

    const nextActiveRegions = new Set(
      activeRegionIds.filter((activeRegionId) => activeRegionId !== regionId),
    );
    const blockedCompetitionIds = competitions
      .filter((competition) => {
        if (!competitionSelection[competition.id]) {
          return false;
        }
        return competitionRequiredRegions(competition).some(
          (requiredRegionId) => !nextActiveRegions.has(requiredRegionId),
        );
      })
      .map((competition) => competition.id);

    setRegionSelection((current) => ({
      ...current,
      [regionId]: false,
    }));
    if (blockedCompetitionIds.length > 0) {
      setCompetitionSelection((current) => {
        const next = { ...current };
        for (const competitionId of blockedCompetitionIds) {
          next[competitionId] = false;
        }
        return next;
      });
      setScopeMessage(
        {
          key: "teamSelect.scopeMessages.regionRemovedDisablesCompetitions",
          values: {
            region: buildRegionLabel(t, regionId),
          },
        },
      );
    } else {
      setScopeMessage(null);
    }
  };

  const handleCompetitionToggle = (competition: LeagueData) => {
    const currentlyEnabled = Boolean(competitionSelection[competition.id]);
    const isLocked = mandatoryCompetitionIds.has(competition.id);

    if (currentlyEnabled) {
      if (isLocked) {
        setScopeMessage({
          key: "teamSelect.scopeMessages.clubCompetitionLocked",
          values: {
            competition: compName(competition),
            club: selectedTeam?.short_name ?? t("teamSelect.yourClub"),
          },
        });
        return;
      }

      setCompetitionSelection((current) => ({
        ...current,
        [competition.id]: false,
      }));
      setScopeMessage(null);
      return;
    }

    const requiredRegions = competitionRequiredRegions(competition);
    const missingRegions = requiredRegions.filter(
      (requiredRegionId) => !activeRegionIds.includes(requiredRegionId),
    );

    if (missingRegions.length > 0) {
      setRegionSelection((current) => {
        const next = { ...current };
        for (const regionId of missingRegions) {
          next[regionId] = true;
        }
        if (selectedHomeRegionId) {
          next[selectedHomeRegionId] = true;
        }
        return next;
      });
      setScopeMessage(
        {
          key: "teamSelect.scopeMessages.autoEnabledRegions",
          values: {
            competition: compName(competition),
            regions: missingRegions
              .map((regionId) => buildRegionLabel(t, regionId))
              .join(", "),
          },
        },
      );
    } else {
      setScopeMessage(null);
    }

    setCompetitionSelection((current) => ({
      ...current,
      [competition.id]: true,
    }));
  };

  const handleConfirm = async () => {
    if (!selectedTeam || isConfirming) return;
    setIsConfirming(true);
    try {
      const updatedGame = await invoke<GameStateData>("select_team", {
        teamId: selectedTeam.id,
        activeRegionIds,
        activeCompetitionIds: enabledCompetitionIds,
      });
      try {
        await prewarmManagerSquadPortraits(updatedGame);
      } catch (portraitError) {
        console.warn(
          "Portrait prewarm failed after team selection:",
          portraitError,
        );
      }
      setGameState(updatedGame);
      const mgr = updatedGame.manager;
      setGameActive(true, `${mgr.first_name} ${mgr.last_name}`);
      navigate("/dashboard");
    } catch (error) {
      console.error("Failed to select team:", error);
      alert(
        t("teamSelect.failedToSelectTeam", {
          error: resolveBackendError(error),
        }),
      );
    } finally {
      setIsConfirming(false);
    }
  };

  return (
    <div className="min-h-screen bg-gray-100 transition-colors duration-300 dark:bg-navy-900">
      <header className="flex items-center justify-between border-b border-gray-200 bg-white px-6 py-4 shadow-sm dark:border-navy-700 dark:bg-navy-800">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate("/")}
            className="rounded-lg p-2 text-gray-400 transition-colors hover:bg-gray-100 hover:text-gray-700 dark:hover:bg-navy-700 dark:hover:text-gray-200"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
          <div>
            <h1 className="font-heading text-xl font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
              {t("teamSelect.title")}
            </h1>
            <p className="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
              {t("teamSelect.subtitle")}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <ThemeToggle />
          {selectedTeam && (
            <button
              onClick={handleConfirm}
              disabled={isConfirming}
              className={`flex items-center gap-2 rounded-lg bg-gradient-to-r from-primary-500 to-primary-600 px-6 py-2.5 font-heading text-sm font-bold uppercase tracking-wider text-white shadow-md transition-all hover:from-primary-600 hover:to-primary-700 hover:shadow-lg hover:shadow-primary-500/20 ${
                isConfirming ? "cursor-wait opacity-70" : ""
              }`}
            >
              <span>
                {isConfirming
                  ? t("teamSelect.confirming")
                  : t("teamSelect.manage", { name: selectedTeam.short_name })}
              </span>
              {isConfirming ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <ChevronRight className="h-4 w-4" />
              )}
            </button>
          )}
        </div>
      </header>

      <div className="space-y-5 p-6">
        {scopeMessage && (
          <Card accent="accent">
            <CardBody className="py-3">
              <p className="text-sm text-gray-700 dark:text-gray-200">
                {t(scopeMessage.key, scopeMessage.values)}
              </p>
            </CardBody>
          </Card>
        )}

        <TeamSelectionScopePanel
          scopeExpanded={scopeExpanded}
          onToggleScopeExpanded={() => setScopeExpanded((value) => !value)}
          regions={regions}
          selectedHomeRegionId={selectedHomeRegionId}
          onSelectHomeRegion={(regionId) => {
            setSelectedHomeRegionId(regionId);
            setScopeMessage(null);
          }}
          selectedCountryCode={selectedCountryCode}
          onSelectCountry={setSelectedCountryCode}
          regionCountries={regionCountries}
          regionSelection={regionSelection}
          onRegionToggle={handleRegionToggle}
          availableCompetitions={availableCompetitions}
          competitionSelection={competitionSelection}
          mandatoryCompetitionIds={mandatoryCompetitionIds}
          activeRegionIds={activeRegionIds}
          onCompetitionToggle={handleCompetitionToggle}
        />

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.2fr)_minmax(340px,0.8fr)]">
          <TeamSelectionGrid
            clubSearch={clubSearch}
            onClubSearchChange={setClubSearch}
            filteredTeamsCount={filteredTeams.length}
            teamGroups={teamGroups}
            selectedTeamId={selectedTeam?.id ?? null}
            onSelectTeam={setSelectedTeamId}
            getTeamAvgOvr={getTeamAvgOvr}
            getTeamPlayerCount={(teamId) => getTeamPlayers(teamId).length}
          />

          <TeamSelectionSidebar
            selectedTeam={selectedTeam}
            selectedTeamXi={selectedTeamXi}
            selectedTeamCompetitions={selectedTeamCompetitions}
            getTeamAvgOvr={getTeamAvgOvr}
          />
        </div>

        <Card>
          <CardBody className="flex flex-wrap items-center justify-between gap-3 py-3">
            <div className="text-sm text-gray-600 dark:text-gray-300">
              {t("teamSelect.scopeSummary", {
                regionsCount: activeRegionIds.length,
                competitionsCount: enabledCompetitionIds.length,
              })}
            </div>
            <div className="flex flex-wrap gap-2">
              {activeRegionIds.map((regionId) => (
                <Badge key={regionId} variant="neutral">
                  {buildRegionLabel(t, regionId)}
                </Badge>
              ))}
            </div>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}

