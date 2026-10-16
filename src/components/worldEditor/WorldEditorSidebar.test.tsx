import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WorldEditorSidebar } from "./WorldEditorSidebar";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { count?: number }) =>
      options?.count === undefined ? key : `${key}:${options.count}`,
  }),
}));

function renderSidebar(issueCount: number, warningCount: number) {
  render(
    <WorldEditorSidebar
      selectedSection="metadata"
      onSelectSection={() => {}}
      confederationCount={0}
      countryCount={0}
      teamCount={0}
      playerCount={0}
      youthCount={0}
      staffCount={0}
      namePoolCount={0}
      competitionCount={0}
      issueCount={issueCount}
      warningCount={warningCount}
      onShowIssues={() => {}}
      showingIssues={false}
    />,
  );
}

describe("WorldEditorSidebar findings", () => {
  // Given only warnings
  // When the sidebar renders
  // Then it counts them, in amber, and does not claim there are no issues.
  it("counts warnings on their own, in amber", () => {
    renderSidebar(0, 2);

    const badge = screen.getByText("worldEditor.warningsBadge:2");
    expect(badge.closest("button")?.className).toContain("amber");
    expect(screen.queryByText("worldEditor.noIssues")).toBeNull();
  });

  // Given errors and warnings
  // When the sidebar renders
  // Then each has its own count.
  it("shows the error count and the warning count side by side", () => {
    renderSidebar(3, 2);

    expect(screen.getByText("worldEditor.issuesBadge:3")).toBeInTheDocument();
    expect(screen.getByText("worldEditor.warningsBadge:2")).toBeInTheDocument();
  });

  // Given nothing found
  // When the sidebar renders
  // Then it says there are no issues.
  it("says there are no issues when nothing was found", () => {
    renderSidebar(0, 0);

    expect(screen.getByText("worldEditor.noIssues")).toBeInTheDocument();
  });
});
