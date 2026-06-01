import { useTranslation } from "react-i18next";
import { MatchSnapshot, MatchEvent, EnginePlayerData } from "./types";
import { getEventDisplay, getEventTypeLabel, getPlayerName } from "./helpers";
import { Badge } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

export function EventFeed({
  events,
  snapshot,
  feedRef,
}: {
  events: MatchEvent[];
  snapshot: MatchSnapshot;
  feedRef: React.RefObject<HTMLDivElement | null>;
}) {
  const { t } = useTranslation();
  return (
    <div ref={feedRef} className="flex flex-col gap-1">
      {events.length === 0 ? (
        <div className="flex items-center justify-center h-40 text-gray-600 dark:text-gray-500">
          <p className="font-heading text-sm uppercase tracking-wider">
            {t("match.waitingKickoff")}
          </p>
        </div>
      ) : (
        events.map((evt, i) => {
          const display = getEventDisplay(evt);
          const isHome = evt.side === "Home";
          return (
            <div
              key={i}
              className={`flex items-start gap-3 px-3 py-2 rounded-lg transition-colors ${display.important ? "bg-white dark:bg-navy-800/80 border border-gray-200 dark:border-navy-700 shadow-sm" : "opacity-60"}`}
            >
              <span className="text-gray-600 dark:text-gray-500 tabular-nums font-heading text-sm w-8 text-right flex-shrink-0 pt-0.5">
                {evt.minute}'
              </span>
              <span className="text-lg flex-shrink-0">{display.icon}</span>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span
                    className={`font-heading font-bold text-xs uppercase tracking-wider ${isHome ? "text-primary-400" : "text-indigo-400"}`}
                  >
                    {isHome ? snapshot.home_team.name : snapshot.away_team.name}
                  </span>
                  <span className="text-xs text-gray-500 dark:text-gray-400">
                    {getEventTypeLabel(evt.event_type, t)}
                  </span>
                </div>
                {evt.player_id && (
                  <p className="text-sm text-gray-700 dark:text-gray-300 font-medium">
                    {getPlayerName(snapshot, evt.player_id)}
                    {evt.secondary_player_id && (
                      <span className="text-gray-500 dark:text-gray-400 font-normal">
                        {evt.event_type === "Goal"
                          ? ` (${t("match.assist", { name: getPlayerName(snapshot, evt.secondary_player_id) })})`
                          : evt.event_type === "Substitution"
                            ? ` ${t("match.subFor", { name: getPlayerName(snapshot, evt.secondary_player_id) })}`
                            : ""}
                      </span>
                    )}
                  </p>
                )}
              </div>
            </div>
          );
        })
      )}
    </div>
  );
}

export function MatchStats({ snapshot }: { snapshot: MatchSnapshot }) {
  const { t } = useTranslation();
  const homeEvents = snapshot.events.filter((e) => e.side === "Home");
  const awayEvents = snapshot.events.filter((e) => e.side === "Away");
  const ct = (events: MatchEvent[], type: string) =>
    events.filter((e) => e.event_type === type).length;

  const stats = [
    {
      label: t("match.possession"),
      home: `${snapshot.home_possession_pct.toFixed(0)}%`,
      away: `${snapshot.away_possession_pct.toFixed(0)}%`,
      homePct: snapshot.home_possession_pct,
    },
    {
      label: t("match.shots"),
      home:
        ct(homeEvents, "Goal") +
        ct(homeEvents, "PenaltyGoal") +
        ct(homeEvents, "ShotSaved") +
        ct(homeEvents, "ShotOffTarget") +
        ct(homeEvents, "ShotBlocked"),
      away:
        ct(awayEvents, "Goal") +
        ct(awayEvents, "PenaltyGoal") +
        ct(awayEvents, "ShotSaved") +
        ct(awayEvents, "ShotOffTarget") +
        ct(awayEvents, "ShotBlocked"),
    },
    {
      label: t("match.shotsOnTarget"),
      home:
        ct(homeEvents, "Goal") +
        ct(homeEvents, "PenaltyGoal") +
        ct(homeEvents, "ShotSaved"),
      away:
        ct(awayEvents, "Goal") +
        ct(awayEvents, "PenaltyGoal") +
        ct(awayEvents, "ShotSaved"),
    },
    {
      label: t("match.fouls"),
      home: ct(homeEvents, "Foul"),
      away: ct(awayEvents, "Foul"),
    },
    {
      label: t("match.corners"),
      home: ct(homeEvents, "Corner"),
      away: ct(awayEvents, "Corner"),
    },
    {
      label: t("match.yellowCards"),
      home: Object.keys(snapshot.home_yellows).length,
      away: Object.keys(snapshot.away_yellows).length,
    },
  ];

  return (
    <div className="max-w-lg mx-auto flex flex-col gap-3">
      {stats.map((stat, i) => {
        const hv = typeof stat.home === "number" ? stat.home : 0;
        const av = typeof stat.away === "number" ? stat.away : 0;
        const total = hv + av || 1;
        const pct = stat.homePct ?? (hv / total) * 100;
        return (
          <div key={i}>
            <div className="flex justify-between text-xs mb-1">
              <span className="font-heading font-bold text-primary-400 tabular-nums">
                {stat.home}
              </span>
              <span className="text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider text-[10px]">
                {stat.label}
              </span>
              <span className="font-heading font-bold text-indigo-400 tabular-nums">
                {stat.away}
              </span>
            </div>
            <div className="flex h-1.5 bg-gray-300 dark:bg-navy-700 rounded-full overflow-hidden transition-colors duration-300">
              <div
                className="h-full bg-primary-500 transition-all duration-500"
                style={{ width: `${pct}%` }}
              />
              <div
                className="h-full bg-indigo-500 transition-all duration-500"
                style={{ width: `${100 - pct}%` }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function Lineups({ snapshot }: { snapshot: MatchSnapshot }) {
  const { t } = useTranslation();
  const renderTeam = (
    team: MatchSnapshot["home_team"],
    bench: EnginePlayerData[],
    side: "Home" | "Away",
    yellows: Record<string, number>,
    sentOff: string[],
  ) => {
    const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];
    const subbedOnIds = new Set(
      snapshot.substitutions
        .filter((s) => s.side === side)
        .map((s) => s.player_on_id),
    );
    const subbedOffIds = new Set(
      snapshot.substitutions
        .filter((s) => s.side === side)
        .map((s) => s.player_off_id),
    );
    return (
      <div className="flex-1">
        <h4
          className={`font-heading font-bold text-sm uppercase tracking-wider mb-3 ${side === "Home" ? "text-primary-400" : "text-indigo-400"}`}
        >
          {team.name}{" "}
          <span className="text-gray-600 dark:text-gray-500 font-normal text-xs">
            ({team.formation})
          </span>
        </h4>
        {positions.map((pos) => {
          const players = team.players.filter((p) => p.position === pos);
          if (players.length === 0) return null;
          return (
            <div key={pos} className="mb-3">
              <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
                {pos}s
              </p>
              {players.map((p) => {
                const isOff = sentOff.includes(p.id);
                const yc = yellows[p.id] || 0;
                const isSubOn = subbedOnIds.has(p.id);
                const condColor =
                  p.condition >= 70
                    ? "bg-primary-500"
                    : p.condition >= 40
                      ? "bg-yellow-500"
                      : "bg-red-500";
                return (
                  <div
                    key={p.id}
                    className={`flex items-center gap-2 py-1 px-2 rounded text-xs ${isOff ? "opacity-40" : ""}`}
                  >
                    {isSubOn && (
                      <span className="text-green-400 text-[10px]">▲</span>
                    )}
                    <span
                      className={`font-medium flex-1 truncate ${isOff ? "line-through text-gray-600 dark:text-gray-500" : "text-gray-700 dark:text-gray-300"}`}
                    >
                      {p.name}
                    </span>
                    {yc > 0 && (
                      <span className="w-3 h-4 rounded-sm bg-yellow-400 text-navy-900 text-[8px] flex items-center justify-center font-bold">
                        {yc > 1 ? yc : ""}
                      </span>
                    )}
                    {isOff && (
                      <span className="w-3 h-4 rounded-sm bg-red-500" />
                    )}
                    <div className="w-14 flex items-center gap-1">
                      <div className="flex-1 h-1.5 bg-gray-300 dark:bg-navy-600 rounded-full overflow-hidden transition-colors duration-300">
                        <div
                          className={`h-full ${condColor} rounded-full transition-all`}
                          style={{ width: `${p.condition}%` }}
                        />
                      </div>
                      <span className="text-gray-500 dark:text-gray-400 tabular-nums text-[10px] w-6 text-right">
                        {Math.round(p.condition)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          );
        })}

        {/* Bench */}
        {bench.length > 0 && (
          <div className="mt-3 pt-3 border-t border-gray-200 dark:border-navy-700">
            <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
              {t("match.bench")}
            </p>
            {bench.map((p) => {
              const wasSubbedOff = subbedOffIds.has(p.id);
              return (
                <div
                  key={p.id}
                  className={`flex items-center gap-2 py-1 px-2 rounded text-xs ${wasSubbedOff ? "opacity-50" : ""}`}
                >
                  {wasSubbedOff && (
                    <span className="text-red-400 text-[10px]">▼</span>
                  )}
                  <span className="text-gray-600 dark:text-gray-400 font-medium flex-1 truncate">
                    {p.name}
                  </span>
                  <Badge variant="neutral" size="sm">
                    {translatePositionAbbreviation(t, p.position)}
                  </Badge>
                  <span className="text-gray-500 dark:text-gray-400 tabular-nums text-[10px] w-6 text-right">
                    {Math.round(p.condition)}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {/* Sub History */}
        {snapshot.substitutions.filter((s) => s.side === side).length > 0 && (
          <div className="mt-3 pt-3 border-t border-gray-200 dark:border-navy-700">
            <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
              {t("match.substitutions")}
            </p>
            {snapshot.substitutions
              .filter((s) => s.side === side)
              .map((sub, i) => (
                <div
                  key={i}
                  className="flex items-center gap-1.5 py-0.5 text-[11px]"
                >
                  <span className="text-gray-600 dark:text-gray-500 tabular-nums w-5 text-right font-heading">
                    {sub.minute}'
                  </span>
                  <span className="text-green-400">▲</span>
                  <span className="text-gray-700 dark:text-gray-300 truncate">
                    {getPlayerName(snapshot, sub.player_on_id)}
                  </span>
                  <span className="text-red-400">▼</span>
                  <span className="text-gray-500 dark:text-gray-400 truncate">
                    {getPlayerName(snapshot, sub.player_off_id)}
                  </span>
                </div>
              ))}
          </div>
        )}
      </div>
    );
  };
  return (
    <div className="flex gap-6">
      {renderTeam(
        snapshot.home_team,
        snapshot.home_bench,
        "Home",
        snapshot.home_yellows,
        snapshot.sent_off,
      )}
      <div className="w-px bg-gray-200 dark:bg-navy-700 transition-colors duration-300" />
      {renderTeam(
        snapshot.away_team,
        snapshot.away_bench,
        "Away",
        snapshot.away_yellows,
        snapshot.sent_off,
      )}
    </div>
  );
}
