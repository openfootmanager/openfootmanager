interface ProgressBarProps {
  value: number; // 0-100
  variant?: "primary" | "accent" | "success" | "danger" | "muted" | "auto";
  size?: "sm" | "md" | "lg";
  showLabel?: boolean;
  className?: string;
}

export function ProgressBar({
  value,
  variant = "auto",
  size = "sm",
  showLabel = false,
  className = "",
}: ProgressBarProps) {
  const clamped = Math.max(0, Math.min(100, value));

  const resolvedVariant = variant === "auto"
    ? clamped >= 70 ? "success" : clamped >= 40 ? "accent" : "danger"
    : variant;

  const barColors = {
    primary: "bg-primary-500",
    accent: "bg-accent-400",
    success: "bg-success-400",
    danger: "bg-red-500",
    muted: "bg-gray-300 dark:bg-navy-500",
  };

  const heights = {
    sm: "h-1.5",
    md: "h-2.5",
    lg: "h-4",
  };

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <div className={`flex-1 bg-gray-200 dark:bg-navy-600 rounded-full ${heights[size]} overflow-hidden`}>
        <div
          className={`${barColors[resolvedVariant]} ${heights[size]} rounded-full transition-all duration-500`}
          style={{ width: `${clamped}%` }}
        />
      </div>
      {showLabel && (
        <span className="text-xs font-bold text-gray-500 dark:text-gray-400 tabular-nums min-w-[2.5rem] text-right">
          {clamped}%
        </span>
      )}
    </div>
  );
}
