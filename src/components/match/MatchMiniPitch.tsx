import { useId, type ReactNode } from "react";
import { SportShoe } from "lucide-react";
import { useTranslation } from "react-i18next";
import { condBgColor } from "../../lib/playerConditionDisplay";
import type { PlayerData } from "../../store/types";
import { PlayerAvatar } from "../ui";
import { FormationPitch } from "./FormationPitch";
import { ratingBubbleColour } from "./ratingColour";
import type { EnginePlayerData, EngineTeamData, MatchSnapshot } from "./types";

interface MatchMiniPitchProps {
  snapshot: MatchSnapshot;
  /** The squad records behind the engine XI, by player id: shirt numbers and portraits. */
  squad: Map<string, PlayerData>;
}

/**
 * Both sides' formations as they stand, one token per starter with portrait,
 * shirt number, surname, stamina, live rating and cards (#410). Positions are static per formation; the
 * pitch redraws from each snapshot, so substitutions and shape changes show up on
 * the next tick.
 */
export function MatchMiniPitch({ snapshot, squad }: MatchMiniPitchProps) {
  return (
    <div className="grid grid-cols-2 gap-4">
      <TeamPitch
        team={snapshot.home_team}
        yellows={snapshot.home_yellows}
        snapshot={snapshot}
        squad={squad}
      />
      <TeamPitch
        team={snapshot.away_team}
        yellows={snapshot.away_yellows}
        snapshot={snapshot}
        squad={squad}
      />
    </div>
  );
}

function TeamPitch({
  team,
  yellows,
  snapshot,
  squad,
}: {
  team: EngineTeamData;
  yellows: Record<string, number>;
  snapshot: MatchSnapshot;
  squad: Map<string, PlayerData>;
}) {
  const ratings = snapshot.player_ratings ?? {};
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} className="flex min-w-0 flex-col gap-2">
      <div className="flex items-baseline justify-between gap-2">
        <h4
          id={headingId}
          className="truncate font-heading text-sm font-bold uppercase tracking-wider text-gray-800 dark:text-gray-200"
        >
          {team.name}
        </h4>
        <span className="font-heading text-xs text-gray-600 dark:text-gray-400">
          {team.formation}
        </span>
      </div>
      {/* Not passing `sentOff`: a dismissed player keeps his slot, dimmed and carded,
          rather than vanishing from the team sheet. */}
      <FormationPitch
        formation={team.formation}
        players={team.players}
        className="aspect-[5/7] w-full"
        renderToken={(player) => (
          <LiveToken
            player={player}
            squadPlayer={squad.get(player.id)}
            rating={ratings[player.id] ?? null}
            goals={snapshot.player_goals?.[player.id] ?? 0}
            assists={snapshot.player_assists?.[player.id] ?? 0}
            yellowCards={yellows[player.id] ?? 0}
            isSentOff={snapshot.sent_off.includes(player.id)}
          />
        )}
      />
    </section>
  );
}

function LiveToken({
  player,
  squadPlayer,
  rating,
  goals,
  assists,
  yellowCards,
  isSentOff,
}: {
  player: EnginePlayerData;
  squadPlayer: PlayerData | undefined;
  rating: number | null;
  goals: number;
  assists: number;
  yellowCards: number;
  isSentOff: boolean;
}) {
  const { t } = useTranslation();
  const displayedRating = rating === null ? null : rating.toFixed(1);
  const condition = Math.round(player.condition);
  const shirtNumber = squadPlayer?.jersey_number;
  // A second yellow is a red: show only the card that is in force.
  const card = isSentOff
    ? { label: t("match.eventTypes.RedCard"), tone: "bg-red-500", face: "bg-red-300" }
    : yellowCards > 0
      ? { label: t("match.eventTypes.YellowCard"), tone: "bg-yellow-400", face: "bg-yellow-200" }
      : null;

  return (
    <div className={`flex flex-col items-center gap-1 ${isSentOff ? "opacity-50" : ""}`}>
      {/* The visible label is the surname only; screen readers get the full name. */}
      <span className="sr-only">{player.name}</span>
      {/* Hidden as a whole: everything it shows is also spoken by the sr-only text around it. */}
      {/* Badges share one look: a thin dark outline plus a light halo, so they lift off
          both the portrait and the green pitch. */}
      <div aria-hidden="true" className="relative">
        <PlayerAvatar
          player={squadPlayer ?? { full_name: player.name, match_name: player.name }}
          // Nudged down onto the name; the badges stay anchored to the unmoved box.
          className="h-12 w-12 translate-y-1 overflow-hidden rounded-full border-2 border-white/70 bg-navy-800 shadow-md"
        />
        <span
          className={`absolute -left-4 -top-2 min-w-9 rounded-full px-1.5 py-0.5 text-center font-sans text-xs font-bold tabular-nums text-white shadow-lg ring-1 ring-navy-900 outline-2 outline-offset-1 outline-white/60 ${ratingBubbleColour(rating)}`}
        >
          {displayedRating ?? "–"}
        </span>
        {shirtNumber != null && (
          <span className="absolute -bottom-1 -right-2 min-w-6 rounded-full bg-navy-900 px-1 text-center font-sans text-xs font-bold text-white ring-1 ring-white/60">
            {shirtNumber}
          </span>
        )}
        {(goals > 0 || assists > 0) && (
          // One badge per kind with a count, so a hat-trick cannot crowd the token.
          <span className="absolute -bottom-1.5 -left-4 flex items-center gap-0.5">
            {goals > 0 && (
              <CountBadge testId="live-token-goals" count={goals}>
                <FootballIcon />
              </CountBadge>
            )}
            {assists > 0 && (
              <CountBadge testId="live-token-assists" count={assists}>
                <SportShoe className="h-3 w-3 text-navy-900" />
              </CountBadge>
            )}
          </span>
        )}
        {card && (
          <span
            className={`absolute -right-1.5 -top-1 h-3.5 w-2.5 rotate-12 rounded-xs shadow-md ring-1 ring-navy-900 outline-2 outline-offset-1 outline-white/60 ${card.tone}`}
          >
            {/* A lighter face inside the card lifts it off the portrait. */}
            <span className={`absolute inset-0.5 rounded-xs ${card.face}`} />
          </span>
        )}
      </div>
      {card && <span className="sr-only">{card.label}</span>}
      {goals > 0 && <span className="sr-only">{t("match.liveGoals", { count: goals })}</span>}
      {assists > 0 && <span className="sr-only">{t("match.liveAssists", { count: assists })}</span>}
      {/* A dark backing keeps the white name readable on the light end of the pitch gradient. */}
      <span
        aria-hidden="true"
        className="max-w-24 truncate rounded bg-navy-900/60 px-1.5 text-center font-sans text-sm font-bold leading-5 text-white"
      >
        {player.name.split(" ").pop()}
      </span>
      {/* A dark, outlined track: a full green bar would otherwise vanish into the pitch. */}
      <div
        aria-hidden="true"
        className="h-2 w-16 overflow-hidden rounded-full bg-navy-900/80 p-px ring-1 ring-white/40"
      >
        <div
          data-testid="live-token-condition-fill"
          className={`h-full rounded-full ${condBgColor(condition)}`}
          style={{ width: `${condition}%` }}
        />
      </div>
      <span className="sr-only">{t("squad.pitchTokenConditionOnly", { condition })}</span>
      {displayedRating !== null && (
        <span className="sr-only">{t("match.liveRating", { rating: displayedRating })}</span>
      )}
    </div>
  );
}

/** A white pill holding an icon, with the count beside it only once there is more than one. */
function CountBadge({
  testId,
  count,
  children,
}: {
  testId: string;
  count: number;
  children: ReactNode;
}) {
  return (
    <span
      data-testid={testId}
      className="flex h-4 items-center gap-px rounded-full bg-white px-0.5 font-sans text-xs font-bold leading-none text-navy-900 shadow ring-1 ring-navy-900"
    >
      {children}
      {count > 1 && <span className="pr-0.5">{count}</span>}
    </span>
  );
}

// The ball from the project's own logo artwork, cropped by viewBox to the ball
// alone (it sits top-right of the canvas, with a motion trail the crop leaves out).
const FOOTBALL_SRC = `${import.meta.env.BASE_URL}openfootball.svg`;

function FootballIcon() {
  return (
    <svg viewBox="20.8 0 43.1 43.1" className="h-3 w-3" aria-hidden="true">
      <image href={FOOTBALL_SRC} width="63.859354" height="59.039909" />
    </svg>
  );
}
