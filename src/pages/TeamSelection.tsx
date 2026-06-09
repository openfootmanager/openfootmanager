import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData, LeagueData, PlayerData } from "../store/gameStore";
import { countryName } from "../lib/countries";
import { formatVal, getPlayerOvr, getActiveCompetitions } from "../lib/helpers";
import { Card, CardBody, Badge, TeamLocation, ThemeToggle } from "../components/ui";
import {
  ArrowLeft,
  Users,
  Trophy,
  Landmark,
  ChevronRight,
  Star,
  Loader2,
  Globe,
  Shield,
  Target,
} from "lucide-react";
import { resolveBackendError } from "../utils/backendI18n";

type CompetitionSelection = Record<string, boolean>;

function buildRegionLabel(regionId: string): string {
  return regionId
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function teamCompetitions(teamId: string, competitions: LeagueData[]): LeagueData[] {
  return competitions.filter((competition) =>
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

export default function TeamSelection() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { gameState, setGameState, setGameActive } = useGameStore();
  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);

  const competitions = useMemo(
    () => (gameState ? getActiveCompetitions(gameState) : []),
    [gameState],
  );

  const regions = useMemo(
    () =>
      gameState?.regions && gameState.regions.length > 0
        ? gameState.regions
        : Array.from(
            new Map(
              (gameState?.teams ?? []).map((team) => [
                team.country,
                {
                  id:
                    competitions.find((competition) => competition.country_id === team.country)
                      ?.region_id ?? "europe",
                  name: buildRegionLabel(
                    competitions.find((competition) => competition.country_id === team.country)
                      ?.region_id ?? "europe",
                  ),
                  country_codes: [team.country],
                },
              ]),
            ).values(),
          ),
    [competitions, gameState],
  );

  const [selectedRegionId, setSelectedRegionId] = useState<string | null>(
    regions[0]?.id ?? null,
  );
  const regionCountries = useMemo(() => {
    const currentRegion = regions.find((region) => region.id === selectedRegionId);
    if (!currentRegion) return [];
    return currentRegion.country_codes;
  }, [regions, selectedRegionId]);
  const [selectedCountryCode, setSelectedCountryCode] = useState<string | null>(
    regionCountries[0] ?? null,
  );

  const availableCompetitions = useMemo(() => {
    if (!selectedRegionId) return competitions;
    return competitions.filter(
      (competition) =>
        competition.region_id === selectedRegionId ||
        competition.scope === "Continental" ||
        competition.scope === "International",
    );
  }, [competitions, selectedRegionId]);

  const [competitionSelection, setCompetitionSelection] = useState<CompetitionSelection>(() =>
    Object.fromEntries(competitions.map((competition) => [competition.id, true])),
  );

  if (!gameState) {
    navigate("/");
    return null;
  }

  const teams = gameState.teams.filter((team) => {
    if (selectedCountryCode) {
      return team.country === selectedCountryCode;
    }
    if (selectedRegionId) {
      return teamCompetitions(team.id, competitions).some(
        (competition) => competition.region_id === selectedRegionId,
      );
    }
    return true;
  });

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
  ): { label: string; variant: "primary" | "accent" | "success" | "danger" | "neutral" } => {
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

  const handleConfirm = async () => {
    if (!selectedTeam || isConfirming) return;
    setIsConfirming(true);
    try {
      const activeCompetitionIds = Object.entries(competitionSelection)
        .filter(([, enabled]) => enabled)
        .map(([competitionId]) => competitionId);
      const updatedGame = await invoke<GameStateData>("select_team", {
        teamId: selectedTeam.id,
        activeRegionIds: selectedRegionId ? [selectedRegionId] : [],
        activeCompetitionIds,
      });
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
    <div className="min-h-screen bg-gray-100 dark:bg-navy-900 transition-colors duration-300">
      <header className="bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 px-6 py-4 flex justify-between items-center shadow-sm">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate("/")}
            className="p-2 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h1 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
              {t("teamSelect.title")}
            </h1>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
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
              className={`bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white px-6 py-2.5 rounded-lg font-heading font-bold uppercase tracking-wider text-sm shadow-md hover:shadow-lg hover:shadow-primary-500/20 transition-all flex items-center gap-2 ${isConfirming ? "opacity-70 cursor-wait" : ""}`}
            >
              <span>
                {isConfirming
                  ? t("teamSelect.confirming")
                  : t("teamSelect.manage", { name: selectedTeam.short_name })}
              </span>
              {isConfirming ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <ChevronRight className="w-4 h-4" />
              )}
            </button>
          )}
        </div>
      </header>

      <div className="max-w-7xl mx-auto p-6 space-y-5">
        <Card>
          <CardBody className="grid gap-4 lg:grid-cols-3">
            <div>
              <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-2">
                {t("createManager.selectCountry")}
              </p>
              <div className="flex flex-wrap gap-2">
                {regions.map((region) => (
                  <button
                    key={region.id}
                    type="button"
                    onClick={() => {
                      setSelectedRegionId(region.id);
                      setSelectedCountryCode(region.country_codes[0] ?? null);
                    }}
                    className={`rounded-full px-3 py-1.5 text-xs font-heading font-bold uppercase tracking-wide transition-colors ${
                      selectedRegionId === region.id
                        ? "bg-primary-500 text-white"
                        : "bg-gray-100 text-gray-600 dark:bg-navy-700 dark:text-gray-300"
                    }`}
                  >
                    {region.name}
                  </button>
                ))}
              </div>
            </div>

            <div>
              <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-2">
                {t("createManager.countryOfOrigin")}
              </p>
              <div className="flex flex-wrap gap-2">
                {regionCountries.map((countryCode) => (
                  <button
                    key={countryCode}
                    type="button"
                    onClick={() => setSelectedCountryCode(countryCode)}
                    className={`rounded-full px-3 py-1.5 text-xs font-heading font-bold uppercase tracking-wide transition-colors ${
                      selectedCountryCode === countryCode
                        ? "bg-accent-500 text-white"
                        : "bg-gray-100 text-gray-600 dark:bg-navy-700 dark:text-gray-300"
                    }`}
                  >
                    {countryName(countryCode, i18n.language)}
                  </button>
                ))}
              </div>
            </div>

            <div>
              <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-2">
                Simulated Competitions
              </p>
              <div className="space-y-2 max-h-40 overflow-y-auto pr-1">
                {availableCompetitions.map((competition) => {
                  const enabled = competitionSelection[competition.id] ?? true;
                  return (
                    <label
                      key={competition.id}
                      className="flex items-center justify-between rounded-xl border border-gray-200 bg-gray-50 px-3 py-2 text-sm dark:border-navy-600 dark:bg-navy-800"
                    >
                      <span className="flex items-center gap-2">
                        <Trophy className="w-4 h-4 text-accent-500" />
                        <span>{competition.name}</span>
                      </span>
                      <input
                        type="checkbox"
                        checked={enabled}
                        onChange={() =>
                          setCompetitionSelection((current) => ({
                            ...current,
                            [competition.id]: !enabled,
                          }))
                        }
                      />
                    </label>
                  );
                })}
              </div>
            </div>
          </CardBody>
        </Card>

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.2fr)_minmax(340px,0.8fr)]">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {teams.map((team) => {
              const isSelected = selectedTeam?.id === team.id;
              const avgOvr = getTeamAvgOvr(team.id);
              const repInfo = getReputationLabel(team.reputation);
              const playerCount = getTeamPlayers(team.id).length;

              return (
                <button
                  key={team.id}
                  onClick={() => setSelectedTeamId(team.id)}
                  className={`text-left transition-all duration-200 rounded-xl ${
                    isSelected
                      ? "ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-navy-900 scale-[1.01]"
                      : "hover:scale-[1.01]"
                  }`}
                >
                  <Card accent={isSelected ? "primary" : "none"} className="h-full">
                    <div
                      className={`p-4 rounded-t-xl ${
                        isSelected
                          ? "bg-gradient-to-r from-primary-600 to-primary-700"
                          : "bg-gradient-to-r from-navy-700 to-navy-800"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <div
                            className={`w-12 h-12 rounded-lg flex items-center justify-center font-heading font-bold text-lg ${
                              isSelected ? "bg-white/20 text-white" : "bg-white/10 text-gray-300"
                            }`}
                          >
                            {team.short_name}
                          </div>
                          <div>
                            <h3 className="font-heading font-bold text-white uppercase tracking-wide text-sm">
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
                        {isSelected && <Star className="w-5 h-5 text-accent-400 fill-current" />}
                      </div>
                    </div>

                    <CardBody className="p-4">
                      <div className="grid grid-cols-2 gap-3">
                        <InfoStat
                          icon={<Trophy className="w-3.5 h-3.5" />}
                          label={t("teamSelect.reputation")}
                          value={<Badge variant={repInfo.variant} size="sm">{repInfo.label}</Badge>}
                        />
                        <InfoStat
                          icon={<Users className="w-3.5 h-3.5" />}
                          label={t("teamSelect.squad")}
                          value={<span className="font-heading font-bold text-gray-800 dark:text-gray-200">{playerCount}</span>}
                        />
                        <InfoStat
                          icon={<Landmark className="w-3.5 h-3.5" />}
                          label={t("teamSelect.finances")}
                          value={<span className="font-heading font-bold text-gray-800 dark:text-gray-200">{formatVal(team.finance)}</span>}
                        />
                        <InfoStat
                          icon={<Star className="w-3.5 h-3.5" />}
                          label={t("teamSelect.avgOvr")}
                          value={<span className="font-heading font-bold text-lg text-primary-500">{avgOvr}</span>}
                        />
                      </div>
                    </CardBody>
                  </Card>
                </button>
              );
            })}
          </div>

          <Card accent="accent" className="h-fit">
            <CardBody className="p-5 space-y-5">
              {selectedTeam ? (
                <>
                  <div>
                    <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                      Selected Club
                    </p>
                    <h2 className="mt-1 text-2xl font-heading font-bold text-gray-900 dark:text-white">
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
                    <DetailTile icon={<Target className="w-4 h-4" />} label="Formation" value={selectedTeam.formation} />
                    <DetailTile icon={<Shield className="w-4 h-4" />} label="Overall" value={String(getTeamAvgOvr(selectedTeam.id))} />
                    <DetailTile icon={<Users className="w-4 h-4" />} label="Likely XI" value={`${selectedTeamXi.length} players`} />
                    <DetailTile icon={<Globe className="w-4 h-4" />} label="Competitions" value={String(selectedTeamCompetitions.length)} />
                  </div>

                  <div>
                    <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-3">
                      Key Players
                    </p>
                    <div className="space-y-2">
                      {selectedTeamXi.slice(0, 5).map((player) => (
                        <div
                          key={player.id}
                          className="flex items-center justify-between rounded-xl bg-gray-50 px-3 py-2 dark:bg-navy-800"
                        >
                          <div>
                            <p className="font-semibold text-gray-900 dark:text-white">{player.match_name}</p>
                            <p className="text-xs text-gray-500 dark:text-gray-400">{player.position}</p>
                          </div>
                          <Badge variant="accent">{getPlayerOvr(player)}</Badge>
                        </div>
                      ))}
                    </div>
                  </div>

                  <div>
                    <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400 mb-3">
                      Active Competitions
                    </p>
                    <div className="flex flex-wrap gap-2">
                      {selectedTeamCompetitions.map((competition) => (
                        <Badge key={competition.id} variant="primary">
                          {competition.name}
                        </Badge>
                      ))}
                    </div>
                  </div>
                </>
              ) : (
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  Select a club to inspect its squad and competitions.
                </p>
              )}
            </CardBody>
          </Card>
        </div>
      </div>
    </div>
  );
}

function InfoStat({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-gray-400 dark:text-gray-500 flex items-center gap-1">
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
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-xl bg-gray-50 px-3 py-3 dark:bg-navy-800">
      <p className="text-xs text-gray-400 dark:text-gray-500 flex items-center gap-1">
        {icon} {label}
      </p>
      <p className="mt-1 font-heading font-bold text-gray-900 dark:text-white">{value}</p>
    </div>
  );
}
