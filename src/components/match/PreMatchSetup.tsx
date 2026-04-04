import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FixtureData, GameStateData } from "../../store/gameStore";
import { getFixtureDisplayLabel } from "../../lib/helpers";
import {
  autoSelectSetPieces,
  changeMatchFormation,
  changeMatchPlayStyle,
  type SetPieceRole,
  setMatchSetPieceTaker,
  swapPreMatchPlayers,
} from "../../services/liveMatchService";
import { MatchSnapshot, FORMATIONS, PLAY_STYLES } from "./types";
import PreMatchLineup from "./PreMatchLineup";
import {
  parseFormationNeeds,
} from "./matchLineupUtils";
import {
  getAutoSelectedSetPieceAssignments,
  planAutoSelectSwaps,
} from "./preMatchSetupUtils";
import MatchScreenLayout from "./MatchScreenLayout";
import SetPieceSelector from "./SetPieceSelector";
import {
  ChevronRight,
  Shield,
  Zap,
  Target,
  RefreshCw,
  Crosshair,
  Flag,
  Crown,
  Footprints,
  CornerDownRight,
  CircleDot,
  Wand2,
} from "lucide-react";

interface PreMatchSetupProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  currentFixture?: FixtureData | null;
  userSide: "Home" | "Away";
  onStart: () => void;
  onUpdateSnapshot: (snap: MatchSnapshot) => void;
}

const PLAY_STYLE_ICONS: Record<string, React.ReactNode> = {
  Balanced: <Target className="w-4 h-4" />,
  Attacking: <Zap className="w-4 h-4" />,
  Defensive: <Shield className="w-4 h-4" />,
  Possession: <RefreshCw className="w-4 h-4" />,
  Counter: <Crosshair className="w-4 h-4" />,
  HighPress: <Flag className="w-4 h-4" />,
};

export default function PreMatchSetup({
  snapshot,
  gameState,
  currentFixture,
  userSide,
  onStart,
  onUpdateSnapshot,
}: PreMatchSetupProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"lineup" | "setpieces">("lineup");
  const [selectedStarterId, setSelectedStarterId] = useState<string | null>(
    null,
  );
  const [isAutoSelecting, setIsAutoSelecting] = useState(false);

  const userTeam =
    userSide === "Home" ? snapshot.home_team : snapshot.away_team;
  const oppTeam = userSide === "Home" ? snapshot.away_team : snapshot.home_team;
  const userSetPieces =
    userSide === "Home" ? snapshot.home_set_pieces : snapshot.away_set_pieces;

  const homeTeamColor =
    gameState.teams.find((t) => t.id === snapshot.home_team.id)?.colors
      ?.primary || "#10b981";
  const awayTeamColor =
    gameState.teams.find((t) => t.id === snapshot.away_team.id)?.colors
      ?.primary || "#6366f1";
  const userColor = userSide === "Home" ? homeTeamColor : awayTeamColor;
  const fixtureLabel = currentFixture
    ? getFixtureDisplayLabel(t, currentFixture)
    : t("match.matchDay");

  // All squad players for this team
  const allSquadPlayers = gameState.players.filter(
    (p) => p.team_id === userTeam.id,
  );
  // Use snapshot bench data (updated after swaps)
  const userBench =
    userSide === "Home" ? snapshot.home_bench || [] : snapshot.away_bench || [];

  const applySnapshotUpdate = async (
    action: () => Promise<MatchSnapshot>,
    errorMessage: string,
  ): Promise<MatchSnapshot | null> => {
    try {
      const nextSnapshot = await action();
      onUpdateSnapshot(nextSnapshot);
      return nextSnapshot;
    } catch (err) {
      console.error(errorMessage, err);
      return null;
    }
  };

  const handleFormationChange = async (formation: string) => {
    await applySnapshotUpdate(
      () => changeMatchFormation(userSide, formation),
      "Formation change failed:",
    );
  };

  const handlePlayStyleChange = async (playStyle: string) => {
    await applySnapshotUpdate(
      () => changeMatchPlayStyle(userSide, playStyle),
      "Play style change failed:",
    );
  };

  const handleSwap = async (benchPlayerId: string) => {
    if (!selectedStarterId) return;
    await applySnapshotUpdate(
      () =>
        swapPreMatchPlayers(
          userSide,
          selectedStarterId,
          benchPlayerId,
        ),
      "Pre-match swap failed:",
    );
    setSelectedStarterId(null);
  };

  const handleSetPieceTaker = async (role: SetPieceRole, playerId: string) => {
    await applySnapshotUpdate(
      () => setMatchSetPieceTaker(userSide, role, playerId),
      "Set piece taker change failed:",
    );
  };

  const handleAutoSelectSetPieces = async () => {
    try {
      const result = await autoSelectSetPieces(userTeam.players.map((player) => player.id));

      for (const assignment of getAutoSelectedSetPieceAssignments(result)) {
        await handleSetPieceTaker(assignment.role, assignment.playerId);
      }
    } catch (err) {
      console.error("Auto-select set pieces failed:", err);
    }
  };

  const formationNeeds = parseFormationNeeds(userTeam.formation);

  const handleAutoSelect = async () => {
    setIsAutoSelecting(true);
    try {
      let snap: MatchSnapshot | null = null;
      const swaps = planAutoSelectSwaps(
        userTeam.players,
        userBench,
        formationNeeds,
      );

      for (const swap of swaps) {
        snap = await swapPreMatchPlayers(
          userSide,
          swap.playerOffId,
          swap.playerOnId,
        );
      }

      if (snap) onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Auto-select failed:", err);
    } finally {
      setIsAutoSelecting(false);
      setSelectedStarterId(null);
    }
  };

  return (
    <MatchScreenLayout
      headerClassName="bg-linear-to-r from-gray-200 via-white to-gray-200 dark:from-navy-800 dark:via-navy-900 dark:to-navy-800"
      headerContentClassName="max-w-5xl py-6"
      contentClassName="overflow-auto"
      header={
        <>
          <div className="flex items-center justify-between mb-6">
            <div className="flex items-center gap-4">
              <div
                className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-lg"
                style={{
                  backgroundColor: homeTeamColor + "30",
                  borderColor: homeTeamColor,
                  borderWidth: 2,
                }}
              >
                {snapshot.home_team.name.substring(0, 3).toUpperCase()}
              </div>
              <div>
                <p className="font-heading font-bold text-lg text-gray-900 dark:text-white">
                  {snapshot.home_team.name}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {t("match.home")} · {snapshot.home_team.formation} ·{" "}
                  {t(`tactics.playStyles.${snapshot.home_team.play_style}`, snapshot.home_team.play_style)}
                </p>
              </div>
            </div>

            <div className="text-center">
              <p className="text-xs font-heading uppercase tracking-widest text-accent-700 dark:text-accent-400 mb-1">
                {fixtureLabel}
              </p>
              <p className="text-3xl font-heading font-bold text-gray-500 dark:text-gray-400">
                VS
              </p>
            </div>

            <div className="flex items-center gap-4">
              <div className="text-right">
                <p className="font-heading font-bold text-lg text-gray-900 dark:text-white">
                  {snapshot.away_team.name}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {t("match.away")} · {snapshot.away_team.formation} ·{" "}
                  {t(`tactics.playStyles.${snapshot.away_team.play_style}`, snapshot.away_team.play_style)}
                </p>
              </div>
              <div
                className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-lg"
                style={{
                  backgroundColor: awayTeamColor + "30",
                  borderColor: awayTeamColor,
                  borderWidth: 2,
                }}
              >
                {snapshot.away_team.name.substring(0, 3).toUpperCase()}
              </div>
            </div>
          </div>

          <div className="flex justify-center mt-2">
            <button
              onClick={onStart}
              className="flex items-center gap-3 px-10 py-3.5 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-xl font-heading font-bold uppercase tracking-wider text-sm text-white shadow-lg shadow-primary-500/20 transition-all hover:scale-[1.02] active:scale-[0.98]"
            >
              {t("match.startMatch")}
              <ChevronRight className="w-5 h-5" />
            </button>
          </div>
        </>
      }
    >
      <div className="max-w-5xl mx-auto px-6 py-6 flex flex-col gap-6">
        {/* Formation & Play Style */}
        <div className="grid grid-cols-2 gap-4">
          {/* Formation */}
          <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
              {t("match.formation")}
            </h3>
            <div className="grid grid-cols-3 gap-2">
              {FORMATIONS.map((f) => (
                <button
                  key={f}
                  onClick={() => handleFormationChange(f)}
                  className={`py-2.5 rounded-lg text-sm font-heading font-bold transition-all ${userTeam.formation === f
                    ? "bg-primary-500/20 text-primary-400 ring-2 ring-primary-500/50"
                    : "bg-gray-100 text-gray-600 hover:text-gray-900 hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-200 dark:hover:bg-navy-600"
                    }`}
                >
                  {f}
                </button>
              ))}
            </div>
          </div>

          {/* Play Style */}
          <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
              {t("match.playStyle")}
            </h3>
            <div className="grid grid-cols-2 gap-2">
              {PLAY_STYLES.map((s) => (
                <button
                  key={s.id}
                  onClick={() => handlePlayStyleChange(s.id)}
                  className={`flex items-center gap-2 py-2.5 px-3 rounded-lg text-sm font-heading font-bold transition-all ${userTeam.play_style === s.id
                    ? "bg-primary-500/20 text-primary-400 ring-2 ring-primary-500/50"
                    : "bg-gray-100 text-gray-600 hover:text-gray-900 hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-200 dark:hover:bg-navy-600"
                    }`}
                >
                  {PLAY_STYLE_ICONS[s.id]}
                  {s.label}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-1 bg-gray-200 dark:bg-navy-800 rounded-lg p-1 self-start transition-colors duration-300">
          <button
            onClick={() => setActiveTab("lineup")}
            className={`px-4 py-2 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-colors ${activeTab === "lineup"
              ? "bg-white text-gray-900 shadow-sm dark:bg-navy-600 dark:text-white"
              : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
              }`}
          >
            {t("match.startingLineup")}
          </button>
          <button
            onClick={() => setActiveTab("setpieces")}
            className={`px-4 py-2 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-colors ${activeTab === "setpieces"
              ? "bg-white text-gray-900 shadow-sm dark:bg-navy-600 dark:text-white"
              : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
              }`}
          >
            {t("match.setPiecesCaptain")}
          </button>
        </div>

        {/* Lineup Tab */}
        {activeTab === "lineup" && (
          <PreMatchLineup
            userTeam={userTeam}
            userBench={userBench}
            oppTeam={oppTeam}
            userColor={userColor}
            homeTeamColor={homeTeamColor}
            awayTeamColor={awayTeamColor}
            userSide={userSide}
            formationNeeds={formationNeeds}
            selectedStarterId={selectedStarterId}
            isAutoSelecting={isAutoSelecting}
            onSelectStarter={setSelectedStarterId}
            onSwap={handleSwap}
            onAutoSelect={handleAutoSelect}
          />
        )}

        {/* Set Pieces Tab */}
        {activeTab === "setpieces" && (
          <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
            <button
              onClick={handleAutoSelectSetPieces}
              className="w-full mb-4 flex items-center justify-center gap-2 px-4 py-2.5 bg-accent-50 hover:bg-accent-100 text-accent-700 dark:bg-accent-500/10 dark:hover:bg-accent-500/20 dark:text-accent-400 rounded-lg font-heading font-bold text-xs uppercase tracking-wider transition-colors border border-accent-200 dark:border-accent-500/20"
            >
              <Wand2 className="w-3.5 h-3.5" />
              {t("match.autoSelectTakers")}
            </button>
            <SetPieceSelector
              label={t("match.captain")}
              icon={<Crown className="w-4 h-4 text-accent-400" />}
              role="captain"
              currentId={userSetPieces.captain}
              players={userTeam.players}
              allSquad={allSquadPlayers}
              onSelect={(id) => handleSetPieceTaker("captain", id)}
            />
            <SetPieceSelector
              label={t("match.penaltyTaker")}
              icon={<CircleDot className="w-4 h-4 text-accent-400" />}
              role="penalty"
              currentId={userSetPieces.penalty_taker}
              players={userTeam.players}
              allSquad={allSquadPlayers}
              onSelect={(id) => handleSetPieceTaker("penalty", id)}
            />
            <SetPieceSelector
              label={t("match.freeKickTaker")}
              icon={<Footprints className="w-4 h-4 text-accent-400" />}
              role="freekick"
              currentId={userSetPieces.free_kick_taker}
              players={userTeam.players}
              allSquad={allSquadPlayers}
              onSelect={(id) => handleSetPieceTaker("freekick", id)}
            />
            <SetPieceSelector
              label={t("match.cornerTaker")}
              icon={<CornerDownRight className="w-4 h-4 text-accent-400" />}
              role="corner"
              currentId={userSetPieces.corner_taker}
              players={userTeam.players}
              allSquad={allSquadPlayers}
              onSelect={(id) => handleSetPieceTaker("corner", id)}
            />
          </div>
        )}
      </div>
    </MatchScreenLayout>
  );
}
