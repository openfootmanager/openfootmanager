interface GeneratedAvatarProps {
  /** Stable seed for the background colour (use the player's full name or id). */
  name: string;
  /** Short text shown on the avatar (e.g. the player's initials). */
  initials: string;
  className?: string;
}

function hashString(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash;
}

/**
 * A procedurally generated player avatar: an initial disc in a deterministic
 * colour derived from the player's name. Used as the `PlayerAvatar` fallback so
 * every player has a visual identity even when no face media is provided.
 */
export function GeneratedAvatar({ name, initials, className }: GeneratedAvatarProps) {
  const hue = hashString(name) % 360;
  const background = `hsl(${hue}, 42%, 46%)`;
  const text = initials.slice(0, 2).toUpperCase();

  return (
    <svg
      viewBox="0 0 40 40"
      className={className}
      role="presentation"
      aria-hidden="true"
    >
      <rect width="40" height="40" rx="9" fill={background} />
      <text
        x="20"
        y="21"
        textAnchor="middle"
        dominantBaseline="central"
        fontSize="15"
        fontWeight="700"
        fill="#ffffff"
        style={{ fontFamily: "inherit" }}
      >
        {text}
      </text>
    </svg>
  );
}

export default GeneratedAvatar;
