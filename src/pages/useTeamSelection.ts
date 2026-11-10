import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import type { GameStateData, LeagueData, TeamData } from "../store/gameStore";
import { getActiveCompetitions } from "../lib/helpers";
import { useTeamSelectionRatings } from "./useTeamSelectionRatings";
import { buildRegionLabel } from "../lib/teamRegions";
import { competitionDisplayName } from "../lib/competitionName";
import { resolveBackendError } from "../utils/backendI18n";
import { prewarmManagerSquadPortraits } from "../services/portraitService";
import { showError } from "../lib/errorDialog";
import {
  regionsFromCompetitions,
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
  const compName = (c: LeagueData) => competitionDisplayName(c, t);

  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [clubSearch, setClubSearch] = useState("");
  const [scopeExpanded, setScopeExpanded] = useState(false);
  const [selectedHomeRegionId, setSelectedHomeRegionId] = useState<string | null>(null);
  const [selectedCountryCode, setSelectedCountryCode] = useState<string | null>(null);
  const [selectedLeagueId, setSelectedLeagueId] = useState<string | null>(null);
  const [regionSelection, setRegionSelection] = useState<RegionSelection>({});
  // The regions the player switched on in the simulation settings, as against the home region the
  // country they browsed made current: only the second is dropped when another country is picked.
  const [playerRegionIds, setPlayerRegionIds] = useState<ReadonlySet<string>>(new Set());
  const [competitionSelection, setCompetitionSelection] = useState<CompetitionSelection>({});
  const [scopeMessage, setScopeMessage] = useState<ScopeMessage | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);
  const { clubRatings, getTeamAvgOvr, getPlayerRating } = useTeamSelectionRatings(gameState);
  const playersById = useMemo(
    () => new Map((gameState?.players ?? []).map((player) => [player.id, player])),
    [gameState],
  );

  const competitions = useMemo(
    () => (gameState ? sortCompetitions(getActiveCompetitions(gameState)) : []),
    [gameState],
  );

  // The game carries no region list: the regions are the ones the competitions declare for the
  // clubs' countries.
  const regions = useMemo(
    () => (gameState ? regionsFromCompetitions(t, gameState, competitions) : []),
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

  // Every country a club can be chosen in, region by region: the rail lists them all, so a club in
  // another region is one pick away, not behind the simulation settings.
  const allCountryCodes = useMemo(
    () => Array.from(new Set(regions.flatMap((region) => region.country_codes))),
    [regions],
  );

  // The home region moves. The one it leaves is simulated no longer, unless the player switched it
  // on themselves: browsing from Sweden to Brazil must not quietly simulate both regions and slow
  // every day down.
  const changeHomeRegion = (regionId: string | null) => {
    const previous = selectedHomeRegionId;
    if (previous && previous !== regionId && !playerRegionIds.has(previous)) {
      setRegionSelection((current) => ({ ...current, [previous]: false }));
    }
    setSelectedHomeRegionId(regionId);
  };

  // Choosing a country in another region makes that region the home one: the career starts there.
  const handleSelectCountry = (countryCode: string) => {
    const region = regions.find((candidate) => candidate.country_codes.includes(countryCode));
    if (region) changeHomeRegion(region.id);
    setSelectedCountryCode(countryCode);
  };

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
        competitions.map((competition) => [competition.id, current[competition.id] ?? true]),
      ),
    );
  }, [competitions]);

  const activeRegionIds = regions
    .filter((region) => region.id === selectedHomeRegionId || Boolean(regionSelection[region.id]))
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
      return regionCountries.includes(team.country);
    }
    return true;
  });

  // Group the country's clubs by their domestic league/division (strongest first), with any club
  // not in a league falling into an "other" bucket: the rail lists these, and the table shows one.
  const leagueGroups = useMemo(() => {
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
    for (const team of teams) {
      const league = leagueByTeam.get(team.id);
      if (!league) {
        ungrouped.push(team);
        continue;
      }
      const group = groups.get(league.id) ?? {
        id: league.id,
        name: competitionDisplayName(league, t),
        order: league.priority ?? 0,
        teams: [],
      };
      group.teams.push(team);
      groups.set(league.id, group);
    }

    const ordered = Array.from(groups.values()).sort(
      (left, right) => left.order - right.order || left.name.localeCompare(right.name),
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
  }, [competitions, teams, t]);

  // The league the table shows: the one picked in the rail, else the strongest of the country.
  const activeLeagueGroup =
    leagueGroups.find((group) => group.id === selectedLeagueId) ?? leagueGroups[0] ?? null;
  const activeLeagueId = activeLeagueGroup?.id ?? null;

  // The clubs the table lists: the league's, narrowed by the search (name or city).
  const clubSearchQuery = clubSearch.trim().toLowerCase();
  const leagueClubs = activeLeagueGroup?.teams ?? [];
  const visibleClubs = clubSearchQuery
    ? leagueClubs.filter(
        (team) =>
          team.name.toLowerCase().includes(clubSearchQuery) ||
          team.city.toLowerCase().includes(clubSearchQuery),
      )
    : leagueClubs;

  // Nothing is chosen on arrival: the player picks a club. A choice that leaves the table (another
  // league or country) is dropped rather than carried to a club no longer in view.
  useEffect(() => {
    if (selectedTeamId !== null && !leagueClubs.some((team) => team.id === selectedTeamId)) {
      setSelectedTeamId(null);
    }
  }, [selectedTeamId, leagueClubs]);

  const selectedTeam = leagueClubs.find((team) => team.id === selectedTeamId) ?? null;
  const selectedTeamXi = selectedTeam ? likelyXi(clubRatings(selectedTeam.id), playersById) : [];
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
      setPlayerRegionIds((current) => new Set(current).add(regionId));
      setScopeMessage(null);
      return;
    }

    const blockedMandatoryCompetition = selectedTeamCompetitions.find((competition) =>
      competitionRequiredRegions(competition).includes(regionId),
    );
    if (blockedMandatoryCompetition) {
      setScopeMessage({
        key: "teamSelect.scopeMessages.regionRequiredByCompetition",
        values: {
          competition: compName(blockedMandatoryCompetition),
          club: selectedTeam?.short_name ?? t("teamSelect.yourClub"),
          region: buildRegionLabel(t, regionId),
        },
      });
      return;
    }

    setPlayerRegionIds((current) => {
      const next = new Set(current);
      next.delete(regionId);
      return next;
    });

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
      setScopeMessage({
        key: "teamSelect.scopeMessages.regionRemovedDisablesCompetitions",
        values: {
          region: buildRegionLabel(t, regionId),
        },
      });
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
      setPlayerRegionIds((current) => new Set([...current, ...missingRegions]));
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
      setScopeMessage({
        key: "teamSelect.scopeMessages.autoEnabledRegions",
        values: {
          competition: compName(competition),
          regions: missingRegions.map((regionId) => buildRegionLabel(t, regionId)).join(", "),
        },
      });
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
        console.warn("Portrait prewarm failed after team selection:", portraitError);
      }
      setGameState(updatedGame);
      const mgr = updatedGame.manager;
      setGameActive(true, mgr.display_name);
      navigate("/dashboard");
    } catch (error) {
      console.error("Failed to select team:", error);
      await showError(
        t("errors.title"),
        t("teamSelect.failedToSelectTeam", { error: resolveBackendError(error) }),
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
    allCountryCodes,
    handleSelectCountry,
    changeHomeRegion,
    activeLeagueId,
    setSelectedLeagueId,
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
    leagueGroups,
    visibleClubs,
    getTeamAvgOvr,
    getPlayerRating,
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
