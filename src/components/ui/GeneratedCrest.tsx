import { useId } from "react";

interface CrestColors {
  primary?: string;
  secondary?: string;
}

interface GeneratedCrestProps {
  /** Stable seed for shape/colour variety (use the club name or id). */
  name: string;
  /** Short text shown on the crest (e.g. the club short name). */
  label: string;
  /** The club's colours; derived from `name` when absent. */
  colors?: CrestColors;
  className?: string;
}

function hashString(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash;
}

/** A pleasant deterministic colour when a club declares none. */
function derivedColor(seed: number, light: boolean): string {
  const hue = seed % 360;
  return `hsl(${hue}, 55%, ${light ? "68%" : "40%"})`;
}

/** Black or white text, whichever reads better on a `#rrggbb` background. */
function readableTextColor(color: string): string {
  const match = /^#?([0-9a-f]{6})$/i.exec(color.trim());
  if (!match) return "#ffffff";
  const value = parseInt(match[1], 16);
  const r = (value >> 16) & 0xff;
  const g = (value >> 8) & 0xff;
  const b = value & 0xff;
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.6 ? "#111827" : "#ffffff";
}

/**
 * A procedurally generated club crest: a rounded tile in the club's two
 * colours with its short name, varied by a hash of the name so clubs look
 * distinct without any image files. Used as the `TeamLogo` fallback so every
 * club has a visual identity even when no logo media is provided.
 */
export function GeneratedCrest({ name, label, colors, className }: GeneratedCrestProps) {
  const clipId = useId();
  const seed = hashString(name);
  const primary = colors?.primary?.trim() || derivedColor(seed, false);
  const secondary = colors?.secondary?.trim() || derivedColor(seed >> 3, true);
  const text = label.slice(0, 3).toUpperCase();
  const textColor = readableTextColor(primary);
  const variant = seed % 4;

  return (
    <svg
      viewBox="0 0 64 64"
      className={className}
      role="presentation"
      aria-hidden="true"
    >
      <clipPath id={clipId}>
        <rect x="2" y="2" width="60" height="60" rx="14" />
      </clipPath>
      <g clipPath={`url(#${clipId})`}>
        <rect x="2" y="2" width="60" height="60" fill={primary} />
        {variant === 0 && <rect x="32" y="0" width="32" height="64" fill={secondary} />}
        {variant === 1 && <rect x="0" y="40" width="64" height="24" fill={secondary} />}
        {variant === 2 && <polygon points="0,0 30,0 0,64" fill={secondary} />}
        {variant === 3 && (
          <polygon points="32,2 62,32 32,62 2,32" fill={secondary} opacity="0.9" />
        )}
      </g>
      <rect
        x="2"
        y="2"
        width="60"
        height="60"
        rx="14"
        fill="none"
        stroke={secondary}
        strokeWidth="2"
      />
      <text
        x="32"
        y="34"
        textAnchor="middle"
        dominantBaseline="central"
        fontSize={text.length > 2 ? 17 : 22}
        fontWeight="800"
        fill={textColor}
        style={{ fontFamily: "inherit", paintOrder: "stroke" }}
        stroke={primary}
        strokeWidth="0.6"
      >
        {text}
      </text>
    </svg>
  );
}

export default GeneratedCrest;
