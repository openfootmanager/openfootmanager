import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { useFinanceActions } from "./useFinanceActions";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");

  return {
    initReactI18next: {
      type: "3rdParty",
      init: () => {},
    },
    useTranslation: () => {
      useRef(null);
      return {
        t: (key: string) => key,
        i18n: { language: "en" },
      };
    },
  };
});

const mockedInvoke = vi.mocked(invoke);

describe("useFinanceActions envelope reallocation", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  /**
   * Given a reallocation is already in flight
   * When the manager presses raise again before it finishes
   * Then only one board-envelope command is sent.
   */
  it("does not start a second reallocation while one is in flight", async () => {
    mockedInvoke.mockImplementation(() => new Promise(() => {}));
    const { result } = renderHook(() => useFinanceActions());

    act(() => {
      void result.current.handleReallocateBoardEnvelopes(1_000);
      void result.current.handleReallocateBoardEnvelopes(1_000);
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledTimes(1);
    });
    expect(mockedInvoke).toHaveBeenCalledWith(
      "reallocate_board_envelopes",
      expect.objectContaining({ wageDeltaWeekly: 1_000 }),
    );
  });
});
