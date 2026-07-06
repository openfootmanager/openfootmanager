import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { FixtureData, GameStateData } from "../../store/gameStore";
import { getFixtureDisplayLabel } from "../../lib/helpers";
import { MatchSnapshot, EnginePlayerData, FORMATIONS, PLAY_STYLES } from "./types";
import PreMatchLineup, { parseFormationNeeds, POSITION_KEY_STATS, statColor, starterOvrColor, getStatVal } from "./PreMatchLineup";
import { condColor } from "../../lib/playerConditionDisplay";
import { getSetPieceStats } from "./SetPieceSelector";
import { FormationPitch } from "./FormationPitch";
import { makeTeamFallback } from "./helpers";
import {
  isPlayerExactForSlot,
  isPlayerOutOfPosition,
  normalisePosition,
  translatePositionAbbreviation,
} from "../squad/SquadTab.helpers";
import { PhaseBlueprintPanel } from "../tactics/PhaseBlueprintPanel";
import { setPlayerRole, setTacticsPhase } from "../../services/squadService";
import { getRoleOptions } from "../../lib/playerRoles";
import type { PlayerRole, TacticsPhaseSettings } from "../../store/types";
import { PitchToken, Select, TeamLogo, ThemeToggle, type PitchFitTone } from "../ui";
import {
  ChevronRight,
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

export default function PreMatchSetup({
  snapshot,
  gameState,
  currentFixture,
  userSide,
  onStart,
  onUpdateSnapshot,
}: PreMatchSetupProps) {
  const { t } = useTranslation();
  const [selectedStarterId, setSelectedStarterId] = useState<string | null>(null);
  const [isAutoSelecting, setIsAutoSelecting] = useState(false);
  const [activeTab, setActiveTab] = useState<"team" | "opponent">("team");
  const [phase, setPhase] = useState<TacticsPhaseSettings | undefined>(() => {
    const uid =
      userSide === "Home" ? snapshot.home_team.id : snapshot.away_team.id;
    return gameState.teams.find((tm) => tm.id === uid)?.tactics_phase;
  });

  const handlePhaseChange = (patch: Partial<TacticsPhaseSettings>) => {
    setPhase((prev) =>
      prev ? { ...prev, ...patch } : ({ ...patch } as TacticsPhaseSettings),
    );
    void setTacticsPhase(patch).catch((err: unknown) => {
      console.error("Failed to set tactics phase:", err);
    });
  };

  // Player roles, editable from the pitch like on the tactics board; optimistic
  // local state persisted fire-and-forget, same pattern as the phase blueprint.
  const [playerRoles, setPlayerRoles] = useState<Record<string, PlayerRole>>(() => {
    const uid =
      userSide === "Home" ? snapshot.home_team.id : snapshot.away_team.id;
    return gameState.teams.find((tm) => tm.id === uid)?.player_roles ?? {};
  });

  const handlePlayerRoleChange = (playerId: string, role: PlayerRole) => {
    const previous = playerRoles[playerId] ?? "Standard";
    setPlayerRoles((prev) => ({ ...prev, [playerId]: role }));
    void setPlayerRole(playerId, role).catch((err: unknown) => {
      console.error("Failed to set player role:", err);
      // Roll back the optimistic value so the UI doesn't show a role that was
      // never persisted — unless the user has already picked something newer.
      setPlayerRoles((prev) =>
        prev[playerId] === role ? { ...prev, [playerId]: previous } : prev,
      );
    });
  };

  const homeTeam = snapshot.home_team;
  const awayTeam = snapshot.away_team;
  const userTeam = userSide === "Home" ? homeTeam : awayTeam;
  const oppTeam = userSide === "Home" ? awayTeam : homeTeam;
  const userSetPieces =
    userSide === "Home" ? snapshot.home_set_pieces : snapshot.away_set_pieces;

  const homeFullTeam = gameState.teams.find((t) => t.id === homeTeam.id);
  const awayFullTeam = gameState.teams.find((t) => t.id === awayTeam.id);
  const homeTeamColor = homeFullTeam?.colors?.primary ?? "#10b981";
  const awayTeamColor = awayFullTeam?.colors?.primary ?? "#6366f1";
  const userColor = userSide === "Home" ? homeTeamColor : awayTeamColor;

  const userFullTeam = userSide === "Home" ? homeFullTeam : awayFullTeam;
  const userPrimary = userFullTeam?.colors?.primary ?? userColor;
  const userSecondary = userFullTeam?.colors?.secondary ?? "#1a3a6b";
  const userPattern = userFullTeam?.kit_pattern ?? "Solid";

  const oppFullTeam = userSide === "Home" ? awayFullTeam : homeFullTeam;
  const oppPrimary = oppFullTeam?.colors?.primary ?? "#6366f1";
  const oppSecondary = oppFullTeam?.colors?.secondary ?? "#1a3a6b";
  const oppPattern = oppFullTeam?.kit_pattern ?? "Solid";

  // Index the full squad so pitch tokens can be enriched with face/jersey/natural
  // position that the lightweight match snapshot player doesn't carry.
  const storeById = useMemo(
    () => new Map(gameState.players.map((p) => [p.id, p])),
    [gameState.players],
  );

  const jerseyNumberById = useMemo(
    () => new Map(gameState.players.map((p) => [p.id, p.jersey_number])),
    [gameState.players],
  );

  // Rich token for the user's command pitch (avatar, kit, OVR, fit ring).
  const renderUserToken = (
    player: EnginePlayerData,
    isSelected: boolean,
    slotPosition?: string,
  ) => {
    const sp = storeById.get(player.id);
    // With a granular slot (slot-aligned pitch), grade fit exactly like the
    // tactics board; otherwise fall back to the coarse group comparison.
    const fit: PitchFitTone = !sp
      ? "exact"
      : slotPosition
        ? isPlayerExactForSlot(sp, slotPosition)
          ? "exact"
          : isPlayerOutOfPosition(sp, slotPosition)
            ? "out"
            : "adapted"
        : normalisePosition(sp.natural_position || sp.position) === player.position
          ? "exact"
          : "out";
    const displayPosition = slotPosition ?? player.position;
    return (
      <div
        className={`flex w-24 flex-col items-center gap-0.5 rounded-xl px-1 py-1 ${
          isSelected ? "bg-accent-500/25 ring-2 ring-accent-300/70" : ""
        }`}
      >
        <PitchToken
          name={(sp?.match_name || player.name).toUpperCase()}
          positionAbbr={translatePositionAbbreviation(t, displayPosition)}
          position={displayPosition}
          ovr={player.ovr}
          condition={player.condition}
          fitTone={fit}
          avatar={
            sp
              ? { full_name: sp.full_name, match_name: sp.match_name, media: sp.media }
              : { full_name: player.name, match_name: player.name }
          }
          jersey={{
            primaryColor: userPrimary,
            secondaryColor: userSecondary,
            pattern: userPattern,
            number: sp?.jersey_number,
          }}
        >
          <div
            draggable={false}
            onClick={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
            className="w-full"
          >
            <Select
              selectSize="sm"
              variant="ghost"
              fullWidth
              value={playerRoles[player.id] ?? "Standard"}
              onChange={(e) => {
                handlePlayerRoleChange(player.id, e.target.value as PlayerRole);
              }}
            >
              {getRoleOptions(
                displayPosition,
                playerRoles[player.id] ?? "Standard",
              ).map((role) => (
                <option key={role} value={role}>
                  {t(`tactics.playerRoles.${role}`, role)}
                </option>
              ))}
            </Select>
          </div>
        </PitchToken>
      </div>
    );
  };

  // Basic token for the opponent's scouting pitch: avatar, kit, and OVR only —
  // no fit ring or role furniture, which is the user's-side detail.
  const renderOppToken = (player: EnginePlayerData, slotPosition?: string) => {
    const sp = storeById.get(player.id);
    const displayPosition = slotPosition ?? player.position;
    return (
      <div className="flex w-24 flex-col items-center gap-0.5 rounded-xl px-1 py-1">
        <PitchToken
          name={(sp?.match_name || player.name).toUpperCase()}
          positionAbbr={translatePositionAbbreviation(t, displayPosition)}
          position={displayPosition}
          ovr={player.ovr}
          condition={player.condition}
          avatar={
            sp
              ? { full_name: sp.full_name, match_name: sp.match_name, media: sp.media }
              : { full_name: player.name, match_name: player.name }
          }
          jersey={{
            primaryColor: oppPrimary,
            secondaryColor: oppSecondary,
            pattern: oppPattern,
            number: sp?.jersey_number,
          }}
        />
      </div>
    );
  };

  const fixtureLabel = currentFixture
    ? getFixtureDisplayLabel(t, currentFixture)
    : t("match.matchDay");

  const allSquadPlayers = gameState.players.filter(
    (p) => p.team_id === userTeam.id,
  );
  const userBench =
    userSide === "Home" ? snapshot.home_bench ?? [] : snapshot.away_bench ?? [];

  const formationNeeds = parseFormationNeeds(userTeam.formation);

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

  const handleAutoSelect = async () => {
    setIsAutoSelecting(true);
    try {
      const pool = [...userTeam.players, ...userBench];
      const idealIds = new Set<string>();

      for (const pos of ["Goalkeeper", "Defender", "Midfielder", "Forward"]) {
        const candidates = pool
          .filter((p) => p.position === pos)
          .sort((a, b) => b.ovr * (b.condition / 100) - a.ovr * (a.condition / 100));
        const needed = formationNeeds[pos] ?? 0;
        for (let i = 0; i < Math.min(needed, candidates.length); i++) {
          idealIds.add(candidates[i].id);
        }
      }

      if (idealIds.size < 11) {
        const rest = pool
          .filter((p) => !idealIds.has(p.id))
          .sort((a, b) => b.ovr * (b.condition / 100) - a.ovr * (a.condition / 100));
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

  const handleAutoSelectSetPieces = async () => {
    try {
      const ids = userTeam.players.map((p) => p.id);
      const result = await invoke<{
        captain: string | null;
        penalty_taker: string | null;
        free_kick_taker: string | null;
        corner_taker: string | null;
      }>("auto_select_set_pieces", { playerIds: ids });
      if (result.captain) await handleSetPieceTaker("captain", result.captain);
      if (result.penalty_taker) await handleSetPieceTaker("penalty", result.penalty_taker);
      if (result.free_kick_taker) await handleSetPieceTaker("freekick", result.free_kick_taker);
      if (result.corner_taker) await handleSetPieceTaker("corner", result.corner_taker);
    } catch (err) {
      console.error("Auto-select set pieces failed:", err);
    }
  };

  const sortedForRole = (role: string) => {
    const allowGk = role === "captain";
    return userTeam.players
      .filter((p) => allowGk || p.position !== "Goalkeeper")
      .map((p) => {
        const fullData = allSquadPlayers.find((sp) => sp.id === p.id);
        const score = fullData ? getSetPieceStats(role, fullData).score : 0;
        return { id: p.id, name: p.name, score };
      })
      .sort((a, b) => b.score - a.score);
  };

  const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];
  const oppPositions = positions.filter((pos) =>
    oppTeam.players.some((p) => p.position === pos),
  );

  const renderSetPieces = () => (
    <div className="rounded-xl border border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800 p-4 shadow-sm transition-colors duration-300">
      <div className="flex items-center justify-between mb-2.5">
        <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
          {t("match.setPiecesCaptain")}
        </p>
        <button
          onClick={handleAutoSelectSetPieces}
          className="flex items-center gap-1.5 rounded-lg border border-accent-200 dark:border-accent-500/20 bg-accent-50 hover:bg-accent-100 dark:bg-accent-500/10 dark:hover:bg-accent-500/20 px-3 py-1.5 font-heading font-bold text-[10px] uppercase tracking-wider text-accent-700 dark:text-accent-400 transition-colors"
        >
          <Wand2 className="h-3 w-3" />
          {t("match.autoSelectTakers")}
        </button>
      </div>
      <div className="grid grid-cols-2 gap-3">
        {setPieceItems.map(({ role, label, Icon, current }) => (
          <div key={role}>
            <label className="mb-1.5 flex items-center gap-1 text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
              <Icon className="h-3 w-3" />
              {label}
            </label>
            <Select
              value={current ?? ""}
              onChange={(e) => handleSetPieceTaker(role, e.target.value)}
              selectSize="xs"
              fullWidth
              aria-label={label}
            >
              <option value="">—</option>
              {sortedForRole(role).map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </div>
        ))}
      </div>
    </div>
  );

  // YOUR TEAM tab: fixed 3-panel (subs+fit / pitch / set-pieces). The page never
  // scrolls — each panel scrolls internally.
  const renderTeamView = () => (
    <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-hidden p-4 xl:grid-cols-[300px_1fr_320px]">
      {/* Left: formation fit + auto-select + substitutes */}
      <div className="min-h-0 overflow-y-auto">
        <PreMatchLineup
          userTeam={userTeam}
          userBench={userBench}
          userColor={userColor}
          formationNeeds={formationNeeds}
          selectedStarterId={selectedStarterId}
          isAutoSelecting={isAutoSelecting}
          onSelectStarter={setSelectedStarterId}
          onSwap={handleSwap}
          onAutoSelect={handleAutoSelect}
          jerseyNumberById={jerseyNumberById}
          formationControls={
            <div className="flex gap-2">
              <Select
                value={
                  FORMATIONS.includes(userTeam.formation)
                    ? userTeam.formation
                    : FORMATIONS[0]
                }
                onChange={(e) => handleFormationChange(e.target.value)}
                selectSize="sm"
                fullWidth
                aria-label={t("tactics.formation")}
              >
                {FORMATIONS.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </Select>
              <Select
                value={userTeam.play_style}
                onChange={(e) => handlePlayStyleChange(e.target.value)}
                selectSize="sm"
                fullWidth
                aria-label={t("tactics.playStyle")}
              >
                {PLAY_STYLES.map((style) => (
                  <option key={style} value={style}>
                    {t(`common.playStyles.${style}`, style)}
                  </option>
                ))}
              </Select>
            </div>
          }
          showStartingList={false}
        />
      </div>
      {/* Center: the pitch — portrait aspect (SVG is 100x140) so it fills the
          column height without squishing, capped by available width. */}
      <div className="flex min-h-0 items-center justify-center overflow-hidden">
        <FormationPitch
          formation={userTeam.formation}
          players={userTeam.players}
          selectedId={selectedStarterId}
          onPlayerClick={(id) =>
            setSelectedStarterId(id === selectedStarterId ? null : id)
          }
          renderToken={(p, { isSelected, slotPosition }) =>
            renderUserToken(p, isSelected, slotPosition)
          }
          className="aspect-[5/7] h-full max-h-full w-auto max-w-full"
        />
      </div>
      {/* Right: set pieces + phase blueprint */}
      <div className="flex min-h-0 flex-col gap-4 overflow-y-auto">
        {renderSetPieces()}
        <div className="rounded-xl border border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800 shadow-sm transition-colors duration-300">
          <div className="border-b border-gray-100 dark:border-navy-700 px-3 py-2.5">
            <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
              {t("tactics.phaseBlueprint")}
            </p>
          </div>
          <PhaseBlueprintPanel
            tacticsPhase={phase}
            onTacticsPhaseChange={handlePhaseChange}
          />
        </div>
      </div>
    </div>
  );

  // OPPONENT tab: full-width scouting (their shape + squad scouting list).
  const renderOpponentView = () => (
    <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-hidden p-4 lg:grid-cols-2">
      {/* Left: opponent shape */}
      <div className="flex min-h-0 flex-col gap-2 overflow-hidden">
        <div>
          <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
            {oppTeam.name}
          </p>
          <p className="text-[10px] text-gray-500 dark:text-gray-400 font-heading mt-0.5">
            {oppTeam.formation} ·{" "}
            {t(`common.playStyles.${oppTeam.play_style}`, oppTeam.play_style)}
          </p>
        </div>
        <div className="flex min-h-0 flex-1 items-center justify-center overflow-hidden">
          <FormationPitch
            formation={oppTeam.formation}
            players={oppTeam.players}
            renderToken={(p, { slotPosition }) => renderOppToken(p, slotPosition)}
            className="aspect-[5/7] h-full max-h-full w-auto max-w-full"
          />
        </div>
      </div>
      {/* Right: scouting list (scrolls internally) */}
      <div className="min-h-0 overflow-y-auto pr-1">
        {oppPositions.map((pos) => {
          const players = oppTeam.players.filter((p) => p.position === pos);
          if (!players.length) return null;
          const keyStats = POSITION_KEY_STATS[pos] ?? [];
          return (
            <div key={pos} className="mb-3">
              <div className="flex items-center justify-between mb-1 px-2">
                <p className="text-[10px] font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                  {t(`common.positionGroups.${pos}`)}
                </p>
                <div className="flex items-center">
                  <span className="text-[8px] font-heading uppercase tracking-widest text-gray-400 dark:text-gray-600 w-7 text-center">
                    OVR
                  </span>
                  {keyStats.map((s) => (
                    <span
                      key={s.label}
                      className="text-[8px] font-heading uppercase tracking-widest text-gray-400 dark:text-gray-600 w-7 text-center"
                    >
                      {s.label}
                    </span>
                  ))}
                  <span className="text-[8px] font-heading uppercase tracking-widest text-gray-400 dark:text-gray-600 w-8 text-right">
                    COND
                  </span>
                </div>
              </div>
              {players.map((p) => (
                <div
                  key={p.id}
                  className="flex items-center gap-2 py-1.5 px-2 rounded hover:bg-gray-100 dark:hover:bg-navy-700/30 transition-colors"
                >
                  <div className="h-7 w-7 shrink-0 rounded-full bg-gray-200 dark:bg-navy-600 flex items-center justify-center text-[10px] font-heading font-bold text-gray-500 dark:text-gray-400 transition-colors duration-300">
                    {p.ovr}
                  </div>
                  <span className="flex-1 truncate text-sm text-gray-700 dark:text-gray-300">
                    {p.name}
                  </span>
                  <div className="flex items-center">
                    <span
                      className={`text-[10px] font-heading font-bold tabular-nums w-7 text-center ${starterOvrColor(p.ovr)}`}
                    >
                      {p.ovr}
                    </span>
                    {keyStats.map((s) => (
                      <span
                        key={s.label}
                        className={`text-[10px] font-heading tabular-nums w-7 text-center ${statColor(getStatVal(p, s.key))}`}
                      >
                        {getStatVal(p, s.key)}
                      </span>
                    ))}
                  </div>
                  <span
                    className={`text-xs tabular-nums w-8 text-right ${condColor(p.condition)}`}
                  >
                    {Math.round(p.condition)}%
                  </span>
                </div>
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );

  const setPieceItems = [
    {
      role: "captain",
      label: t("match.captain"),
      Icon: Crown,
      current: userSetPieces.captain,
    },
    {
      role: "penalty",
      label: t("match.penaltyTaker"),
      Icon: CircleDot,
      current: userSetPieces.penalty_taker,
    },
    {
      role: "freekick",
      label: t("match.freeKickTaker"),
      Icon: Footprints,
      current: userSetPieces.free_kick_taker,
    },
    {
      role: "corner",
      label: t("match.cornerTaker"),
      Icon: CornerDownRight,
      current: userSetPieces.corner_taker,
    },
  ];

  return (
    <div className="flex h-screen flex-col bg-gray-100 dark:bg-navy-900 text-gray-900 dark:text-white transition-colors duration-300">
      {/* Header */}
      <header className="shrink-0 border-b border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800 transition-colors duration-300">
        <div className="flex items-center gap-6 px-6 pt-5 pb-4">
          {/* Home team */}
          <div className="flex flex-1 items-center gap-4 min-w-0">
            <TeamLogo
              team={homeFullTeam ?? makeTeamFallback(homeTeam.name)}
              className="h-14 w-14 shrink-0 rounded-xl flex items-center justify-center font-heading font-bold text-lg overflow-hidden"
              imageClassName="h-11 w-11 object-contain drop-shadow"
              style={{
                backgroundColor: homeTeamColor + "30",
                borderColor: homeTeamColor,
                borderWidth: 2,
              }}
            />
            <div className="min-w-0">
              <p className="font-heading font-bold text-lg text-gray-900 dark:text-white truncate">
                {homeTeam.name}
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                {homeTeam.formation} ·{" "}
                {t(`common.playStyles.${homeTeam.play_style}`, homeTeam.play_style)}
              </p>
            </div>
          </div>

          {/* Center: fixture label + VS + Start */}
          <div className="shrink-0 flex flex-col items-center gap-3">
            <div className="text-center">
              <p className="text-[10px] font-heading uppercase tracking-widest text-accent-600 dark:text-accent-400">
                {fixtureLabel}
              </p>
              <p className="text-2xl font-heading font-bold text-gray-400 dark:text-gray-600">
                VS
              </p>
            </div>
            <button
              onClick={onStart}
              className="flex items-center gap-2 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 px-8 py-3 font-heading font-bold uppercase tracking-wider text-sm text-white shadow-lg shadow-primary-500/20 transition-all hover:from-primary-600 hover:to-primary-700 hover:scale-[1.02] active:scale-[0.98]"
            >
              {t("match.startMatch")}
              <ChevronRight className="h-4 w-4" />
            </button>
          </div>

          {/* Away team */}
          <div className="flex flex-1 items-center gap-4 flex-row-reverse min-w-0">
            <TeamLogo
              team={awayFullTeam ?? makeTeamFallback(awayTeam.name)}
              className="h-14 w-14 shrink-0 rounded-xl flex items-center justify-center font-heading font-bold text-lg overflow-hidden"
              imageClassName="h-11 w-11 object-contain drop-shadow"
              style={{
                backgroundColor: awayTeamColor + "30",
                borderColor: awayTeamColor,
                borderWidth: 2,
              }}
            />
            <div className="min-w-0 text-right">
              <p className="font-heading font-bold text-lg text-gray-900 dark:text-white truncate">
                {awayTeam.name}
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                {awayTeam.formation} ·{" "}
                {t(`common.playStyles.${awayTeam.play_style}`, awayTeam.play_style)}
              </p>
            </div>
          </div>

          <div className="shrink-0">
            <ThemeToggle />
          </div>
        </div>
      </header>

      {/* Your Team / Opponent tabs */}
      <div className="shrink-0 flex items-center gap-1 border-b border-gray-200 dark:border-navy-700 bg-gray-50/80 dark:bg-navy-800/50 px-4">
        {([
          { id: "team" as const, label: userTeam.name },
          { id: "opponent" as const, label: `${t("match.opponent")} · ${oppTeam.name}` },
        ]).map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setActiveTab(tab.id)}
            className={`-mb-px border-b-2 px-4 py-2.5 text-[11px] font-heading font-bold uppercase tracking-widest transition-colors ${
              activeTab === tab.id
                ? "border-primary-500 text-primary-600 dark:text-primary-400"
                : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {activeTab === "team" ? renderTeamView() : renderOpponentView()}
    </div>
  );
}
