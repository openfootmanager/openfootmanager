import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TeamForm } from "./TeamForm";
import { emptyTeam } from "./helpers";
import type { TeamDef } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../../../hooks/useAssetDataUrl", () => ({
  useAssetDataUrl: () => null,
  evictAssetDataUrl: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key.startsWith("worldEditor.repBands.")) {
        return key.replace("worldEditor.repBands.", "");
      }
      if (key === "worldEditor.repReadoutSingle") {
        return `${params?.min}-${params?.max} of ${params?.scaleMax} · ${params?.band}`;
      }
      if (key === "worldEditor.repReadoutSpan") {
        return `${params?.min}-${params?.max} of ${params?.scaleMax} · ${params?.minBand} to ${params?.maxBand}`;
      }
      if (key === "worldEditor.budgetReadout") {
        return `${params?.min}-${params?.max} per month · transfer budget ${params?.transferMin}-${params?.transferMax}`;
      }
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function renderTeam(overrides: Partial<TeamDef>) {
  const team: TeamDef = { ...emptyTeam(), name: "Santos FC", ...overrides };
  render(
    <TeamForm
      editingTeam={team}
      editingTeamIndex={0}
      isBusy={false}
      onBack={() => {}}
      onSave={() => {}}
      updateField={() => {}}
    />,
  );
}

describe("TeamForm club range guidance", () => {
  it("reads a reputation range on the 0-1000 scale", () => {
    // 92 looks elite on a 0-100 scale and is actually below amateur. The
    // readout is what stops an author guessing the scale wrong.
    renderTeam({ reputationRange: [92, 95] });

    expect(screen.getByText("92-95 of 1000 · amateur")).toBeInTheDocument();
  });

  it("names both bands when a range straddles them", () => {
    renderTeam({ reputationRange: [700, 900] });

    expect(screen.getByText("700-900 of 1000 · midTable to elite")).toBeInTheDocument();
  });

  it("shows the transfer budget a finance range will produce", () => {
    // The author sets finance; the game derives a 15% transfer budget from it,
    // which was invisible until now.
    renderTeam({ financeRange: [3_000_000, 4_000_000] });

    expect(
      screen.getByText("3M-4M per month · transfer budget 450K-600K"),
    ).toBeInTheDocument();
  });

  it("omits the readouts when no range is set", () => {
    renderTeam({ reputationRange: null, financeRange: null });

    expect(screen.queryByText(/of 1000/)).not.toBeInTheDocument();
    expect(screen.queryByText(/transfer budget/)).not.toBeInTheDocument();
  });
});
