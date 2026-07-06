import { useId, type ReactNode } from "react";
import { buildPitchRows } from "../squad/SquadTab.helpers";
import type { EnginePlayerData } from "./types";

interface FormationSlot {
  player: EnginePlayerData;
  x: number;
  y: number;
  /** Granular formation slot (e.g. "CenterBack") — only in slot-aligned mode. */
  slotPosition?: string;
}

/**
 * Slot-aligned layout: the engine XI is ordered so entry i plays formation
 * slot i (team_builder keeps this invariant, including through swaps and
 * substitutions), which lets us place each player at their actual granular
 * slot instead of re-bucketing by coarse position group. Returns null when
 * the invariant can't hold (player count ≠ slot count), so the caller can
 * fall back to the grouped layout.
 */
function buildSlotAlignedSlots(
  formation: string,
  players: EnginePlayerData[],
  sentOff: string[],
): FormationSlot[] | null {
  const rows = buildPitchRows(formation);
  const slotCount = rows.reduce((sum, row) => sum + row.positions.length, 0);
  if (slotCount !== players.length) {
    return null;
  }

  const slots: FormationSlot[] = [];
  let slotIndex = 0;
  for (const row of rows) {
    const y = Number.parseFloat(row.y);
    row.positions.forEach((slotPosition, colIdx) => {
      const player = players[slotIndex];
      slotIndex += 1;
      if (sentOff.includes(player.id)) {
        return;
      }
      slots.push({
        player,
        x:
          row.positions.length === 1
            ? 50
            : Math.round((100 * (colIdx + 1)) / (row.positions.length + 1)),
        y: Number.isFinite(y) ? y : 50,
        slotPosition,
      });
    });
  }
  return slots;
}

export function buildFormationSlots(
  formation: string,
  players: EnginePlayerData[],
  sentOff: string[] = [],
): FormationSlot[] {
  const active = players.filter((p) => !sentOff.includes(p.id));
  const nums = formation.split("-").map(Number);
  // A valid formation has at least three lines (def-mid-fwd). Anything shorter
  // or non-numeric ("442", "5-5", "abc") can't be laid out by the row logic
  // below without dropping the midfield/forward rows, so fall back to an even
  // single-row spread that still renders every player.
  if (nums.length < 3 || nums.some((n) => isNaN(n))) {
    return active.map((p, i) => ({
      player: p,
      x: Math.round((100 * (i + 1)) / (active.length + 1)),
      y: 50,
    }));
  }

  const gks = active.filter((p) => p.position === "Goalkeeper");
  const defs = active.filter((p) => p.position === "Defender");
  const mids = active.filter((p) => p.position === "Midfielder");
  const fwds = active.filter((p) => p.position === "Forward");

  const rows: EnginePlayerData[][] = [gks];
  const n = nums.length;
  let midCursor = 0;
  for (let i = 0; i < n; i++) {
    const count = nums[i];
    // Never drop a player: put every defender in the back line, every forward up
    // top, and let the last midfield row absorb any remaining midfielders. A
    // lopsided XI (e.g. an AI side that ended up a defender short) then still
    // renders all 11 rather than silently hiding the overflow.
    if (i === 0) rows.push(defs);
    else if (i === n - 1) rows.push(fwds);
    else if (i === n - 2) rows.push(mids.slice(midCursor));
    else {
      rows.push(mids.slice(midCursor, midCursor + count));
      midCursor += count;
    }
  }

  const bottom = 85;
  const top = 15;
  const step = rows.length > 1 ? (bottom - top) / (rows.length - 1) : 0;
  return rows.flatMap((rowPlayers, rowIdx) => {
    const y = Math.round(bottom - rowIdx * step);
    return rowPlayers.map((p, colIdx) => ({
      player: p,
      x:
        rowPlayers.length === 1
          ? 50
          : Math.round((100 * (colIdx + 1)) / (rowPlayers.length + 1)),
      y,
    }));
  });
}

interface FormationPitchProps {
  formation: string;
  players: EnginePlayerData[];
  sentOff?: string[];
  selectedId?: string | null;
  subbedOnIds?: Set<string>;
  onPlayerClick?: (id: string) => void;
  className?: string;
  /**
   * Optional custom token renderer. When provided it replaces the default
   * initials token, letting callers (e.g. the pre-match screen) render a richer
   * token while reusing this pitch's SVG and slot layout. The pitch still owns
   * positioning, selection state, and click wiring.
   */
  renderToken?: (
    player: EnginePlayerData,
    state: { isSelected: boolean; isSubOn: boolean; slotPosition?: string },
  ) => ReactNode;
}

export function FormationPitch({
  formation,
  players,
  sentOff = [],
  selectedId,
  subbedOnIds,
  onPlayerClick,
  className,
  renderToken,
}: FormationPitchProps) {
  const uid = useId();
  const surfaceId = `pitch-surface-${uid}`;
  const stripesId = `pitch-stripes-${uid}`;
  const slots =
    buildSlotAlignedSlots(formation, players, sentOff) ??
    buildFormationSlots(formation, players, sentOff);

  return (
    <div
      className={`relative overflow-hidden rounded-xl bg-gradient-to-b from-primary-500 to-primary-700 ${className ?? ""}`}
    >
      <svg
        className="absolute inset-0 h-full w-full"
        viewBox="0 0 100 140"
        preserveAspectRatio="none"
      >
        <defs>
          <linearGradient id={surfaceId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="rgba(63,172,99,0.35)" />
            <stop offset="100%" stopColor="rgba(31,109,61,0.25)" />
          </linearGradient>
          <pattern
            id={stripesId}
            x="0"
            y="0"
            width="100"
            height="10"
            patternUnits="userSpaceOnUse"
          >
            <rect
              x="0"
              y="0"
              width="100"
              height="5"
              fill="rgba(255,255,255,0.04)"
            />
          </pattern>
        </defs>
        <rect
          x="0"
          y="0"
          width="100"
          height="140"
          fill={`url(#${surfaceId})`}
        />
        <rect
          x="0"
          y="0"
          width="100"
          height="140"
          fill={`url(#${stripesId})`}
        />
        <rect
          x="4"
          y="4"
          width="92"
          height="132"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <line
          x1="4"
          y1="70"
          x2="96"
          y2="70"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <circle
          cx="50"
          cy="70"
          r="11"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <circle cx="50" cy="70" r="0.8" fill="rgba(255,255,255,0.75)" />
        <rect
          x="18"
          y="4"
          width="64"
          height="18"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <rect
          x="18"
          y="118"
          width="64"
          height="18"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <rect
          x="30"
          y="4"
          width="40"
          height="8"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <rect
          x="30"
          y="128"
          width="40"
          height="8"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <path
          d="M 38 22 A 12 12 0 0 0 62 22"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
        <path
          d="M 38 118 A 12 12 0 0 1 62 118"
          fill="none"
          stroke="rgba(255,255,255,0.55)"
          strokeWidth="0.6"
        />
      </svg>
      {slots.map(({ player: p, x, y, slotPosition }) => {
        const isSelected = selectedId === p.id;
        const isSubOn = subbedOnIds?.has(p.id) ?? false;
        const initials = p.name
          .split(" ")
          .map((n) => n[0])
          .slice(0, 2)
          .join("")
          .toUpperCase();
        const sharedClass = `absolute z-20 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-0.5 transition-all ${onPlayerClick ? "cursor-pointer hover:scale-110" : ""} ${isSelected ? "scale-110" : ""}`;
        const sharedStyle = { left: `${x}%`, top: `${y}%` };
        const tokenContent = renderToken ? (
          renderToken(p, { isSelected, isSubOn, slotPosition })
        ) : (
          <>
            <div
              className={`flex h-7 w-7 items-center justify-center rounded-full border-2 font-heading text-[9px] font-bold text-white shadow-md transition-all ${
                isSelected
                  ? "border-red-300 bg-red-500/80 ring-2 ring-red-500/50"
                  : p.condition < 50
                    ? "border-yellow-400/80 bg-yellow-600/70"
                    : "border-white/30 bg-navy-800/80"
              }`}
            >
              {isSubOn ? "▲" : initials}
            </div>
            <span
              className={`max-w-[44px] truncate text-center font-heading text-[8px] font-bold drop-shadow ${isSelected ? "text-red-300" : "text-white/80"}`}
            >
              {p.name.split(" ").pop()}
            </span>
          </>
        );

        if (onPlayerClick) {
          // div-with-button-role rather than <button>: rich tokens can embed
          // interactive controls (e.g. the role combobox), which HTML forbids
          // inside a real <button>.
          return (
            <div
              key={p.id}
              role="button"
              tabIndex={0}
              aria-label={p.name}
              className={`${sharedClass} rounded-xl focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-300/70`}
              style={sharedStyle}
              onClick={() => onPlayerClick(p.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onPlayerClick(p.id);
                }
              }}
            >
              {tokenContent}
            </div>
          );
        }
        return (
          <div key={p.id} className={sharedClass} style={sharedStyle}>
            {tokenContent}
          </div>
        );
      })}
    </div>
  );
}
