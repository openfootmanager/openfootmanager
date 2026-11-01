import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PlayersTab } from "./PlayersTab";
import { emptyPlayer } from "./helpers";

// The backend's position table has not arrived: the stand-in is empty, so `loaded` is false.
vi.mock("../../../hooks/usePositionGroups", async () => {
  const { createPositionTable } = await import("../../../lib/positionTable");
  const table = createPositionTable([]);
  return {
    usePositionGroups: () => () => null,
    usePositionTable: () => table,
    resetPositionGroupsCache: () => {},
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../hooks/useAssetDataUrl", () => ({
  useAssetDataUrl: () => null,
  evictAssetDataUrl: vi.fn(),
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: string | Record<string, string | number>) =>
      typeof opts === "string" ? opts : key,
    i18n: { language: "en" },
  }),
}));

describe("PlayersTab position filter before the position table loads", () => {
  /**
   * Given a package's players and a position table that has not arrived
   * When the position filter is opened
   * Then the "all <group>" options are disabled (a group would match no one), while "All" and
   *      the exact positions stay enabled.
   */
  it("disables the group options and keeps All and the exact positions", () => {
    render(
      <PlayersTab
        players={[{ ...emptyPlayer(), id: "cb", name: "Ben", position: "CenterBack" }]}
        onAdd={() => {}}
        onEdit={() => {}}
        onDelete={() => {}}
      />,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "worldEditor.filterByPosition" }));

    expect(
      screen.getByRole("option", { name: "worldEditor.positionGroupFilter.Defender" }),
    ).toBeDisabled();
    expect(screen.getByRole("option", { name: "worldEditor.allPositions" })).toBeEnabled();
    expect(screen.getByRole("option", { name: "common.positions.CenterBack" })).toBeEnabled();
  });
});
