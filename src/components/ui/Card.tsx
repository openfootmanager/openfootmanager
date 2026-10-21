import type { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
  accent?: "primary" | "accent" | "success" | "danger" | "none";
}

export function Card({ children, className = "", accent = "none" }: CardProps) {
  const accentBorder = {
    primary: "border-t-4 border-t-primary-500",
    accent: "border-t-4 border-t-accent-400",
    success: "border-t-4 border-t-success-400",
    danger: "border-t-4 border-t-danger-solid",
    none: "border border-line",
  }[accent];

  return (
    <div
      className={`
        bg-panel
        ${accent === "none" ? accentBorder : `border ${accentBorder} border-line`}
        rounded-xl shadow-sm dark:shadow-md
        transition-colors duration-300
        ${className}
      `}
    >
      {children}
    </div>
  );
}

interface CardHeaderProps {
  children: ReactNode;
  action?: ReactNode;
  className?: string;
}

export function CardHeader({ children, action, className = "" }: CardHeaderProps) {
  return (
    <div
      className={`px-6 py-4 border-b border-line flex items-center justify-between ${className}`}
    >
      <h3 className="text-heading font-bold font-heading uppercase tracking-label text-fg">
        {children}
      </h3>
      {action}
    </div>
  );
}

interface CardBodyProps {
  children: ReactNode;
  className?: string;
}

export function CardBody({ children, className = "" }: CardBodyProps) {
  return <div className={`p-6 ${className}`}>{children}</div>;
}
