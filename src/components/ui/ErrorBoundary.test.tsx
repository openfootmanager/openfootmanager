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

  it("renders its children while nothing throws", () => {
    render(
      <ErrorBoundary>
        <Boom shouldThrow={false} />
      </ErrorBoundary>,
    );

    expect(screen.getByText("all good")).toBeInTheDocument();
  });

  it("shows a recoverable fallback instead of a blank screen", () => {
    render(
      <ErrorBoundary>
        <Boom shouldThrow={true} />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("heading", { name: "Something went wrong" })).toBeInTheDocument();
    // Queried by role and name, so a fallback that loses its accessible name fails here.
    expect(screen.getByRole("button", { name: "Try again" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reload the game" })).toBeInTheDocument();
  });

  it("writes the error and component stack to the log file", () => {
    render(
      <ErrorBoundary>
        <Boom shouldThrow={true} />
      </ErrorBoundary>,
    );

    expect(logError).toHaveBeenCalledTimes(1);
    const message = logError.mock.calls[0][0] as string;
    expect(message).toContain("render exploded");
    // The component stack is the part that says *where*; without it a report only says that
    // something threw.
    expect(message).toContain("Boom");
  });

  it("recovers when the child stops throwing", () => {
    // The throw is driven from outside the component. React re-invokes a failing render while it
    // works out where the error came from, so a component that decides for itself whether to
    // throw flips its own flag on a render the test never asked for.
    let shouldThrow = true;
    function Flaky() {
      if (shouldThrow) throw new Error("render exploded");
      return <p>all good</p>;
    }

    render(
      <ErrorBoundary>
        <Flaky />
      </ErrorBoundary>,
    );
    expect(screen.getByRole("heading", { name: "Something went wrong" })).toBeInTheDocument();

    shouldThrow = false;
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(screen.getByText("all good")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Something went wrong" })).not.toBeInTheDocument();
  });
});
