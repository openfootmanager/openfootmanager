import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentPropsWithoutRef } from "react";
import { describe, expect, it, vi } from "vitest";

import GenerationStep from "./WorldSelect";
import type { PackageInfo } from "./WorldSelect";

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

const baseProps = {
  isStarting: false,
  startYear: 2032,
  startPhase: "midSeason" as const,
  historyDepthYears: 24,
  onChangeHistoryDepthYears: vi.fn(),
  onStart: vi.fn(),
  onBack: vi.fn(),
  onClose: vi.fn(),
};

const dbPackage: PackageInfo = {
  id: "pkg-db",
  name: "Premier League",
  version: "1.0.0",
  author: "Test",
  description: "",
  license: "MIT",
  game_min_version: "1.0.0",
  package_type: "database",
  team_count: 20,
  player_count: 480,
  competition_count: 1,
  installed_path: "/path/to/pkg",
};

describe("GenerationStep (WorldSelect)", () => {
  it("shows history depth selector and summary when no database packages are active", () => {
    const onChangeHistoryDepthYears = vi.fn();

    render(
      <GenerationStep
        {...baseProps}
        onChangeHistoryDepthYears={onChangeHistoryDepthYears}
        activePackages={[]}
      />,
    );

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

  it("hides history depth selector and shows coverage section when database packages are active", () => {
    render(
      <GenerationStep
        {...baseProps}
        startPhase="seasonStart"
        activePackages={[dbPackage]}
      />,
    );

    expect(screen.getByText("generation.coverage")).toBeInTheDocument();
    expect(screen.queryByText("worldSelect.historyDepth.label")).not.toBeInTheDocument();
  });
});
