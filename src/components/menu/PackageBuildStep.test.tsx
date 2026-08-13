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

/** The card is a button named after the package — the accessible handle a
 *  player actually uses, so tests reach for it the same way. */
function cardFor(pkg: PackageInfo): HTMLElement {
  return screen.getByRole("button", { name: new RegExp(pkg.name) });
}

describe("PackageBuildStep package card", () => {
  // Name pools are a first-class entity type that stacks per key across
  // packages, so "does my stack bring name pools at all?" is a real question.
  // Before this, two packages differing only in that one ships 40 name pools
  // and the other none looked identical here.
  it("shows the name pool, country and confederation counts", () => {
    render(<PackageBuildStep {...baseProps} installedPackages={[namePack]} />);

    const card = cardFor(namePack);
    expect(card).toHaveTextContent("worldSelect.namePools:40");
    expect(card).toHaveTextContent("worldSelect.countries:3");
    expect(card).toHaveTextContent("worldSelect.confederations:1");
  });

  it("still shows the team, player and competition counts", () => {
    render(<PackageBuildStep {...baseProps} installedPackages={[namePack]} />);

    const card = cardFor(namePack);
    expect(card).toHaveTextContent("worldSelect.teams:0");
    expect(card).toHaveTextContent("worldSelect.players:0");
    expect(card).toHaveTextContent("worldSelect.competitions:0");
  });

  // The other half of the conditional: a plain club database should not carry
  // three "0" badges it never had before. Without this the suppression branch
  // was written but never exercised.
  it("hides the three new counts when the package supplies none", () => {
    const clubsOnly: PackageInfo = {
      ...namePack,
      id: "clubs-only",
      name: "Clubs Only",
      teamCount: 20,
      playerCount: 480,
      competitionCount: 1,
      namePoolCount: 0,
      countryCount: 0,
      confederationCount: 0,
    };

    render(<PackageBuildStep {...baseProps} installedPackages={[clubsOnly]} />);

    const card = cardFor(clubsOnly);
    expect(card).toHaveTextContent("worldSelect.teams:20");
    expect(card).not.toHaveTextContent("worldSelect.namePools");
    expect(card).not.toHaveTextContent("worldSelect.countries");
    expect(card).not.toHaveTextContent("worldSelect.confederations");
  });
});
