import type { ReactNode } from "react";

import { ThemeToggle } from "../ui";

interface MatchScreenLayoutProps {
  children: ReactNode;
  contentClassName?: string;
  footer?: ReactNode;
  header?: ReactNode;
  headerClassName?: string;
  headerContentClassName?: string;
  showThemeToggle?: boolean;
  themeToggleClassName?: string;
}

function joinClasses(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(" ");
}

export default function MatchScreenLayout({
  children,
  contentClassName,
  footer,
  header,
  headerClassName,
  headerContentClassName,
  showThemeToggle = true,
  themeToggleClassName,
}: MatchScreenLayoutProps) {
  return (
    <div className="min-h-screen bg-gray-100 text-gray-900 dark:bg-navy-900 dark:text-white flex flex-col transition-colors duration-300">
      {header && (
        <header
          className={joinClasses("border-b border-gray-200 dark:border-navy-700", headerClassName)}
        >
          <div className={joinClasses("relative mx-auto w-full px-6", headerContentClassName)}>
            <div className={showThemeToggle ? "pr-14" : undefined}>{header}</div>
            {showThemeToggle && (
              <ThemeToggle
                className={joinClasses("absolute right-6 top-4", themeToggleClassName)}
              />
            )}
          </div>
        </header>
      )}

      <div className={joinClasses("flex-1", contentClassName)}>{children}</div>

      {footer}
    </div>
  );
}
