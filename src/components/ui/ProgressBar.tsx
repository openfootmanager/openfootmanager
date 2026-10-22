import type { RatedTone } from "../../store/types";
import { TONE_FILL } from "./ratedTone";

interface ProgressBarProps {
  value: number; // 0-100
  variant?: "primary" | "accent" | "success" | "danger" | "muted" | "auto";
  /** The backend's tone for the value; when given it fills the bar and `variant` is ignored. */
  tone?: RatedTone;
  size?: "sm" | "md" | "lg";
  showLabel?: boolean;
  /**
   * What the bar measures, translated. With it the bar is a named progress bar; without it the
   * bar is plain decoration, never an unnamed progress bar announced with no idea what it measures.
   */
  label?: string;
  className?: string;
}

export function ProgressBar({
  value,
  variant = "auto",
  tone,
  size = "sm",
  showLabel = false,
  label,
  className = "",
}: ProgressBarProps) {
  const clamped = Math.max(0, Math.min(100, value));

  const resolvedVariant =
    variant === "auto"
      ? clamped >= 70
        ? "success"
        : clamped >= 40
          ? "warning"
          : "danger"
      : variant;

  const barColors = {
    primary: "bg-primary-500",
    accent: "bg-accent-400",
    // The auto scale is a status, not the brand, so it fills with the status solids.
    success: "bg-positive-solid",
    warning: "bg-warning-solid",
    danger: "bg-danger-solid",
    muted: "bg-fg-subtle",
  };

  const heights = {
    sm: "h-1.5",
    md: "h-2.5",
    lg: "h-4",
  };

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <div
        className={`flex-1 bg-line rounded-full ${heights[size]} overflow-hidden`}
        {...(label
          ? {
              role: "progressbar",
              "aria-label": label,
              "aria-valuenow": clamped,
              "aria-valuemin": 0,
              "aria-valuemax": 100,
            }
          : {})}
      >
        <div
          className={`${tone ? TONE_FILL[tone] : barColors[resolvedVariant]} ${heights[size]} rounded-full transition-all duration-500`}
          style={{ width: `${clamped}%` }}
        />
      </div>
      {showLabel && (
        <span className="text-caption font-bold text-fg-muted tabular-nums min-w-[2.5rem] text-right">
          {clamped}%
        </span>
      )}
    </div>
  );
}
