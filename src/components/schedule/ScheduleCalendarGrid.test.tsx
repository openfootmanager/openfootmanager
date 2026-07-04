import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import ScheduleCalendarGrid from "./ScheduleCalendarGrid";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    i18n: { language: "en" },
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

describe("ScheduleCalendarGrid", () => {
  // Regression: viewYear/viewMonth were seeded from focusDate only on mount,
  // so when the schedule slice refreshed and the next user match moved into a
  // new month, the calendar kept showing the stale month.
  it("re-anchors the displayed month when focusDate changes", () => {
    const { rerender } = render(
      <ScheduleCalendarGrid
        groups={[]}
        userTeamId="team-1"
        today="2026-08-10"
        focusDate="2026-08-15"
        onSelectDate={() => {}}
      />,
    );
    expect(screen.getByText(/August 2026/)).toBeInTheDocument();

    rerender(
      <ScheduleCalendarGrid
        groups={[]}
        userTeamId="team-1"
        today="2026-08-30"
        focusDate="2026-09-12"
        onSelectDate={() => {}}
      />,
    );
    expect(screen.getByText(/September 2026/)).toBeInTheDocument();
  });

  it("keeps the month stable when focusDate is unchanged", () => {
    const { rerender } = render(
      <ScheduleCalendarGrid
        groups={[]}
        userTeamId="team-1"
        today="2026-08-10"
        focusDate="2026-08-15"
        onSelectDate={() => {}}
      />,
    );

    rerender(
      <ScheduleCalendarGrid
        groups={[]}
        userTeamId="team-1"
        today="2026-08-10"
        focusDate="2026-08-15"
        onSelectDate={() => {}}
      />,
    );
    expect(screen.getByText(/August 2026/)).toBeInTheDocument();
  });
});
