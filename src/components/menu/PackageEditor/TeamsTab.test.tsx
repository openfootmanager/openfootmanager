import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TeamsTab } from "./TeamsTab";
import { emptyTeam } from "./helpers";
import type { TeamDef } from "./types";

const useAssetDataUrl = vi.fn(() => null);

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../hooks/useAssetDataUrl", () => ({
  useAssetDataUrl: (...args: unknown[]) => useAssetDataUrl(...(args as [])),
  evictAssetDataUrl: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: string | Record<string, string | number>) => {
      if (opts && typeof opts === "object") {
        const parts = Object.entries(opts)
          .filter(([name]) => name !== "defaultValue")
          .map(([name, value]) => `${name}=${value}`);
        return parts.length > 0 ? `${key} ${parts.join(" ")}` : key;
      }
      return typeof opts === "string" ? opts : key;
    },
    i18n: { language: "en" },
  }),
}));

function teams(count: number, overrides: (i: number) => Partial<TeamDef> = () => ({})): TeamDef[] {
  return Array.from({ length: count }, (_, i) => ({
    ...emptyTeam(),
    id: `t${i}`,
    name: `Team ${i}`,
    ...overrides(i),
  }));
}

function renderTab(props: Partial<React.ComponentProps<typeof TeamsTab>> = {}) {
  const onEdit = vi.fn();
  render(
    <TeamsTab
      teams={teams(500)}
      onAdd={() => {}}
      onEdit={onEdit}
      onDelete={() => {}}
      {...props}
    />,
  );
  return { onEdit };
}

function editButtons() {
  return screen.queryAllByRole("button", { name: "worldEditor.editTeam" });
}

describe("TeamsTab", () => {
  it("renders one page of rows rather than every team", () => {
    renderTab();

    expect(editButtons()).toHaveLength(50);
    expect(screen.getByText("worldEditor.showingEntries shown=50 total=500")).toBeInTheDocument();
  });

  it("reveals another page on request", () => {
    renderTab();

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));

    expect(editButtons()).toHaveLength(100);
  });

  it("still addresses a revealed row by its index in the whole list", () => {
    const { onEdit } = renderTab();

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));
    fireEvent.click(editButtons()[50]);

    expect(onEdit).toHaveBeenCalledWith(50);
  });

  it("says so when a search matches nothing, instead of going blank", () => {
    renderTab();

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchTeams" }), {
      target: { value: "no such club" },
    });

    expect(editButtons()).toHaveLength(0);
    expect(screen.getByText("common.noResults")).toBeInTheDocument();
  });

  it("reads at most one page of crests on mount", () => {
    useAssetDataUrl.mockClear();
    renderTab({ teams: teams(500, () => ({ logo: "logos/a.png" })), projectDir: "/pkg" });

    expect(useAssetDataUrl.mock.calls.length).toBeLessThanOrEqual(50);
  });

  it("leaves a short list uncapped", () => {
    renderTab({ teams: teams(4) });

    expect(editButtons()).toHaveLength(4);
    expect(screen.queryByRole("button", { name: "common.loadMore" })).not.toBeInTheDocument();
  });
});
