import { render, screen } from "@testing-library/react";
import type { ComponentPropsWithoutRef } from "react";
import { describe, expect, it, vi } from "vitest";

import PackageBuildStep from "./PackageBuildStep";
import type { PackageInfo } from "./WorldSelect";

const invoke = vi.fn().mockResolvedValue([]);
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { count?: number }) =>
      options?.count === undefined ? key : `${key}:${options.count}`,
  }),
}));

vi.mock("../ui", () => ({
  Button: ({ children, iconRight: _iconRight, ...props }: ComponentPropsWithoutRef<"button"> & { iconRight?: unknown }) => (
    <button {...props}>{children}</button>
  ),
}));

const namePack: PackageInfo = {
  id: "brazil-names",
  name: "Brazilian Name Pools",
  version: "1.0.0",
  author: "Test",
  description: "",
  license: "CC0-1.0",
  gameMinVersion: "0.3.0",
  packageType: "database",
  teamCount: 0,
  playerCount: 0,
  competitionCount: 0,
  namePoolCount: 40,
  countryCount: 3,
  confederationCount: 1,
  installedPath: "/packages/brazil-names.ofm",
};

const baseProps = {
  activePackageIds: [],
  isInstallingPackage: false,
  onTogglePackage: vi.fn(),
  onInstallPackage: vi.fn(),
  onUninstallPackage: vi.fn(),
  onNext: vi.fn(),
  onBack: vi.fn(),
  onClose: vi.fn(),
};

describe("PackageBuildStep package card", () => {
  // Name pools are a first-class entity type that stacks per key across
  // packages, so "does my stack cover the nationalities my teams use?" is a
  // real question. Before this, two packages differing only in that one ships
  // 40 name pools and the other none looked identical here.
  it("shows the name pool, country and confederation counts", () => {
    render(<PackageBuildStep {...baseProps} installedPackages={[namePack]} />);

    expect(screen.getByText("worldSelect.namePools:40")).toBeInTheDocument();
    expect(screen.getByText("worldSelect.countries:3")).toBeInTheDocument();
    expect(screen.getByText("worldSelect.confederations:1")).toBeInTheDocument();
  });

  it("still shows the team, player and competition counts", () => {
    render(<PackageBuildStep {...baseProps} installedPackages={[namePack]} />);

    expect(screen.getByText("worldSelect.teams:0")).toBeInTheDocument();
    expect(screen.getByText("worldSelect.players:0")).toBeInTheDocument();
    expect(screen.getByText("worldSelect.competitions:0")).toBeInTheDocument();
  });
});
