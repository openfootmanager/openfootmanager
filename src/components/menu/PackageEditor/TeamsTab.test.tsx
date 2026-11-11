import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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
    <div data-testid="scroller" data-scroll-container="">
      <TeamsTab
        teams={teams(500)}
        onAdd={() => {}}
        onEdit={onEdit}
        onDelete={() => {}}
        {...props}
      />
    </div>,
  );
  return { onEdit };
}

function editButtons() {
  return screen.queryAllByRole("button", { name: "worldEditor.editTeam" });
}

describe("TeamsTab", () => {
  it("renders a window of rows rather than every team", () => {
    renderTab();

    expect(editButtons().length).toBeGreaterThan(5);
    expect(editButtons().length).toBeLessThan(40);
    expect(screen.getByText("common.nResults count=500")).toBeInTheDocument();
  });

  it("offers no Load more: the rest of the list is a scroll away", () => {
    renderTab();

    expect(screen.queryByRole("button", { name: "common.loadMore" })).not.toBeInTheDocument();
  });

  it("still addresses a row it scrolled to by its index in the whole list", async () => {
    const { onEdit } = renderTab();
    const scroller = screen.getByTestId("scroller");

    scroller.scrollTop = 300 * 72;
    fireEvent.scroll(scroller);
    await waitFor(() => expect(screen.getByText("Team 300")).toBeInTheDocument());
    const row = screen.getByText("Team 300").closest('[role="listitem"]') as HTMLElement;
    fireEvent.click(row.querySelector('[title="worldEditor.editTeam"]') as HTMLElement);

    expect(onEdit).toHaveBeenCalledWith(300);
  });

  it("says so when a search matches nothing, instead of going blank", () => {
    renderTab();

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchTeams" }), {
      target: { value: "no such club" },
    });

    expect(editButtons()).toHaveLength(0);
    expect(screen.getByText("common.noResults")).toBeInTheDocument();
  });

  it("reads only the crests of the rows in the window on mount", () => {
    useAssetDataUrl.mockClear();
    renderTab({ teams: teams(500, () => ({ logo: "logos/a.png" })), projectDir: "/pkg" });

    expect(useAssetDataUrl.mock.calls.length).toBeLessThanOrEqual(40);
  });

  it("leaves a short list uncapped", () => {
    renderTab({ teams: teams(4) });

    expect(editButtons()).toHaveLength(4);
  });
});
