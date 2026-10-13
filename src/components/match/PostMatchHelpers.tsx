import { useTranslation } from "react-i18next";
import type { MatchSnapshot, MatchEvent } from "./types";
import { getPlayerName } from "./helpers";
import { Badge } from "../ui";
import { Circle, Star } from "lucide-react";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

// ---------------------------------------------------------------------------
// QuickStat bar
// ---------------------------------------------------------------------------

export function QuickStat({
  label,
  home,
  away,
  homePct,
}: {
  label: string;
  home: number | string;
  away: number | string;
  homePct?: number;
}) {
  const hv = typeof home === "number" ? home : 0;
  const av = typeof away === "number" ? away : 0;
  const total = hv + av || 1;
  const pct = homePct ?? (hv / total) * 100;

  return (
    <div className="mb-2 last:mb-0">
      <div className="flex justify-between text-xs mb-0.5">
        <span className="font-heading font-bold text-primary-400 tabular-nums">{home}</span>
        <span className="text-gray-600 dark:text-gray-500 font-heading uppercase tracking-wider text-[10px]">
          {label}
        </span>
        <span className="font-heading font-bold text-indigo-400 tabular-nums">{away}</span>
      </div>
      <div className="flex h-1 bg-gray-300 dark:bg-navy-700 rounded-full overflow-hidden transition-colors duration-300">
        <div className="h-full bg-primary-500" style={{ width: `${pct}%` }} />
        <div className="h-full bg-indigo-500" style={{ width: `${100 - pct}%` }} />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Scorer list per side
// ---------------------------------------------------------------------------

export function renderScorers(
  snapshot: MatchSnapshot,
  events: MatchEvent[],
  side: "Home" | "Away",
) {
  const goals = events.filter(
    (e) => e.side === side && (e.event_type === "Goal" || e.event_type === "PenaltyGoal"),
  );
  if (goals.length === 0) return null;

  const team = side === "Home" ? snapshot.home_team : snapshot.away_team;
  return (
    <div className="mb-3 last:mb-0">
      <p
        className={`text-[10px] font-heading uppercase tracking-widest mb-1 ${
          side === "Home" ? "text-primary-400" : "text-indigo-400"
        }`}
      >
        {team.name}
      </p>
      {goals.map((g, i) => (
        <div key={i} className="flex items-center gap-2 text-xs py-0.5">
          <span className="text-gray-600 dark:text-gray-500 tabular-nums w-6 text-right font-heading">
            {g.minute}'
          </span>
          <Circle className="w-3 h-3 fill-current text-accent-400" />
          <span className="text-gray-800 dark:text-gray-200 font-medium">
            {getPlayerName(snapshot, g.player_id)}
          </span>
          {g.event_type === "PenaltyGoal" && (
            <Badge variant="accent" size="sm">
              PEN
            </Badge>
          )}
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Player Ratings panel for one side
// ---------------------------------------------------------------------------

function ratingColour(rating: number | null): string {
  if (rating === null) return "text-gray-400 dark:text-gray-500";
  if (rating >= 8) return "text-accent-700 dark:text-accent-400";
  if (rating >= 7) return "text-green-700 dark:text-green-400";
  if (rating >= 6) return "text-gray-600 dark:text-gray-300";
  if (rating >= 5) return "text-yellow-700 dark:text-yellow-400";
  return "text-red-700 dark:text-red-400";
}

export function PlayerRatingsPanel({
  snapshot,
  side,
  teamColor,
  userSide,
}: {
  snapshot: MatchSnapshot;
  side: "Home" | "Away";
  teamColor: string;
  userSide: "Home" | "Away" | null;
}) {
  const { t } = useTranslation();
  const team = side === "Home" ? snapshot.home_team : snapshot.away_team;
  const bench = side === "Home" ? snapshot.home_bench : snapshot.away_bench;
  const engineRatings = snapshot.player_ratings ?? {};
  const playedFromBench = (id: string) =>
    id in engineRatings ||
    snapshot.substitutions.some((sub) => sub.player_on_id === id || sub.player_off_id === id);
  const sorted = [...team.players, ...bench.filter((p) => playedFromBench(p.id))]
    .map((p) => ({ ...p, rating: engineRatings[p.id] ?? null }))
    .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0));
  const motm = sorted[0]?.rating != null ? sorted[0] : null;

  return (
    <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
      <div className="flex items-center gap-2 mb-3">
        <Star className="w-4 h-4 text-accent-700 dark:text-accent-400" />
        <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
          {t("match.ratings", { team: team.name })}
        </h3>
        <div className="w-2 h-2 rounded-full ml-auto" style={{ backgroundColor: teamColor }} />
      </div>
      {motm && side === (userSide || "Home") && (
        <div className="flex items-center gap-3 mb-3 p-2 bg-accent-50 dark:bg-accent-500/10 rounded-lg border border-accent-200 dark:border-accent-500/20 transition-colors duration-300">
          <div className="w-8 h-8 rounded-lg bg-accent-100 dark:bg-accent-500/20 flex items-center justify-center transition-colors duration-300">
            <span className="text-sm font-heading font-bold text-accent-700 dark:text-accent-400">
              {motm.rating?.toFixed(1)}
            </span>
          </div>
          <div>
            <p className="text-xs font-heading font-bold text-accent-700 dark:text-accent-400 uppercase tracking-wider">
              {t("match.motm")}
            </p>
            <p className="text-sm text-gray-800 dark:text-gray-200 font-medium">{motm.name}</p>
          </div>
        </div>
      )}
      <div className="flex flex-col gap-0.5 max-h-40 overflow-auto">
        {sorted.map((p) => (
          <div key={p.id} className="flex items-center gap-2 px-1 py-0.5 text-xs">
            <span className={`font-heading font-bold tabular-nums w-8 ${ratingColour(p.rating)}`}>
              {p.rating === null ? "–" : p.rating.toFixed(1)}
            </span>
            <span className="text-gray-600 dark:text-gray-400 truncate flex-1">{p.name}</span>
            <span className="text-gray-600 dark:text-gray-500 text-[10px] font-heading uppercase">
              {translatePositionAbbreviation(t, p.position)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
