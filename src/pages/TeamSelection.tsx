import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useGameStore } from "../store/gameStore";
import { buildRegionLabel } from "../lib/teamRegions";
import { Badge, Card, CardBody, ThemeToggle } from "../components/ui";
import { ArrowLeft, ChevronRight, Loader2 } from "lucide-react";
import TeamSelectionScopePanel from "./TeamSelectionScopePanel";
import TeamSelectionGrid from "./TeamSelectionGrid";
import TeamSelectionSidebar from "./TeamSelectionSidebar";
import { useTeamSelection } from "./useTeamSelection";

export default function TeamSelection() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { gameState, setGameState, setGameActive } = useGameStore();

  const {
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
  } = useTeamSelection({ gameState, setGameState, setGameActive, navigate });

  if (!gameState) {
    navigate("/");
    return null;
  }

  return (
    <div className="min-h-screen bg-gray-100 transition-colors duration-300 dark:bg-navy-900">
      <header className="flex items-center justify-between border-b border-gray-200 bg-white px-6 py-4 pt-safe shadow-sm dark:border-navy-700 dark:bg-navy-800">
        <div className="flex items-center gap-4">
          <button
            type="button"
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
              type="button"
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

