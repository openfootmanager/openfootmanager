interface UnreadBadgeProps {
  /** A count (capped at 99+) or a short mark such as "!". */
  count: number | string;
  className?: string;
}

/**
 * A count on a navigation item: a circle for one digit (as wide as it is tall), and more digits
 * widen it only by their own width. Its parent names the count for screen readers (the nav item's
 * accessible name says "Inbox (3)"), so the badge is plain text.
 */
export function UnreadBadge({ count, className = "" }: UnreadBadgeProps) {
  const text = typeof count === "number" && count > 99 ? "99+" : String(count);
  return (
    <span
      className={`inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-primary-700 px-1 text-caption font-bold leading-none text-on-brand tabular-nums ${className}`}
    >
      {text}
    </span>
  );
}
