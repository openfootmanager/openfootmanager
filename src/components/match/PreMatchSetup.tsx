import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { FixtureData, GameStateData } from "../../store/gameStore";
import { getFixtureDisplayLabel } from "../../lib/helpers";
import { MatchSnapshot, FORMATIONS, PLAY_STYLES } from "./types";
import PreMatchLineup, {
  parseFormationNeeds,
} from "./PreMatchLineup";
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

  console.info("[PreMatchSetup] render", {
    activeTab,
    allSquadCount: allSquadPlayers.length,
    awayTeam: snapshot.away_team.name,
    benchCount: userBench.length,
    homeTeam: snapshot.home_team.name,
    phase: snapshot.phase,
    playStyle: userTeam.play_style,
    selectedStarterId,
    setPieces: userSetPieces,
    startingPlayerCount: userTeam.players.length,
    userSide,
    userTeam: userTeam.name,
  });

  const handleFormationChange = async (formation: string) => {
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { ChangeFormation: { side: userSide, formation } },
      });
      onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Formation change failed:", err);
    }
  };

  const handlePlayStyleChange = async (playStyle: string) => {
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { ChangePlayStyle: { side: userSide, play_style: playStyle } },
      });
      onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Play style change failed:", err);
    }
  };

  const handleSwap = async (benchPlayerId: string) => {
    if (!selectedStarterId) return;
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: {
          PreMatchSwap: {
            side: userSide,
            player_off_id: selectedStarterId,
            player_on_id: benchPlayerId,
          },
        },
      });
      onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Pre-match swap failed:", err);
    }
    setSelectedStarterId(null);
  };

  const handleSetPieceTaker = async (role: string, playerId: string) => {
    const commandMap: Record<string, string> = {
      penalty: "SetPenaltyTaker",
      freekick: "SetFreeKickTaker",
      corner: "SetCornerTaker",
      captain: "SetCaptain",
    };
    const cmdKey = commandMap[role];
    if (!cmdKey) return;
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { [cmdKey]: { side: userSide, player_id: playerId } },
      });
      onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Set piece taker change failed:", err);
    }
  };

  const formationNeeds = parseFormationNeeds(userTeam.formation);

  const handleAutoSelect = async () => {
    setIsAutoSelecting(true);
    try {
      const pool = [...userTeam.players, ...userBench];
      const idealIds = new Set<string>();

      for (const pos of ["Goalkeeper", "Defender", "Midfielder", "Forward"]) {
        const candidates = pool
          .filter((p) => p.position === pos)
          .sort(
            (a, b) =>
              b.ovr * (b.condition / 100) - a.ovr * (a.condition / 100),
          );
        const needed = formationNeeds[pos] || 0;
        for (let i = 0; i < Math.min(needed, candidates.length); i++) {
          idealIds.add(candidates[i].id);
        }
      }

      // Fill remaining slots if fewer than 11 (e.g. not enough of a position)
      if (idealIds.size < 11) {
        const rest = pool
          .filter((p) => !idealIds.has(p.id))
          .sort(
            (a, b) =>
              b.ovr * (b.condition / 100) - a.ovr * (a.condition / 100),
          );
        for (const p of rest) {
          if (idealIds.size >= 11) break;
          idealIds.add(p.id);
        }
      }

      const currentIds = new Set(userTeam.players.map((p) => p.id));
      const toAdd = [...idealIds].filter((id) => !currentIds.has(id));
      const toRemove = [...currentIds].filter((id) => !idealIds.has(id));

      let snap: MatchSnapshot | null = null;
      for (let i = 0; i < Math.min(toAdd.length, toRemove.length); i++) {
        snap = await invoke<MatchSnapshot>("apply_match_command", {
          command: {
            PreMatchSwap: {
              side: userSide,
              player_off_id: toRemove[i],
              player_on_id: toAdd[i],
            },
          },
        });
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
                  {t(`common.playStyles.${snapshot.home_team.play_style}`, snapshot.home_team.play_style)}
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
                  {t(`common.playStyles.${snapshot.away_team.play_style}`, snapshot.away_team.play_style)}
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
              {PLAY_STYLES.map((style) => (
                <button
                  key={style}
                  onClick={() => handlePlayStyleChange(style)}
                  className={`flex items-center gap-2 py-2.5 px-3 rounded-lg text-sm font-heading font-bold transition-all ${userTeam.play_style === style
                    ? "bg-primary-500/20 text-primary-400 ring-2 ring-primary-500/50"
                    : "bg-gray-100 text-gray-600 hover:text-gray-900 hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-200 dark:hover:bg-navy-600"
                    }`}
                >
                  {PLAY_STYLE_ICONS[style]}
                  {t(`common.playStyles.${style}`)}
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
              onClick={async () => {
                try {
                  const ids = userTeam.players.map((p) => p.id);
                  const result = await invoke<{
                    captain: string | null;
                    penalty_taker: string | null;
                    free_kick_taker: string | null;
                    corner_taker: string | null;
                  }>("auto_select_set_pieces", { playerIds: ids });
                  if (result.captain)
                    await handleSetPieceTaker("captain", result.captain);
                  if (result.penalty_taker)
                    await handleSetPieceTaker(
                      "penalty",
                      result.penalty_taker,
                    );
                  if (result.free_kick_taker)
                    await handleSetPieceTaker(
                      "freekick",
                      result.free_kick_taker,
                    );
                  if (result.corner_taker)
                    await handleSetPieceTaker("corner", result.corner_taker);
                } catch (err) {
                  console.error("Auto-select set pieces failed:", err);
                }
              }}
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
