import type { ReactNode, ButtonHTMLAttributes } from "react";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "accent" | "ghost" | "outline";
  size?: "sm" | "md" | "lg";
  children: ReactNode;
  icon?: ReactNode;
  iconRight?: ReactNode;
}

export function Button({
  variant = "primary",
  size = "md",
  children,
  icon,
  iconRight,
  className = "",
  disabled,
  ...props
}: ButtonProps) {
  const base =
    "inline-flex items-center justify-center gap-2 font-heading font-bold uppercase tracking-label rounded-lg transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed";

  const variants = {
    primary:
      "bg-primary-700 hover:bg-primary-800 active:bg-primary-900 text-on-brand shadow-md hover:shadow-lg focus:ring-primary-700 dark:focus:ring-offset-navy-800",
    accent:
      "bg-accent-400 hover:bg-accent-500 active:bg-accent-600 text-navy-900 shadow-md hover:shadow-lg focus:ring-accent-400 dark:focus:ring-offset-navy-800",
    ghost:
      "bg-transparent hover:bg-hover text-fg-muted hover:text-fg focus:ring-line-strong dark:focus:ring-offset-navy-800",
    outline:
      "bg-transparent border-2 border-line-strong hover:border-primary-500 dark:hover:border-primary-400 text-fg-muted hover:text-positive focus:ring-primary-500 dark:focus:ring-offset-navy-800",
  };

  const sizes = {
    sm: "px-3 py-1.5 text-caption",
    md: "px-5 py-2.5 text-body",
    lg: "px-7 py-3.5 text-lead",
  };

  return (
    <button
      className={`${base} ${variants[variant]} ${sizes[size]} ${className}`}
      disabled={disabled}
      {...props}
    >
      {icon && <span className="[&>svg]:w-4 [&>svg]:h-4">{icon}</span>}
      {children}
      {iconRight && <span className="[&>svg]:w-4 [&>svg]:h-4">{iconRight}</span>}
    </button>
  );
}
