import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import {
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  GameStateData,
  LeagueData,
  PlayerData,
  TeamData,
  WorldRegionData,
  useGameStore,
} from "../store/gameStore";
import { countryName } from "../lib/countries";
import { formatVal, getActiveCompetitions, getPlayerOvr } from "../lib/helpers";
import { buildRegionLabel, inferRegionId } from "../lib/teamRegions";
import { competitionDisplayName } from "../lib/competitionName";
import { Badge, Card, CardBody, Checkbox, Select, TeamLocation, TeamLogo, ThemeToggle } from "../components/ui";
import {
  ArrowLeft,
  ChevronRight,
  Globe,
  Landmark,
  Loader2,
  Shield,
  Star,
  Target,
  Trophy,
  Users,
} from "lucide-react";
import { resolveBackendError } from "../utils/backendI18n";
import { prewarmManagerSquadPortraits } from "../services/portraitService";

type CompetitionSelection = Record<string, boolean>;
type RegionSelection = Record<string, boolean>;
type ScopeMessage = {
  key: string;
  values?: Record<string, string | number>;
};

function competitionScopeLabel(t: TFunction, scope?: string): string | null {
  if (!scope) {
    return null;
  }
  return t(`teamSelect.scopes.${scope}`, { defaultValue: scope });
}

function competitionKindLabel(t: TFunction, kind?: string): string | null {
  if (!kind) {
    return null;
  }
  return t(`teamSelect.kinds.${kind}`, { defaultValue: kind });
}

function buildFallbackRegions(
  t: TFunction,
  gameState: GameStateData,
  competitions: LeagueData[],
): WorldRegionData[] {
  const byRegion = new Map<string, Set<string>>();

  for (const team of gameState.teams) {
    const regionId =
      competitions.find((competition) => competition.country_id === team.country)?.region_id ??
      inferRegionId(team.country);
    if (!regionId) {
      continue;
    }

    const existing = byRegion.get(regionId) ?? new Set<string>();
    existing.add(team.country);
    byRegion.set(regionId, existing);
  }

  return Array.from(byRegion.entries())
    .map(([id, countryCodes]) => ({
      id,
      name: buildRegionLabel(t, id),
      country_codes: Array.from(countryCodes).sort(),
    }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

function competitionRequiredRegions(competition: LeagueData): string[] {
  const regionIds = new Set(competition.required_region_ids ?? []);
  if (
    (competition.scope === "Domestic" || competition.scope === "Regional") &&
    competition.region_id
  ) {
    regionIds.add(competition.region_id);
  }
  return Array.from(regionIds).sort();
}

function teamCompetitions(teamId: string, competitions: LeagueData[]): LeagueData[] {
  return competitions.filter(
    (competition) =>
      competition.participant_ids?.includes(teamId) ??
      competition.fixtures.some(
        (fixture) => fixture.home_team_id === teamId || fixture.away_team_id === teamId,
      ),
  );
}

function likelyXi(players: PlayerData[]): PlayerData[] {
  return [...players]
    .sort((left, right) => getPlayerOvr(right) - getPlayerOvr(left))
    .slice(0, 11);
}

function sortCompetitions(competitions: LeagueData[]): LeagueData[] {
  return [...competitions].sort(
    (left, right) =>
      (left.priority ?? Number.MAX_SAFE_INTEGER) -
        (right.priority ?? Number.MAX_SAFE_INTEGER) ||
      left.name.localeCompare(right.name),
  );
}

export default function TeamSelection() {
  const { t, i18n } = useTranslation();
  const compName = (c: LeagueData) => competitionDisplayName(c, t);
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
        name: competitionDisplayName(league, t),
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

  const getReputationLabel = (
    rep: number,
  ): {
    label: string;
    variant: "primary" | "accent" | "success" | "danger" | "neutral";
  } => {
    if (rep >= 750) return { label: t("teamSelect.repWorldClass"), variant: "accent" };
    if (rep >= 600) return { label: t("teamSelect.repStrong"), variant: "success" };
    if (rep >= 400) return { label: t("teamSelect.repAverage"), variant: "neutral" };
    return { label: t("teamSelect.repDeveloping"), variant: "danger" };
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

        <Card>
          <button
            type="button"
            onClick={() => setScopeExpanded((value) => !value)}
            className="flex w-full items-center justify-between gap-3 px-5 py-3 text-left"
          >
            <span className="font-heading text-sm font-bold uppercase tracking-wide text-gray-700 dark:text-gray-200">
              {t("teamSelect.simulationScope")}
            </span>
            <span className="flex items-center gap-2">
              {!scopeExpanded && (
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  {[
                    selectedHomeRegionId
                      ? buildRegionLabel(t, selectedHomeRegionId)
                      : null,
                    selectedCountryCode
                      ? countryName(selectedCountryCode, i18n.language)
                      : t("teamSelect.allCountries"),
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              )}
              <ChevronRight
                className={`h-4 w-4 text-gray-400 transition-transform ${
                  scopeExpanded ? "rotate-90" : ""
                }`}
              />
            </span>
          </button>
          {scopeExpanded && (
          <CardBody className="grid gap-4 lg:grid-cols-4">
            <div>
              <p className="mb-2 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("teamSelect.homeRegion")}
              </p>
              <Select
                value={selectedHomeRegionId ?? ""}
                onChange={(e) => {
                  setSelectedHomeRegionId(e.target.value || null);
                  setScopeMessage(null);
                }}
                fullWidth
                aria-label={t("teamSelect.homeRegion")}
              >
                {regions.map((region) => (
                  <option key={region.id} value={region.id}>
                    {buildRegionLabel(t, region.id, region.name)}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <p className="mb-2 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("teamSelect.homeCountry")}
              </p>
              <Select
                value={selectedCountryCode ?? ""}
                onChange={(event) =>
                  setSelectedCountryCode(event.target.value || null)
                }
                fullWidth
                aria-label={t("teamSelect.homeCountry")}
              >
                <option value="">{t("teamSelect.allCountries")}</option>
                {regionCountries.map((countryCode) => (
                  <option key={countryCode} value={countryCode}>
                    {countryName(countryCode, i18n.language)}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <p className="mb-2 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("teamSelect.simulatedRegions")}
              </p>
              <div className="space-y-2">
                {regions.map((region) => {
                  const enabled =
                    region.id === selectedHomeRegionId || Boolean(regionSelection[region.id]);
                  const isLocked = region.id === selectedHomeRegionId;
                  return (
                    <div
                      key={region.id}
                      role="button"
                      aria-pressed={enabled}
                      aria-disabled={isLocked}
                      tabIndex={isLocked ? -1 : 0}
                      onClick={() => !isLocked && handleRegionToggle(region.id)}
                      onKeyDown={(e) => {
                        if (!isLocked && (e.key === "Enter" || e.key === " ")) {
                          e.preventDefault();
                          handleRegionToggle(region.id);
                        }
                      }}
                      className={`flex items-center justify-between rounded-xl border border-gray-200 bg-gray-50 px-3 py-2 text-sm dark:border-navy-600 dark:bg-navy-800 ${!isLocked ? "cursor-pointer" : ""}`}
                    >
                      <span className="flex items-center gap-2">
                        <Globe className="h-4 w-4 text-primary-500" />
                        <span>{buildRegionLabel(t, region.id, region.name)}</span>
                        {isLocked && (
                          <Badge variant="primary" size="sm">
                            {t("teamSelect.homeBadge")}
                          </Badge>
                        )}
                      </span>
                      <span
                        onClick={(e) => e.stopPropagation()}
                        onKeyDown={(e) => e.stopPropagation()}
                      >
                        <Checkbox
                          checked={enabled}
                          disabled={isLocked}
                          onChange={() => handleRegionToggle(region.id)}
                          aria-label={buildRegionLabel(t, region.id, region.name)}
                        />
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>

            <div>
              <p className="mb-2 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("teamSelect.simulatedCompetitions")}
              </p>
              <div className="max-h-52 space-y-2 overflow-y-auto pr-1">
                {availableCompetitions.map((competition) => {
                  const enabled =
                    Boolean(competitionSelection[competition.id]) ||
                    mandatoryCompetitionIds.has(competition.id);
                  const isLocked = mandatoryCompetitionIds.has(competition.id);
                  const requiredRegions = competitionRequiredRegions(competition);
                  const missingRegions = requiredRegions.filter(
                    (regionId) => !activeRegionIds.includes(regionId),
                  );

                  return (
                    <div
                      key={competition.id}
                      role="button"
                      aria-pressed={enabled}
                      aria-disabled={isLocked}
                      tabIndex={isLocked ? -1 : 0}
                      onClick={() => !isLocked && handleCompetitionToggle(competition)}
                      onKeyDown={(e) => {
                        if (!isLocked && (e.key === "Enter" || e.key === " ")) {
                          e.preventDefault();
                          handleCompetitionToggle(competition);
                        }
                      }}
                      className={`block rounded-xl border border-gray-200 bg-gray-50 px-3 py-2 text-sm dark:border-navy-600 dark:bg-navy-800 ${!isLocked ? "cursor-pointer" : ""}`}
                    >
                      <div className="flex items-center justify-between gap-3">
                        <span className="flex min-w-0 items-center gap-2">
                          <Trophy className="h-4 w-4 shrink-0 text-accent-500" />
                          <span className="truncate">{compName(competition)}</span>
                        </span>
                        <span
                          onClick={(e) => e.stopPropagation()}
                          onKeyDown={(e) => e.stopPropagation()}
                        >
                          <Checkbox
                            checked={enabled}
                            disabled={isLocked}
                            onChange={() => handleCompetitionToggle(competition)}
                            aria-label={compName(competition)}
                          />
                        </span>
                      </div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {competitionScopeLabel(t, competition.scope) && (
                          <Badge variant="neutral" size="sm">
                            {competitionScopeLabel(t, competition.scope)}
                          </Badge>
                        )}
                        {competition.kind &&
                          competition.kind !== "League" &&
                          competitionKindLabel(t, competition.kind) && (
                          <Badge variant="accent" size="sm">
                            {competitionKindLabel(t, competition.kind)}
                          </Badge>
                        )}
                        {isLocked && (
                          <Badge variant="primary" size="sm">
                            {t("teamSelect.yourClubBadge")}
                          </Badge>
                        )}
                      </div>
                      {missingRegions.length > 0 && (
                        <p className="mt-2 text-[11px] text-amber-600 dark:text-amber-400">
                          {t("teamSelect.requiresRegions", {
                            regions: missingRegions
                              .map((regionId) => buildRegionLabel(t, regionId))
                              .join(", "),
                          })}
                        </p>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </CardBody>
          )}
        </Card>

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.2fr)_minmax(340px,0.8fr)]">
          <div>
            <div className="mb-3 flex flex-wrap items-center gap-3">
              <input
                type="text"
                value={clubSearch}
                onChange={(event) => setClubSearch(event.target.value)}
                placeholder={t("teamSelect.searchClubs")}
                className="min-w-0 flex-1 rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-700 placeholder:text-gray-400 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-200"
              />
              <span className="shrink-0 text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("teamSelect.clubCount", { n: filteredTeams.length })}
              </span>
            </div>
            {filteredTeams.length === 0 ? (
              <p className="py-10 text-center text-sm text-gray-500 dark:text-gray-400">
                {t("teamSelect.noClubsMatch")}
              </p>
            ) : (
              <div className="max-h-[640px] space-y-5 overflow-y-auto pr-1">
                {teamGroups.map((group) => (
                  <div key={group.id}>
                    <p className="mb-2 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                      {group.name}
                    </p>
                    <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            {group.teams.map((team) => {
              const isSelected = selectedTeam?.id === team.id;
              const avgOvr = getTeamAvgOvr(team.id);
              const repInfo = getReputationLabel(team.reputation);
              const playerCount = getTeamPlayers(team.id).length;

              return (
                <button
                  key={team.id}
                  onClick={() => setSelectedTeamId(team.id)}
                  className={`rounded-xl text-left transition-all duration-200 ${
                    isSelected
                      ? "scale-[1.01] ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-navy-900"
                      : "hover:scale-[1.01]"
                  }`}
                >
                  <Card accent={isSelected ? "primary" : "none"} className="h-full">
                    <div
                      className={`rounded-t-xl p-4 ${
                        isSelected
                          ? "bg-gradient-to-r from-primary-600 to-primary-700"
                          : "bg-gradient-to-r from-navy-700 to-navy-800"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <TeamLogo
                            team={team}
                            className={`flex h-12 w-12 items-center justify-center overflow-hidden rounded-lg font-heading text-lg font-bold ${
                              isSelected ? "bg-white/20 text-white" : "bg-white/10 text-gray-300"
                            }`}
                          />
                          <div>
                            <h3 className="font-heading text-sm font-bold uppercase tracking-wide text-white">
                              {team.name}
                            </h3>
                            <TeamLocation
                              city={team.city}
                              countryCode={team.country}
                              locale={i18n.language}
                              className="mt-0.5 text-xs text-gray-300"
                              iconClassName="w-3 h-3"
                              flagClassName="text-xs leading-none"
                            />
                          </div>
                        </div>
                        {isSelected && <Star className="h-5 w-5 fill-current text-accent-400" />}
                      </div>
                    </div>

                    <CardBody className="p-4">
                      <div className="grid grid-cols-2 gap-3">
                        <InfoStat
                          icon={<Trophy className="h-3.5 w-3.5" />}
                          label={t("teamSelect.reputation")}
                          value={
                            <Badge variant={repInfo.variant} size="sm">
                              {repInfo.label}
                            </Badge>
                          }
                        />
                        <InfoStat
                          icon={<Users className="h-3.5 w-3.5" />}
                          label={t("teamSelect.squad")}
                          value={
                            <span className="font-heading font-bold text-gray-800 dark:text-gray-200">
                              {playerCount}
                            </span>
                          }
                        />
                        <InfoStat
                          icon={<Landmark className="h-3.5 w-3.5" />}
                          label={t("teamSelect.finances")}
                          value={
                            <span className="font-heading font-bold text-gray-800 dark:text-gray-200">
                              {formatVal(team.finance)}
                            </span>
                          }
                        />
                        <InfoStat
                          icon={<Star className="h-3.5 w-3.5" />}
                          label={t("teamSelect.avgOvr")}
                          value={
                            <span className="font-heading text-lg font-bold text-primary-500">
                              {avgOvr}
                            </span>
                          }
                        />
                      </div>
                    </CardBody>
                  </Card>
                </button>
              );
            })}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          <Card accent="accent" className="h-fit">
            <CardBody className="space-y-5 p-5">
              {selectedTeam ? (
                <>
                  <div>
                    <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                      {t("teamSelect.selectedClub")}
                    </p>
                    <h2 className="mt-1 font-heading text-2xl font-bold text-gray-900 dark:text-white">
                      {selectedTeam.name}
                    </h2>
                    <TeamLocation
                      city={selectedTeam.city}
                      countryCode={selectedTeam.country}
                      locale={i18n.language}
                      className="mt-2 text-sm text-gray-500 dark:text-gray-400"
                    />
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <DetailTile
                      icon={<Target className="h-4 w-4" />}
                      label={t("teamSelect.formation")}
                      value={selectedTeam.formation}
                    />
                    <DetailTile
                      icon={<Shield className="h-4 w-4" />}
                      label={t("teamSelect.overall")}
                      value={String(getTeamAvgOvr(selectedTeam.id))}
                    />
                    <DetailTile
                      icon={<Users className="h-4 w-4" />}
                      label={t("teamSelect.likelyXi")}
                      value={t("teamSelect.playersCount", { count: selectedTeamXi.length })}
                    />
                    <DetailTile
                      icon={<Globe className="h-4 w-4" />}
                      label={t("teamSelect.competitions")}
                      value={String(selectedTeamCompetitions.length)}
                    />
                  </div>

                  <div>
                    <p className="mb-3 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                      {t("teamSelect.keyPlayers")}
                    </p>
                    <div className="space-y-2">
                      {selectedTeamXi.slice(0, 5).map((player) => (
                        <div
                          key={player.id}
                          className="flex items-center justify-between rounded-xl bg-gray-50 px-3 py-2 dark:bg-navy-800"
                        >
                          <div>
                            <p className="font-semibold text-gray-900 dark:text-white">
                              {player.match_name}
                            </p>
                            <p className="text-xs text-gray-500 dark:text-gray-400">
                              {player.position}
                            </p>
                          </div>
                          <Badge variant="accent">{getPlayerOvr(player)}</Badge>
                        </div>
                      ))}
                    </div>
                  </div>

                  <div>
                    <p className="mb-3 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                      {t("teamSelect.activeCompetitions")}
                    </p>
                    <div className="flex flex-wrap gap-2">
                      {selectedTeamCompetitions.map((competition) => (
                        <Badge key={competition.id} variant="primary">
                          {compName(competition)}
                        </Badge>
                      ))}
                    </div>
                    <p className="mt-3 text-xs text-gray-500 dark:text-gray-400">
                      {t("teamSelect.clubCompetitionsAlwaysSimulated")}
                    </p>
                  </div>
                </>
              ) : (
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  {t("teamSelect.selectClubPrompt")}
                </p>
              )}
            </CardBody>
          </Card>
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

function InfoStat({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <span className="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500">
        {icon} {label}
      </span>
      {value}
    </div>
  );
}

function DetailTile({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-xl bg-gray-50 px-3 py-3 dark:bg-navy-800">
      <p className="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500">
        {icon} {label}
      </p>
      <p className="mt-1 font-heading font-bold text-gray-900 dark:text-white">{value}</p>
    </div>
  );
}
