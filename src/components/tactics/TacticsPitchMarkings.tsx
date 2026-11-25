import type { JSX } from "react";

import type { TacticsPhaseSettings } from "../../store/types";

// SVG viewBox is 0 0 100 140; pitch bounds x=[4,96] y=[4,136]; midfield at y=70.
// Team attacks upward (toward y=4). Defensive line sits in the lower half.
function getDefensiveLineY(line: TacticsPhaseSettings["defensive_line"]): number {
  switch (line) {
    case "High":
      return 77;
    case "Low":
      return 105;
    case "VeryLow":
      return 118;
    default:
      return 91; // Medium
  }
}

function getPressingZoneOpacity(intensity: TacticsPhaseSettings["pressing_intensity"]): number {
  switch (intensity) {
    case "Aggressive":
      return 0.13;
    case "Passive":
      return 0;
    default:
      return 0.07; // Medium
  }
}

function getPressingZoneTop(intensity: TacticsPhaseSettings["pressing_intensity"]): number {
  // Aggressive: press from opponent's half; Medium: press from 35m line; Passive: no zone
  switch (intensity) {
    case "Aggressive":
      return 4;
    case "Passive":
      return 70;
    default:
      return 35;
  }
}

function TacticalOverlays({ phase }: { phase: TacticsPhaseSettings }): JSX.Element {
  const lineY = getDefensiveLineY(phase.defensive_line);
  const pressOpacity = getPressingZoneOpacity(phase.pressing_intensity);
  const pressTop = getPressingZoneTop(phase.pressing_intensity);

  return (
    <>
      {/* Pressing zone: shaded band in the opponent's half */}
      {pressOpacity > 0 && (
        <rect
          x="4"
          y={pressTop}
          width="92"
          height={70 - pressTop}
          fill={`rgba(255,220,100,${pressOpacity})`}
          pointerEvents="none"
        />
      )}

      {/* Defensive line: dashed horizontal line */}
      <line
        x1="4"
        y1={lineY}
        x2="96"
        y2={lineY}
        stroke="rgba(255,80,80,0.75)"
        strokeWidth="0.8"
        strokeDasharray="3,2"
        pointerEvents="none"
      />
    </>
  );
}

/**
 * The pitch's surface and white markings, with the pressing zone and defensive line drawn over them
 * when the team's phase settings are known. Decoration only: the words are the pitch's sr-only list.
 */
export default function TacticsPitchMarkings({
  tacticsPhase,
}: {
  tacticsPhase?: TacticsPhaseSettings;
}): JSX.Element {
  return (
    <svg
      viewBox="0 0 100 140"
      preserveAspectRatio="none"
      className="absolute inset-0 h-full w-full"
      aria-hidden="true"
    >
      <defs>
        <linearGradient id="tactics-pitch-surface" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="rgba(63, 172, 99, 0.94)" />
          <stop offset="100%" stopColor="rgba(31, 109, 61, 0.98)" />
        </linearGradient>
        <pattern id="tactics-pitch-stripes" width="100" height="20" patternUnits="userSpaceOnUse">
          <rect width="100" height="10" fill="rgba(255,255,255,0.04)" />
        </pattern>
      </defs>
      <rect x="0" y="0" width="100" height="140" fill="url(#tactics-pitch-surface)" />
      <rect x="0" y="0" width="100" height="140" fill="url(#tactics-pitch-stripes)" />
      <rect
        x="4"
        y="4"
        width="92"
        height="132"
        fill="none"
        stroke="rgba(255,255,255,0.55)"
        strokeWidth="0.6"
      />
      <line x1="4" y1="70" x2="96" y2="70" stroke="rgba(255,255,255,0.55)" strokeWidth="0.6" />
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
        x="31"
        y="4"
        width="38"
        height="8"
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
        x="31"
        y="128"
        width="38"
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
      {tacticsPhase ? <TacticalOverlays phase={tacticsPhase} /> : null}
    </svg>
  );
}
