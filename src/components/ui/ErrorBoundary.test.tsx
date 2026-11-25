import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ErrorBoundary } from "./ErrorBoundary";

const logError = vi.fn();
vi.mock("../../lib/logger", () => ({
  logError: (message: string) => logError(message),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const strings: Record<string, string> = {
        "errorBoundary.title": "Something went wrong",
        "errorBoundary.description": "This screen hit an error it could not recover from.",
        "errorBoundary.retry": "Try again",
        "errorBoundary.reload": "Reload the game",
        "errorBoundary.menu": "Back to the main menu",
      };
      return strings[key] ?? key;
    },
  }),
}));

function Boom({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) throw new Error("render exploded");
  return <p>all good</p>;
}

describe("ErrorBoundary", () => {
  beforeEach(() => {
    logError.mockReset();
    // React logs the caught error itself; that noise is not what these tests are about.
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /**
   * Given a tree that does not throw
   * When it renders inside the boundary
   * Then the boundary shows it and nothing else.
   */
  it("renders its children while nothing throws", () => {
    render(
      <ErrorBoundary>
        <Boom shouldThrow={false} />
      </ErrorBoundary>,
    );

    expect(screen.getByText("all good")).toBeInTheDocument();
  });

  /**
   * Given a child that throws while rendering
   * When the boundary catches it
   * Then an alert with a named heading and three named ways back stands where a blank window would
   *      have been (without a boundary one throw unmounts the whole app).
   */
  it("shows an error screen with a way back instead of a blank window", () => {
    render(
      <ErrorBoundary>
        <Boom shouldThrow={true} />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Something went wrong" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Try again" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reload the game" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back to the main menu" })).toBeInTheDocument();
  });

  /**
   * Given a child that throws while rendering
   * When the boundary catches it
   * Then the error and the component stack (which says where) are written to the log file.
   */
  it("writes the error and its component stack to the log", () => {
    render(
      <ErrorBoundary>
        <Boom shouldThrow={true} />
      </ErrorBoundary>,
    );

    expect(logError).toHaveBeenCalledTimes(1);
    const message = logError.mock.calls[0][0] as string;
    expect(message).toContain("render exploded");
    expect(message).toContain("Component stack:");
    expect(message).toContain("Boom");
  });

  /**
   * Given the error screen showing, and the cause gone
   * When the player presses Try again
   * Then the children render again.
   */
  it("renders its children again after Try again, once the cause is gone", () => {
    let shouldThrow = true;
    function Flaky() {
      return <Boom shouldThrow={shouldThrow} />;
    }
    render(
      <ErrorBoundary>
        <Flaky />
      </ErrorBoundary>,
    );
    shouldThrow = false;

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(screen.getByText("all good")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  /**
   * Given the error screen showing
   * When the player presses Reload the game, and in another case Back to the main menu
   * Then the page reloads, and goes to the app's root, respectively.
   */
  it("reloads the page, or goes back to the main menu, from the error screen", () => {
    const reload = vi.fn();
    const assign = vi.fn();
    const original = window.location;
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { ...original, reload, assign },
    });
    try {
      render(
        <ErrorBoundary>
          <Boom shouldThrow={true} />
        </ErrorBoundary>,
      );

      fireEvent.click(screen.getByRole("button", { name: "Reload the game" }));
      expect(reload).toHaveBeenCalledTimes(1);
      fireEvent.click(screen.getByRole("button", { name: "Back to the main menu" }));
      expect(assign).toHaveBeenCalledWith("/");
    } finally {
      Object.defineProperty(window, "location", { configurable: true, value: original });
    }
  });

  /**
   * Given a boundary that caught an error, remounted under a new key (as the dashboard does when
   *       the player opens another tab)
   * When the new key renders a healthy child
   * Then the error screen is gone.
   */
  it("clears the error screen when it is remounted under a new key", () => {
    const { rerender } = render(
      <ErrorBoundary key="tactics">
        <Boom shouldThrow={true} />
      </ErrorBoundary>,
    );
    expect(screen.getByRole("alert")).toBeInTheDocument();

    rerender(
      <ErrorBoundary key="squad">
        <Boom shouldThrow={false} />
      </ErrorBoundary>,
    );

    expect(screen.getByText("all good")).toBeInTheDocument();
  });
});
