import { ArrowLeft, ChevronRight, Loader2, SlidersHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import ChosenClubCard from "../components/teamSelection/ChosenClubCard";
import ClubsTable from "../components/teamSelection/ClubsTable";
import CountryLeagueRail from "../components/teamSelection/CountryLeagueRail";
import { keyPlayers } from "../components/teamSelection/keyPlayers";
import TeamSelectionStepper from "../components/teamSelection/TeamSelectionStepper";
import { Button, Card, CardBody, ThemeToggle } from "../components/ui";
import { competitionDisplayName } from "../lib/competitionName";
import { calcAgeOnDate } from "../lib/valueFormatting";
import { useGameStore } from "../store/gameStore";
import TeamSelectionScopePanel from "./TeamSelectionScopePanel";
import { useTeamSelection } from "./useTeamSelection";

const SCOPE_BAND_ID = "team-selection-scope";

export default function TeamSelection() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { gameState, setGameState, setGameActive } = useGameStore();

  const {
    clubSearch,
    setClubSearch,
    scopeExpanded,
    setScopeExpanded,
    setSelectedHomeRegionId,
    selectedHomeRegionId,
    selectedCountryCode,
    setSelectedCountryCode,
    activeLeagueId,
    setSelectedLeagueId,
    regionSelection,
    setSelectedTeamId,
    scopeMessage,
    setScopeMessage,
    isConfirming,
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
    handleRegionToggle,
    handleCompetitionToggle,
    handleConfirm,
  } = useTeamSelection({ gameState, setGameState, setGameActive, navigate });

  if (!gameState) {
    navigate("/");
    return null;
  }

  const activeLeague = leagueGroups.find((group) => group.id === activeLeagueId) ?? null;
  const today = gameState.clock.current_date;
  let manageLabel = t("teamSelect.manageNone");
  if (isConfirming) manageLabel = t("teamSelect.confirming");
  else if (selectedTeam) manageLabel = t("teamSelect.manage", { name: selectedTeam.short_name });

  return (
    <div className="flex min-h-screen flex-col bg-page text-fg lg:h-screen lg:overflow-hidden">
      <header className="flex min-h-16 shrink-0 flex-wrap items-center gap-3.5 border-b border-line bg-panel px-5 py-2">
        <Button
          variant="ghost"
          size="sm"
          aria-label={t("teamSelect.back")}
          onClick={() => navigate("/")}
          icon={<ArrowLeft aria-hidden="true" />}
        >
          {null}
        </Button>
        <div className="flex flex-col gap-1">
          <h1 className="font-heading text-title font-bold uppercase leading-none tracking-label">
            {t("teamSelect.title")}
          </h1>
          <TeamSelectionStepper />
        </div>
        <div className="ml-auto flex items-center gap-2.5">
          <ThemeToggle />
          <Button
            variant="outline"
            size="sm"
            aria-expanded={scopeExpanded}
            aria-controls={SCOPE_BAND_ID}
            onClick={() => setScopeExpanded((value) => !value)}
            icon={<SlidersHorizontal aria-hidden="true" />}
          >
            {t("teamSelect.simulation")}
          </Button>
          <Button
            disabled={!selectedTeam || isConfirming}
            onClick={handleConfirm}
            iconRight={
              isConfirming ? (
                <Loader2 aria-hidden="true" className="animate-spin" />
              ) : (
                <ChevronRight aria-hidden="true" />
              )
            }
          >
            {manageLabel}
          </Button>
        </div>
      </header>

      {/* The simulation's settings, until they move into a dialog of their own. */}
      <div id={SCOPE_BAND_ID} className="shrink-0 overflow-auto lg:max-h-[45vh]">
        {scopeExpanded ? (
          <div className="space-y-3 px-5 pt-3">
            {scopeMessage && (
              <Card accent="accent">
                <CardBody className="py-3">
                  <p className="text-body text-fg">{t(scopeMessage.key, scopeMessage.values)}</p>
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
          </div>
        ) : null}
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3.5 px-5 pb-4 pt-4 lg:grid-cols-[14.5rem_minmax(0,1fr)_20rem]">
        <CountryLeagueRail
          countryCodes={regionCountries}
          selectedCountryCode={selectedCountryCode}
          onSelectCountry={setSelectedCountryCode}
          leagues={leagueGroups.map((group) => ({
            id: group.id,
            name: group.name,
            clubCount: group.teams.length,
          }))}
          activeLeagueId={activeLeagueId}
          onSelectLeague={setSelectedLeagueId}
        />

        <ClubsTable
          leagueName={activeLeague?.name ?? ""}
          clubs={visibleClubs}
          clubCount={activeLeague?.teams.length ?? 0}
          search={clubSearch}
          onSearchChange={setClubSearch}
          selectedTeamId={selectedTeam?.id ?? null}
          onSelectTeam={setSelectedTeamId}
          getTeamAvgOvr={getTeamAvgOvr}
        />

        <aside
          aria-label={t("teamSelect.card.label")}
          className="flex min-h-0 flex-col overflow-auto"
        >
          <ChosenClubCard
            team={selectedTeam}
            average={selectedTeam ? getTeamAvgOvr(selectedTeam.id) : null}
            keyPlayers={keyPlayers(selectedTeamXi, getPlayerRating, (player) =>
              calcAgeOnDate(player.date_of_birth, today),
            )}
            competitionNames={selectedTeamCompetitions.map((competition) =>
              competitionDisplayName(competition, t),
            )}
          />
        </aside>
      </div>
    </div>
  );
}
