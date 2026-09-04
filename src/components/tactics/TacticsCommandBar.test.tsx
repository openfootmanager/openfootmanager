import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import TacticsCommandBar, {
  type TacticsLibraryEntry,
} from "./TacticsCommandBar";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) =>
      typeof fallback === "string" ? fallback : key,
    i18n: { language: "en" },
  }),
}));

afterEach(() => {
  vi.useRealTimers();
});

const customTactic: TacticsLibraryEntry = {
  description: "A custom tactic",
  formation: "4-4-2",
  id: "custom:1",
  name: "My Tactic",
  playStyle: "Balanced",
  sourcePresetName: null,
  type: "custom",
};

const presetTactic: TacticsLibraryEntry = {
  description: "A preset tactic",
  formation: "4-3-3",
  id: "preset:balanced-control",
  name: "Balanced Control",
  playStyle: "Balanced",
  sourcePresetName: null,
  type: "preset",
};

function buildProps(
  overrides: Partial<React.ComponentProps<typeof TacticsCommandBar>> = {},
): React.ComponentProps<typeof TacticsCommandBar> {
  return {
    activeTactic: customTactic,
    activePlayStyle: "Balanced",
    formation: "4-4-2",
    isDirty: false,
    onCreateNew: vi.fn(),
    onDuplicate: vi.fn(),
    onFormationChange: vi.fn(),
    onPlayStyleChange: vi.fn(),
    onSave: vi.fn(),
    onSelectTactic: vi.fn(),
    tacticLibrary: [customTactic, presetTactic],
    ...overrides,
  };
}

function renderCommandBar(
  overrides: Partial<React.ComponentProps<typeof TacticsCommandBar>> = {},
) {
  return render(<TacticsCommandBar {...buildProps(overrides)} />);
}

describe("TacticsCommandBar", () => {
  it("disables the save button when the active custom tactic is already synced", () => {
    renderCommandBar({ activeTactic: customTactic, isDirty: false });

    expect(
      screen.getByRole("button", { name: "tactics.updateTactic" }),
    ).toBeDisabled();
  });

  it("enables the save button once the active custom tactic has unsaved changes", () => {
    renderCommandBar({ activeTactic: customTactic, isDirty: true });

    expect(
      screen.getByRole("button", { name: "tactics.updateTactic" }),
    ).toBeEnabled();
  });

  it("keeps the save button enabled for a preset even when nothing changed", () => {
    // Saving a preset always creates a new custom tactic, so it is never a
    // no-op even when isDirty is false.
    renderCommandBar({ activeTactic: presetTactic, isDirty: false });

    expect(
      screen.getByRole("button", { name: "tactics.saveAsTactic" }),
    ).toBeEnabled();
  });

  it("shows a temporary Saved confirmation after a successful save click", () => {
    vi.useFakeTimers();
    const onSave = vi.fn();
    const props = buildProps({ activeTactic: customTactic, isDirty: true, onSave });

    const { rerender } = render(<TacticsCommandBar {...props} />);

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.updateTactic" }),
    );
    expect(onSave).toHaveBeenCalledTimes(1);

    // A successful save syncs the stored tactic, so the parent re-renders
    // with isDirty flipped back to false.
    rerender(<TacticsCommandBar {...props} isDirty={false} />);

    expect(
      screen.getByRole("button", { name: "tactics.tacticSaved" }),
    ).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(2000);
    });

    expect(
      screen.getByRole("button", { name: "tactics.updateTactic" }),
    ).toBeInTheDocument();
  });

  it("clears the Saved cue immediately when new edits make the tactic dirty again mid-cue", () => {
    vi.useFakeTimers();
    const onSave = vi.fn();
    const props = buildProps({
      activeTactic: customTactic,
      isDirty: true,
      onSave,
    });

    const { rerender } = render(<TacticsCommandBar {...props} />);

    fireEvent.click(
      screen.getByRole("button", { name: "tactics.updateTactic" }),
    );

    // The save synced the tactic — parent re-renders with isDirty: false —
    // so the "Saved" cue becomes visible.
    rerender(<TacticsCommandBar {...props} isDirty={false} />);
    expect(
      screen.getByRole("button", { name: "tactics.tacticSaved" }),
    ).toBeInTheDocument();

    // The user edits the tactic again before the 2s cue timeout fires.
    act(() => {
      vi.advanceTimersByTime(500);
    });
    rerender(<TacticsCommandBar {...props} isDirty />);

    const saveButton = screen.getByRole("button", {
      name: "tactics.updateTactic",
    });
    expect(saveButton).toBeInTheDocument();
    expect(saveButton).toBeEnabled();
    expect(
      screen.queryByRole("button", { name: "tactics.tacticSaved" }),
    ).not.toBeInTheDocument();
  });
});
