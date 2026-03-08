import { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GameStateData, PlayerData } from "../store/gameStore";
import { Card, Badge, ProgressBar } from "./ui";
import { Star, ArrowRightLeft, Users, Shield, Crosshair, Zap, Target, RefreshCw, Flag, AlertTriangle, User, ShoppingCart, Repeat, GitCompareArrows, ChevronDown } from "lucide-react";
import { formatVal, positionBadgeVariant, calcOvr, calcAge } from "../lib/helpers";
import { TraitList } from "./TraitBadge";
import { useTranslation } from "react-i18next";
import ContextMenu from "./ContextMenu";

interface SquadTabProps {
  gameState: GameStateData;
  managerId: string;
  onSelectPlayer: (id: string) => void;
  onGameUpdate?: (g: GameStateData) => void;
}

const FORMATIONS = ["4-4-2", "4-3-3", "3-5-2", "4-5-1", "4-2-3-1", "3-4-3"];
const PLAY_STYLES = [
  { id: "Balanced", icon: <Target className="w-3.5 h-3.5" /> },
  { id: "Attacking", icon: <Zap className="w-3.5 h-3.5" /> },
  { id: "Defensive", icon: <Shield className="w-3.5 h-3.5" /> },
  { id: "Possession", icon: <RefreshCw className="w-3.5 h-3.5" /> },
  { id: "Counter", icon: <Crosshair className="w-3.5 h-3.5" /> },
  { id: "HighPress", icon: <Flag className="w-3.5 h-3.5" /> },
];

function parseFormationSlots(formation: string): { def: number; mid: number; fwd: number } {
  const parts = formation.split('-').map(Number);
  if (parts.length === 4) return { def: parts[0], mid: parts[1] + parts[2], fwd: parts[3] };
  if (parts.length === 3) return { def: parts[0], mid: parts[1], fwd: parts[2] };
  return { def: 4, mid: 4, fwd: 2 };
}

export default function SquadTab({ gameState, managerId, onSelectPlayer, onGameUpdate }: SquadTabProps) {
  const { t } = useTranslation();
  const myTeam = gameState.teams.find(t => t.manager_id === managerId);
  const [swapSource, setSwapSource] = useState<{ id: string; from: "xi" | "bench" } | null>(null);
  const [activeView, setActiveView] = useState<"lineup" | "roster" | "compare">("lineup");
  const [compareA, setCompareA] = useState<string | null>(null);
  const [compareB, setCompareB] = useState<string | null>(null);

  if (!myTeam) return <p className="text-gray-500 dark:text-gray-400">{t('common.unemployed')}</p>;

  const posOrder: Record<string, number> = { "Goalkeeper": 1, "Defender": 2, "Midfielder": 3, "Forward": 4 };
  const roster = gameState.players
    .filter(p => p.team_id === myTeam.id)
    .sort((a, b) => (posOrder[a.position] || 99) - (posOrder[b.position] || 99) || calcOvr(b) - calcOvr(a));

  const formation = myTeam.formation || "4-4-2";
  const slots = parseFormationSlots(formation);

  // Build starting XI from persistent IDs, fallback to auto-select
  const available = roster.filter(p => !p.injury);
  const startingXI = useMemo(() => {
    const savedIds = myTeam.starting_xi_ids || [];
    // Use saved lineup if it has valid entries
    if (savedIds.length > 0) {
      const validPlayers = savedIds
        .map(id => available.find(p => p.id === id))
        .filter((p): p is PlayerData => p != null);
      // If most of the saved lineup is still valid, use it (fill remaining slots)
      if (validPlayers.length >= 8) {
        const used = new Set(validPlayers.map(p => p.id));
        const remaining = available.filter(p => !used.has(p.id)).sort((a, b) => calcOvr(b) - calcOvr(a));
        const result = [...validPlayers];
        for (const p of remaining) {
          if (result.length >= 11) break;
          result.push(p);
        }
        return result.slice(0, 11);
      }
    }
    // Auto-select by position and OVR
    const xi: PlayerData[] = [];
    const used = new Set<string>();
    const pick = (pos: string, count: number) => {
      const candidates = available.filter(p => p.position === pos && !used.has(p.id)).sort((a, b) => calcOvr(b) - calcOvr(a));
      for (let i = 0; i < count && i < candidates.length; i++) {
        xi.push(candidates[i]);
        used.add(candidates[i].id);
      }
    };
    pick("Goalkeeper", 1);
    pick("Defender", slots.def);
    pick("Midfielder", slots.mid);
    pick("Forward", slots.fwd);
    return xi;
  }, [available.map(p => p.id).join(','), formation, (myTeam.starting_xi_ids || []).join(',')]);

  const xiIds = new Set(startingXI.map(p => p.id));
  const bench = roster.filter(p => !xiIds.has(p.id));

  const handleFormationChange = async (f: string) => {
    try {
      const updated = await invoke<GameStateData>("set_formation", { formation: f });
      onGameUpdate?.(updated);
    } catch (err) {
      console.error("Failed to set formation:", err);
    }
  };

  const handlePlayStyleChange = async (ps: string) => {
    try {
      const updated = await invoke<GameStateData>("set_play_style", { playStyle: ps });
      onGameUpdate?.(updated);
    } catch (err) {
      console.error("Failed to set play style:", err);
    }
  };

  const handleSwapClick = async (playerId: string, from: "xi" | "bench") => {
    if (swapSource) {
      if (swapSource.id === playerId) {
        setSwapSource(null);
        return;
      }
      // Perform the swap: compute new XI ids
      const currentXiIds = startingXI.map(p => p.id);
      let newXiIds: string[];
      if (swapSource.from === "xi" && from === "bench") {
        // Swap XI player out for bench player
        newXiIds = currentXiIds.map(id => id === swapSource.id ? playerId : id);
      } else if (swapSource.from === "bench" && from === "xi") {
        // Swap bench player in for XI player
        newXiIds = currentXiIds.map(id => id === playerId ? swapSource.id : id);
      } else if (swapSource.from === "xi" && from === "xi") {
        // Swap positions within XI
        const idx1 = currentXiIds.indexOf(swapSource.id);
        const idx2 = currentXiIds.indexOf(playerId);
        newXiIds = [...currentXiIds];
        newXiIds[idx1] = playerId;
        newXiIds[idx2] = swapSource.id;
      } else {
        setSwapSource(null);
        return;
      }
      try {
        const updated = await invoke<GameStateData>("set_starting_xi", { playerIds: newXiIds });
        onGameUpdate?.(updated);
      } catch (err) {
        console.error("Failed to set starting XI:", err);
      }
      setSwapSource(null);
    } else {
      setSwapSource({ id: playerId, from });
    }
  };

  // Determine which positions are "expected" for the XI based on formation
  const expectedPositionCounts: Record<string, number> = { Goalkeeper: 1, Defender: slots.def, Midfielder: slots.mid, Forward: slots.fwd };
  const xiPositionCounts: Record<string, number> = {};
  for (const p of startingXI) {
    xiPositionCounts[p.position] = (xiPositionCounts[p.position] || 0) + 1;
  }
  // A player is "out of position" if there are more of their position in XI than the formation needs
  const isOutOfPosition = (player: PlayerData, sec: "xi" | "bench"): boolean => {
    if (sec !== "xi") return false;
    const needed = expectedPositionCounts[player.position] || 0;
    const inXi = xiPositionCounts[player.position] || 0;
    // If we have more of this position than needed, the excess players are out of position
    if (inXi <= needed) return false;
    // Check if this player is one of the "excess" — sort by OVR, bottom ones are out of position
    const samePos = startingXI.filter(p => p.position === player.position).sort((a, b) => calcOvr(b) - calcOvr(a));
    const excessCount = inXi - needed;
    return samePos.slice(-excessCount).some(p => p.id === player.id);
  };

  const renderPlayerRow = (player: PlayerData, section: "xi" | "bench") => {
    const ovr = calcOvr(player);
    const age = calcAge(player.date_of_birth);
    const isSwapSource = swapSource?.id === player.id;
    const isSwapTarget = swapSource && swapSource.id !== player.id;
    const wrongPos = section === "xi" && isOutOfPosition(player, section);

    const contextItems = [
      { label: "View Profile", icon: <User className="w-4 h-4" />, onClick: () => onSelectPlayer(player.id) },
      { label: "Swap Player", icon: <ArrowRightLeft className="w-4 h-4" />, onClick: () => handleSwapClick(player.id, section), disabled: !!player.injury },
      { label: "", icon: undefined, onClick: () => {}, divider: true },
      { label: player.transfer_listed ? "Remove from Transfer List" : "Add to Transfer List", icon: <ShoppingCart className="w-4 h-4" />, onClick: async () => {
        try {
          const updated = await invoke<GameStateData>("toggle_transfer_list", { playerId: player.id });
          onGameUpdate?.(updated);
        } catch { /* command may not exist yet */ }
      }},
      { label: player.loan_listed ? "Remove from Loan List" : "Add to Loan List", icon: <Repeat className="w-4 h-4" />, onClick: async () => {
        try {
          const updated = await invoke<GameStateData>("toggle_loan_list", { playerId: player.id });
          onGameUpdate?.(updated);
        } catch { /* command may not exist yet */ }
      }},
    ];

    return (
      <ContextMenu items={contextItems} key={player.id}>
      <tr
        className={`transition-colors group ${isSwapSource ? 'bg-accent-500/10 dark:bg-accent-500/10' : isSwapTarget ? 'hover:bg-primary-500/10 dark:hover:bg-primary-500/10 cursor-pointer' : wrongPos ? 'bg-amber-500/5 dark:bg-amber-500/5' : 'hover:bg-gray-50 dark:hover:bg-navy-700/50'}`}
      >
        <td className="py-2.5 px-4">
          <div className="flex items-center gap-1">
            <Badge variant={positionBadgeVariant(player.position)} size="sm">
              {player.position.substring(0, 3).toUpperCase()}
            </Badge>
            {wrongPos && (
              <span title="Playing out of natural position" className="text-amber-500">
                <AlertTriangle className="w-3.5 h-3.5" />
              </span>
            )}
          </div>
        </td>
        <td className="py-2.5 px-4">
          <button onClick={() => onSelectPlayer(player.id)} className="text-left">
            <div className="font-semibold text-sm text-gray-900 dark:text-gray-100 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">{player.full_name}</div>
            <div className="text-xs text-gray-400 dark:text-gray-500">{player.nationality}</div>
          </button>
        </td>
        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">{age}</td>
        <td className="py-2.5 px-4 w-28">
          <ProgressBar value={player.condition} variant="auto" size="sm" showLabel />
        </td>
        <td className="py-2.5 px-4 text-sm text-gray-500 dark:text-gray-400 tabular-nums">{player.morale}</td>
        <td className="py-2.5 px-4">
          {player.traits && player.traits.length > 0 ? (
            <TraitList traits={player.traits} size="xs" max={2} />
          ) : (
            <span className="text-xs text-gray-500">—</span>
          )}
        </td>
        <td className="py-2.5 px-4">
          <span className={`font-heading font-bold text-base tabular-nums ${
            ovr >= 75 ? 'text-success-500 dark:text-success-400' :
            ovr >= 55 ? 'text-accent-600 dark:text-accent-400' :
            'text-gray-500 dark:text-gray-400'
          }`}>{ovr}</span>
        </td>
        <td className="py-2.5 px-4">
          {player.injury ? (
            <Badge variant="danger" size="sm">{t('common.injured')}</Badge>
          ) : (
            <button
              onClick={() => handleSwapClick(player.id, section)}
              className={`p-1.5 rounded-lg transition-colors ${isSwapSource ? 'bg-accent-500 text-white' : 'text-gray-400 hover:text-primary-500 hover:bg-gray-100 dark:hover:bg-navy-600'}`}
              title="Swap player"
            >
              <ArrowRightLeft className="w-3.5 h-3.5" />
            </button>
          )}
        </td>
      </tr>
      </ContextMenu>
    );
  };

  const tableHead = (
    <thead>
      <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('squad.pos')}</th>
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.name')}</th>
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.age')}</th>
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.condition')}</th>
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.morale')}</th>
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('squad.traits')}</th>
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.ovr')}</th>
        <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 w-12"></th>
      </tr>
    </thead>
  );

  return (
    <div className="max-w-6xl mx-auto flex flex-col gap-4">
      {/* Header with Formation & Play Style */}
      <div className="flex flex-wrap gap-4">
        {/* Formation Card */}
        <Card className="flex-1 min-w-[280px]">
          <div className="p-4">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">{t('tactics.formation')}</h3>
            <div className="flex flex-wrap gap-1.5">
              {FORMATIONS.map(f => (
                <button
                  key={f}
                  onClick={() => handleFormationChange(f)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold transition-all ${
                    formation === f
                      ? "bg-primary-500 text-white shadow-sm"
                      : "bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600"
                  }`}
                >
                  {f}
                </button>
              ))}
            </div>
          </div>
        </Card>

        {/* Play Style Card */}
        <Card className="flex-1 min-w-[280px]">
          <div className="p-4">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">{t('tactics.playStyle')}</h3>
            <div className="flex flex-wrap gap-1.5">
              {PLAY_STYLES.map(s => (
                <button
                  key={s.id}
                  onClick={() => handlePlayStyleChange(s.id)}
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-heading font-bold transition-all ${
                    myTeam.play_style === s.id
                      ? "bg-primary-500 text-white shadow-sm"
                      : "bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600"
                  }`}
                >
                  {s.icon}
                  {t(`common.playStyles.${s.id}`)}
                </button>
              ))}
            </div>
          </div>
        </Card>
      </div>

      {/* View Tabs */}
      <div className="flex gap-1 bg-gray-100 dark:bg-navy-800 rounded-lg p-1 w-fit">
        <button
          onClick={() => setActiveView("lineup")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${
            activeView === "lineup" ? "bg-white dark:bg-navy-600 text-primary-600 dark:text-primary-400 shadow-sm" : "text-gray-500 dark:text-gray-400"
          }`}
        >
          <Star className="w-4 h-4" /> {t('preMatch.startingXI', 'Starting XI')}
        </button>
        <button
          onClick={() => setActiveView("roster")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${
            activeView === "roster" ? "bg-white dark:bg-navy-600 text-primary-600 dark:text-primary-400 shadow-sm" : "text-gray-500 dark:text-gray-400"
          }`}
        >
          <Users className="w-4 h-4" /> {t('squad.fullRoster', 'Full Roster')}
        </button>
        <button
          onClick={() => setActiveView("compare")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${
            activeView === "compare" ? "bg-white dark:bg-navy-600 text-primary-600 dark:text-primary-400 shadow-sm" : "text-gray-500 dark:text-gray-400"
          }`}
        >
          <GitCompareArrows className="w-4 h-4" /> {t('squad.compare', 'Compare')}
        </button>
      </div>

      {activeView === "compare" ? (
        <CompareView
          roster={roster}
          compareA={compareA}
          compareB={compareB}
          setCompareA={setCompareA}
          setCompareB={setCompareB}
        />
      ) : activeView === "lineup" ? (
        <>
          {/* Starting XI */}
          <Card>
            <div className="p-4 border-b border-gray-100 dark:border-navy-600 flex justify-between items-center bg-gradient-to-r from-navy-700 to-navy-800 rounded-t-xl">
              <div>
                <h3 className="text-sm font-heading font-bold text-white uppercase tracking-wide flex items-center gap-2">
                  <Star className="text-accent-400 w-4 h-4 fill-current" />
                  {t('preMatch.startingXI', 'Starting XI')} — {formation}
                </h3>
                <p className="text-xs text-gray-400 mt-0.5">{startingXI.length} / 11 {t('squad.selected', 'selected')}</p>
              </div>
              {swapSource && (
                <button onClick={() => setSwapSource(null)} className="text-xs text-accent-400 font-heading font-bold uppercase tracking-wider hover:text-accent-300">
                  {t('common.cancel')}
                </button>
              )}
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse">
                {tableHead}
                <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                  {startingXI.map(p => renderPlayerRow(p, "xi"))}
                </tbody>
              </table>
            </div>
          </Card>

          {/* Bench */}
          <Card>
            <div className="p-4 border-b border-gray-100 dark:border-navy-600">
              <h3 className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200 uppercase tracking-wide flex items-center gap-2">
                {t('preMatch.substitutes', 'Substitutes')}
              </h3>
              <p className="text-xs text-gray-400 mt-0.5">{bench.length} {t('squad.playersLabel', 'players')}</p>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse">
                {tableHead}
                <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                  {bench.map(p => renderPlayerRow(p, "bench"))}
                </tbody>
              </table>
              {bench.length === 0 && (
                <div className="p-6 text-center text-gray-500 dark:text-gray-400 text-sm">{t('squad.noBench', 'No bench players.')}</div>
              )}
            </div>
          </Card>
        </>
      ) : (
        /* Full Roster view */
        <Card>
          <div className="p-4 border-b border-gray-100 dark:border-navy-600 bg-gradient-to-r from-navy-700 to-navy-800 rounded-t-xl">
            <h3 className="text-sm font-heading font-bold text-white uppercase tracking-wide flex items-center gap-2">
              <Users className="w-4 h-4 text-accent-400" />
              {t('squad.title', { team: myTeam.name })}
            </h3>
            <p className="text-xs text-gray-400 mt-0.5">{t('squad.playerCount', { count: roster.length })}</p>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('squad.pos')}</th>
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.name')}</th>
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.age')}</th>
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.condition')}</th>
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.morale')}</th>
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('squad.traits')}</th>
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.value')}</th>
                  <th className="py-2.5 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.ovr')}</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {roster.map(player => {
                  const ovr = calcOvr(player);
                  const age = calcAge(player.date_of_birth);
                  const inXI = xiIds.has(player.id);
                  return (
                    <tr key={player.id} onClick={() => onSelectPlayer(player.id)} className="hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors group cursor-pointer">
                      <td className="py-2.5 px-4">
                        <div className="flex items-center gap-1.5">
                          {inXI && <span className="w-1.5 h-1.5 rounded-full bg-primary-500" title="Starting XI" />}
                          <Badge variant={positionBadgeVariant(player.position)} size="sm">
                            {player.position.substring(0, 3).toUpperCase()}
                          </Badge>
                        </div>
                      </td>
                      <td className="py-2.5 px-4">
                        <div className="font-semibold text-sm text-gray-900 dark:text-gray-100 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">{player.full_name}</div>
                        <div className="text-xs text-gray-400 dark:text-gray-500">{player.nationality}</div>
                      </td>
                      <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">{age}</td>
                      <td className="py-2.5 px-4 w-28">
                        <ProgressBar value={player.condition} variant="auto" size="sm" showLabel />
                      </td>
                      <td className="py-2.5 px-4 text-sm text-gray-500 dark:text-gray-400 tabular-nums">{player.morale}</td>
                      <td className="py-2.5 px-4">
                        {player.traits && player.traits.length > 0 ? (
                          <TraitList traits={player.traits} size="xs" max={2} />
                        ) : (
                          <span className="text-xs text-gray-500">—</span>
                        )}
                      </td>
                      <td className="py-2.5 px-4 text-xs text-gray-600 dark:text-gray-400 font-medium">{formatVal(player.market_value)}</td>
                      <td className="py-2.5 px-4">
                        <span className={`font-heading font-bold text-base tabular-nums ${
                          ovr >= 75 ? 'text-success-500 dark:text-success-400' :
                          ovr >= 55 ? 'text-accent-600 dark:text-accent-400' :
                          'text-gray-500 dark:text-gray-400'
                        }`}>{ovr}</span>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            {roster.length === 0 && (
              <div className="p-8 text-center text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider text-sm">{t('squad.noPlayers')}</div>
            )}
          </div>
        </Card>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Player Comparison View
// ---------------------------------------------------------------------------

const ATTR_GROUPS: { label: string; attrs: { key: keyof PlayerData["attributes"]; label: string }[] }[] = [
  {
    label: "Physical",
    attrs: [
      { key: "pace", label: "Pace" },
      { key: "stamina", label: "Stamina" },
      { key: "strength", label: "Strength" },
      { key: "agility", label: "Agility" },
    ],
  },
  {
    label: "Technical",
    attrs: [
      { key: "passing", label: "Passing" },
      { key: "shooting", label: "Shooting" },
      { key: "dribbling", label: "Dribbling" },
      { key: "tackling", label: "Tackling" },
    ],
  },
  {
    label: "Mental",
    attrs: [
      { key: "positioning", label: "Positioning" },
      { key: "vision", label: "Vision" },
      { key: "decisions", label: "Decisions" },
      { key: "composure", label: "Composure" },
      { key: "teamwork", label: "Teamwork" },
      { key: "leadership", label: "Leadership" },
      { key: "aggression", label: "Aggression" },
    ],
  },
  {
    label: "Goalkeeping",
    attrs: [
      { key: "handling", label: "Handling" },
      { key: "reflexes", label: "Reflexes" },
      { key: "aerial", label: "Aerial" },
    ],
  },
];

function CompareView({
  roster, compareA, compareB, setCompareA, setCompareB,
}: {
  roster: PlayerData[];
  compareA: string | null;
  compareB: string | null;
  setCompareA: (id: string | null) => void;
  setCompareB: (id: string | null) => void;
}) {
  const { t } = useTranslation();
  const playerA = roster.find(p => p.id === compareA) || null;
  const playerB = roster.find(p => p.id === compareB) || null;

  const renderSelector = (value: string | null, onChange: (id: string | null) => void, otherId: string | null) => (
    <div className="relative flex-1">
      <select
        value={value || ""}
        onChange={e => onChange(e.target.value || null)}
        className="appearance-none w-full pl-3 pr-10 py-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:ring-2 focus:ring-primary-500/50 transition-all font-heading font-bold"
      >
        <option value="">{t('squadCompare.selectPlayerA')}...</option>
        {roster.filter(p => p.id !== otherId).map(p => (
          <option key={p.id} value={p.id}>
            {p.full_name} ({p.position.substring(0, 3)}, OVR {calcOvr(p)})
          </option>
        ))}
      </select>
      <ChevronDown className="w-4 h-4 absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500 pointer-events-none" />
    </div>
  );

  const attrColor = (val: number) =>
    val >= 80 ? "text-success-500" : val >= 65 ? "text-primary-500" : val >= 50 ? "text-accent-500" : "text-gray-400";

  const barColor = (val: number) =>
    val >= 80 ? "bg-success-500" : val >= 65 ? "bg-primary-500" : val >= 50 ? "bg-accent-500" : "bg-gray-300 dark:bg-navy-600";

  const betterClass = "ring-2 ring-primary-500/30 bg-primary-500/5";

  return (
    <Card>
      <div className="p-4 border-b border-gray-100 dark:border-navy-600 bg-gradient-to-r from-navy-700 to-navy-800 rounded-t-xl">
        <h3 className="text-sm font-heading font-bold text-white uppercase tracking-wide flex items-center gap-2">
          <GitCompareArrows className="w-4 h-4 text-accent-400" />
          {t('squadCompare.compare')}
        </h3>
      </div>
      <div className="p-4">
        {/* Player selectors */}
        <div className="grid grid-cols-2 gap-4 mb-6">
          <div>
            <label className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-400 mb-1.5 block">{t('squadCompare.selectPlayerA')}</label>
            {renderSelector(compareA, setCompareA, compareB)}
          </div>
          <div>
            <label className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-400 mb-1.5 block">{t('squadCompare.selectPlayerB')}</label>
            {renderSelector(compareB, setCompareB, compareA)}
          </div>
        </div>

        {playerA && playerB ? (
          <>
            {/* Summary header */}
            <div className="grid grid-cols-[1fr_auto_1fr] gap-4 mb-6 items-center">
              <div className="text-center">
                <p className="font-heading font-bold text-gray-900 dark:text-white">{playerA.full_name}</p>
                <div className="flex items-center justify-center gap-2 mt-1">
                  <Badge variant={positionBadgeVariant(playerA.position)} size="sm">{playerA.position.substring(0, 3).toUpperCase()}</Badge>
                  <span className="text-xs text-gray-500">{calcAge(playerA.date_of_birth)} yrs</span>
                  <span className="font-heading font-bold text-lg text-primary-500">{calcOvr(playerA)}</span>
                </div>
              </div>
              <div className="text-gray-300 dark:text-navy-600 text-2xl font-heading font-bold">VS</div>
              <div className="text-center">
                <p className="font-heading font-bold text-gray-900 dark:text-white">{playerB.full_name}</p>
                <div className="flex items-center justify-center gap-2 mt-1">
                  <Badge variant={positionBadgeVariant(playerB.position)} size="sm">{playerB.position.substring(0, 3).toUpperCase()}</Badge>
                  <span className="text-xs text-gray-500">{calcAge(playerB.date_of_birth)} yrs</span>
                  <span className="font-heading font-bold text-lg text-primary-500">{calcOvr(playerB)}</span>
                </div>
              </div>
            </div>

            {/* Attribute groups */}
            <div className="flex flex-col gap-5">
              {ATTR_GROUPS.map(group => {
                const isGK = group.label === "Goalkeeping";
                const aIsGK = playerA.position === "Goalkeeper";
                const bIsGK = playerB.position === "Goalkeeper";
                if (isGK && !aIsGK && !bIsGK) return null;

                return (
                  <div key={group.label}>
                    <h4 className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-400 dark:text-gray-500 mb-2">{group.label}</h4>
                    <div className="flex flex-col gap-1.5">
                      {group.attrs.map(attr => {
                        const valA = playerA.attributes[attr.key];
                        const valB = playerB.attributes[attr.key];
                        const aWins = valA > valB;
                        const bWins = valB > valA;
                        return (
                          <div key={attr.key} className="grid grid-cols-[1fr_100px_1fr] gap-2 items-center">
                            {/* Player A bar (right-aligned) */}
                            <div className={`flex items-center justify-end gap-2 px-2 py-1 rounded-lg ${aWins ? betterClass : ""}`}>
                              <span className={`text-xs font-heading font-bold tabular-nums ${attrColor(valA)}`}>{valA}</span>
                              <div className="w-24 h-2 rounded-full bg-gray-100 dark:bg-navy-700 overflow-hidden flex justify-end">
                                <div className={`h-full rounded-full ${barColor(valA)}`} style={{ width: `${valA}%` }} />
                              </div>
                            </div>
                            {/* Label center */}
                            <div className="text-center text-[10px] font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                              {attr.label}
                            </div>
                            {/* Player B bar (left-aligned) */}
                            <div className={`flex items-center gap-2 px-2 py-1 rounded-lg ${bWins ? betterClass : ""}`}>
                              <div className="w-24 h-2 rounded-full bg-gray-100 dark:bg-navy-700 overflow-hidden">
                                <div className={`h-full rounded-full ${barColor(valB)}`} style={{ width: `${valB}%` }} />
                              </div>
                              <span className={`text-xs font-heading font-bold tabular-nums ${attrColor(valB)}`}>{valB}</span>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Stats comparison */}
            <div className="mt-6 pt-4 border-t border-gray-100 dark:border-navy-700">
              <h4 className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-400 dark:text-gray-500 mb-3">{t('squadCompare.seasonStats')}</h4>
              <div className="grid grid-cols-[1fr_100px_1fr] gap-2 text-xs">
                {([
                  ["appearances", "Apps"],
                  ["goals", "Goals"],
                  ["assists", "Assists"],
                  ["yellow_cards", "Yellows"],
                  ["red_cards", "Reds"],
                ] as [keyof PlayerData["stats"], string][]).map(([key, label]) => {
                  const vA = playerA.stats[key] as number;
                  const vB = playerB.stats[key] as number;
                  return (
                    <div key={key} className="contents">
                      <div className="text-right font-heading font-bold text-gray-700 dark:text-gray-300">{vA}</div>
                      <div className="text-center text-[10px] font-heading font-bold uppercase tracking-wider text-gray-400">{label}</div>
                      <div className="text-left font-heading font-bold text-gray-700 dark:text-gray-300">{vB}</div>
                    </div>
                  );
                })}
              </div>
            </div>
          </>
        ) : (
          <div className="text-center py-12">
            <GitCompareArrows className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
            <p className="text-sm text-gray-500 dark:text-gray-400">{t('squadCompare.noPlayersSelected')}</p>
          </div>
        )}
      </div>
    </Card>
  );
}
