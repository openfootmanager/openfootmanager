import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import TournamentsViewTabs from "./TournamentsViewTabs";

vi.mock("react-i18next", async () => {
  const { useRef } = await import("react");
  return {
    useTranslation: () => {
      useRef(null);
      return { t: (key: string) => key };
    },
  };
});

function renderTabs(
  props: Partial<React.ComponentProps<typeof TournamentsViewTabs>> = {},
) {
  return render(
    <TournamentsViewTabs
      view="overview"
      onSelectView={vi.fn()}
      isKnockout={false}
      {...props}
    />,
  );
}

describe("TournamentsViewTabs", () => {
  // The tabs are a list now rather than a chain of ternaries, so the order is
  // data. It is also what a player's muscle memory reaches for.
  it("keeps the tabs in order", () => {
    renderTabs();

    expect(
      screen.getAllByRole("button").map((button) => button.textContent),
    ).toEqual([
      "tournaments.overview",
      "schedule.standings",
      "schedule.fixtures",
      "tournaments.awardsTab",
    ]);
  });

  it("calls the standings tab a bracket in a knockout competition", () => {
    renderTabs({ isKnockout: true });

    expect(
      screen.getAllByRole("button").map((button) => button.textContent),
    ).toEqual([
      "tournaments.overview",
      "tournaments.bracket",
      "schedule.fixtures",
      "tournaments.awardsTab",
    ]);
  });

  it("marks the tab currently being shown", () => {
    renderTabs({ view: "fixtures" });

    expect(
      screen.getByRole("button", { name: "schedule.fixtures" }).className,
    ).toContain("bg-primary-500");
    expect(
      screen.getByRole("button", { name: "tournaments.overview" }).className,
    ).not.toContain("bg-primary-500");
  });

  it("switches view when a tab is picked", () => {
    const onSelectView = vi.fn();
    renderTabs({ onSelectView });

    fireEvent.click(screen.getByRole("button", { name: "tournaments.awardsTab" }));

    expect(onSelectView).toHaveBeenCalledWith("awards");
  });
});
