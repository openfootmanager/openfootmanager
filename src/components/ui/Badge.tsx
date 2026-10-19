import type { ReactNode } from "react";

interface BadgeProps {
  children: ReactNode;
  variant?: "primary" | "accent" | "success" | "danger" | "neutral";
  size?: "sm" | "md";
  className?: string;
}

export function Badge({ children, variant = "neutral", size = "sm", className = "" }: BadgeProps) {
  const variants = {
    primary: "bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300",
    accent: "bg-accent-100 text-accent-700 dark:bg-accent-900/40 dark:text-accent-300",
    success: "bg-positive-tint text-positive",
    danger: "bg-danger-tint text-danger",
    // The inset line keeps it visible on a white panel, where sunken alone is 1.04:1.
    neutral: "bg-sunken text-fg-muted ring-1 ring-inset ring-line",
  };

  const sizes = {
    sm: "px-2 py-0.5 text-caption",
    md: "px-3 py-1 text-body",
  };

  return (
    <span
      className={`inline-flex items-center font-bold font-heading uppercase tracking-label rounded-md ${variants[variant]} ${sizes[size]} ${className}`}
    >
      {children}
    </span>
  );
}
