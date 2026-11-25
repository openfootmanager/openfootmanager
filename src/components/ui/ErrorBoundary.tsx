import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

import { logError } from "../../lib/logger";
import { Button } from "./Button";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Fills the window (the app's root); false inside a page, which keeps the rest of the screen. */
  fullScreen?: boolean;
}

interface ErrorBoundaryState {
  error: Error | undefined;
}

/**
 * Catches a render error so it becomes a screen the player can act on.
 *
 * Without a boundary anywhere in the tree, one throw during render unmounts the whole app and
 * leaves a blank window: no message, nothing in the log file, and nothing for a bug report to
 * describe. A boundary is remounted under a new `key` to clear it (the dashboard does so when the
 * player opens another tab).
 *
 * A class component because `componentDidCatch` has no hook equivalent; the fallback is a function
 * component so it can use `useTranslation`.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: undefined };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // The component stack says where. React prints it to the console, which on a player's machine is
    // nowhere, so it is written into the message that goes to disk.
    logError(
      `[ui] Unhandled render error: ${error.stack ?? error.message}\nComponent stack:${
        errorInfo.componentStack ?? " (unavailable)"
      }`,
    );
  }

  private handleRetry = (): void => {
    this.setState({ error: undefined });
  };

  render(): ReactNode {
    if (this.state.error === undefined) return this.props.children;
    return <ErrorFallback fullScreen={this.props.fullScreen ?? true} onRetry={this.handleRetry} />;
  }
}

function ErrorFallback({ fullScreen, onRetry }: { fullScreen: boolean; onRetry: () => void }) {
  const { t } = useTranslation();

  return (
    <div
      role="alert"
      className={`flex items-center justify-center px-6 py-12 ${
        fullScreen ? "min-h-screen bg-gray-100 dark:bg-navy-900" : ""
      }`}
    >
      <div className="w-full max-w-md rounded-xl border border-gray-200 bg-white p-8 text-center dark:border-navy-600 dark:bg-navy-800">
        <AlertTriangle aria-hidden="true" className="mx-auto h-10 w-10 text-accent-400" />
        <h1 className="mt-4 font-heading text-xl font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
          {t("errorBoundary.title")}
        </h1>
        <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">
          {t("errorBoundary.description")}
        </p>
        <div className="mt-6 flex flex-col justify-center gap-3 sm:flex-row sm:flex-wrap">
          <Button onClick={onRetry}>{t("errorBoundary.retry")}</Button>
          <Button
            variant="outline"
            onClick={() => {
              window.location.reload();
            }}
          >
            {t("errorBoundary.reload")}
          </Button>
          <Button
            variant="outline"
            onClick={() => {
              window.location.assign("/");
            }}
          >
            {t("errorBoundary.menu")}
          </Button>
        </div>
      </div>
    </div>
  );
}
