import { act, renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useSettingsStore } from "../store/settingsStore";
import { useListView } from "./useListView";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("useListView", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, list_view: "table" },
    });
  });

  /**
   * Given the setting at its default
   * When a list reads the view and then asks for cards
   * Then it reads "table" first, then "cards", and the new setting is saved through Rust.
   */
  it("reads list_view and writes a change back through the settings", async () => {
    const { result } = renderHook(() => useListView());
    expect(result.current[0]).toBe("table");

    await act(async () => result.current[1]("cards"));

    expect(result.current[0]).toBe("cards");
    expect(invoke).toHaveBeenCalledWith(
      "save_settings",
      expect.objectContaining({ settings: expect.objectContaining({ list_view: "cards" }) }),
    );
  });

  /**
   * Given a setting another list already changed
   * When a second list mounts
   * Then it opens in the view the first one chose.
   */
  it("opens every list in the view the setting already holds", () => {
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, list_view: "cards" },
    });
    expect(renderHook(() => useListView()).result.current[0]).toBe("cards");
  });
});
