import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type FocusEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import type { KitPattern } from "../../store/types";
import { condBgColor } from "../../lib/playerConditionDisplay";
import { getPositionColor } from "../../lib/positionColors";
import { PlayerAvatar } from "./PlayerAvatar";
import JerseyIcon from "./JerseyIcon";

/** How well a player fits the slot they occupy — drives the avatar ring colour. */
export type PitchFitTone = "exact" | "adapted" | "out" | "empty";

/** A small role marker chip (Captain, Penalty taker, etc.) stacked top-left. */
export interface PitchTokenMarker {
  key: string;
  shortLabel: string;
  /** Tailwind classes for the chip background/border/text. */
  toneClassName: string;
}

export interface PitchTokenProps {
  /** Display name (already formatted/uppercased by the caller if desired). */
  name: string;
  /** Short position label shown top-right (e.g. "ST", "GK"). */
  positionAbbr: string;
  /** Raw position enum (e.g. "CenterBack"); colours the position badge. */
  position?: string;
  ovr: number;
  /** 0–100 short-term condition; drives the bar at the bottom. */
  condition: number;
  fitTone?: PitchFitTone;
  /** Connect the owning player control to the persistent summary. */
  descriptionId?: string;
  /** The owning player control or one of its children has keyboard focus. */
  focused?: boolean;
  /** When present, renders a face/generated avatar; otherwise initials from name. */
  avatar?: { full_name: string; match_name: string; media?: { face?: string } };
  /** Optional kit jersey rendered under the avatar. */
  jersey?: {
    primaryColor: string;
    secondaryColor: string;
    pattern: KitPattern;
    number?: number | null;
  };
  /** Plain "#N" fallback shown when no kit `jersey` is available. */
  jerseyNumber?: number | null;
  /**
   * When true, skips the face avatar and kit jersey graphics, replacing the
   * avatar with a plain fit-tone ring. Used by the tactics board, where the
   * portraits and kit colours added clutter without helping tactical setup.
   */
  hidePortrait?: boolean;
  /** Role markers stacked at the top-left (max 3 shown). */
  markers?: PitchTokenMarker[];
  /** Optional slot below the name — e.g. a tactical-role combobox. */
  children?: ReactNode;
}

/** Shared focus/description wiring for the tactics and pre-match player controls. */
export function usePitchTokenFocus() {
  const id = useId();
  const [focusedPlayerId, focusToken] = useState<string | null>(null);
  function blurToken(event: FocusEvent<HTMLElement>) {
    if (
      !(event.relatedTarget instanceof Node) ||
      !event.currentTarget.contains(event.relatedTarget)
    ) {
      focusToken(null);
    }
  }
  return {
    focusedPlayerId,
    focusToken,
    blurToken,
    getDescriptionId: (playerId: string) => `${id}-${playerId}-description`,
  };
}

function fitRingClass(fitTone: PitchFitTone): string {
  switch (fitTone) {
    case "exact":
      return "ring-2 ring-success-400";
    case "adapted":
      return "ring-2 ring-accent-400";
    case "out":
      return "ring-2 ring-red-400";
    default:
      return "ring-1 ring-white/25";
  }
}

/** Long enough to cross the gap between a token and its summary, short enough not to feel stuck. */
const HOVER_LEAVE_GRACE_MS = 150;

/** Hover state that survives the pointer crossing from the token to its portaled summary. */
function useGracefulHover() {
  const [hovered, setHovered] = useState(false);
  const leaveTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(leaveTimer.current), []);
  return {
    hovered,
    startHover: () => {
      clearTimeout(leaveTimer.current);
      setHovered(true);
    },
    endHover: () => {
      clearTimeout(leaveTimer.current);
      leaveTimer.current = setTimeout(() => setHovered(false), HOVER_LEAVE_GRACE_MS);
    },
  };
}

/** The existing label for how a player fits the slot; `empty` has no verdict to state. */
function fitLabelKey(fitTone: PitchFitTone): string | null {
  switch (fitTone) {
    case "exact":
      return "squad.naturalFit";
    case "adapted":
      return "squad.adaptedFit";
    case "out":
      return "squad.outOfPosition";
    default:
      return null;
  }
}

/**
 * Presentational pitch token shared by the tactics board and the pre-match
 * screen: a circular avatar with a fit-tone ring, corner badges (position +
 * OVR), stacked role markers, an optional kit jersey, the player name, an
 * optional control slot (e.g. a role combobox), and a condition bar.
 *
 * It renders visuals only — wrap it in a button / drag handle and wire
 * interactions at the call site.
 */
export function PitchToken({
  name,
  positionAbbr,
  position,
  ovr,
  condition,
  fitTone = "empty",
  descriptionId,
  focused = false,
  avatar,
  jersey,
  jerseyNumber,
  hidePortrait = false,
  markers,
  children,
}: PitchTokenProps) {
  const { t } = useTranslation();
  const tokenRef = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLSpanElement>(null);
  const { hovered, startHover, endHover } = useGracefulHover();
  const [dismissed, setDismissed] = useState(false);
  const [tooltipPosition, setTooltipPosition] = useState<{ left: number; top: number } | null>(
    null,
  );
  const showTooltip = (hovered || focused) && !dismissed;
  const fitKey = fitLabelKey(fitTone);
  // Each channel says one thing — ring: fit, bar: condition — and the words
  // carry both, for hover and for screen readers (#330).
  const summary = fitKey
    ? t("squad.pitchTokenTooltip", { condition: Math.round(condition), fit: t(fitKey) })
    : t("squad.pitchTokenConditionOnly", { condition: Math.round(condition) });

  useEffect(() => {
    if (!hovered && !focused) setDismissed(false);
  }, [hovered, focused]);

  useEffect(() => {
    if (!showTooltip) return;
    function dismiss(event: KeyboardEvent) {
      if (event.key === "Escape") setDismissed(true);
    }
    // Capture phase: role controls inside the token stop keydown from bubbling.
    window.addEventListener("keydown", dismiss, true);
    return () => window.removeEventListener("keydown", dismiss, true);
  }, [showTooltip]);

  // Pitches clip their contents. A portal keeps the summary readable even for
  // edge slots; reposition it when the surrounding workspace scrolls.
  useLayoutEffect(() => {
    if (!showTooltip) return;
    function reposition() {
      const anchor = tokenRef.current?.getBoundingClientRect();
      const tooltip = tooltipRef.current?.getBoundingClientRect();
      if (!anchor || !tooltip) return;
      const below = anchor.bottom + 8;
      setTooltipPosition({
        left: Math.max(
          8,
          Math.min(
            anchor.left + (anchor.width - tooltip.width) / 2,
            window.innerWidth - tooltip.width - 8,
          ),
        ),
        top: Math.max(
          8,
          below + tooltip.height <= window.innerHeight - 8
            ? below
            : anchor.top - tooltip.height - 8,
        ),
      });
    }
    reposition();
    window.addEventListener("resize", reposition);
    window.addEventListener("scroll", reposition, true);
    return () => {
      window.removeEventListener("resize", reposition);
      window.removeEventListener("scroll", reposition, true);
    };
  }, [showTooltip, summary]);

  return (
    <div
      ref={tokenRef}
      className="flex w-full flex-col items-center gap-0.5"
      onMouseEnter={startHover}
      onMouseLeave={endHover}
    >
      {/* Avatar with overlaid badges */}
      <div className="relative">
        {markers && markers.length > 0 && (
          <div className="absolute -left-1.5 -top-1.5 z-10 flex flex-col gap-0.5">
            {markers.slice(0, 3).map((marker) => (
              <span
                key={marker.key}
                className={`rounded-full border px-1.5 py-0.5 text-[10px] font-heading font-bold leading-4 ${marker.toneClassName}`}
              >
                {marker.shortLabel}
              </span>
            ))}
          </div>
        )}
        <div className="absolute -right-1.5 -top-1.5 z-10">
          <span
            className={`rounded-full ${position ? getPositionColor(position) : "bg-gray-900"} px-2 py-0.5 text-xs font-heading font-bold uppercase leading-4 text-white ring-1 ring-white/40`}
          >
            {positionAbbr}
          </span>
        </div>
        {hidePortrait ? (
          <div className={`h-11 w-11 rounded-full bg-gray-800/60 ${fitRingClass(fitTone)}`} />
        ) : (
          <PlayerAvatar
            player={avatar ?? { full_name: name, match_name: name }}
            className={`h-11 w-11 overflow-hidden rounded-full ${fitRingClass(fitTone)}`}
          />
        )}
        <div className="absolute -bottom-1 -right-1.5 z-10">
          <span className="rounded-full bg-gray-900 px-2 py-0.5 text-xs font-heading font-bold leading-4 text-white ring-1 ring-white/30">
            {ovr}
          </span>
        </div>
      </div>

      {!hidePortrait &&
        (jersey ? (
          <JerseyIcon
            size="md"
            primaryColor={jersey.primaryColor}
            secondaryColor={jersey.secondaryColor}
            pattern={jersey.pattern}
            number={jersey.number}
          />
        ) : jerseyNumber != null ? (
          <span className="text-[10px] font-heading font-bold text-white/80">#{jerseyNumber}</span>
        ) : null)}

      <div className="max-w-full truncate text-xs font-heading font-bold uppercase tracking-[0.12em] text-white drop-shadow-sm">
        {name}
      </div>

      {children}

      <div className="w-full">
        {!showTooltip && (
          <span
            id={descriptionId}
            aria-hidden={descriptionId ? true : undefined}
            className="sr-only"
          >
            {summary}
          </span>
        )}
        <div className="h-1.5 overflow-hidden rounded-full bg-white/10">
          <div
            data-testid="pitch-token-condition-fill"
            className={`h-full rounded-full ${condBgColor(condition)}`}
            style={{ width: `${Math.max(20, condition)}%` }}
          />
        </div>
      </div>
      {showTooltip &&
        createPortal(
          <span
            id={descriptionId}
            ref={tooltipRef}
            role="tooltip"
            className={`fixed z-50 w-max max-w-64 rounded-md border border-gray-200 bg-white px-3 py-2 font-sans text-xs font-medium normal-case tracking-normal text-gray-900 shadow-lg dark:border-navy-600 dark:bg-navy-800 dark:text-gray-100 ${tooltipPosition ? "" : "invisible"}`}
            style={tooltipPosition ?? { left: 0, top: 0 }}
          >
            {summary}
          </span>,
          document.body,
        )}
    </div>
  );
}
