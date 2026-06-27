import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { FixtureData, GameStateData } from "../../store/gameStore";
import { getFixtureDisplayLabel } from "../../lib/helpers";
import { MatchSnapshot, EnginePlayerData, FORMATIONS, PLAY_STYLES } from "./types";
import PreMatchLineup, { parseFormationNeeds, POSITION_KEY_STATS, condColor, statColor, starterOvrColor, getStatVal } from "./PreMatchLineup";
import { getSetPieceStats } from "./SetPieceSelector";
import { FormationPitch } from "./FormationPitch";
import { makeTeamFallback } from "./helpers";
import { normalisePosition, translatePositionAbbreviation } from "../squad/SquadTab.helpers";
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

  // Index the full squad so pitch tokens can be enriched with face/jersey/natural
  // position that the lightweight match snapshot player doesn't carry.
  const storeById = useMemo(
    () => new Map(gameState.players.map((p) => [p.id, p])),
    [gameState.players],
  );

  // Rich token for the user's command pitch (avatar, kit, OVR, fit ring).
  const renderUserToken = (player: EnginePlayerData, isSelected: boolean) => {
    const sp = storeById.get(player.id);
    const fit: PitchFitTone = !sp
      ? "exact"
      : normalisePosition(sp.natural_position || sp.position) === player.position
        ? "exact"
        : "out";
    return (
      <div
        className={`w-16 rounded-xl px-1 py-1 ${
          isSelected ? "bg-accent-500/25 ring-2 ring-accent-300/70" : ""
        }`}
      >
        <PitchToken
          name={(sp?.match_name || player.name).toUpperCase()}
          positionAbbr={translatePositionAbbreviation(t, player.position)}
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

  // Header shows Home left / Away right; this decides which side gets the
  // editable formation & play-style selects (the user's side).
  const leftIsUser = userSide === "Home";

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

  const renderUserColumn = () => (
    <div className="flex min-w-0 flex-[2] flex-col overflow-y-auto border-r border-gray-200 dark:border-navy-700">
      <div className="shrink-0 border-b border-gray-200 dark:border-navy-700 bg-gray-50/80 dark:bg-navy-800/50 px-4 py-2">
        <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-primary-600 dark:text-primary-400">
          {userTeam.name}
        </p>
      </div>
      <div className="flex flex-col gap-4 p-4">
        <FormationPitch
          formation={userTeam.formation}
          players={userTeam.players}
          selectedId={selectedStarterId}
          onPlayerClick={(id) =>
            setSelectedStarterId(id === selectedStarterId ? null : id)
          }
          renderToken={(p, { isSelected }) => renderUserToken(p, isSelected)}
          className="h-[420px]"
        />
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
          showStartingList={false}
        />
        {renderSetPieces()}
      </div>
    </div>
  );

  const renderOpponentColumn = () => (
    <div className="flex min-w-0 flex-1 flex-col overflow-y-auto">
      <div className="shrink-0 border-b border-gray-200 dark:border-navy-700 bg-gray-50/80 dark:bg-navy-800/50 px-4 py-2">
        <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
          {t("match.opponent")} · {oppTeam.name}
        </p>
        <p className="text-[10px] text-gray-500 dark:text-gray-400 font-heading mt-0.5">
          {oppTeam.formation} ·{" "}
          {t(`common.playStyles.${oppTeam.play_style}`, oppTeam.play_style)}
        </p>
      </div>
      <div className="shrink-0 px-4 pt-4 pb-2">
        <FormationPitch
          formation={oppTeam.formation}
          players={oppTeam.players}
          className="h-[200px]"
        />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
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
                    FIT
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
              {leftIsUser ? (
                <div className="flex items-center gap-2 mt-1.5">
                  <Select
                    value={
                      FORMATIONS.includes(userTeam.formation)
                        ? userTeam.formation
                        : FORMATIONS[0]
                    }
                    onChange={(e) => handleFormationChange(e.target.value)}
                    selectSize="xs"
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
                    selectSize="xs"
                    aria-label={t("tactics.playStyle")}
                  >
                    {PLAY_STYLES.map((style) => (
                      <option key={style} value={style}>
                        {t(`common.playStyles.${style}`, style)}
                      </option>
                    ))}
                  </Select>
                </div>
              ) : (
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  {homeTeam.formation} ·{" "}
                  {t(`common.playStyles.${homeTeam.play_style}`, homeTeam.play_style)}
                </p>
              )}
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
              {!leftIsUser ? (
                <div className="flex items-center gap-2 mt-1.5 justify-end">
                  <Select
                    value={
                      FORMATIONS.includes(userTeam.formation)
                        ? userTeam.formation
                        : FORMATIONS[0]
                    }
                    onChange={(e) => handleFormationChange(e.target.value)}
                    selectSize="xs"
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
                    selectSize="xs"
                    aria-label={t("tactics.playStyle")}
                  >
                    {PLAY_STYLES.map((style) => (
                      <option key={style} value={style}>
                        {t(`common.playStyles.${style}`, style)}
                      </option>
                    ))}
                  </Select>
                </div>
              ) : (
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  {awayTeam.formation} ·{" "}
                  {t(`common.playStyles.${awayTeam.play_style}`, awayTeam.play_style)}
                </p>
              )}
            </div>
          </div>

          <div className="shrink-0">
            <ThemeToggle />
          </div>
        </div>
      </header>

      {/* Command (your team) + scout rail (opponent) */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {renderUserColumn()}
        {renderOpponentColumn()}
      </div>
    </div>
  );
}
