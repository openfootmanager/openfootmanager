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
    danger: "border-t-4 border-t-red-500",
    none: "border border-gray-200 dark:border-navy-600",
  }[accent];

  return (
    <div
      className={`
        bg-white dark:bg-navy-700
        ${accent === "none" ? accentBorder : `border ${accentBorder} border-gray-200 dark:border-navy-600`}
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
      className={`px-6 py-4 border-b border-gray-100 dark:border-navy-600 flex items-center justify-between ${className}`}
    >
      <h3 className="text-lg font-bold font-heading uppercase tracking-wide text-gray-800 dark:text-gray-100">
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
