import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentPropsWithoutRef } from "react";
import { describe, expect, it, vi } from "vitest";

import WorldSelect from "./WorldSelect";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { year?: number; count?: number }) => {
      if (key === "worldSelect.summary.midSeason.generated") {
        return `worldSelect.summary.midSeason.generated:${options?.year ?? "missing"}:${options?.count ?? "missing"}`;
      }

      if (key === "worldSelect.historyDepth.applied") {
        return `worldSelect.historyDepth.applied:${options?.count ?? "missing"}`;
      }

      if (key === "worldSelect.historyDepth.option") {
        return `worldSelect.historyDepth.option:${options?.count ?? "missing"}`;
      }

      return key;
    },
  }),
}));

vi.mock("../ui", () => ({
  Button: ({ children, iconRight: _iconRight, ...props }: ComponentPropsWithoutRef<"button"> & { iconRight?: unknown }) => (
    <button {...props}>{children}</button>
  ),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendText: (value: string) => value,
}));

describe("WorldSelect", () => {
  it("shows the selected world history mode, configurable depth, and mid-season inheritance summary", () => {
    const onChangeHistoryDepthYears = vi.fn();

    render(
      <WorldSelect
        worldDatabases={[
          {
            id: "random",
            name: "Random World",
            description: "Fresh roster baseline",
            team_count: 16,
            player_count: 352,
            source: "builtin",
            path: "",
            history_mode: "reference",
          },
        ]}
        selectedWorldId="random"
        isLoadingWorlds={false}
        isStarting={false}
        startYear={2032}
        startPhase="midSeason"
        historyDepthYears={24}
        onSelectWorld={vi.fn()}
        onChangeHistoryDepthYears={onChangeHistoryDepthYears}
        onImportFile={vi.fn()}
        onStart={vi.fn()}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(
      screen.getAllByText("worldSelect.historyMode.generated"),
    ).toHaveLength(2);
    expect(
      screen.getByText("worldSelect.summary.midSeason.generated:2032:24"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("worldSelect.historyDepth.applied:24"),
    ).toBeInTheDocument();
    expect(screen.getByText("worldSelect.summary.startYear")).toBeInTheDocument();

    fireEvent.click(screen.getByText("worldSelect.historyDepth.option:6"));

    expect(onChangeHistoryDepthYears).toHaveBeenCalledWith(6);
  });

  it("shows the generated history control as inactive for reference worlds", () => {
    render(
      <WorldSelect
        worldDatabases={[
          {
            id: "historic",
            name: "Historical Snapshot",
            description: "Reference world",
            team_count: 20,
            player_count: 480,
            source: "builtin",
            path: "",
            history_mode: "reference",
          },
        ]}
        selectedWorldId="historic"
        isLoadingWorlds={false}
        isStarting={false}
        startYear={2032}
        startPhase="seasonStart"
        historyDepthYears={12}
        onSelectWorld={vi.fn()}
        onChangeHistoryDepthYears={vi.fn()}
        onImportFile={vi.fn()}
        onStart={vi.fn()}
        onBack={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("worldSelect.historyDepth.reference")).toBeInTheDocument();
    expect(
      screen.getByText("worldSelect.historyDepth.option:12").closest("button"),
    ).toBeDisabled();
  });

  const baseWorldProps = {
    worldDatabases: [
      {
        id: "random",
        name: "Random World",
        description: "Fresh roster baseline",
        team_count: 16,
        player_count: 352,
        source: "builtin",
        path: "",
      },
    ],
    selectedWorldId: "random",
    isLoadingWorlds: false,
    isStarting: false,
    startYear: 2032,
    startPhase: "midSeason" as const,
    historyDepthYears: 12,
    onSelectWorld: vi.fn(),
    onChangeHistoryDepthYears: vi.fn(),
    onImportFile: vi.fn(),
    onStart: vi.fn(),
    onBack: vi.fn(),
    onClose: vi.fn(),
  };

  it("offers the competitions file picker and lists validation errors", () => {
    render(
      <WorldSelect
        {...baseWorldProps}
        onImportCompetitionDefs={vi.fn()}
        onClearCompetitionDefs={vi.fn()}
        competitionDefsFileName="my-comps.json"
        competitionDefsErrors={[
          { code: "be.error.competitionDef.unknownTeam", competition_id: "tr-1", params: { team: "ghost" } },
        ]}
      />,
    );

    expect(screen.getByText("my-comps.json")).toBeInTheDocument();
    const errorBox = screen.getByTestId("competition-defs-errors");
    expect(errorBox).toHaveTextContent("be.error.competitionDef.unknownTeam");
    expect(errorBox).toHaveTextContent("[tr-1]");
    // A file with errors blocks starting the career.
    expect(
      screen.getByText("worldSelect.startCareer").closest("button"),
    ).toBeDisabled();
  });

  it("hides the competitions picker when no handler is provided", () => {
    render(<WorldSelect {...baseWorldProps} />);
    expect(screen.queryByTestId("competition-defs-input")).not.toBeInTheDocument();
    expect(
      screen.getByText("worldSelect.startCareer").closest("button"),
    ).not.toBeDisabled();
  });
});