import { renderHook } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../../store/gameStore";
import { useTacticsLibrary } from "./useTacticsLibrary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, fallback?: unknown) => (typeof fallback === "string" ? fallback : key),
    i18n: { language: "en" },
  }),
}));

const mockedInvoke = vi.mocked(invoke);

function renderLibrary(gameState: GameStateData | null) {
  return renderHook(() =>
    useTacticsLibrary({
      gameState,
      formation: "4-4-2",
      activePlayStyle: "Balanced",
      initialPreset: null,
      onFormationChange: vi.fn(),
      onPlayStyleChange: vi.fn(),
      onApplyPreset: vi.fn(),
    }),
  );
}

describe("useTacticsLibrary blueprint status", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue({ modified: false });
  });

  /**
   * Given no game is loaded
   * When the hook runs
   * Then the backend is not asked about a blueprint.
   */
  it("does not ask the backend without a game", () => {
    renderLibrary(null);

    expect(mockedInvoke).not.toHaveBeenCalledWith("get_tactics_blueprint_status");
  });

  /**
   * Given a game whose manager has no club
   * When the hook runs
   * Then the backend is not asked about a blueprint.
   */
  it("does not ask the backend without a club", () => {
    const clubless = {
      clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
      teams: [],
      players: [],
      competitions: [],
      manager: { team_id: null },
    } as unknown as GameStateData;

    renderLibrary(clubless);

    expect(mockedInvoke).not.toHaveBeenCalledWith("get_tactics_blueprint_status");
  });
});
