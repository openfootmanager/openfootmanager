import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WorldEditorTopBar } from "./WorldEditorTopBar";

vi.mock("react-router-dom", () => ({ useNavigate: () => () => {} }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock("../../i18n", () => ({
  default: { language: "en", changeLanguage: () => {} },
  SUPPORTED_LANGUAGES: [],
}));
vi.mock("../ui/ThemeToggle", () => ({ ThemeToggle: () => null }));
vi.mock("../ui/Select", () => ({ Select: () => null }));

function renderTopBar(issueCount: number, warningCount: number) {
  render(
    <WorldEditorTopBar
      packageName="Pkg"
      packageDir="/p"
      saveState="idle"
      isBusy={false}
      issueCount={issueCount}
      warningCount={warningCount}
      autoSave={false}
      canUndo={false}
      canRedo={false}
      isDirty={false}
      onValidate={() => {}}
      onBuild={() => {}}
      onSave={() => {}}
      onUndo={() => {}}
      onRedo={() => {}}
      onToggleAutoSave={() => {}}
    />,
  );
}

describe("WorldEditorTopBar findings", () => {
  // Given errors and warnings
  // When the top bar renders
  // Then the Validate button carries a red error count and a separate amber warning count.
  it("shows separate error and warning counts on Validate", () => {
    renderTopBar(3, 2);

    expect(screen.getByText("3").className).toContain("red");
    expect(screen.getByText("2").className).toContain("amber");
  });

  // Given only warnings
  // When it renders
  // Then there is an amber count and no red one.
  it("shows only the amber count for warnings alone", () => {
    renderTopBar(0, 2);

    expect(screen.getByText("2").className).toContain("amber");
    expect(document.querySelector(".bg-red-500")).toBeNull();
  });
});
