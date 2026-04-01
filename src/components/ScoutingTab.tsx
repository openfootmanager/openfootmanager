import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { GameStateData } from "../store/gameStore";
import { Card, CardHeader, CardBody, Badge, ProgressBar, CountryFlag } from "./ui";
import { Eye, ScanSearch, Clock, User, Search, ChevronLeft, ChevronRight } from "lucide-react";
import { calcOvr, calcAge, formatVal } from "../lib/helpers";
import { countryName } from "../lib/countries";

interface ScoutingTabProps {
  gameState: GameStateData;
  onGameUpdate: (state: GameStateData) => void;
  onSelectPlayer?: (id: string) => void;
}

const SCOUTING_PAGE_SIZE = 20;

export default function ScoutingTab({ gameState, onGameUpdate, onSelectPlayer }: ScoutingTabProps) {
  const { t, i18n } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [posFilter, setPosFilter] = useState<string>("All");
  const [sending, setSending] = useState<string | null>(null);
  const [page, setPage] = useState(0);

  const myTeamId = gameState.manager.team_id;
  const scouts = useMemo(
    () => gameState.staff.filter(s => s.role === "Scout" && s.team_id === myTeamId),
    [gameState.staff, myTeamId]
  );
  const assignments = gameState.scouting_assignments || [];
  const playersById = useMemo(
    () => new Map(gameState.players.map(player => [player.id, player])),
    [gameState.players]
  );
  const scoutsById = useMemo(
    () => new Map(gameState.staff.map(staff => [staff.id, staff])),
    [gameState.staff]
  );
  const teamNameById = useMemo(
    () => new Map(gameState.teams.map(team => [team.id, team.name])),
    [gameState.teams]
  );
  const normalizedSearchQuery = searchQuery.trim().toLowerCase();

  // Determine scout capacity: judging_ability >= 80 → 5 slots, >= 60 → 4, >= 40 → 3, >= 20 → 2, else 1
  const scoutMaxSlots = (ability: number) => ability >= 80 ? 5 : ability >= 60 ? 4 : ability >= 40 ? 3 : ability >= 20 ? 2 : 1;
  const assignmentCountByScout = useMemo(() => {
    const counts = new Map<string, number>();
    assignments.forEach(assignment => {
      counts.set(assignment.scout_id, (counts.get(assignment.scout_id) || 0) + 1);
    });
    return counts;
  }, [assignments]);
  const assignmentsByScout = useMemo(() => {
    const grouped = new Map<string, typeof assignments>();
    assignments.forEach(assignment => {
      const current = grouped.get(assignment.scout_id);
      if (current) {
        current.push(assignment);
      } else {
        grouped.set(assignment.scout_id, [assignment]);
      }
    });
    return grouped;
  }, [assignments]);
  const availableScouts = useMemo(
    () => scouts.filter(scout => (assignmentCountByScout.get(scout.id) || 0) < scoutMaxSlots(scout.attributes.judging_ability)),
    [assignmentCountByScout, scouts]
  );

  const alreadyScoutingIds = useMemo(
    () => new Set(assignments.map(assignment => assignment.player_id)),
    [assignments]
  );

  // Players from other teams that can be scouted
  const allScoutable = useMemo(() => (
    gameState.players
      .filter(player => player.team_id !== myTeamId)
      .filter(player => posFilter === "All" || player.position === posFilter)
      .filter(player => {
        if (!normalizedSearchQuery) return true;
        return player.full_name.toLowerCase().includes(normalizedSearchQuery) ||
          player.nationality.toLowerCase().includes(normalizedSearchQuery) ||
          ((player.team_id && teamNameById.get(player.team_id)) || "").toLowerCase().includes(normalizedSearchQuery);
      })
      .sort((a, b) => calcOvr(b) - calcOvr(a))
  ), [gameState.players, myTeamId, normalizedSearchQuery, posFilter, teamNameById]);

  const totalPages = Math.max(1, Math.ceil(allScoutable.length / SCOUTING_PAGE_SIZE));
  const safePage = Math.min(page, totalPages - 1);
  const scoutablePlayers = allScoutable.slice(safePage * SCOUTING_PAGE_SIZE, (safePage + 1) * SCOUTING_PAGE_SIZE);

  const handleSendScout = async (playerId: string) => {
    if (availableScouts.length === 0) return;
    const scout = availableScouts[0];
    setSending(playerId);
    try {
      const updated = await invoke<GameStateData>("send_scout", { scoutId: scout.id, playerId });
      onGameUpdate(updated);
    } catch (err) {
      console.error("Failed to send scout:", err);
    } finally {
      setSending(null);
    }
  };

  return (
    <div className="max-w-6xl mx-auto flex flex-col gap-5">
      {/* Header */}
      <div className="flex items-center gap-3">
        <ScanSearch className="w-5 h-5 text-primary-500" />
        <h2 className="text-lg font-heading font-bold uppercase tracking-wider text-gray-800 dark:text-gray-100">{t('scouting.title')}</h2>
      </div>

      {/* Scout Staff Overview */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardBody>
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-accent-500/10 flex items-center justify-center">
                <Eye className="w-5 h-5 text-accent-500" />
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">{t('scouting.scouts')}</p>
                <p className="text-xl font-heading font-bold text-gray-800 dark:text-gray-100">{scouts.length}</p>
              </div>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-primary-500/10 flex items-center justify-center">
                <Clock className="w-5 h-5 text-primary-500" />
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">{t('scouting.activeAssignments')}</p>
                <p className="text-xl font-heading font-bold text-gray-800 dark:text-gray-100">{assignments.length} / {scouts.reduce((sum, scout) => sum + scoutMaxSlots(scout.attributes.judging_ability), 0)}</p>
              </div>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-green-500/10 flex items-center justify-center">
                <User className="w-5 h-5 text-green-500" />
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">{t('scouting.freeSlots')}</p>
                <p className="text-xl font-heading font-bold text-gray-800 dark:text-gray-100">{availableScouts.length}</p>
              </div>
            </div>
          </CardBody>
        </Card>
      </div>

      {/* Active Assignments */}
      {assignments.length > 0 && (
        <Card>
          <CardHeader>{t('scouting.activeScoutingAssignments')}</CardHeader>
          <CardBody>
            <div className="flex flex-col gap-2">
              {assignments.map(a => {
                const scout = scoutsById.get(a.scout_id);
                const player = playersById.get(a.player_id);
                if (!scout || !player) return null;
                const team = player.team_id ? (teamNameById.get(player.team_id) || t('common.freeAgent')) : t('common.freeAgent');
                return (
                  <div key={a.id} className="flex items-center gap-4 p-3 rounded-lg bg-gray-50 dark:bg-navy-700/50">
                    <div className="flex-1 min-w-0">
                      <button onClick={() => onSelectPlayer?.(player.id)} className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100 hover:text-primary-500 transition-colors truncate block">
                        {player.full_name}
                      </button>
                      <p className="text-xs text-gray-500 dark:text-gray-400">{player.natural_position || player.position} · {team}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-xs text-gray-500 dark:text-gray-400">{t('scouting.scoutLabel', { name: `${scout.first_name} ${scout.last_name}` })}</p>
                      <div className="flex items-center gap-1.5 justify-end mt-0.5">
                        <Clock className="w-3 h-3 text-accent-500" />
                        <span className="text-xs font-heading font-bold text-accent-500">{t('scouting.daysLeft', { days: a.days_remaining })}</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </CardBody>
        </Card>
      )}

      {/* Scout Staff Details */}
      {scouts.length > 0 && (
        <Card>
          <CardHeader>{t('scouting.yourScouts')}</CardHeader>
          <CardBody>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {scouts.map(s => {
                const count = assignmentCountByScout.get(s.id) || 0;
                const maxSlots = scoutMaxSlots(s.attributes.judging_ability);
                const isFull = count >= maxSlots;
                const scoutAssigns = assignmentsByScout.get(s.id) || [];
                return (
                  <div key={s.id} className="p-3 rounded-lg border border-gray-200 dark:border-navy-600">
                    <div className="flex items-center gap-3">
                      <div className="w-9 h-9 rounded-lg bg-accent-500/10 flex items-center justify-center">
                        <Eye className="w-4 h-4 text-accent-500" />
                      </div>
                      <div className="flex-1">
                        <p className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100">{s.first_name} {s.last_name}</p>
                        <div className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5 flex items-center gap-1">
                          <CountryFlag code={s.nationality} locale={i18n.language} className="text-xs leading-none" />
                          <span>{countryName(s.nationality, i18n.language)}</span>
                        </div>
                      </div>
                      <Badge variant={isFull ? "accent" : "success"} size="sm">
                        {t('scouting.slotUsage', { count, max: maxSlots })}
                      </Badge>
                    </div>
                    <div className="mt-2 grid grid-cols-2 gap-2">
                      <div>
                        <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase">{t('scouting.judgingAbility')}</p>
                        <ProgressBar value={s.attributes.judging_ability} variant="auto" size="sm" />
                      </div>
                      <div>
                        <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase">{t('scouting.judgingPotential')}</p>
                        <ProgressBar value={s.attributes.judging_potential} variant="auto" size="sm" />
                      </div>
                    </div>
                    {scoutAssigns.length > 0 && (
                      <div className="mt-2 flex flex-col gap-1">
                        {scoutAssigns.map(a => {
                          const tp = playersById.get(a.player_id);
                          return tp ? (
                            <p key={a.id} className="text-xs text-gray-500 dark:text-gray-400">
                              {t('scouting.scoutLabel', { name: '' })}<span className="font-heading font-bold text-gray-700 dark:text-gray-300">{tp.full_name}</span> — {a.days_remaining}d
                            </p>
                          ) : null;
                        })}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </CardBody>
        </Card>
      )}

      {scouts.length === 0 && (
        <Card>
          <CardBody>
            <div className="flex flex-col items-center gap-3 py-8">
              <Eye className="w-10 h-10 text-gray-300 dark:text-navy-600" />
              <p className="text-sm text-gray-500 dark:text-gray-400 text-center">
                {t('scouting.noScouts')}<br />
                <span className="text-xs">{t('scouting.noScoutsHint')}</span>
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      {/* Player Search for Scouting */}
      {scouts.length > 0 && (
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3 w-full">
              <span>{t('scouting.findPlayers')}</span>
              <div className="ml-auto flex items-center gap-2">
                {["All", "Goalkeeper", "Defender", "Midfielder", "Forward"].map(pos => (
                  <button
                    key={pos}
                    onClick={() => { setPosFilter(pos); setPage(0); }}
                    className={`px-2.5 py-1 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-colors ${
                      posFilter === pos
                        ? "bg-primary-500 text-white"
                        : "bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600"
                    }`}
                  >
                    {pos === "All" ? t('common.all') : pos.slice(0, 3)}
                  </button>
                ))}
              </div>
            </div>
          </CardHeader>
          <CardBody>
            <div className="relative mb-3">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
              <input
                type="text"
                placeholder={t('scouting.searchPlaceholder')}
                value={searchQuery}
                onChange={e => { setSearchQuery(e.target.value); setPage(0); }}
                className="w-full pl-9 pr-4 py-2 text-sm bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500/50 text-gray-800 dark:text-gray-100 placeholder:text-gray-400"
              />
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider border-b border-gray-100 dark:border-navy-700">
                    <th className="text-left py-2 px-2">{t('scouting.player')}</th>
                    <th className="text-left py-2 px-1">{t('scouting.pos')}</th>
                    <th className="text-center py-2 px-1">{t('scouting.age')}</th>
                    <th className="text-left py-2 px-1">{t('scouting.team')}</th>
                    <th className="text-center py-2 px-1">{t('scouting.value')}</th>
                    <th className="text-right py-2 px-2">{t('scouting.action')}</th>
                  </tr>
                </thead>
                <tbody>
                  {scoutablePlayers.map(p => {
                    const isScouting = alreadyScoutingIds.has(p.id);
                    const team = p.team_id ? (teamNameById.get(p.team_id) || t('common.freeAgent')) : t('common.freeAgent');
                    return (
                      <tr key={p.id} className="border-b border-gray-50 dark:border-navy-700/50 hover:bg-gray-50 dark:hover:bg-navy-700/30 transition-colors">
                        <td className="py-2 px-2">
                          <button onClick={() => onSelectPlayer?.(p.id)} className="font-heading font-bold text-gray-800 dark:text-gray-100 hover:text-primary-500 transition-colors text-left">
                            {p.full_name}
                          </button>
                          <div className="text-[10px] text-gray-400 mt-0.5 flex items-center gap-1">
                            <CountryFlag code={p.nationality} locale={i18n.language} className="text-xs leading-none" />
                            <span>{countryName(p.nationality, i18n.language)}</span>
                          </div>
                        </td>
                        <td className="py-2 px-1">
                          <Badge variant={
                            p.position === "Goalkeeper" ? "accent" :
                            p.position === "Defender" ? "primary" :
                            p.position === "Midfielder" ? "success" : "danger"
                          } size="sm">{p.position.slice(0, 3)}</Badge>
                        </td>
                        <td className="text-center py-2 px-1 text-gray-600 dark:text-gray-400">{calcAge(p.date_of_birth)}</td>
                        <td className="py-2 px-1 text-gray-600 dark:text-gray-400 text-xs truncate max-w-[120px]">{team}</td>
                        <td className="text-center py-2 px-1 text-gray-600 dark:text-gray-400 text-xs">{formatVal(p.market_value)}</td>
                        <td className="text-right py-2 px-2">
                          {isScouting ? (
                            <span className="text-xs text-primary-400 font-heading font-bold">{t('scouting.scoutingInProgress')}</span>
                          ) : availableScouts.length === 0 ? (
                            <span className="text-xs text-gray-400">{t('scouting.noScoutsFree')}</span>
                          ) : (
                            <button
                              disabled={sending === p.id}
                              onClick={() => handleSendScout(p.id)}
                              className="flex items-center gap-1 ml-auto px-2.5 py-1 rounded-lg bg-primary-500/10 text-primary-500 hover:bg-primary-500/20 transition-colors text-xs font-heading font-bold uppercase tracking-wider disabled:opacity-50"
                            >
                              <ScanSearch className="w-3 h-3" />
                              {sending === p.id ? "..." : t('scouting.scoutBtn')}
                            </button>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              {scoutablePlayers.length === 0 && (
                <p className="text-center text-sm text-gray-400 py-4">{t('scouting.noPlayersFound')}</p>
              )}
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="flex items-center justify-between pt-3 border-t border-gray-100 dark:border-navy-700 mt-3">
                <span className="text-xs text-gray-400 dark:text-gray-500">
                  {t('scouting.showingRange', { from: safePage * SCOUTING_PAGE_SIZE + 1, to: Math.min((safePage + 1) * SCOUTING_PAGE_SIZE, allScoutable.length), total: allScoutable.length })}
                </span>
                <div className="flex items-center gap-2">
                  <button
                    disabled={safePage === 0}
                    onClick={() => setPage(p => Math.max(0, p - 1))}
                    className="p-1.5 rounded-lg bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                  >
                    <ChevronLeft className="w-4 h-4" />
                  </button>
                  <span className="text-xs font-heading font-bold text-gray-500 dark:text-gray-400 tabular-nums">
                    {safePage + 1} / {totalPages}
                  </span>
                  <button
                    disabled={safePage >= totalPages - 1}
                    onClick={() => setPage(p => Math.min(totalPages - 1, p + 1))}
                    className="p-1.5 rounded-lg bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                  >
                    <ChevronRight className="w-4 h-4" />
                  </button>
                </div>
              </div>
            )}
          </CardBody>
        </Card>
      )}
    </div>
  );
}
