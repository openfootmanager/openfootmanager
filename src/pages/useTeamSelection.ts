import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import {
  GameStateData,
  LeagueData,
  PlayerData,
  TeamData,
} from "../store/gameStore";
import { getActiveCompetitions, getPlayerOvr } from "../lib/helpers";
import { buildRegionLabel, inferRegionId } from "../lib/teamRegions";
import { resolveBackendError } from "../utils/backendI18n";
import { prewarmManagerSquadPortraits } from "../services/portraitService";
import {
  buildFallbackRegions,
  competitionRequiredRegions,
  likelyXi,
  sortCompetitions,
  teamCompetitions,
} from "./TeamSelection.helpers";

type CompetitionSelection = Record<string, boolean>;
type RegionSelection = Record<string, boolean>;
export type ScopeMessage = {
  key: string;
  values?: Record<string, string | number>;
};

interface UseTeamSelectionArgs {
  gameState: GameStateData | null;
  setGameState: (game: GameStateData) => void;
  setGameActive: (active: boolean, managerName: string) => void;
  navigate: (path: string) => void;
}

export function useTeamSelection({
  gameState,
  setGameState,
  setGameActive,
  navigate,
}: UseTeamSelectionArgs) {
  const { t } = useTranslation();
  const compName = (c: LeagueData) =>
    c.name_key ? t(c.name_key, { year: c.season }) : c.name;

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

  const activeRegionIds = regions
    .filter(
      (region) =>
        region.id === selectedHomeRegionId || Boolean(regionSelection[region.id]),
    )
    .map((region) => region.id);

  const homeRegionTeamIds = new Set(
    (gameState?.teams ?? [])
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

  const teams = (gameState?.teams ?? []).filter((team) => {
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
    (gameState?.players ?? []).filter((player) => player.team_id === teamId);

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

  return {
    clubSearch,
    setClubSearch,
    scopeExpanded,
    setScopeExpanded,
    selectedHomeRegionId,
    setSelectedHomeRegionId,
    selectedCountryCode,
    setSelectedCountryCode,
    regionSelection,
    setSelectedTeamId,
    scopeMessage,
    setScopeMessage,
    isConfirming,
    competitions,
    regions,
    regionCountries,
    activeRegionIds,
    availableCompetitions,
    filteredTeams,
    teamGroups,
    getTeamPlayers,
    getTeamAvgOvr,
    selectedTeam,
    selectedTeamXi,
    selectedTeamCompetitions,
    mandatoryCompetitionIds,
    competitionSelection,
    enabledCompetitionIds,
    handleRegionToggle,
    handleCompetitionToggle,
    handleConfirm,
  };
}
