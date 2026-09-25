import { Component, type ErrorInfo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle } from "lucide-react";

import { logError } from "../../lib/logger";
import { Button } from "./Button";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | undefined;
}

/**
 * Catches a render error so it becomes a screen the player can act on.
 *
 * Without a boundary anywhere in the tree, one throw during render unmounts the whole app and
 * leaves a white window: no message, nothing in the log file, and nothing for a bug report to
 * describe. The player's only move is to kill the process, and ours is to guess.
 *
 * A class component because `componentDidCatch` has no hook equivalent. The fallback is split out
 * as a function component so it can use `useTranslation` — calling a hook from a class is not
 * possible, and inlining English here would put untranslated text in front of eleven other
 * languages at the exact moment the app is least trustworthy.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: undefined };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // The component stack is the half that says *where*. React prints it to the console, which on
    // a player's machine is nowhere, so it has to be written into the message we send to disk.
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
    return <ErrorFallback onRetry={this.handleRetry} />;
  }
}

function ErrorFallback({ onRetry }: { onRetry: () => void }) {
  const { t } = useTranslation();

  return (
    <div
      role="alert"
      className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-navy-900 px-6 py-12"
    >
      <div className="w-full max-w-md bg-white dark:bg-navy-800 rounded-2xl border border-gray-200 dark:border-navy-700 shadow-sm p-8 text-center">
        <AlertTriangle aria-hidden="true" className="w-10 h-10 mx-auto text-accent-400" />
        <h1 className="mt-4 font-heading font-bold uppercase tracking-wider text-xl text-gray-900 dark:text-gray-100">
          {t("errorBoundary.title")}
        </h1>
        <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
          {t("errorBoundary.description")}
        </p>
        <div className="mt-6 flex flex-col sm:flex-row gap-3 justify-center">
          <Button onClick={onRetry}>{t("errorBoundary.retry")}</Button>
          <Button
            variant="outline"
            onClick={() => {
              window.location.reload();
            }}
          >
            {t("errorBoundary.reload")}
          </Button>
        </div>
      </div>
    </div>
  );
}
