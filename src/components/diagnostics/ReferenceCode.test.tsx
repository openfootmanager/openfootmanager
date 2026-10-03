import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const logError = vi.fn();
vi.mock("../../lib/logger", () => ({
  logError: (message: string) => logError(message),
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

import { ReferenceCode } from "./ReferenceCode";

describe("ReferenceCode", () => {
  const writeText = vi.fn();

  beforeEach(() => {
    writeText.mockReset().mockResolvedValue(undefined);
    logError.mockReset();
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /**
   * Given a reference code on screen,
   * when a keyboard user activates the copy button,
   * then the exact code goes to the clipboard and "Copied" is announced.
   */
  it("copies the code from a keyboard-reachable button and announces it", async () => {
    render(<ReferenceCode code="7K2M9Q4R" />);

    expect(screen.getByText("7K2M9Q4R")).toBeInTheDocument();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "reportBug.copyCode" }));
    });

    expect(writeText).toHaveBeenCalledWith("7K2M9Q4R");
    expect(screen.getByRole("status")).toHaveTextContent("reportBug.codeCopied");
  });

  /**
   * Given a clipboard the webview refuses to write to,
   * when the copy button is pressed,
   * then nothing claims the code was copied and the failure is logged.
   */
  it("does not claim a copy that failed", async () => {
    writeText.mockRejectedValue(new Error("denied"));
    render(<ReferenceCode code="7K2M9Q4R" />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "reportBug.copyCode" }));
    });

    expect(screen.getByRole("status")).not.toHaveTextContent("reportBug.codeCopied");
    expect(logError).toHaveBeenCalled();
  });
});
