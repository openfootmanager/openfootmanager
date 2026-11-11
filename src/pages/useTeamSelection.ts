import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import type { GameStateData, LeagueData, TeamData } from "../store/gameStore";
import { getActiveCompetitions } from "../lib/helpers";
import { useTeamSelectionRatings } from "./useTeamSelectionRatings";
import { competitionDisplayName } from "../lib/competitionName";
import { resolveBackendError } from "../utils/backendI18n";
import { prewarmManagerSquadPortraits } from "../services/portraitService";
import { showError } from "../lib/errorDialog";
import {
  activeRegionIds as scopeActiveRegionIds,
  competitionRequiredRegions,
  enabledCompetitionIds as scopeEnabledCompetitionIds,
  lockedRegions,
  type ScopeDraft,
} from "../components/teamSelection/simulationScope";
import {
  regionsFromCompetitions,
  likelyXi,
  sortCompetitions,
  teamCompetitions,
} from "./TeamSelection.helpers";

type CompetitionSelection = Record<string, boolean>;

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

  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [clubSearch, setClubSearch] = useState("");
  const [selectedHomeRegionId, setSelectedHomeRegionId] = useState<string | null>(null);
  const [selectedCountryCode, setSelectedCountryCode] = useState<string | null>(null);
  const [selectedLeagueId, setSelectedLeagueId] = useState<string | null>(null);
  // The regions the player chose in the simulation dialog, once they have applied it. Until then it
  // is null: nobody has touched the settings, and the career simulates what it always has, every
  // competition and the regions they need (the game widened the old flow's home-only region list
  // the same way). Applying the dialog makes the choice explicit, region by region.
  const [playerRegionIds, setPlayerRegionIds] = useState<ReadonlySet<string> | null>(null);
  const [competitionSelection, setCompetitionSelection] = useState<CompetitionSelection>({});
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

  // Choosing a country in another region makes that region the home one: the career starts there.
  const handleSelectCountry = (countryCode: string) => {
    const region = regions.find((candidate) => candidate.country_codes.includes(countryCode));
    if (region) setSelectedHomeRegionId(region.id);
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
    if (competitions.length === 0) {
      return;
    }

    setCompetitionSelection((current) =>
      Object.fromEntries(
        competitions.map((competition) => [competition.id, current[competition.id] ?? true]),
      ),
    );
  }, [competitions]);

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
  // What the simulation runs. The regions are the home one, the ones the chosen club's competitions
  // need, and the ones the player switched on; a competition is simulated when its regions are all
  // on and it is the club's or the player's choice. A blocked competition (a region off) is not sent
  // on to the game.
  // The regions an untouched career simulates: those every competition switched on needs.
  const defaultRegionIds = new Set(
    competitions
      .filter((competition) => competitionSelection[competition.id] !== false)
      .flatMap(competitionRequiredRegions),
  );
  const scopeDraft: ScopeDraft = {
    homeRegionId: selectedHomeRegionId,
    homeCountryCode: selectedCountryCode,
    regions: Object.fromEntries(
      Array.from(playerRegionIds ?? defaultRegionIds, (id) => [id, true]),
    ),
    competitions: competitionSelection,
  };
  const activeRegionIds = scopeActiveRegionIds(
    scopeDraft,
    lockedRegions(selectedHomeRegionId, selectedTeamCompetitions),
  );
  const enabledCompetitionIds = scopeEnabledCompetitionIds(
    competitions,
    scopeDraft,
    mandatoryCompetitionIds,
    new Set(activeRegionIds),
  );

  /** Apply the simulation dialog's draft: where the career starts, the regions and the competitions. */
  const applyScope = (draft: ScopeDraft) => {
    setSelectedHomeRegionId(draft.homeRegionId);
    setSelectedCountryCode(draft.homeCountryCode);
    setPlayerRegionIds(
      new Set(
        Object.entries(draft.regions)
          .filter(([, on]) => on)
          .map(([regionId]) => regionId),
      ),
    );
    setCompetitionSelection(draft.competitions);
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
    selectedHomeRegionId,
    selectedCountryCode,
    allCountryCodes,
    handleSelectCountry,
    activeLeagueId,
    setSelectedLeagueId,
    setSelectedTeamId,
    isConfirming,
    competitions,
    regions,
    activeRegionIds,
    leagueGroups,
    visibleClubs,
    getTeamAvgOvr,
    getPlayerRating,
    selectedTeam,
    selectedTeamXi,
    selectedTeamCompetitions,
    scopeDraft,
    applyScope,
    enabledCompetitionIds,
    handleConfirm,
  };
}
