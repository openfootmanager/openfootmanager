import { ArrowLeft, ChevronRight, Loader2, SlidersHorizontal } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import ChosenClubCard from "../components/teamSelection/ChosenClubCard";
import ClubsTable from "../components/teamSelection/ClubsTable";
import CountryLeagueRail from "../components/teamSelection/CountryLeagueRail";
import SimulationDialog from "../components/teamSelection/SimulationDialog";
import TeamSelectionStepper from "../components/teamSelection/TeamSelectionStepper";
import { Button, ThemeToggle } from "../components/ui";
import { calcAgeOnDate } from "../lib/valueFormatting";
import { useGameStore } from "../store/gameStore";
import { useTeamSelection } from "./useTeamSelection";

const MANAGE_HINT_ID = "team-selection-manage-hint";

export default function TeamSelection() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { gameState, setGameState, setGameActive } = useGameStore();
  const [scopeOpen, setScopeOpen] = useState(false);
  // A key player's age is worked out from his date of birth in the game state: found by id.
  const playersById = useMemo(
    () => new Map((gameState?.players ?? []).map((player) => [player.id, player])),
    [gameState],
  );

  const {
    clubSearch,
    setClubSearch,
    selectedCountryCode,
    allCountryCodes,
    handleSelectCountry,
    activeLeagueId,
    setSelectedLeagueId,
    setSelectedTeamId,
    isConfirming,
    competitions,
    regions,
    leagueGroups,
    visibleClubs,
    getTeamAvgOvr,
    getClubFacts,
    selectedTeam,
    selectedClubFacts,
    selectedTeamCompetitions,
    scopeDraft,
    applyScope,
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
      <header className="sticky top-0 z-10 flex min-h-16 shrink-0 flex-wrap items-center gap-3.5 border-b border-line bg-panel px-5 py-2">
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
            aria-haspopup="dialog"
            // The dialog locks the chosen club's own competitions and regions, which the backend names
            // with the club's facts: it waits for them rather than show the club as having none.
            disabled={selectedTeam !== null && selectedClubFacts === undefined}
            onClick={() => setScopeOpen(true)}
            icon={<SlidersHorizontal aria-hidden="true" />}
          >
            {t("teamSelect.simulation")}
          </Button>
          {selectedTeam ? null : (
            <span id={MANAGE_HINT_ID} className="sr-only">
              {t("teamSelect.card.emptyTitle")}
            </span>
          )}
          <Button
            disabled={!selectedTeam || isConfirming}
            aria-describedby={selectedTeam ? undefined : MANAGE_HINT_ID}
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

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3.5 px-5 pb-4 pt-4 lg:grid-cols-[14.5rem_minmax(0,1fr)_20rem]">
        <CountryLeagueRail
          countryCodes={allCountryCodes}
          selectedCountryCode={selectedCountryCode}
          onSelectCountry={handleSelectCountry}
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
          getClubFacts={getClubFacts}
        />

        <aside
          aria-label={t("teamSelect.card.label")}
          className="flex min-h-0 flex-col overflow-auto"
        >
          <ChosenClubCard
            team={selectedTeam}
            average={selectedTeam ? getTeamAvgOvr(selectedTeam.id) : null}
            facts={selectedClubFacts}
            competitions={competitions}
            ageOf={(playerId) => {
              const player = playersById.get(playerId);
              return player ? calcAgeOnDate(player.date_of_birth, today) : null;
            }}
          />
        </aside>
      </div>

      {scopeOpen ? (
        <SimulationDialog
          regions={regions}
          competitions={competitions}
          teams={gameState.teams}
          yourCompetitions={selectedTeamCompetitions}
          initial={scopeDraft}
          onApply={(draft) => {
            applyScope(draft);
            setScopeOpen(false);
          }}
          onCancel={() => setScopeOpen(false)}
        />
      ) : null}
    </div>
  );
}
